#!/usr/bin/env python3
"""Tests for signing every request with a device key.

A session is a bearer token: obtained once, presented many times. A
request-carried proof commits to one method, one path and one body, so it is
minted per call and cannot be lifted onto another request. It is what lets this
client drive a relay it has no account on — nothing was ever issued to it.

All of this runs without a node. A connection is built, not used: what is under
test is that the keys are accepted, validated, and refused in the combinations
that cannot work. Whether a node then honours the proof depends on it running
with `--delegated-access`, which is covered by core's `delegated-proof.yml`.

The credential is the one `test_warrant.py` froze, for the same reason: it has
to certify exactly the key SECRET holds, and generating a pair here would mean
reimplementing the certification, at which point the test exercises the fixture
rather than the code.
"""

import pytest
from calimero_client_py import create_connection

from test_warrant import CREDENTIAL, SECRET

URL = "http://127.0.0.1:2528"


def test_no_keys_builds_an_ordinary_connection():
    """Every existing caller is unaffected."""
    conn = create_connection(URL)
    assert conn.api_url.startswith("http://127.0.0.1:2528")


def test_credential_and_secret_build_a_two_link_signer():
    conn = create_connection(URL, None, CREDENTIAL, SECRET)
    assert conn.api_url.startswith("http://127.0.0.1:2528")


def test_a_session_selects_the_three_link_chain():
    # The statement is not a valid encoding, which is the point: it must be
    # rejected as a statement rather than ignored. A session that were silently
    # dropped would leave the caller on the two-link chain, which carries no
    # node binding — a downgrade they never asked for.
    with pytest.raises(ValueError, match="device_session"):
        create_connection(URL, None, CREDENTIAL, SECRET, "aabb")


@pytest.mark.parametrize(
    "credential,secret",
    [(CREDENTIAL, None), (None, SECRET)],
    ids=["credential-alone", "secret-alone"],
)
def test_half_a_pair_is_refused(credential, secret):
    """Either alone is inert, so it is refused rather than quietly ignored.

    A credential with no key signs nothing; a key with no credential produces a
    signature the node cannot attribute. Building the connection anyway would
    hand back something the caller believes signs and which does not.
    """
    with pytest.raises(ValueError, match="together"):
        create_connection(URL, None, credential, secret)


def test_a_session_alone_names_no_device():
    with pytest.raises(ValueError, match="device_credential"):
        create_connection(URL, None, None, None, "aabb")


def test_a_malformed_secret_says_which_argument_it_was():
    # The three are indistinguishable as hex, and passing them in the wrong
    # order is the likely mistake — so the error names the one it rejected.
    with pytest.raises(ValueError, match="device_secret"):
        create_connection(URL, None, CREDENTIAL, "not-hex")

    with pytest.raises(ValueError, match="device_secret.*32 bytes"):
        create_connection(URL, None, CREDENTIAL, "aabb")


def test_a_malformed_credential_says_which_argument_it_was():
    with pytest.raises(ValueError, match="device_credential"):
        create_connection(URL, None, "not-hex", SECRET)
