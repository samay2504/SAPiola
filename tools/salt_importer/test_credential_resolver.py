#!/usr/bin/env python3
"""
Unit tests for credential_resolver.py

Tests the resolution chain, UAA-only detection, and error messages.
"""

import os
import sys
import json
import tempfile
import pytest

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "tools", "salt_importer"))
from credential_resolver import resolve_hana_credentials, CredentialError


class TestEnvVarResolution:
    """Tests for resolution via environment variables (priority 1)."""

    def test_all_env_vars_set(self, monkeypatch):
        """When all 4 env vars are set, use them directly."""
        monkeypatch.setenv("SAPIOLA_HANA_HOST", "test-host.hana.cloud")
        monkeypatch.setenv("SAPIOLA_HANA_PORT", "443")
        monkeypatch.setenv("SAPIOLA_HANA_USER", "TESTUSER")
        monkeypatch.setenv("SAPIOLA_HANA_PASSWORD", "testpass")

        creds = resolve_hana_credentials(key_path="/nonexistent/path.json")
        assert creds["host"] == "test-host.hana.cloud"
        assert creds["port"] == "443"
        assert creds["user"] == "TESTUSER"
        assert creds["password"] == "testpass"

    def test_partial_env_vars_with_key_file(self, monkeypatch, tmp_path):
        """Env vars take priority, key file fills gaps for host/port."""
        key_file = tmp_path / "key.json"
        key_file.write_text(json.dumps({
            "host": "from-key.hana.cloud",
            "port": 443,
            "user": "KEY_USER",
            "password": "key_pass"
        }))
        # Only set user/password via env; host/port from key file
        monkeypatch.setenv("SAPIOLA_HANA_USER", "ENV_USER")
        monkeypatch.setenv("SAPIOLA_HANA_PASSWORD", "env_pass")
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)

        creds = resolve_hana_credentials(key_path=str(key_file))
        assert creds["host"] == "from-key.hana.cloud"
        assert creds["user"] == "ENV_USER"  # Env takes priority


class TestServiceKeyResolution:
    """Tests for resolution via service key file (priority 2)."""

    def test_full_key_file(self, monkeypatch, tmp_path):
        """Key file with host, port, user, password."""
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_USER", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PASSWORD", raising=False)

        key_file = tmp_path / "key.json"
        key_file.write_text(json.dumps({
            "host": "full-key.hana.cloud",
            "port": 443,
            "user": "FULL_USER",
            "password": "full_pass"
        }))

        creds = resolve_hana_credentials(key_path=str(key_file))
        assert creds["host"] == "full-key.hana.cloud"
        assert creds["user"] == "FULL_USER"
        assert creds["password"] == "full_pass"


class TestUaaOnlyDetection:
    """Tests for UAA-only service key detection."""

    def test_uaa_only_key_fails_with_actionable_message(self, monkeypatch, tmp_path):
        """UAA-only key without env vars -> immediate failure with clear message."""
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_USER", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PASSWORD", raising=False)

        key_file = tmp_path / "btp-key.json"
        key_file.write_text(json.dumps({
            "host": "uaa-host.hana.cloud",
            "port": 443,
            "uaa": {
                "url": "https://auth.example.com",
                "clientid": "sb-some-client",
                "clientsecret": "secret123"
            }
        }))

        with pytest.raises(CredentialError) as exc_info:
            resolve_hana_credentials(key_path=str(key_file))

        error_msg = str(exc_info.value)
        assert "OAuth UAA credentials" in error_msg
        assert "SAPIOLA_HANA_USER" in error_msg
        assert "SAPIOLA_HANA_PASSWORD" in error_msg
        assert "https://auth.example.com" in error_msg
        # The message should warn AGAINST using DBADMIN, not suggest it as a login
        assert "Do NOT use DBADMIN" in error_msg

    def test_uaa_key_with_env_user_succeeds(self, monkeypatch, tmp_path):
        """UAA key file but user/password provided via env vars -> success."""
        monkeypatch.setenv("SAPIOLA_HANA_USER", "SAPIOLA_TEST")
        monkeypatch.setenv("SAPIOLA_HANA_PASSWORD", "test_password")
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)

        key_file = tmp_path / "btp-key.json"
        key_file.write_text(json.dumps({
            "host": "uaa-host.hana.cloud",
            "port": 443,
            "uaa": {
                "url": "https://auth.example.com",
                "clientid": "sb-some-client",
                "clientsecret": "secret123"
            }
        }))

        creds = resolve_hana_credentials(key_path=str(key_file))
        assert creds["host"] == "uaa-host.hana.cloud"
        assert creds["user"] == "SAPIOLA_TEST"


class TestFailureMessages:
    """Tests for clear error messages on missing credentials."""

    def test_no_credentials_at_all(self, monkeypatch, tmp_path):
        """No env vars, no key file -> fail with clear message."""
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_USER", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PASSWORD", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_SERVICE_KEY", raising=False)
        # Use a temp dir as CWD to avoid finding the real sapiola-dev-key.json
        monkeypatch.chdir(tmp_path)
        # Also patch _default_key_path to prevent finding the real key via __file__
        import credential_resolver as cr
        monkeypatch.setattr(cr, "_default_key_path", lambda: str(tmp_path / "nonexistent.json"))

        with pytest.raises(CredentialError) as exc_info:
            resolve_hana_credentials(key_path=str(tmp_path / "nonexistent.json"))

        error_msg = str(exc_info.value)
        assert "SAPIOLA_HANA_HOST" in error_msg
        assert "Cannot resolve" in error_msg
        assert "Never hardcode" in error_msg

    def test_malformed_key_file(self, monkeypatch, tmp_path):
        """Malformed JSON key file -> treated as missing, not a crash."""
        monkeypatch.delenv("SAPIOLA_HANA_HOST", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PORT", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_USER", raising=False)
        monkeypatch.delenv("SAPIOLA_HANA_PASSWORD", raising=False)

        key_file = tmp_path / "bad-key.json"
        key_file.write_text("not valid json {{{")

        with pytest.raises(CredentialError):
            resolve_hana_credentials(key_path=str(key_file))


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
