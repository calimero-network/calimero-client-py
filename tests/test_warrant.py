#!/usr/bin/env python3
"""Tests for warrant minting and the delegated-intent binding.

`sign_warrant` contacts nothing — it signs a statement with a device secret and
returns bytes — so all of this runs without a node. That is the point of the
feature as much as a convenience for the tests: the key that signs a warrant must
never reach the node that runs the request.

The credential below was captured from `merod account sign-cert --generate`
against the well-known "legal winner thank year..." test phrase. It owns nothing.
It is frozen rather than generated because it has to certify exactly the key
SECRET holds, and generating a consistent pair here would mean reimplementing the
certification — at which point the test would exercise the fixture, not the code.
"""

import pytest
from calimero_client_py import create_client, create_connection, sign_warrant

CREDENTIAL = (
    "02b2a942ff4c98718bed76e255987f6d59b1a72d3b2cd2510003e6170ac63a9ffb00000000"
    "0e2cd2d3dc84e1db5088e32510ca45bc491e4033bbb0f6bbb733bc0c7b7f5e304d0774b93e"
    "8028899a745dbe03d7727fa31fc2f060945b5789cb36c23cba380366245580f7aa816a35d1"
    "ff324a714355995ef44a72bcd2341e21d9587d16efce973135e50bc7280f06bb32a53a5669"
    "83cf0f0c8428be4b461df54264f073195400000000000000"
    "00e0c3743677508f5cfbe245f043f2d7bc3ba6c88c001464cae581e2e9ec8cb63780f1f5c2"
    "a393521a0038b357fffe63092403fa6e0e2ec12da5e96d50692d400f"
)
SECRET = "4987ccd0fb7ef36bf7f61e8f99fd150d33e6adac47649f23bfd7109c2e36a3ba"
ACCOUNT = "0e2cd2d3dc84e1db5088e32510ca45bc491e4033bbb0f6bbb733bc0c7b7f5e30"
# Hex, as every id is now. The same 32 bytes (`00 01 .. 1f`) this was base58 for,
# so the signatures below are unchanged.
CONTEXT = "000102030405060708090a0b0c0d0e0f101112131415161718191a1b1c1d1e1f"
# The old base58 spelling of those bytes, kept only to assert it is refused.
CONTEXT_B58 = "1thX6LZfHDZZKUs92febYZhYRcXddmzfzF2NvTkPNE"


def mint(args='{"key":"k","value":"v"}', nonce=1, **kwargs):
    """Mint with the fixture's identity, overriding whatever the test needs."""
    params = dict(
        context_id=CONTEXT,
        executor=ACCOUNT,
        method="set",
        args=args,
        nonce=nonce,
        device_secret=SECRET,
        credential=CREDENTIAL,
    )
    params.update(kwargs)
    return sign_warrant(**params)


def test_sign_warrant_is_exported():
    """It is a module-level function, not a client method.

    Deliberate: minting needs no connection, and requiring one would imply the
    node is involved in signing — which is exactly the thing that must not be
    true.
    """
    from calimero_client_py import sign_warrant as imported

    assert imported is not None


def test_mint_returns_the_warrant_and_the_facts_beside_it():
    minted = mint()

    # Hex-encoded borsh, so a caller can hand it straight to perform_intent.
    assert isinstance(minted["warrant"], str)
    assert len(minted["warrant"]) > 0
    bytes.fromhex(minted["warrant"])  # raises if it is not hex

    # Read out of the credential rather than taken as an argument, so a caller
    # cannot pass an account inconsistent with the key that signs.
    assert minted["authorAccount"] == ACCOUNT
    assert minted["nonce"] == 1
    assert len(minted["intentHash"]) == 64
    assert minted["notAfter"] > 0


def test_reformatting_the_arguments_cannot_change_the_commitment():
    """The property the whole binding rests on.

    The node recomputes H(method, args) from the argsJson it receives. If this
    helper hashed the caller's literal text, a re-indented or differently-ordered
    body would mint a warrant that verifies nowhere — and the failure would look
    like a forged signature at the node rather than a formatting difference here.
    """
    compact = mint('{"key":"k","value":"v"}')
    spaced = mint('{ "key" : "k" ,\n  "value" : "v" }')
    reordered = mint('{"value":"v","key":"k"}')

    assert compact["intentHash"] == spaced["intentHash"]
    assert compact["intentHash"] == reordered["intentHash"]


def test_different_arguments_commit_differently():
    assert mint('{"key":"k","value":"v"}')["intentHash"] != (
        mint('{"key":"k","value":"w"}')["intentHash"]
    )


def test_a_credential_certifying_another_key_is_refused():
    """Refused at mint time rather than by a peer.

    Such a warrant is well-formed and travels fine; it is rejected on arrival,
    and a silently dropped write is the hardest kind of failure to attribute.
    """
    with pytest.raises(ValueError, match="certifies a different key"):
        mint(device_secret="11" * 32)


def test_both_ids_are_hex_and_base58_is_refused():
    """Base58 is refused for either id.

    This inverts `test_a_context_is_base58_and_an_executor_is_hex`, which pinned
    a context and an account as not interchangeable "though both are 32 bytes".
    They are interchangeable now — both are 64 hex — so nothing at this layer can
    tell a caller they swapped them. What still holds is that the old spelling
    does not slip through, in either position.
    """
    with pytest.raises(ValueError, match="context_id"):
        mint(context_id=CONTEXT_B58)

    with pytest.raises(ValueError, match="executor"):
        mint(executor=CONTEXT_B58)


@pytest.mark.parametrize("bad_args", ["not json", "[1,2", ""])
def test_malformed_arguments_are_named(bad_args):
    with pytest.raises(ValueError, match="args is not valid JSON"):
        mint(args=bad_args)


def test_malformed_credentials_are_named():
    with pytest.raises(ValueError, match="device_secret is not hex"):
        mint(device_secret="zz")

    with pytest.raises(ValueError, match="credential is not a device credential"):
        mint(credential="beef")


def test_valid_for_moves_the_expiry():
    """Checked by the relay at admission, never by peers at apply.

    A wall clock at apply time would make acceptance depend on which node is
    asking, which is divergence rather than security.
    """
    short = mint(valid_for=60)
    long = mint(valid_for=3600)
    assert long["notAfter"] > short["notAfter"]


def test_perform_intent_is_bound_on_the_client():
    """The binding exists and validates its context id without a node.

    A bad id has to fail here naming itself, rather than reaching the node and
    coming back as a 400 that says less.
    """
    connection = create_connection(
        api_url="https://test.merod.dev.p2p.aws.calimero.network",
        node_name="test-dev-node",
    )
    client = create_client(connection)
    assert hasattr(client, "perform_intent")

    with pytest.raises(ValueError, match="Invalid context ID"):
        client.perform_intent("not-a-context", "set", "{}", "aa", "bb")
