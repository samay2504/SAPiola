#!/usr/bin/env python3
"""
Live Integration Tests for Real SAP HANA Catalog Introspector (HanaIntrospector)
"""

import unittest
from hdbcli import dbapi
from hana_introspector import HanaIntrospector

class TestHanaLiveIntrospector(unittest.TestCase):

    def setUp(self):
        self.introspector = HanaIntrospector.from_env()
        self.conn = self.introspector._get_connection()
        self.cursor = self.conn.cursor()

        # Helper for safe drop
        def safe_drop(tbl):
            try:
                self.cursor.execute(f"DROP TABLE DBADMIN.{tbl}")
            except Exception:
                pass

        safe_drop("VBAP")
        safe_drop("VBAK")
        safe_drop("KNA1")

        self.cursor.execute("""
            CREATE TABLE DBADMIN.KNA1 (
                KUNNR VARCHAR(10) PRIMARY KEY,
                NAME1 VARCHAR(35)
            )
        """)

        self.cursor.execute("""
            CREATE TABLE DBADMIN.VBAK (
                VBELN VARCHAR(10) PRIMARY KEY,
                KUNNR VARCHAR(10),
                FOREIGN KEY (KUNNR) REFERENCES DBADMIN.KNA1(KUNNR)
            )
        """)

        self.cursor.execute("""
            CREATE TABLE DBADMIN.VBAP (
                VBELN VARCHAR(10),
                POSNR VARCHAR(6),
                MATNR VARCHAR(18),
                PRIMARY KEY (VBELN, POSNR),
                FOREIGN KEY (VBELN) REFERENCES DBADMIN.VBAK(VBELN)
            )
        """)

        # Insert test records
        self.cursor.execute("INSERT INTO DBADMIN.KNA1 VALUES ('CUST001', 'Acme Corp')")
        self.cursor.execute("INSERT INTO DBADMIN.VBAK VALUES ('DOC1001', 'CUST001')")
        self.cursor.execute("INSERT INTO DBADMIN.VBAP VALUES ('DOC1001', '000010', 'MAT001')")
        self.conn.commit()

    def tearDown(self):
        def safe_drop(tbl):
            try:
                self.cursor.execute(f"DROP TABLE DBADMIN.{tbl}")
            except Exception:
                pass

        safe_drop("VBAP")
        safe_drop("VBAK")
        safe_drop("KNA1")
        self.conn.commit()
        self.cursor.close()
        self.conn.close()

    def test_live_schema_fetch(self):
        schemas = self.introspector.fetch_table_schemas(schema_name="DBADMIN")
        self.assertIn("VBAK", schemas)
        self.assertIn("VBAP", schemas)
        self.assertIn("KNA1", schemas)

        self.assertEqual(schemas["KNA1"], ["KUNNR", "NAME1"])
        self.assertEqual(schemas["VBAK"], ["VBELN", "KUNNR"])
        self.assertEqual(schemas["VBAP"], ["VBELN", "POSNR", "MATNR"])

    def test_live_referential_constraints(self):
        constraints = self.introspector.fetch_referential_constraints(schema_name="DBADMIN")
        self.assertTrue(len(constraints) >= 2)
        tables = [c["table"] for c in constraints]
        self.assertIn("VBAK", tables)
        self.assertIn("VBAP", tables)

    def test_live_foreign_key_confidence_scoring(self):
        score = self.introspector.score_foreign_key_confidence(
            source_table="VBAK",
            source_col="KUNNR",
            target_table="KNA1",
            target_col="KUNNR",
            schema_name="DBADMIN"
        )
        self.assertEqual(score, 1.0)

if __name__ == "__main__":
    unittest.main()
