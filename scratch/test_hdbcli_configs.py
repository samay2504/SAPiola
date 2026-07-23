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

test_configs = [
    # Config 1: userJWT with encrypt True
    {"address": host, "port": port, "userJWT": token, "encrypt": True, "sslValidateCertificate": True},
    # Config 2: userJWT without certificate validation
    {"address": host, "port": port, "userJWT": token, "encrypt": True, "sslValidateCertificate": False},
    # Config 3: userJWT with sslHostNameInCertificate
    {"address": host, "port": port, "userJWT": token, "encrypt": True, "sslValidateCertificate": True, "sslHostNameInCertificate": host},
    # Config 4: token parameter
    {"address": host, "port": port, "token": token, "encrypt": True, "sslValidateCertificate": False},
    # Config 5: clientJWT parameter
    {"address": host, "port": port, "clientJWT": token, "encrypt": True, "sslValidateCertificate": False},
    # Config 6: assertion parameter
    {"address": host, "port": port, "assertion": token, "encrypt": True, "sslValidateCertificate": False},
]

for idx, config in enumerate(test_configs):
    print(f"\n--- Testing Config {idx+1}: {config.keys()} ---")
    try:
        conn = dbapi.connect(**config)
        print("SUCCESS! Connected.")
        cursor = conn.cursor()
        cursor.execute("SELECT 1 FROM DUMMY")
        print("Query Result:", cursor.fetchone())
        cursor.close()
        conn.close()
        break
    except Exception as e:
        print("FAILED:", e)
