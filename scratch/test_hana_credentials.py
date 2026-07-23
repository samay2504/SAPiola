#!/usr/bin/env python3
import json
import os
import sys
from hdbcli import dbapi

with open(r"d:\Projects2.0\SAPiola\sapiola-dev-key.json", "r") as f:
    key_data = json.load(f)

host = key_data.get("host")
port = int(key_data.get("port", 443))

user = os.environ.get("SAPIOLA_HANA_USER", "DBADMIN")
password = os.environ.get("SAPIOLA_HANA_PASSWORD", "")

print(f"Target: {host}:{port}")
print(f"User: {user}")

if not password:
    print("SAPIOLA_HANA_PASSWORD not set. Testing common variations...")

# Let's test different parameter sets in hdbcli for SAP HANA Cloud
test_params = [
    # Basic SSL
    {"address": host, "port": port, "user": user, "password": password, "encrypt": True},
    {"address": host, "port": port, "user": user, "password": password, "encrypt": True, "sslValidateCertificate": False},
    {"address": host, "port": port, "user": user, "password": password, "encrypt": True, "sslValidateCertificate": True},
    {"address": host, "port": port, "user": user, "password": password, "encrypt": True, "sslHostNameInCertificate": host},
]

for idx, p in enumerate(test_params):
    if not p["password"]:
        continue
    print(f"\nTesting combination {idx+1}...")
    try:
        conn = dbapi.connect(**p)
        print("SUCCESS! Connected.")
        cur = conn.cursor()
        cur.execute("SELECT 1 FROM DUMMY")
        print("Result:", cur.fetchone())
        cur.close()
        conn.close()
        break
    except Exception as e:
        print(f"FAILED: {e}")
