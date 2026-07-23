#!/usr/bin/env python3
import json
import os
import requests
from hdbcli import dbapi

with open(r"d:\Projects2.0\SAPiola\sapiola-dev-key.json", "r") as f:
    key_data = json.load(f)

host = key_data.get("host")
port = int(key_data.get("port", 443))
uaa = key_data.get("uaa", {})
client_id = uaa.get("clientid")
client_secret = uaa.get("clientsecret")
uaa_url = uaa.get("url")

resp = requests.post(f"{uaa_url}/oauth/token", data={"grant_type": "client_credentials"}, auth=(client_id, client_secret))
token = resp.json().get("access_token")

print("Host:", host)
print("Token acquired successfully.")

# Test combinations
combos = [
    {"userJWT": token, "sslValidateCertificate": False},
    {"userJWT": token, "encrypt": "true"},
    {"token": token, "sslValidateCertificate": False},
    {"user": "DBADMIN", "password": "...", "encrypt": "true"},
]

for idx, opts in enumerate(combos):
    print(f"\n--- Testing Option {idx+1}: {opts.keys()} ---")
    try:
        conn = dbapi.connect(address=host, port=port, **opts)
        print("SUCCESS!")
        cursor = conn.cursor()
        cursor.execute("SELECT 1 FROM DUMMY")
        print("Query result:", cursor.fetchone())
        conn.close()
    except Exception as e:
        print("FAILED:", e)
