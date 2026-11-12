"""The recovery disk's files: what core sealed and signed opens here."""

import json
from pathlib import Path

import pytest

pytest.importorskip("cryptography")

from calimero_client_py import recovery  # noqa: E402

FIXTURE = json.loads(
    (Path(__file__).parent / "fixtures" / "recovery-rust.json").read_text()
)


def test_a_header_core_signed_is_checked_and_its_key_opens():
    header = recovery.verify_header(FIXTURE["header"], FIXTURE["signature"])
    assert header["holder"] == {"kind": "org", "id": "org-1"}
    key = recovery.open_data_key(header, bytes.fromhex(FIXTURE["secret"]))
    assert key.hex() == FIXTURE["dataKey"]


def test_a_changed_header_is_refused():
    changed = FIXTURE["header"].replace('"index":42', '"index":43')
    with pytest.raises(ValueError, match="signature"):
        recovery.verify_header(changed, FIXTURE["signature"])


def test_another_key_opens_nothing():
    header = recovery.verify_header(FIXTURE["header"], FIXTURE["signature"])
    with pytest.raises(ValueError, match="not the one"):
        recovery.open_data_key(header, recovery.new_backup_key_pair()["secret"])


def test_the_public_key_is_the_one_core_derives():
    assert (
        recovery.backup_public_key(bytes.fromhex(FIXTURE["secret"]))
        == FIXTURE["public"]
    )


def test_the_holder_body_takes_only_that_holders_files():
    listed = [
        {
            "name": "app-app-1.rbak",
            "header": FIXTURE["header"],
            "signature": FIXTURE["signature"],
        }
    ]
    secret = bytes.fromhex(FIXTURE["secret"])
    with pytest.raises(ValueError, match="no holder file"):
        recovery.holder_recovery_body({"kind": "org", "id": "org-1"}, listed, secret)
    with pytest.raises(ValueError, match="not org:other"):
        recovery.holder_recovery_body({"kind": "org", "id": "other"}, listed, secret)


BINDING = json.loads(
    (Path(__file__).parent / "fixtures" / "recovery-binding-rust.json").read_text()
)


def test_the_nonce_and_binding_are_the_ones_core_computes():
    assert recovery.recovery_attestation_nonce("c", 1) == BINDING["nonce"]
    key = bytes.fromhex(BINDING["signingKey"])
    assert (
        recovery.receipt_key_binding(key, BINDING["governance"]) == BINDING["binding"]
    )


def test_the_quote_verifier_is_asked_for_that_nonce_and_binding():
    header = dict(recovery.verify_header(FIXTURE["header"], FIXTURE["signature"]))
    header["signingKey"] = BINDING["signingKey"]
    header["attestation"] = {
        "quote": "cXVvdGU=",
        "nonce": BINDING["nonce"],
        "cluster": "c",
        "node": 1,
        "receiptKey": BINDING["signingKey"],
        "governance": BINDING["governance"],
    }
    asked = []
    recovery.verify_attestation(header, lambda q, n, s: asked.append((q, n, s)) or True)
    assert asked == [("cXVvdGU=", BINDING["nonce"], BINDING["binding"])]
    header["attestation"]["receiptKey"] = "00" * 32
    with pytest.raises(ValueError, match="signed"):
        recovery.verify_attestation(header, lambda q, n, s: True)
