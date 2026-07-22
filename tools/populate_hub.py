#!/usr/bin/env python3
"""
SAPiola Hub-Spoke Graph Population Tool

Populates the SAPiola graph engine via sap-ai-gateway with a hub vertex (node 1000)
and 50 spoke nodes (1001-1050) connected to it for adversarial BFS testing.
"""

import os
import requests
import sys

GATEWAY_URL = os.environ.get("GATEWAY_URL", "http://localhost:8000")
PRINCIPAL = os.environ.get("PRINCIPAL", "alice")
TENANT = os.environ.get("TENANT", "test_tenant")

def populate():
    print(f"Connecting to SAPiola AI Gateway at {GATEWAY_URL}...")
    
    # 1. Create Hub Node 1000
    hub_res = requests.post(f"{GATEWAY_URL}/write/vertex", json={
        "tenant_id": TENANT,
        "principal": PRINCIPAL,
        "node_id": 1000,
        "properties": {"name": "Hub Node", "type": "PLANT"}
    })
    print(f"Hub Node 1000 status: {hub_res.status_code}")

    # 2. Create 50 Spoke Nodes & Edges
    for i in range(1001, 1051):
        v_res = requests.post(f"{GATEWAY_URL}/write/vertex", json={
            "tenant_id": TENANT,
            "principal": PRINCIPAL,
            "node_id": i,
            "properties": {"name": f"Spoke {i}", "type": "MATERIAL"}
        })
        e_res = requests.post(f"{GATEWAY_URL}/write/edge", json={
            "tenant_id": TENANT,
            "principal": PRINCIPAL,
            "source_id": 1000,
            "target_id": i,
            "properties": {"relation": "SUPPLIES"}
        })

    print("Successfully populated 1 Hub node (1000) and 50 Spoke nodes (1001-1050).")

if __name__ == "__main__":
    populate()
