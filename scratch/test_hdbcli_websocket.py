#!/usr/bin/env python3
import json
import requests
from hdbcli import dbapi

with open(r"d:\Projects2.0\SAPiola\sapiola-dev-key.json", "r") as f:
    key_data = json.load(f)

host = key_data.get("host")
port = int(key_data.get("port", 443))
jdbc_url = key_data.get("url")
uaa = key_data.get("uaa", {})
client_id = uaa.get("clientid")
client_secret = uaa.get("clientsecret")
uaa_url = uaa.get("url")

resp = requests.post(f"{uaa_url}/oauth/token", data={"grant_type": "client_credentials"}, auth=(client_id, client_secret))
token = resp.json().get("access_token")

print("Token acquired.")

# Test options with connection string or websocket
test_configs = [
    # 1. userJWT + websocketURL
    {"address": host, "port": port, "userJWT": token, "encrypt": True, "websocketURL": f"/ws/v1"},
    # 2. userJWT + websocket
    {"address": host, "port": port, "userJWT": token, "encrypt": True, "websocket": True},
    # 3. connection string
    {"connectionString": f"jdbc:sap://{host}:{port}?encrypt=true", "userJWT": token},
]

for idx, config in enumerate(test_configs):
    print(f"\n--- Testing Config {idx+1}: {config} ---")
    try:
        conn = dbapi.connect(**config)
        print("SUCCESS!")
        cursor = conn.cursor()
        cursor.execute("SELECT 1 FROM DUMMY")
        print("Query Result:", cursor.fetchone())
        cursor.close()
        conn.close()
        break
    except Exception as e:
        print("FAILED:", e)
