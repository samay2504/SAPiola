#!/usr/bin/env python3
"""
Step 7: Cross-Schema Mandatory Gating Test

Tests dynamic schema discovery against a COMPLETELY DIFFERENT SAP domain
(Procurement / MM: LFA1_VENDOR_TEST, EKKO_PO_TEST, EKPO_POITEM_TEST)
with zero code changes.
"""

import os
import sys
import json
import pathlib
import subprocess
from hdbcli import dbapi

tools_path = pathlib.Path(__file__).parent.parent / "tools" / "salt_importer"
sys.path.insert(0, str(tools_path))

from credential_resolver import resolve_hana_credentials
from hana_introspector import HanaIntrospector


def main():
    creds = resolve_hana_credentials()
    schema = os.environ.get("SAPIOLA_HANA_SCHEMA", "DBADMIN")

    print(f"[Step 7 Gating Test] Connecting to SAP HANA Cloud ({creds['host']}:{creds['port']} as {creds['user']})...")
    conn = dbapi.connect(
        address=creds["host"],
        port=int(creds["port"]),
        user=creds["user"],
        password=creds["password"],
        encrypt=True,
        sslValidateCertificate=False
    )
    cursor = conn.cursor()

    # Drop old test tables if exist
    for tbl in ["EKPO_POITEM_TEST", "EKKO_PO_TEST", "LFA1_VENDOR_TEST"]:
        try:
            cursor.execute(f'DROP TABLE "{schema}"."{tbl}"')
        except Exception:
            pass

    # 1. Vendor Master Table (LFA1)
    print("1. Creating LFA1_VENDOR_TEST...")
    cursor.execute(f"""
        CREATE TABLE "{schema}"."LFA1_VENDOR_TEST" (
            LIFNR VARCHAR(10) PRIMARY KEY,
            NAME1 VARCHAR(35),
            LAND1 VARCHAR(3)
        )
    """)
    vendors = [
        ("VEND_001", "Global Industrial Supplies", "DE"),
        ("VEND_002", "Acme Logistics Corp", "US"),
        ("VEND_003", "Tokyo Heavy Industries", "JP"),
    ]
    for v in vendors:
        cursor.execute(f'INSERT INTO "{schema}"."LFA1_VENDOR_TEST" VALUES (?, ?, ?)', v)

    # 2. Purchase Order Header Table (EKKO)
    print("2. Creating EKKO_PO_TEST...")
    cursor.execute(f"""
        CREATE TABLE "{schema}"."EKKO_PO_TEST" (
            EBELN VARCHAR(10) PRIMARY KEY,
            LIFNR VARCHAR(10),
            BEDAT VARCHAR(8),
            NETWR DECIMAL(15,2)
        )
    """)
    pos = [
        ("4500000001", "VEND_001", "20260115", 12500.00),
        ("4500000002", "VEND_002", "20260118", 8750.50),
        ("4500000003", "VEND_001", "20260120", 43000.00),
    ]
    for p in pos:
        cursor.execute(f'INSERT INTO "{schema}"."EKKO_PO_TEST" VALUES (?, ?, ?, ?)', p)

    # 3. Purchase Order Item Table (EKPO) - Composite PK
    print("3. Creating EKPO_POITEM_TEST...")
    cursor.execute(f"""
        CREATE TABLE "{schema}"."EKPO_POITEM_TEST" (
            EBELN VARCHAR(10),
            EBELP VARCHAR(5),
            MATNR VARCHAR(18),
            WERKS VARCHAR(4),
            MENGE DECIMAL(13,3),
            PRIMARY KEY (EBELN, EBELP)
        )
    """)
    po_items = [
        ("4500000001", "00010", "STEEL_PIPE_01", "1000", 50.0),
        ("4500000001", "00020", "VALVE_FLANGE_02", "1000", 120.0),
        ("4500000002", "00010", "PALLET_PLASTIC", "2000", 200.0),
        ("4500000003", "00010", "TURBINE_BLADE", "1000", 5.0),
    ]
    for item in po_items:
        cursor.execute(f'INSERT INTO "{schema}"."EKPO_POITEM_TEST" VALUES (?, ?, ?, ?, ?)', item)

    cursor.close()
    conn.close()

    # 4. Run Introspector against the new domain with ZERO code changes
    print("\n4. Running Schema Discovery Engine on Procurement domain (%_TEST)...")
    introspector = HanaIntrospector.from_credentials(creds)
    output_dir = pathlib.Path(__file__).parent / "procurement_schema"
    output_dir.mkdir(exist_ok=True)

    manifest = introspector.generate_manifest(
        schema_name=schema,
        table_filter="%_TEST",
        min_fk_confidence=0.8,
        output_dir=str(output_dir)
    )

    # 5. Assertions on Discovered Manifest
    print("\n5. Validating Manifest Integrity...")
    tables = manifest["tables"]
    assert "LFA1_VENDOR_TEST" in tables, "LFA1_VENDOR_TEST missing from discovery"
    assert "EKKO_PO_TEST" in tables, "EKKO_PO_TEST missing from discovery"
    assert "EKPO_POITEM_TEST" in tables, "EKPO_POITEM_TEST missing from discovery"

    assert tables["LFA1_VENDOR_TEST"]["primary_keys"] == ["LIFNR"]
    assert tables["EKKO_PO_TEST"]["primary_keys"] == ["EBELN"]
    assert set(tables["EKPO_POITEM_TEST"]["primary_keys"]) == {"EBELN", "EBELP"}

    rels = manifest["relationships"]
    rel_map = {(r["from_table"], r["from_col"], r["to_table"], r["to_col"]): r["confidence"] for r in rels}

    print("Discovered Relationships:")
    for k, v in rel_map.items():
        print(f"  {k[0]}.{k[1]} -> {k[2]}.{k[3]} (Confidence: {v})")

    assert ("EKKO_PO_TEST", "LIFNR", "LFA1_VENDOR_TEST", "LIFNR") in rel_map, "EKKO -> LFA1 FK missing"
    assert ("EKPO_POITEM_TEST", "EBELN", "EKKO_PO_TEST", "EBELN") in rel_map, "EKPO -> EKKO FK missing"

    print("\n==========================================================================")
    print("MANDATORY CROSS-SCHEMA GATING TEST: PASSED 100%")
    print(f"Discovered Domain: Procurement (3 tables: LFA1_VENDOR_TEST, EKKO_PO_TEST, EKPO_POITEM_TEST)")
    print(f"Fingerprint: {manifest['fingerprint']}")
    print("Zero internal code changes were required to onboard this new SAP domain!")
    print("==========================================================================")


if __name__ == "__main__":
    main()
