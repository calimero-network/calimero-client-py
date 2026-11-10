"""
An organization's members' lockout risk (core's
`GET /orgs/{org}/members?detail=security`) through Client.

This checks the signature only; the route itself is exercised against a real
cluster in test_cloud_v2_api.py (skipped on a node without it).
"""

import inspect

from calimero_client_py import Client


def test_org_members_security_takes_the_org():
    params = list(inspect.signature(Client.org_members_security).parameters)
    assert params[1:] == ["org"]
