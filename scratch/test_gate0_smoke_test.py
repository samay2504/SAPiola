#!/usr/bin/env python3
"""
Gate 0 Smoke Test for SAP HANA Cloud XSUAA / Service-Key Authentication
"""

import json
import os
import sys
import requests
from hdbcli import dbapi

SERVICE_KEY_PATH = os.environ.get("SAPIOLA_HANA_SERVICE_KEY", r"d:\Projects2.0\SAPiola\sapiola-dev-key.json")

def run_smoke_test():
    print(f"Reading service key from: {SERVICE_KEY_PATH}")
    if not os.path.exists(SERVICE_KEY_PATH):
        print(f"ERROR: Service key file not found at {SERVICE_KEY_PATH}")
        sys.exit(1)
        
    with open(SERVICE_KEY_PATH, "r") as f:
        key_data = json.load(f)
        
    host = key_data.get("host")
    port = int(key_data.get("port", 443))
    uaa = key_data.get("uaa", {})
    client_id = uaa.get("clientid")
    client_secret = uaa.get("clientsecret")
    uaa_url = uaa.get("url")
    
    print(f"HANA Host: {host}:{port}")
    print(f"XSUAA Token URL: {uaa_url}/oauth/token")
    print(f"Client ID: {client_id[:20]}...")
    
    # 1. Obtain XSUAA Access Token via client_credentials flow
    token_endpoint = f"{uaa_url}/oauth/token"
    payload = {
        "grant_type": "client_credentials"
    }
    
    print("Requesting OAuth access token from XSUAA...")
    resp = requests.post(token_endpoint, data=payload, auth=(client_id, client_secret), timeout=15)
    
    if resp.status_code != 200:
        print(f"ERROR: Token fetch failed with HTTP {resp.status_code}: {resp.text}")
        sys.exit(1)
        
    token_data = resp.json()
    access_token = token_data.get("access_token")
    token_type = token_data.get("token_type")
    expires_in = token_data.get("expires_in")
    
    print(f"Successfully obtained XSUAA token! (type={token_type}, expires_in={expires_in}s)")
    
    # 2. Attempt HANA SQL Connection using token
    print("\nAttempting connection to SAP HANA Cloud with JWT/OAuth token...")
    
    conn = None
    errors = []
    
    # Method A: userJWT parameter
    try:
        print("  -> Trying dbapi.connect(address, port, userJWT=access_token, encrypt=True)...")
        conn = dbapi.connect(
            address=host,
            port=port,
            userJWT=access_token,
            encrypt=True,
            sslValidateCertificate=True
        )
        print("     [SUCCESS] Connected via userJWT!")
    except Exception as e:
        print(f"     [FAILED] userJWT method: {e}")
        errors.append(("userJWT", e))
        
    # Method B: token parameter (if A failed)
    if not conn:
        try:
            print("  -> Trying dbapi.connect(address, port, token=access_token, encrypt=True)...")
            conn = dbapi.connect(
                address=host,
                port=port,
                token=access_token,
                encrypt=True,
                sslValidateCertificate=True
            )
            print("     [SUCCESS] Connected via token!")
        except Exception as e:
            print(f"     [FAILED] token method: {e}")
            errors.append(("token", e))

    # Method C: password=access_token (if previous failed)
    if not conn:
        try:
            print("  -> Trying dbapi.connect(address, port, password=access_token, encrypt=True)...")
            conn = dbapi.connect(
                address=host,
                port=port,
                password=access_token,
                encrypt=True,
                sslValidateCertificate=True
            )
            print("     [SUCCESS] Connected via password=access_token!")
        except Exception as e:
            print(f"     [FAILED] password=access_token method: {e}")
            errors.append(("password=token", e))

    if not conn:
        print("\n=======================================================")
        print("GATE 0 SMOKE TEST RESULT: FAILED")
        print("HANA rejected the XSUAA access token across all tested connection modes.")
        print("Detailed errors:")
        for mode, err in errors:
            print(f"  - {mode}: {err}")
        print("=======================================================")
        sys.exit(1)
        
    # 3. Run SELECT 1 FROM DUMMY
    print("\nExecuting query: SELECT 1 FROM DUMMY...")
    cursor = conn.cursor()
    cursor.execute("SELECT 1 FROM DUMMY")
    row = cursor.fetchone()
    print(f"Result: {row}")
    cursor.close()
    conn.close()
    
    print("\n=======================================================")
    print("GATE 0 SMOKE TEST RESULT: PASSED 100% SUCCESS")
    print("XSUAA OAuth token authenticated successfully against real SAP HANA Cloud!")
    print("=======================================================")

if __name__ == "__main__":
    run_smoke_test()
