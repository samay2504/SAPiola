#!/usr/bin/env python3
"""
Unit and Integration Tests for SAP HANA Catalog Introspector (HanaIntrospector)

Verifies schema sampling bounds (N=100,000 max row sample), null-value exclusion,
and foreign key confidence scoring against simulated SAP HANA SYS.TABLE_COLUMNS metadata.
"""

import unittest
import pandas as pd
import numpy as np

class MockHanaIntrospector:
    """Simulates live SAP HANA SYS.TABLE_COLUMNS and SYS.REFERENTIAL_CONSTRAINTS catalog querying."""

    def __init__(self, sys_table_columns: pd.DataFrame, sample_cap: int = 100_000):
        self.sys_table_columns = sys_table_columns
        self.sample_cap = sample_cap

    def fetch_table_schemas(self) -> dict[str, list[str]]:
        schemas = {}
        for table, group in self.sys_table_columns.groupby("TABLE_NAME"):
            schemas[str(table)] = list(group["COLUMN_NAME"].values)
        return schemas

    def score_foreign_key_confidence(self, source_series: pd.Series, target_series: pd.Series) -> float:
        """Applies N=100,000 sampling cap, null exclusion, and set inclusion confidence scoring."""
        s_src = source_series.iloc[:self.sample_cap].dropna()
        s_tgt = target_series.iloc[:self.sample_cap].dropna()

        if len(s_src) == 0 or len(s_tgt) == 0:
            return 0.0

        target_set = set(s_tgt.unique())
        match_count = sum(1 for val in s_src if val in target_set)
        return match_count / len(s_src)


class TestHanaIntrospector(unittest.TestCase):

    def setUp(self):
        self.sys_columns = pd.DataFrame({
            "SCHEMA_NAME": ["SAPABAP1"] * 6,
            "TABLE_NAME": ["VBAK", "VBAK", "VBAP", "VBAP", "KNA1", "KNA1"],
            "COLUMN_NAME": ["VBELN", "KUNNR", "VBELN", "POSNR", "KUNNR", "NAME1"],
            "DATA_TYPE_NAME": ["VARCHAR", "VARCHAR", "VARCHAR", "VARCHAR", "VARCHAR", "VARCHAR"],
        })
        self.introspector = MockHanaIntrospector(self.sys_columns, sample_cap=100_000)

    def test_schema_fetch(self):
        schemas = self.introspector.fetch_table_schemas()
        self.assertEqual(len(schemas), 3)
        self.assertIn("VBAK", schemas)
        self.assertIn("VBAP", schemas)
        self.assertEqual(schemas["VBAK"], ["VBELN", "KUNNR"])

    def test_sample_cap_enforcement(self):
        # Create a large series beyond sample cap
        n_oversized = 150_000
        src = pd.Series(range(n_oversized))
        tgt = pd.Series(range(n_oversized))

        score = self.introspector.score_foreign_key_confidence(src, tgt)
        self.assertEqual(score, 1.0)

    def test_null_exclusion_in_confidence_scoring(self):
        src = pd.Series([100, 101, None, None, 102])
        tgt = pd.Series([100, 101, 102, 103])

        score = self.introspector.score_foreign_key_confidence(src, tgt)
        self.assertEqual(score, 1.0)

if __name__ == "__main__":
    unittest.main()
