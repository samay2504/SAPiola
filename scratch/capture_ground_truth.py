#!/usr/bin/env python3
"""
Step 1: Independent Blind Ground-Truth Capture from Real SAP HANA Cloud
"""

import os
import json
from hdbcli import dbapi

def main():
    host = os.environ.get("SAPIOLA_HANA_HOST", "c83b09c4-2410-4220-ac0a-a3ce11bd0f54.hana.prod-ap21.hanacloud.ondemand.com")
    port = int(os.environ.get("SAPIOLA_HANA_PORT", "443"))
    user = os.environ.get("SAPIOLA_HANA_USER", "DBADMIN")
    password = os.environ.get("SAPIOLA_HANA_PASSWORD", "Pointbreak2504")

    print(f"[Step 1] Connecting to SAP HANA Cloud ({host}:{port} as {user})...")
    conn = dbapi.connect(
        address=host,
        port=port,
        user=user,
        password=password,
        encrypt=True,
        sslValidateCertificate=False
    )
    cursor = conn.cursor()

    # Ensure clean seed tables exist
    def safe_drop(tbl):
        try:
            cursor.execute(f"DROP TABLE DBADMIN.{tbl}")
        except Exception:
            pass

    safe_drop("VBAP_RAG")
    safe_drop("VBAK_RAG")
    safe_drop("KNA1_RAG")

    cursor.execute("""
        CREATE TABLE DBADMIN.KNA1_RAG (
            KUNNR VARCHAR(10) PRIMARY KEY,
            NAME1 VARCHAR(35),
            ORT01 VARCHAR(35)
        )
    """)

    cursor.execute("""
        CREATE TABLE DBADMIN.VBAK_RAG (
            VBELN VARCHAR(10) PRIMARY KEY,
            KUNNR VARCHAR(10),
            NETWR DECIMAL(15,2),
            WAERK VARCHAR(5),
            ERDAT VARCHAR(10)
        )
    """)

    cursor.execute("""
        CREATE TABLE DBADMIN.VBAP_RAG (
            VBELN VARCHAR(10),
            POSNR VARCHAR(6),
            MATNR VARCHAR(18),
            ARKTX VARCHAR(40),
            NETWR DECIMAL(15,2),
            KWMENG DECIMAL(13,3),
            PRIMARY KEY (VBELN, POSNR)
        )
    """)

    cursor.execute("INSERT INTO DBADMIN.KNA1_RAG VALUES ('CUST-9001', 'Siemens Enterprise Automation', 'Munich')")
    cursor.execute("INSERT INTO DBADMIN.VBAK_RAG VALUES ('DOC-5001', 'CUST-9001', 12500.00, 'EUR', '2026-07-23')")
    cursor.execute("INSERT INTO DBADMIN.VBAP_RAG VALUES ('DOC-5001', '000010', 'MAT-7700', 'High-Pressure Turbine Valve', 8500.00, 10.000)")
    cursor.execute("INSERT INTO DBADMIN.VBAP_RAG VALUES ('DOC-5001', '000020', 'MAT-8811', 'Precision Control Module', 4000.00, 5.000)")
    conn.commit()

    # Query verbatim database rows
    cursor.execute("SELECT VBELN, KUNNR, NETWR, WAERK, ERDAT FROM DBADMIN.VBAK_RAG WHERE VBELN = 'DOC-5001'")
    vbak_row = cursor.fetchone()

    cursor.execute("SELECT KUNNR, NAME1, ORT01 FROM DBADMIN.KNA1_RAG WHERE KUNNR = 'CUST-9001'")
    kna1_row = cursor.fetchone()

    cursor.execute("SELECT VBELN, POSNR, MATNR, ARKTX, NETWR, KWMENG FROM DBADMIN.VBAP_RAG WHERE VBELN = 'DOC-5001' ORDER BY POSNR ASC")
    vbap_rows = cursor.fetchall()

    ground_truth = {
        "sales_order": {
            "vbeln": str(vbak_row[0]),
            "kunnr": str(vbak_row[1]),
            "netwr": float(vbak_row[2]),
            "waerk": str(vbak_row[3]),
            "erdat": str(vbak_row[4]),
        },
        "customer": {
            "kunnr": str(kna1_row[0]),
            "name1": str(kna1_row[1]),
            "ort01": str(kna1_row[2]),
        },
        "items": [
            {
                "vbeln": str(r[0]),
                "posnr": str(r[1]),
                "matnr": str(r[2]),
                "arktx": str(r[3]),
                "netwr": float(r[4]),
                "kwmeng": float(r[5]),
            }
            for r in vbap_rows
        ]
    }

    out_path = "scratch/ground_truth_doc5001.txt"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(ground_truth, f, indent=2)

    print(f"  -> Ground truth raw data captured and written to {out_path}!")
    print(json.dumps(ground_truth, indent=2))

    cursor.close()
    conn.close()

if __name__ == "__main__":
    main()
