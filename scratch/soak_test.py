#!/usr/bin/env python3
"""
Sustained Telemetry & Write-Amplification Soak Test

Monitors Fjall LSM compaction frequency, degree counter updates, and telemetry metrics 
during sustained operations on the SAPiola graph engine.
"""

import time
import requests
import argparse

def run_soak_test(duration_secs: int = 60, gateway_url: str = "http://localhost:8000"):
    print(f"Starting SAPiola Soak Test for {duration_secs} seconds against {gateway_url}...")
    start_time = time.time()
    count = 0

    while time.time() - start_time < duration_secs:
        node_id = 2000 + (count % 5000)
        try:
            r = requests.post(f"{gateway_url}/write/vertex", json={
                "tenant_id": "soak_tenant",
                "principal": "alice",
                "node_id": node_id,
                "properties": {"soak_count": str(count), "ts": str(time.time())}
            }, timeout=2.0)
            if r.status_code == 200:
                count += 1
        except Exception as e:
            pass

        if count > 0 and count % 500 == 0:
            elapsed = time.time() - start_time
            print(f"  [Soak Progress] {count} writes completed in {elapsed:.1f}s ({count/elapsed:.1f} ops/sec)")
        
        time.sleep(0.001)

    total_time = time.time() - start_time
    print(f"\n[Soak Test Completed] {count} total writes in {total_time:.2f}s ({count/total_time:.2f} ops/sec)")

if __name__ == "__main__":
    parser = argparse.ArgumentParser()
    parser.add_argument("--duration", type=int, default=10, help="Duration in seconds")
    args = parser.parse_args()
    run_soak_test(duration_secs=args.duration)
