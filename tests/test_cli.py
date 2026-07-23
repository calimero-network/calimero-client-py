#!/usr/bin/env python3
"""
Tests for the calimero-client-py CLI (calimero.cli).

The CLI is otherwise uncovered by the suite. These tests patch the native
client factories so they run without a live node, and exercise the behaviors
this covers: version/help output, the list-contexts count (including the
``{"data": [...]}`` envelope), the --auth-mode mismatch warning, and that
--node-name is threaded through to the connection.
"""

import sys
import pytest

from calimero import __version__
import calimero.cli as cli


class _FakeAuthMode:
    def __init__(self, value):
        self.value = value


class _FakeConnection:
    def __init__(self, detected="none"):
        self._detected = detected
        self.api_url = "http://localhost:2428"

    def detect_auth_mode(self):
        return _FakeAuthMode(self._detected)


class _FakeClient:
    def __init__(self, contexts):
        self._contexts = contexts

    def list_contexts(self):
        return self._contexts


@pytest.fixture
def patched(monkeypatch):
    """Patch the CLI's client factories; returns state the test can tweak."""
    state = {"detected": "none", "contexts": [], "node_name": None, "api_url": None}

    def fake_create_connection(api_url, node_name=None):
        state["api_url"] = api_url
        state["node_name"] = node_name
        return _FakeConnection(detected=state["detected"])

    def fake_create_client(connection):
        return _FakeClient(state["contexts"])

    monkeypatch.setattr(cli, "create_connection", fake_create_connection)
    monkeypatch.setattr(cli, "create_client", fake_create_client)
    return state


def _run(monkeypatch, *argv):
    monkeypatch.setattr(sys, "argv", ["calimero-client-py", *argv])
    cli.main()


def test_version_exits_zero_and_prints_version(monkeypatch, capsys):
    monkeypatch.setattr(sys, "argv", ["calimero-client-py", "--version"])
    with pytest.raises(SystemExit) as exc:
        cli.main()
    assert exc.value.code == 0
    assert __version__ in capsys.readouterr().out


def test_no_command_prints_help(monkeypatch, capsys, patched):
    # No command prints help and returns without touching the node.
    _run(monkeypatch)
    assert "usage" in capsys.readouterr().out.lower()


def test_list_contexts_counts_bare_list(monkeypatch, capsys, patched):
    patched["contexts"] = ["ctx-a", "ctx-b", "ctx-c"]
    _run(monkeypatch, "--base-url", "http://localhost:2428", "list-contexts")
    assert "Found 3 contexts:" in capsys.readouterr().out


def test_list_contexts_counts_data_envelope(monkeypatch, capsys, patched):
    # Responses may wrap the list under a "data" key; the count must still work.
    patched["contexts"] = {"data": ["ctx-a", "ctx-b"]}
    _run(monkeypatch, "list-contexts")
    assert "Found 2 contexts:" in capsys.readouterr().out


def test_auth_mode_mismatch_warns(monkeypatch, capsys, patched):
    patched["detected"] = "required"
    _run(monkeypatch, "--auth-mode", "none", "list-contexts")
    err = capsys.readouterr().err.lower()
    assert "auth-mode" in err
    assert "required" in err


def test_auth_mode_match_does_not_warn(monkeypatch, capsys, patched):
    patched["detected"] = "none"
    _run(monkeypatch, "--auth-mode", "none", "list-contexts")
    assert "auth-mode" not in capsys.readouterr().err.lower()


def test_node_name_threaded_to_connection(monkeypatch, capsys, patched):
    _run(monkeypatch, "--node-name", "prod-1", "list-contexts")
    assert patched["node_name"] == "prod-1"
