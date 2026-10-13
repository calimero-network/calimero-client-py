"""
The client reads a token from stdin only when the caller opts in.

Each case runs in a subprocess with its own HOME and a token waiting on stdin,
then makes an authenticated call with no stored token.
"""

import json
import os
import subprocess
import sys

# The port is closed: authentication happens before the request is sent.
SCRIPT = """
import sys
from calimero_client_py import create_connection

options = {"interactive": True} if "--interactive" in sys.argv else {}
connection = create_connection("http://127.0.0.1:9", node_name="node", **options)
try:
    connection.get("admin-api/contexts")
except RuntimeError as error:
    print(error)
"""


def _call_without_a_token(home, *args):
    return subprocess.run(
        [sys.executable, "-c", SCRIPT, *args],
        input="pasted-token\n\n",
        capture_output=True,
        text=True,
        timeout=60,
        env={**os.environ, "HOME": str(home)},
    )


def test_a_library_caller_is_not_prompted(tmp_path):
    result = _call_without_a_token(tmp_path)
    assert result.returncode == 0, result.stderr
    assert list(tmp_path.rglob("*")) == [], result.stdout


def test_an_interactive_caller_is_prompted_and_the_token_saved(tmp_path):
    result = _call_without_a_token(tmp_path, "--interactive")
    assert result.returncode == 0, result.stderr
    [cache] = tmp_path.rglob("*.json")
    assert json.loads(cache.read_text())["access_token"] == "pasted-token"
