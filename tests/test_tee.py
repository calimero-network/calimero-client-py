#!/usr/bin/env python3
"""Tests for the attested transports to a TEE node.

A node's quote proves what runs inside its TD, and nothing about who reads the
traffic on the way there, unless the client uses a key the quote commits to:
`attested_tls` pins the TLS key the TD serves, `sealed` encrypts every request
to the node's attested transport key.

None of this needs a node. The pinning runs against a local HTTPS server with a
throwaway self-signed certificate, which no certificate authority would accept:
the pin is the only thing that can make the request go through. The sealing
runs against a plain server that does not seal, to show that a sealed
connection refuses rather than falls back to sending in the clear. Sealed
requests against a real node, and quote verification, are covered by core's
`calimero-client` tests, which this binds.
"""

import http.server
import json
import shutil
import ssl
import subprocess
import threading

import pytest
from calimero_client_py import TeePolicy, create_connection

MRTD = "aa" * 48
KEY = "11" * 32


# ---- TeePolicy ----


def test_a_policy_pins_the_image_it_trusts():
    policy = TeePolicy([MRTD])
    assert policy.allowed_mrtd == [MRTD]


def test_a_policy_that_could_accept_nothing_is_refused():
    with pytest.raises(ValueError, match="allowed_mrtd is empty"):
        TeePolicy([])


def test_a_measurement_that_is_not_one_is_refused():
    with pytest.raises(ValueError, match="allowed_mrtd"):
        TeePolicy(["aabb"])
    with pytest.raises(ValueError, match="allowed_rtmr1"):
        TeePolicy([MRTD], allowed_rtmr1=["not hex"])


def test_an_application_is_named_with_the_hash_it_must_run():
    app = "22" * 32
    with pytest.raises(ValueError, match="together"):
        TeePolicy([MRTD], application_id=app)
    TeePolicy([MRTD], application_id=app, application_hash="33" * 32)


# ---- combinations that cannot do what they say ----


@pytest.mark.parametrize(
    "kwargs,message",
    [
        ({"transport_public_key": KEY}, "needs sealed=True"),
        ({"tls_spki_sha256": KEY}, "needs attested_tls=True"),
        ({"tee": TeePolicy([MRTD])}, "neither was asked for"),
        ({"sealed": True}, "sealed=True needs tee"),
        ({"attested_tls": True}, "attested_tls=True needs tee"),
    ],
    ids=["key-without-seal", "pin-without-tls", "policy-unused", "seal-no-key", "tls-no-key"],
)
def test_a_protection_that_would_not_run_is_refused(kwargs, message):
    """Each would leave the caller believing traffic is protected when it is not."""
    with pytest.raises(ValueError, match=message):
        create_connection("https://node.example", **kwargs)


def test_attested_tls_needs_https():
    with pytest.raises(ValueError, match="https"):
        create_connection("http://node.example", attested_tls=True, tls_spki_sha256=KEY)


def test_a_malformed_key_says_which_argument_it_was():
    with pytest.raises(ValueError, match="transport_public_key"):
        create_connection("http://node.example", sealed=True, transport_public_key="aabb")
    with pytest.raises(ValueError, match="tls_spki_sha256"):
        create_connection("https://node.example", attested_tls=True, tls_spki_sha256="zz")


# ---- against a local server ----


class _Recorder(http.server.BaseHTTPRequestHandler):
    """Answers every path with a small JSON body, remembering what it saw."""

    seen = []

    def _answer(self):
        _Recorder.seen.append((self.command, self.path))
        body = json.dumps({"data": {"status": "alive"}}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    do_GET = _answer
    do_POST = _answer

    def log_message(self, *_args):
        pass


def _serve(context=None):
    server = http.server.ThreadingHTTPServer(("127.0.0.1", 0), _Recorder)
    if context is not None:
        server.socket = context.wrap_socket(server.socket, server_side=True)
    threading.Thread(target=server.serve_forever, daemon=True).start()
    return server


@pytest.fixture
def tls_server(tmp_path):
    """An HTTPS server with a fresh self-signed P-256 key, and that key's pin."""
    if shutil.which("openssl") is None:
        pytest.skip("openssl is needed to make a certificate")
    key, cert = tmp_path / "key.pem", tmp_path / "cert.pem"
    subprocess.run(
        ["openssl", "req", "-x509", "-newkey", "ec", "-pkeyopt",
         "ec_paramgen_curve:prime256v1", "-nodes", "-keyout", str(key),
         "-out", str(cert), "-days", "1", "-subj", "/CN=calimero-fleet-node"],
        check=True, capture_output=True,
    )  # fmt: skip
    # The pin as ordinary tools compute it: SHA-256 of the SubjectPublicKeyInfo.
    pubkey = subprocess.run(
        ["openssl", "x509", "-in", str(cert), "-pubkey", "-noout"],
        check=True, capture_output=True,
    ).stdout  # fmt: skip
    der = subprocess.run(
        ["openssl", "pkey", "-pubin", "-outform", "der"],
        input=pubkey, check=True, capture_output=True,
    ).stdout  # fmt: skip
    import hashlib

    context = ssl.SSLContext(ssl.PROTOCOL_TLS_SERVER)
    context.load_cert_chain(cert, key)
    server = _serve(context)
    yield f"https://127.0.0.1:{server.server_address[1]}", hashlib.sha256(der).hexdigest()
    server.shutdown()


def test_the_pinned_key_is_accepted_where_no_authority_would_be(tls_server):
    url, pin = tls_server
    conn = create_connection(url, attested_tls=True, tls_spki_sha256=pin)
    assert conn.get("admin-api/health") == {"data": {"status": "alive"}}

    # Without the pin, the self-signed certificate reached by IP is refused.
    with pytest.raises(RuntimeError):
        create_connection(url).get("admin-api/health")


def test_a_server_without_the_pinned_key_is_refused(tls_server):
    url, pin = tls_server
    other = "%064x" % (int(pin, 16) ^ 1)
    conn = create_connection(url, attested_tls=True, tls_spki_sha256=other)
    with pytest.raises(RuntimeError):
        conn.get("admin-api/health")


def test_a_node_that_does_not_attest_is_refused_while_connecting(tls_server):
    """With a policy, the node attests while the connection is built.

    This server has no attestation endpoint worth the name: it answers the
    request with a body that is no attestation at all, so nothing is pinned.
    """
    url, _ = tls_server
    with pytest.raises(RuntimeError, match="attestation"):
        create_connection(url, tee=TeePolicy([MRTD]), attested_tls=True)


def test_a_sealed_connection_never_falls_back_to_the_clear():
    server = _serve()
    try:
        _Recorder.seen.clear()
        url = f"http://127.0.0.1:{server.server_address[1]}"
        conn = create_connection(url, sealed=True, transport_public_key=KEY)
        with pytest.raises(RuntimeError, match="sealed request refused"):
            conn.get("admin-api/contexts")
        # The server saw a handshake it could not answer, and never the request.
        assert ("GET", "/admin-api/contexts") not in _Recorder.seen
        assert ("POST", "/sealed/v2/handshake") in _Recorder.seen
    finally:
        server.shutdown()
