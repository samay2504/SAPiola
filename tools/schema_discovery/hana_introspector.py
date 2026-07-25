#!/usr/bin/env python3
# ==============================================================================
# COPYRIGHT & PATENT NOTICE
# Copyright (c) 2026 Samay Mehar (ID: 487266569521). All Rights Reserved.
#
# PATENTS PENDING & INTELLECTUAL PROPERTY NOTICE:
# This source code, algorithm, architecture, and underlying inventions are the
# proprietary intellectual property of Samay Mehar (ID: 487266569521).
# Protected under national and international copyright, patent, and trade secret laws.
#
# COMMERCIAL LICENSE NOTICE:
# Unauthorized copying, modification, distribution, reverse engineering, or commercial
# exploitation of this software in whole or in part without express written authorization
# from Samay Mehar is strictly prohibited.
# ==============================================================================
"""
Unified SAP HANA Schema Discovery Engine for SAPiola.

Queries live SAP HANA SYS.TABLE_COLUMNS metadata, detects primary keys via
uniqueness sampling, discovers foreign keys via empirical value-overlap
scoring (Phase 0.5 validated method), and generates schema_manifest.json
+ mapping DSL files.

FK Detection Strategy:
  PRIMARY: Empirical value-overlap sampling (score_foreign_key_confidence)
  BOOSTER: SYS.REFERENTIAL_CONSTRAINTS (+0.1 arbitrary placeholder)
  
  Real SAP production tables do NOT have DB-level FK constraints.
  The empirical method is the mandatory, load-bearing signal.
"""

import os
import sys
import json
import hashlib
import argparse
import re
from typing import Dict, List, Tuple, Optional, Any
from hdbcli import dbapi

# Import unified credential resolver
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from credential_resolver import resolve_hana_credentials, CredentialError


class HanaIntrospector:
    """Introspects schema metadata and data relationships from an active SAP HANA instance."""

    # Arbitrary placeholder — will almost never fire against real SAP data.
    # See implementation_plan.md "Tuning Parameters" section.
    DECLARED_FK_CONFIDENCE_BOOST = 0.1

    def __init__(self, host: str, port: int, user: str, password: str,
                 sample_cap: int = 100_000):
        self.host = host
        self.port = port
        self.user = user
        self.password = password
        self.sample_cap = sample_cap

    @classmethod
    def from_credentials(cls, creds: Dict[str, str], sample_cap: int = 100_000):
        """Construct from credential resolver output."""
        return cls(
            host=creds["host"],
            port=int(creds["port"]),
            user=creds["user"],
            password=creds["password"],
            sample_cap=sample_cap,
        )

    def _get_connection(self):
        return dbapi.connect(
            address=self.host,
            port=self.port,
            user=self.user,
            password=self.password,
            encrypt=True,
            sslValidateCertificate=False,
        )

    # ─── Schema Discovery ─────────────────────────────────────────────────

    def fetch_table_schemas(self, schema_name: str,
                            table_filter: str = "*") -> Dict[str, Dict[str, str]]:
        """
        Query SYS.TABLE_COLUMNS to return map of table_name -> {col_name: data_type}.
        
        table_filter: SQL LIKE pattern (e.g., '%_RAG', 'VBAK%') or '*' for all.
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        if table_filter == "*":
            query = """
                SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE_NAME
                FROM SYS.TABLE_COLUMNS
                WHERE SCHEMA_NAME = ?
                ORDER BY TABLE_NAME, POSITION
            """
            cursor.execute(query, (schema_name,))
        else:
            query = """
                SELECT TABLE_NAME, COLUMN_NAME, DATA_TYPE_NAME
                FROM SYS.TABLE_COLUMNS
                WHERE SCHEMA_NAME = ? AND TABLE_NAME LIKE ?
                ORDER BY TABLE_NAME, POSITION
            """
            cursor.execute(query, (schema_name, table_filter))

        rows = cursor.fetchall()
        cursor.close()
        conn.close()

        schemas: Dict[str, Dict[str, str]] = {}
        for table, col, dtype in rows:
            if table not in schemas:
                schemas[table] = {}
            schemas[table][col] = dtype
        return schemas

    # ─── Primary Key Detection ────────────────────────────────────────────

    def detect_primary_keys(self, schema_name: str,
                            tables: Dict[str, Dict[str, str]]) -> Dict[str, List[str]]:
        """
        Detect primary keys via uniqueness sampling.
        
        Strategy:
        1. Try SYS.CONSTRAINTS for declared PKs first (cheap, sometimes works)
        2. Fall back to sampling: a column is a PK candidate if all sampled
           non-null values are unique
        """
        # First try declared constraints
        declared_pks = self._fetch_declared_pks(schema_name)

        result: Dict[str, List[str]] = {}
        conn = self._get_connection()
        cursor = conn.cursor()

        for table_name, columns in tables.items():
            # Use declared PK if available
            if table_name in declared_pks and declared_pks[table_name]:
                result[table_name] = declared_pks[table_name]
                continue

            # Fall back to uniqueness sampling
            pk_candidates = []
            for col_name in columns:
                try:
                    query = f"""
                        SELECT COUNT(*) as total,
                               COUNT(DISTINCT "{col_name}") as distinct_count
                        FROM "{schema_name}"."{table_name}"
                        WHERE "{col_name}" IS NOT NULL
                    """
                    cursor.execute(query)
                    row = cursor.fetchone()
                    if row and row[0] > 0 and row[0] == row[1]:
                        pk_candidates.append(col_name)
                except Exception:
                    continue  # Skip columns that error (e.g., LOB types)

            # Heuristic: prefer columns with 'ID' in the name, or first unique col
            if pk_candidates:
                id_cols = [c for c in pk_candidates if "ID" in c.upper()]
                result[table_name] = id_cols[:1] if id_cols else pk_candidates[:1]
            else:
                # No unique column found — use first column as fallback
                result[table_name] = [list(columns.keys())[0]] if columns else []

        cursor.close()
        conn.close()
        return result

    def _fetch_declared_pks(self, schema_name: str) -> Dict[str, List[str]]:
        """Query SYS.CONSTRAINTS for declared PRIMARY KEY constraints."""
        conn = self._get_connection()
        cursor = conn.cursor()
        try:
            query = """
                SELECT TABLE_NAME, COLUMN_NAME
                FROM SYS.CONSTRAINTS
                WHERE SCHEMA_NAME = ? AND IS_PRIMARY_KEY = 'TRUE'
                ORDER BY TABLE_NAME, POSITION
            """
            cursor.execute(query, (schema_name,))
            rows = cursor.fetchall()
        except Exception:
            rows = []
        finally:
            cursor.close()
            conn.close()

        pks: Dict[str, List[str]] = {}
        for table, col in rows:
            if table not in pks:
                pks[table] = []
            pks[table].append(col)
        return pks

    # ─── Foreign Key Detection (Empirical — PRIMARY signal) ───────────────

    def score_foreign_key_confidence(self, source_table: str, source_col: str,
                                      target_table: str, target_col: str,
                                      schema_name: str) -> float:
        """
        Calculate set inclusion confidence score of source_col values in target_col values.
        
        This is the PRIMARY FK detection method. Validated in Phase 0.5.
        
        Returns: float between 0.0 and 1.0 representing the fraction of
                 non-null source values that exist in target values.
        """
        conn = self._get_connection()
        cursor = conn.cursor()

        src_query = (
            f'SELECT "{source_col}" FROM "{schema_name}"."{source_table}" '
            f'WHERE "{source_col}" IS NOT NULL LIMIT {self.sample_cap}'
        )
        tgt_query = (
            f'SELECT "{target_col}" FROM "{schema_name}"."{target_table}" '
            f'WHERE "{target_col}" IS NOT NULL LIMIT {self.sample_cap}'
        )

        try:
            cursor.execute(src_query)
            src_vals = [row[0] for row in cursor.fetchall()]

            cursor.execute(tgt_query)
            tgt_vals = set(row[0] for row in cursor.fetchall())
        except Exception:
            cursor.close()
            conn.close()
            return 0.0

        cursor.close()
        conn.close()

        if not src_vals or not tgt_vals:
            return 0.0

        matches = sum(1 for val in src_vals if val in tgt_vals)
        return matches / len(src_vals)

    def discover_foreign_keys(self, schema_name: str,
                               tables: Dict[str, Dict[str, str]],
                               primary_keys: Dict[str, List[str]],
                               min_confidence: float = 0.8) -> List[Dict[str, Any]]:
        """
        Discover foreign key relationships via empirical value-overlap.
        
        Strategy:
        1. For each (source_table.col, target_table.pk_col) pair where
           column names match, score via value-overlap sampling.
        2. Check SYS.REFERENTIAL_CONSTRAINTS for declared FKs and add
           +0.1 confidence boost (arbitrary placeholder — see plan).
        3. Return all relationships above min_confidence threshold.
        """
        # Get declared constraints for confidence boosting
        declared_fks = self._fetch_declared_fks(schema_name)

        candidates = []
        table_names = list(tables.keys())

        for src_table in table_names:
            src_cols = tables[src_table]
            for tgt_table in table_names:
                if src_table == tgt_table:
                    continue

                tgt_pk_cols = primary_keys.get(tgt_table, [])

                for src_col in src_cols:
                    for tgt_pk in tgt_pk_cols:
                        # Candidate heuristic: column names match
                        if src_col.upper() != tgt_pk.upper():
                            continue

                        # Skip if this is the source table's own PK
                        src_pks = primary_keys.get(src_table, [])
                        if src_col in src_pks and len(src_pks) == 1:
                            continue  # Self-referencing PK, not an FK

                        # EMPIRICAL scoring — the primary signal
                        confidence = self.score_foreign_key_confidence(
                            src_table, src_col, tgt_table, tgt_pk, schema_name
                        )

                        # Declared constraint boost (arbitrary +0.1 placeholder)
                        constraint_declared = _is_declared_fk(
                            declared_fks, src_table, tgt_table
                        )
                        if constraint_declared:
                            confidence = min(1.0, confidence + self.DECLARED_FK_CONFIDENCE_BOOST)

                        if confidence >= min_confidence:
                            edge_label = f"RefersTo_{_pascal_case(tgt_table)}"
                            candidates.append({
                                "from_table": src_table,
                                "from_col": src_col,
                                "to_table": tgt_table,
                                "to_col": tgt_pk,
                                "edge_label": edge_label,
                                "confidence": round(confidence, 4),
                                "detection_method": "empirical_value_overlap",
                                "constraint_declared": constraint_declared,
                            })

        return candidates

    def _fetch_declared_fks(self, schema_name: str) -> List[Dict[str, str]]:
        """Query SYS.REFERENTIAL_CONSTRAINTS (confidence booster only)."""
        conn = self._get_connection()
        cursor = conn.cursor()
        try:
            query = """
                SELECT CONSTRAINT_NAME, TABLE_NAME, REFERENCED_TABLE_NAME
                FROM SYS.REFERENTIAL_CONSTRAINTS
                WHERE SCHEMA_NAME = ?
            """
            cursor.execute(query, (schema_name,))
            rows = cursor.fetchall()
        except Exception:
            rows = []
        finally:
            cursor.close()
            conn.close()

        return [
            {"constraint": r[0], "table": r[1], "referenced_table": r[2]}
            for r in rows
        ]

    # ─── Manifest Generation (Orchestrator) ───────────────────────────────

    def generate_manifest(self, schema_name: str,
                           table_filter: str = "*",
                           min_fk_confidence: float = 0.8,
                           output_dir: str = ".") -> dict:
        """
        Full pipeline: discover tables → detect PKs → empirical FK scoring →
        generate manifest + DSL.

        FK detection uses score_foreign_key_confidence() (empirical value-overlap)
        as the PRIMARY signal. SYS.REFERENTIAL_CONSTRAINTS is checked only as a
        confidence booster (+0.1) when a declared constraint corroborates an
        empirical match.
        """
        print(f"[1/5] Discovering tables in {schema_name} (filter: {table_filter})...")
        tables = self.fetch_table_schemas(schema_name, table_filter)
        print(f"       Found {len(tables)} tables: {list(tables.keys())}")

        print(f"[2/5] Detecting primary keys via uniqueness sampling...")
        primary_keys = self.detect_primary_keys(schema_name, tables)
        for t, pks in primary_keys.items():
            print(f"       {t}: PK = {pks}")

        print(f"[3/5] Discovering foreign keys via empirical value-overlap "
              f"(min confidence: {min_fk_confidence})...")
        relationships = self.discover_foreign_keys(
            schema_name, tables, primary_keys, min_fk_confidence
        )
        for rel in relationships:
            print(f"       {rel['from_table']}.{rel['from_col']} -> "
                  f"{rel['to_table']}.{rel['to_col']} "
                  f"(confidence: {rel['confidence']}, "
                  f"declared: {rel['constraint_declared']})")

        print(f"[4/5] Generating DSL...")
        dsl_content = self._generate_dsl(tables, primary_keys, relationships)

        print(f"[5/5] Writing manifest and DSL...")
        manifest = {
            "source": {
                "host": self.host,
                "port": self.port,
                "schema": schema_name,
                "table_filter": table_filter,
            },
            "tables": {},
            "relationships": relationships,
            "generated_dsl": dsl_content,
            "relationships_override": [],
            "min_fk_confidence_used": min_fk_confidence,
            "declared_fk_boost": self.DECLARED_FK_CONFIDENCE_BOOST,
        }

        for table_name, columns in tables.items():
            pks = primary_keys.get(table_name, [])
            manifest["tables"][table_name] = {
                "columns": columns,
                "primary_keys": pks,
                "pk_detection_method": "declared" if self._has_declared_pk(
                    schema_name, table_name) else "uniqueness_sampling",
                "is_node": True,
                "node_label": _pascal_case(table_name),
            }

        # Compute fingerprint
        manifest_json = json.dumps(manifest, sort_keys=True, default=str)
        manifest["fingerprint"] = "sha256:" + hashlib.sha256(
            manifest_json.encode()
        ).hexdigest()

        # Write files
        manifest_path = os.path.join(output_dir, "schema_manifest.json")
        with open(manifest_path, "w", encoding="utf-8") as f:
            json.dump(manifest, f, indent=2, default=str)
        print(f"       Manifest: {manifest_path}")

        dsl_path = os.path.join(output_dir, "hana_mapping.dsl")
        with open(dsl_path, "w", encoding="utf-8") as f:
            f.write(dsl_content)
        print(f"       DSL:      {dsl_path}")

        print(f"\nDone. {len(tables)} tables, {len(relationships)} relationships.")
        return manifest

    def _has_declared_pk(self, schema_name: str, table_name: str) -> bool:
        """Check if a table has a declared PK in SYS.CONSTRAINTS."""
        pks = self._fetch_declared_pks(schema_name)
        return table_name in pks and len(pks[table_name]) > 0

    def _generate_dsl(self, tables: Dict[str, Dict[str, str]],
                      primary_keys: Dict[str, List[str]],
                      relationships: List[Dict[str, Any]]) -> str:
        """Generate Pest-compliant mapping DSL from discovered schema."""
        lines = [
            "// AUTO-GENERATED by HanaIntrospector.generate_manifest()",
            "// Do not edit manually — regenerate via:",
            "//   python tools/salt_importer/hana_introspector.py --schema <SCHEMA>",
            "",
        ]

        # Node definitions
        for table_name in sorted(tables.keys()):
            pks = primary_keys.get(table_name, [])
            id_col = pks[0] if pks else list(tables[table_name].keys())[0]
            label = _pascal_case(table_name)

            # Properties: all columns except the ID column
            prop_cols = [c for c in tables[table_name] if c != id_col]
            if prop_cols:
                props_str = f" PROPERTIES ({', '.join(prop_cols)})"
            else:
                props_str = ""

            lines.append(f"NODE {label} FROM {table_name} WITH id = {id_col}{props_str}")

        lines.append("")

        # Edge definitions
        for rel in relationships:
            lines.append(
                f"EDGE {rel['edge_label']} FROM {rel['from_table']} "
                f"USING {rel['from_col']} -> {rel['to_col']}"
            )
            lines.append(
                f"//   -> targets {rel['to_table']}({rel['to_col']}) "
                f"[Confidence: {rel['confidence'] * 100:.1f}%]"
            )

        return "\n".join(lines) + "\n"


# ─── Helpers ──────────────────────────────────────────────────────────────────


def _pascal_case(name: str) -> str:
    """Convert a table name to PascalCase. E.g., 'VBAK_RAG' -> 'VbakRag'."""
    parts = name.lower().split("_")
    return "".join(word.capitalize() for word in parts if word)


def _is_declared_fk(declared_fks: List[Dict[str, str]],
                    src_table: str, tgt_table: str) -> bool:
    """Check if a declared FK constraint exists between two tables."""
    return any(
        fk["table"] == src_table and fk["referenced_table"] == tgt_table
        for fk in declared_fks
    )


# ─── CLI Entry Point ─────────────────────────────────────────────────────────


def main():
    parser = argparse.ArgumentParser(
        description="SAP HANA Schema Discovery Engine for SAPiola",
        epilog="Generates schema_manifest.json and mapping DSL from live HANA metadata."
    )
    parser.add_argument(
        "--schema", required=True,
        help="SAP HANA schema name to introspect (e.g., DBADMIN). "
             "Also settable via SAPIOLA_HANA_SCHEMA env var."
    )
    parser.add_argument(
        "--table-filter", default=None,
        help="SQL LIKE pattern for table names (e.g., '%%_RAG'). "
             "Default: all tables. Also settable via SAPIOLA_TABLE_FILTER."
    )
    parser.add_argument(
        "--min-fk-confidence", type=float, default=0.8,
        help="Minimum empirical FK confidence threshold (default: 0.8). "
             "Uncalibrated placeholder — see docs/SCHEMA_DISCOVERY.md."
    )
    parser.add_argument(
        "--output-dir", default=".",
        help="Output directory for schema_manifest.json and .dsl file."
    )
    parser.add_argument(
        "--service-key", default=None,
        help="Path to sapiola-dev-key.json service key file."
    )
    parser.add_argument(
        "--sample-cap", type=int, default=100_000,
        help="Max rows to sample per column for FK confidence scoring."
    )

    args = parser.parse_args()

    # Resolve schema from arg or env
    schema = args.schema or os.environ.get("SAPIOLA_HANA_SCHEMA")
    if not schema:
        parser.error("--schema is required (or set SAPIOLA_HANA_SCHEMA env var)")

    table_filter = args.table_filter or os.environ.get("SAPIOLA_TABLE_FILTER", "*")

    # Resolve credentials
    try:
        creds = resolve_hana_credentials(key_path=args.service_key)
    except CredentialError as e:
        print(str(e), file=sys.stderr)
        sys.exit(1)

    print(f"Connecting to {creds['host']}:{creds['port']} as {creds['user']}...")

    introspector = HanaIntrospector.from_credentials(creds, sample_cap=args.sample_cap)

    os.makedirs(args.output_dir, exist_ok=True)

    manifest = introspector.generate_manifest(
        schema_name=schema,
        table_filter=table_filter,
        min_fk_confidence=args.min_fk_confidence,
        output_dir=args.output_dir,
    )

    print(f"\nFingerprint: {manifest.get('fingerprint', 'N/A')}")


if __name__ == "__main__":
    main()
