#!/usr/bin/env python3
import json
import os
import sys
from hdbcli import dbapi

with open(r"d:\Projects2.0\SAPiola\sapiola-dev-key.json", "r") as f:
    key_data = json.load(f)

host = key_data.get("host")
port = int(key_data.get("port", 443))

user = "DBADMIN"
password = "Pointbreak2504"

print(f"Connecting to SAP HANA Cloud...")
print(f"Host: {host}:{port}")
print(f"User: {user}\n")

try:
    conn = dbapi.connect(
        address=host,
        port=port,
        user=user,
        password=password,
        encrypt=True,
        sslValidateCertificate=False
    )
    print("=======================================================")
    print("SUCCESSFULLY CONNECTED TO REAL SAP HANA CLOUD!")
    print("=======================================================")
    
    cur = conn.cursor()
    
    # 1. SELECT 1 FROM DUMMY
    cur.execute("SELECT 1 FROM DUMMY")
    print("Query Result (SELECT 1 FROM DUMMY):", cur.fetchone())
    
    # 2. Database Version
    cur.execute("SELECT VERSION FROM SYS.M_DATABASE")
    print("HANA Database Version:", cur.fetchone()[0])
    
    # 3. Current User & Schema
    cur.execute("SELECT CURRENT_USER, CURRENT_SCHEMA FROM DUMMY")
    user_schema = cur.fetchone()
    print(f"Current User: {user_schema[0]}, Current Schema: {user_schema[1]}")
    
    # 4. Count tables in SYS.TABLES
    cur.execute("SELECT COUNT(*) FROM SYS.TABLES")
    table_count = cur.fetchone()[0]
    print(f"Total System & User Tables in Catalog: {table_count}")
    
    cur.close()
    conn.close()
    print("\nGATE 0 LIVE SMOKE TEST: PASSED 100% GREEN!")
except Exception as e:
    print(f"\n[ERROR] Connection failed: {e}")
    sys.exit(1)
