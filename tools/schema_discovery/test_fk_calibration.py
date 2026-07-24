#!/usr/bin/env python3
"""
Step 6: Empirical FK Confidence Threshold Calibration Test

Creates temporary tables in SAP HANA Cloud with imperfect value overlap,
scores foreign key confidence using HanaIntrospector.score_foreign_key_confidence(),
and records exact confidence values for calibration.
"""

import os
import sys
from hdbcli import dbapi

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from credential_resolver import resolve_hana_credentials
from hana_introspector import HanaIntrospector


def main():
    creds = resolve_hana_credentials()
    schema = os.environ.get("SAPIOLA_HANA_SCHEMA", "DBADMIN")

    print(f"[Calibration Test] Connecting to SAP HANA Cloud ({creds['host']}:{creds['port']} as {creds['user']})...")
    conn = dbapi.connect(
        address=creds["host"],
        port=int(creds["port"]),
        user=creds["user"],
        password=creds["password"],
        encrypt=True,
        sslValidateCertificate=False
    )
    cursor = conn.cursor()

    # Drop old calibration tables if exist
    for tbl in ["SRC_CALIB_RAG", "TGT_CALIB_RAG"]:
        try:
            cursor.execute(f'DROP TABLE "{schema}"."{tbl}"')
        except Exception:
            pass

    # Create target table (Parent)
    cursor.execute(f"""
        CREATE TABLE "{schema}"."TGT_CALIB_RAG" (
            ID VARCHAR(10) PRIMARY KEY,
            NAME VARCHAR(50)
        )
    """)

    # Create source table (Child referencing Parent.ID)
    cursor.execute(f"""
        CREATE TABLE "{schema}"."SRC_CALIB_RAG" (
            DOC_NO VARCHAR(10) PRIMARY KEY,
            REF_ID VARCHAR(10)
        )
    """)

    # Seed Parent table with 100 rows: TGT_001 to TGT_100
    for i in range(1, 101):
        cursor.execute(
            f'INSERT INTO "{schema}"."TGT_CALIB_RAG" VALUES (?, ?)',
            (f"TGT_{i:03d}", f"Target Row {i}")
        )

    # -------------------------------------------------------------------------
    # Scenario A: 90% overlap (90 matching rows, 10 orphaned rows)
    # -------------------------------------------------------------------------
    print("\n--- Testing Scenario A: 90 matching, 10 orphaned (90.0% expected overlap) ---")
    for i in range(1, 91):
        cursor.execute(
            f'INSERT INTO "{schema}"."SRC_CALIB_RAG" VALUES (?, ?)',
            (f"DOC_{i:03d}", f"TGT_{i:03d}")  # Match
        )
    for i in range(91, 101):
        cursor.execute(
            f'INSERT INTO "{schema}"."SRC_CALIB_RAG" VALUES (?, ?)',
            (f"DOC_{i:03d}", f"ORPHAN_{i:03d}")  # Orphan
        )

    introspector = HanaIntrospector.from_credentials(creds)
    score_a = introspector.score_foreign_key_confidence(
        source_table="SRC_CALIB_RAG",
        source_col="REF_ID",
        target_table="TGT_CALIB_RAG",
        target_col="ID",
        schema_name=schema
    )
    print(f"Scenario A Score recorded: {score_a:.4f} (90/100 = 0.9000)")
    assert round(score_a, 2) == 0.90, f"Expected 0.90, got {score_a}"
    assert score_a >= 0.80, f"Scenario A should pass threshold 0.80"

    # -------------------------------------------------------------------------
    # Scenario B: 69.2% overlap (add 30 more orphaned rows -> 90/130 match)
    # -------------------------------------------------------------------------
    print("\n--- Testing Scenario B: Adding 30 more orphaned rows (69.23% expected overlap) ---")
    for i in range(101, 131):
        cursor.execute(
            f'INSERT INTO "{schema}"."SRC_CALIB_RAG" VALUES (?, ?)',
            (f"DOC_{i:03d}", f"ORPHAN_{i:03d}")  # Orphan
        )

    score_b = introspector.score_foreign_key_confidence(
        source_table="SRC_CALIB_RAG",
        source_col="REF_ID",
        target_table="TGT_CALIB_RAG",
        target_col="ID",
        schema_name=schema
    )
    print(f"Scenario B Score recorded: {score_b:.4f} (90/130 = 0.6923)")
    assert round(score_b, 4) == round(90 / 130, 4), f"Expected 0.6923, got {score_b}"
    assert score_b < 0.80, f"Scenario B should be rejected by threshold 0.80"

    # Clean up calibration tables
    for tbl in ["SRC_CALIB_RAG", "TGT_CALIB_RAG"]:
        try:
            cursor.execute(f'DROP TABLE "{schema}"."{tbl}"')
        except Exception:
            pass

    cursor.close()
    conn.close()

    print("\n==========================================================================")
    print("CALIBRATION RECORD SUMMARY:")
    print(f"  - 90% overlap case score:  {score_a:.4f} -> Accepted by 0.8 threshold")
    print(f"  - 69.2% overlap case score: {score_b:.4f} -> Rejected by 0.8 threshold")
    print("FK threshold 0.8 sensitivity verified successfully against live SAP HANA!")
    print("==========================================================================")


if __name__ == "__main__":
    main()
