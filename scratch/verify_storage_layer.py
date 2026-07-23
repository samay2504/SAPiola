#!/usr/bin/env python3
"""
Step 2: Storage-Layer Isolation & Disk Footprint Verification (Zero LLM)
Populates poly-lsm-core (Fjall engine), checks disk footprint, queries gRPC StorageService directly.
"""

import os
import sys
import json
import asyncio
import pathlib
import grpc

gateway_path = pathlib.Path(__file__).parent.parent / "sap-ai-gateway"
sys.path.insert(0, str(gateway_path))

# Add gen to sys.path
gen_path = gateway_path / "sapiola_ai" / "gen"
sys.path.insert(0, str(gen_path))

from sapiola.v1 import storage_pb2
from sapiola.v1 import storage_pb2_grpc

async def main():
    target = os.environ.get("SAPIOLA_GRAPH_GRPC_TARGET", "127.0.0.1:50053")
    db_path = os.environ.get("LSM_DB_PATH", "./data/lsm_stress_db")

    print(f"[Step 2] Connecting directly to sap-graph-server gRPC StorageService at {target}...")
    channel = grpc.aio.insecure_channel(target)
    stub = storage_pb2_grpc.StorageServiceStub(channel)

    tenant_metadata = (("tenant-id", "default"),)

    # 1. Ingest Vertices into PolyLsmEngine
    print("  -> Ingesting ground-truth graph vertices into poly-lsm-core...")
    nodes = [
        (5001, {"label": "SalesOrder", "vbeln": "DOC-5001", "netwr": "12500.00", "waerk": "EUR", "erdat": "2026-07-23", "domain": "procurement"}),
        (9001, {"label": "Customer", "kunnr": "CUST-9001", "name1": "Siemens Enterprise Automation", "ort01": "Munich", "domain": "procurement"}),
        (7700, {"label": "Material", "matnr": "MAT-7700", "arktx": "High-Pressure Turbine Valve", "netwr": "8500.00", "kwmeng": "10.0", "domain": "inventory"}),
        (8811, {"label": "Material", "matnr": "MAT-8811", "arktx": "Precision Control Module", "netwr": "4000.00", "kwmeng": "5.0", "domain": "inventory"}),
    ]

    for node_id, props in nodes:
        req = storage_pb2.PutVertexRequest(node_id=node_id, properties=props)
        resp = await stub.PutVertex(req, metadata=tenant_metadata)
        assert resp.success, f"Failed to put vertex {node_id}"

    # 2. Ingest Edges into PolyLsmEngine
    print("  -> Ingesting graph edges (PLACED_BY, CONTAINS_ITEM)...")
    edges = [
        (5001, 9001), # SalesOrder -> Customer (PLACED_BY)
        (5001, 7700), # SalesOrder -> Material 7700 (CONTAINS_ITEM)
        (5001, 8811), # SalesOrder -> Material 8811 (CONTAINS_ITEM)
    ]
    for src, dst in edges:
        req = storage_pb2.PutEdgeRequest(source_id=src, target_id=dst)
        resp = await stub.PutEdge(req, metadata=tenant_metadata)
        assert resp.success, f"Failed to put edge {src}->{dst}"

    # 3. Direct gRPC Query: GetVertex(5001) & GetNeighbors(5001) - Zero LLM
    print("  -> Executing direct gRPC GetVertex & GetNeighbors for SalesOrder node_id=5001...")
    get_v_req = storage_pb2.GetVertexRequest(node_id=5001)
    get_v_resp = await stub.GetVertex(get_v_req, metadata=tenant_metadata)
    
    print("     GetVertex Response Properties:")
    for k, v in get_v_resp.properties.items():
        print(f"       * {k}: {v}")

    assert get_v_resp.properties["vbeln"] == "DOC-5001"
    assert get_v_resp.properties["netwr"] == "12500.00"

    get_n_req = storage_pb2.GetNeighborsRequest(node_id=5001)
    get_n_resp = await stub.GetNeighbors(get_n_req, metadata=tenant_metadata)

    out_edges = list(get_n_resp.out_edges)
    print(f"     GetNeighbors Out-Edges: {out_edges}")
    assert set(out_edges) == {9001, 7700, 8811}, f"Unexpected out_edges: {out_edges}"

    # 4. Check Disk Footprint of Fjall Engine
    print("  -> Auditing Fjall LSM engine footprint on disk...")
    p = pathlib.Path(db_path)
    if not p.exists():
        p = pathlib.Path("./data/lsm_db") # fallback default
    assert p.exists(), f"Fjall database directory {p} does not exist!"
    
    files = list(p.glob("**/*"))
    total_bytes = sum(f.stat().st_size for f in files if f.is_file())
    print(f"     Fjall Disk Path: {p.absolute()}")
    print(f"     Total Files: {len(files)}, Total Size: {total_bytes} bytes")
    assert total_bytes > 0, "Fjall database files are empty (0 bytes)!"

    # 5. Compare Raw Storage Results against Ground-Truth Baseline
    with open("scratch/ground_truth_doc5001.txt", "r", encoding="utf-8") as f:
        gt = json.load(f)

    # Format Normalization Rules Validation
    assert get_v_resp.properties["vbeln"].strip().upper() == gt["sales_order"]["vbeln"].strip().upper()
    assert float(get_v_resp.properties["netwr"]) == float(gt["sales_order"]["netwr"])
    assert get_v_resp.properties["erdat"] == gt["sales_order"]["erdat"]

    await channel.close()
    print("\n[SUCCESS] Step 2 Storage-Layer Verification & Disk Audit PASSED 100%!")

if __name__ == "__main__":
    asyncio.run(main())
