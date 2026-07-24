#!/usr/bin/env python3
"""
Unified SAP HANA credential resolver for SAPiola.

Resolution chain (first match wins):
  1. Environment variables: SAPIOLA_HANA_HOST, SAPIOLA_HANA_PORT,
     SAPIOLA_HANA_USER, SAPIOLA_HANA_PASSWORD
  2. Service key file (sapiola-dev-key.json) for host/port,
     env vars for user/password
  3. Fail with clear, actionable error message

Never hardcodes fallback credentials.
"""

import os
import json
import sys
from typing import Dict, Optional


class CredentialError(Exception):
    """Raised when credentials cannot be resolved."""
    pass


def resolve_hana_credentials(key_path: Optional[str] = None) -> Dict[str, str]:
    """
    Resolve SAP HANA connection credentials.

    Returns dict with keys: host, port, user, password

    Raises CredentialError with actionable message on failure.
    """
    host = os.environ.get("SAPIOLA_HANA_HOST")
    port = os.environ.get("SAPIOLA_HANA_PORT")
    user = os.environ.get("SAPIOLA_HANA_USER")
    password = os.environ.get("SAPIOLA_HANA_PASSWORD")

    # If all four env vars are set, use them directly — no file needed
    if all([host, port, user, password]):
        return {
            "host": host,
            "port": str(port),
            "user": user,
            "password": password,
        }

    # Try service key file for host/port if not set via env
    key_data = _load_service_key(key_path)

    if key_data is not None:
        if not host:
            host = key_data.get("host")
        if not port:
            port = str(key_data.get("port", 443))

        # Check for plain user/password in the key file itself
        file_user = key_data.get("user")
        file_password = key_data.get("password")

        if file_user and file_password:
            if not user:
                user = file_user
            if not password:
                password = file_password

        # UAA-only key detection: uaa block present but no plain credentials
        if (not user or not password) and "uaa" in key_data:
            uaa_url = key_data.get("uaa", {}).get("url", "<unknown UAA URL>")
            resolved_path = key_path or _default_key_path()
            raise CredentialError(
                f"ERROR: Service key at '{resolved_path}' contains only OAuth UAA credentials\n"
                f"(clientid/clientsecret), not a direct database user/password.\n"
                f"\n"
                f"To connect to SAP HANA Cloud, either:\n"
                f"  1. Set SAPIOLA_HANA_USER and SAPIOLA_HANA_PASSWORD env vars\n"
                f"     (e.g., a dedicated DB user like SAPIOLA_TEST), OR\n"
                f"  2. Implement OAuth token exchange using the UAA endpoint\n"
                f"     at {uaa_url}\n"
                f"\n"
                f"Do NOT use DBADMIN credentials in production."
            )

    # Final validation
    missing = []
    if not host:
        missing.append("SAPIOLA_HANA_HOST")
    if not port:
        missing.append("SAPIOLA_HANA_PORT")
    if not user:
        missing.append("SAPIOLA_HANA_USER")
    if not password:
        missing.append("SAPIOLA_HANA_PASSWORD")

    if missing:
        key_file_status = "not found" if key_data is None else "found but incomplete"
        raise CredentialError(
            f"ERROR: Cannot resolve SAP HANA credentials.\n"
            f"\n"
            f"Missing: {', '.join(missing)}\n"
            f"Service key file: {key_file_status}\n"
            f"\n"
            f"Provide credentials via:\n"
            f"  1. Environment variables: {', '.join(missing)}\n"
            f"  2. Service key file at SAPIOLA_HANA_SERVICE_KEY or\n"
            f"     {_default_key_path()}\n"
            f"\n"
            f"Never hardcode credentials in source code."
        )

    return {
        "host": host,
        "port": str(port),
        "user": user,
        "password": password,
    }


def _default_key_path() -> str:
    """Returns the default service key path, searching common locations."""
    candidates = [
        os.environ.get("SAPIOLA_HANA_SERVICE_KEY", ""),
        os.path.join(os.getcwd(), "sapiola-dev-key.json"),
        os.path.join(os.path.dirname(os.path.abspath(__file__)), "..", "..", "sapiola-dev-key.json"),
    ]
    for path in candidates:
        if path and os.path.exists(path):
            return os.path.abspath(path)
    return "sapiola-dev-key.json"


def _load_service_key(key_path: Optional[str] = None) -> Optional[dict]:
    """Load and parse a SAP HANA service key JSON file."""
    if key_path and os.path.exists(key_path):
        path = key_path
    else:
        path = _default_key_path()
        if not os.path.exists(path):
            return None

    try:
        with open(path, "r", encoding="utf-8") as f:
            return json.load(f)
    except (json.JSONDecodeError, IOError) as e:
        print(f"WARNING: Could not parse service key at {path}: {e}", file=sys.stderr)
        return None
