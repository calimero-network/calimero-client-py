"""
The account's sessions (core #4894's `/auth/sessions`) through Client:
`sessions()` lists them and `revoke_session(session)` ends one.

These check the signatures only; the routes themselves are exercised against
a real cluster in test_cloud_v2_api.py (skipped on a node without them).
"""

import inspect

from calimero_client_py import Client


def params(method) -> list:
    return list(inspect.signature(getattr(Client, method)).parameters)


def test_sessions_takes_nothing():
    assert params("sessions")[1:] == []


def test_revoke_session_takes_the_session_id():
    assert params("revoke_session")[1:] == ["session"]


def test_me_documents_the_passkey_times():
    doc = Client.me.__doc__ or ""
    for field in ("passkeys", "addedAt", "lastSignInAt", "passkeyId", "orgs"):
        assert field in doc, field
