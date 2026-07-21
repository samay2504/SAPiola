import os
import sys
import argparse
import grpc
from datasets import load_dataset
import datetime

# Add the gen directory to PYTHONPATH so we can import the stubs
sys.path.insert(0, os.path.join(os.path.dirname(__file__), "gen"))

from sapiola.v1 import source_pb2
from sapiola.v1 import source_pb2_grpc
from sapiola.v1 import cdc_pb2
from sapiola.v1 import common_pb2

DATASET_NAME = "sap-ai-research/SALT"

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
                mappings[table] = pk
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

def synthesize_cdc_events(table_name: str, ds, pk_col: str):
    """Generator that yields CdcEvent protobufs from a huggingface dataset split."""
    lsn = 1
    for row in ds:
        # Construct primary key string. 
        # For salesdocument_items it needs a composite key since UNKNOWN_ID was found.
        # We will handle that edge case here.
        if pk_col == "UNKNOWN_ID":
            # For SALT items, the PK is SALESDOCUMENT + SALESDOCUMENTITEM
            pk_val = str(row.get("SALESDOCUMENT", "")) + "|" + str(row.get("SALESDOCUMENTITEM", ""))
        else:
            pk_val = str(row.get(pk_col, ""))
            
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
    token = os.environ.get("HF_TOKEN")
    if not token:
        print("Error: HF_TOKEN environment variable is required.")
        sys.exit(1)
        
    grpc_target = os.environ.get("GRPC_ADDR", "localhost:50051")
    dsl_path = "salt_mapping.dsl"
    
    if not os.path.exists(dsl_path):
        print(f"Error: {dsl_path} not found. Run the introspector first.")
        sys.exit(1)
        
    mappings = load_dsl_mapping(dsl_path)
    
    print(f"Connecting to Ingest Gateway at {grpc_target}...")
    channel = grpc.insecure_channel(grpc_target)
    stub = source_pb2_grpc.CdcIngestStub(channel)
    
    for table_name, pk_col in mappings.items():
        print(f"Streaming {table_name} (PK: {pk_col})...")
        ds = load_dataset(DATASET_NAME, table_name, split="train", streaming=True, token=token)
        
        event_stream = synthesize_cdc_events(table_name, ds, pk_col)
        
        try:
            response = stub.PublishEventsStream(event_stream)
            print(f" -> Successfully streamed {response.processed_count} events for {table_name}.")
        except grpc.RpcError as e:
            print(f" -> gRPC Error while streaming {table_name}: {e}")

if __name__ == "__main__":
    main()
