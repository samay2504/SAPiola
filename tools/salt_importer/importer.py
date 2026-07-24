import os
import sys
import argparse
import json
import grpc
from datasets import load_dataset
import datetime

# Add the gen directory to PYTHONPATH so we can import the stubs
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "gen"))

from sapiola.v1 import source_pb2
from sapiola.v1 import source_pb2_grpc
from sapiola.v1 import cdc_pb2
from sapiola.v1 import common_pb2

def load_dsl_mapping(dsl_path: str) -> dict:
    """Parses the salt_mapping.dsl to extract table -> primary key mappings."""
    mappings = {}
    with open(dsl_path, "r") as f:
        for line in f:
            line = line.strip()
            if line.startswith("NODE"):
                # NODE SalesDocument FROM salesdocuments WITH id = SALESDOCUMENT
                parts = line.split(" ")
                table = parts[3]
                pk = parts[7]
                mappings[table] = [pk]  # Single PK as list
    return mappings


def load_manifest_mapping(manifest_path: str) -> dict:
    """Load primary key mappings from schema_manifest.json."""
    with open(manifest_path, "r") as f:
        manifest = json.load(f)
    mappings = {}
    for table_name, table_info in manifest.get("tables", {}).items():
        pks = table_info.get("primary_keys", [])
        if pks:
            mappings[table_name] = pks
    return mappings

def any_to_value(val) -> common_pb2.Value:
    """Converts a Python primitive to a Protobuf Value oneof."""
    if val is None:
        return common_pb2.Value(string_value="") # Fallback for null
    
    if isinstance(val, int):
        return common_pb2.Value(int_value=val)
    elif isinstance(val, float):
        return common_pb2.Value(double_value=val)
    elif isinstance(val, bool):
        return common_pb2.Value(bool_value=val)
    elif isinstance(val, str):
        return common_pb2.Value(string_value=val)
    else:
        return common_pb2.Value(string_value=str(val))

def synthesize_cdc_events(table_name: str, ds, pk_cols: list):
    """Generator that yields CdcEvent protobufs from a huggingface dataset split."""
    lsn = 1
    for row in ds:
        # Build primary key string from manifest-defined PK columns
        # For composite keys (>1 column), join with '|'
        if len(pk_cols) > 1:
            pk_val = "|".join(str(row.get(col, "")) for col in pk_cols)
        elif len(pk_cols) == 1:
            pk_val = str(row.get(pk_cols[0], ""))
        else:
            pk_val = str(lsn)  # Fallback: use LSN as PK
            
        # Convert row dict to map<string, Value>
        after_vals = {k: any_to_value(v) for k, v in row.items()}
        
        event = cdc_pb2.CdcEvent(
            table=table_name,
            primary_key=pk_val,
            operation=cdc_pb2.OPERATION_INSERT,
            after=after_vals,
            txid="SALT_INIT",
            log_sequence_number=lsn
        )
        # Set timestamp to current time for observability
        event.timestamp.FromDatetime(datetime.datetime.now(datetime.timezone.utc))
        
        lsn += 1
        yield event

def main():
    parser = argparse.ArgumentParser(
        description="SAPiola Data Importer — streams HuggingFace datasets to gRPC ingest gateway"
    )
    parser.add_argument(
        "--dataset", default=os.environ.get("SAPIOLA_HF_DATASET", "sap-ai-research/SALT"),
        help="Hugging Face dataset name"
    )
    parser.add_argument(
        "--manifest", default=None,
        help="Path to schema_manifest.json (preferred over --dsl)"
    )
    parser.add_argument(
        "--dsl", default="salt_mapping.dsl",
        help="Path to mapping DSL file (fallback if no manifest)"
    )
    parser.add_argument(
        "--grpc-target", default=os.environ.get("GRPC_ADDR", "localhost:50051"),
        help="gRPC ingest gateway address"
    )
    args = parser.parse_args()

    token = os.environ.get("HF_TOKEN")
    if not token:
        print("Error: HF_TOKEN environment variable is required.")
        sys.exit(1)

    # Load mappings: prefer manifest, fall back to DSL
    if args.manifest and os.path.exists(args.manifest):
        print(f"Loading mappings from manifest: {args.manifest}")
        mappings = load_manifest_mapping(args.manifest)
    elif os.path.exists(args.dsl):
        print(f"Loading mappings from DSL: {args.dsl}")
        mappings = load_dsl_mapping(args.dsl)
    else:
        print(f"Error: Neither manifest nor DSL file found.")
        sys.exit(1)
    
    print(f"Connecting to Ingest Gateway at {args.grpc_target}...")
    channel = grpc.insecure_channel(args.grpc_target)
    stub = source_pb2_grpc.CdcIngestStub(channel)
    
    for table_name, pk_cols in mappings.items():
        # Normalize pk_cols to list (DSL loader returns list, manifest returns list)
        if isinstance(pk_cols, str):
            pk_cols = [pk_cols]
        print(f"Streaming {table_name} (PK: {pk_cols})...")
        ds = load_dataset(args.dataset, table_name, split="train", streaming=True, token=token)
        
        event_stream = synthesize_cdc_events(table_name, ds, pk_cols)
        
        try:
            response = stub.PublishEventsStream(event_stream)
            print(f" -> Successfully streamed {response.processed_count} events for {table_name}.")
        except grpc.RpcError as e:
            print(f" -> gRPC Error while streaming {table_name}: {e}")

if __name__ == "__main__":
    main()
