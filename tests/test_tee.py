#!/usr/bin/env python3
"""Tests for sealed requests to a TEE node.

A node's quote proves what runs inside its TD, and nothing about who reads the
traffic on the way there, unless the client uses a key the quote commits to.
`sealed=True` encrypts every request to the node's attested transport key.

None of this needs a node. What is under test here is the Python surface: the
policy and the combinations that cannot work are refused, and a sealed
connection never falls back to sending in the clear. Sealed requests against
the node's own sealed transport, restarts included, are covered by core's
`calimero-client` tests, which this binds.
"""

import subprocess
import sys

import pytest
from calimero_client_py import TeePolicy, create_connection

MRTD = "aa" * 48
KEY = "11" * 32


# ---- TeePolicy ----


def test_a_policy_pins_the_image_it_trusts():
    assert TeePolicy([MRTD]).allowed_mrtd == [MRTD]


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
        ({"tee": TeePolicy([MRTD])}, "needs sealed=True"),
        ({"sealed": True}, "sealed=True needs tee"),
        (
            {"sealed": True, "tee": TeePolicy([MRTD]), "transport_public_key": KEY},
            "not both",
        ),
    ],
    ids=["key-without-seal", "policy-without-seal", "seal-no-key", "both-sources"],
)
def test_a_protection_that_would_not_run_is_refused(kwargs, message):
    """Each would leave the caller believing traffic is protected when it is not."""
    with pytest.raises(ValueError, match=message):
        create_connection("https://node.example", **kwargs)


def test_a_malformed_key_says_which_argument_it_was():
    with pytest.raises(ValueError, match="transport_public_key"):
        create_connection(
            "http://node.example", sealed=True, transport_public_key="aabb"
        )


def test_a_sealed_connection_with_a_policy_builds_without_contacting_the_node():
    """The node attests on the first request, not while the connection is built."""
    conn = create_connection("http://127.0.0.1:9", tee=TeePolicy([MRTD]), sealed=True)
    assert conn.api_url.startswith("http://127.0.0.1:9")


# ---- against a local server that does not seal ----

# A server that answers every request with a small JSON body and records what it
# saw. It runs in its own process: a connection's calls hold the GIL while they
# wait for the answer, so a server on a thread of this process could never give
# one.
_RECORDER = """
import http.server, json, sys
log = open(sys.argv[1], "a", buffering=1)

class Recorder(http.server.BaseHTTPRequestHandler):
    def _answer(self):
        log.write(f"{self.command} {self.path}\\n")
        body = json.dumps({"data": {"status": "alive"}}).encode()
        self.send_response(200)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    do_GET = do_POST = _answer

    def log_message(self, *_args):
        pass

server = http.server.HTTPServer(("127.0.0.1", 0), Recorder)
print(server.server_address[1], flush=True)
server.serve_forever()
"""


@pytest.fixture
def recorder(tmp_path):
    """The server's URL, and a function returning the requests it saw."""
    log = tmp_path / "seen"
    log.touch()
    server = subprocess.Popen(
        [sys.executable, "-c", _RECORDER, str(log)], stdout=subprocess.PIPE, text=True
    )
    try:
        port = int(server.stdout.readline())
        yield f"http://127.0.0.1:{port}", lambda: log.read_text().splitlines()
    finally:
        server.kill()
        server.wait()


def test_a_sealed_connection_never_falls_back_to_the_clear(recorder):
    url, seen = recorder
    conn = create_connection(url, sealed=True, transport_public_key=KEY)
    with pytest.raises(RuntimeError, match="sealed request refused"):
        conn.get("admin-api/contexts")
    # The server saw a handshake it could not answer, and never the request.
    assert seen() == ["POST /sealed/v2/handshake"]


def test_a_node_whose_quote_does_not_verify_gets_nothing(recorder):
    """With a policy, the request is sent only after the node's quote verifies.

    This server answers the attestation with something that is no attestation,
    so nothing is sealed and nothing is sent.
    """
    url, seen = recorder
    conn = create_connection(url, tee=TeePolicy([MRTD]), sealed=True)
    with pytest.raises(RuntimeError, match="attestation"):
        conn.get("admin-api/contexts")
    assert seen() == ["POST /admin-api/tee/attest"]
