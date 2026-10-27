"""The package exports the release-v2 client and nothing of v1's.

The v1 client (`create_connection`, `create_client`, `Client`, warrants, request
proofs, `TeePolicy`, the token cache) was deleted with v1's peer-to-peer node;
CHANGELOG.md says what replaces each name. This keeps one from coming back
half-wired.
"""

import calimero
import calimero_client_py

RELEASE_V2 = {"CloudClient", "CloudError", "SoftPasskey"}

V1 = {
    "create_connection",
    "create_client",
    "ConnectionInfo",
    "Client",
    "JwtToken",
    "ClientError",
    "AuthMode",
    "TeePolicy",
    "get_token_cache_path",
    "get_token_cache_dir",
    "save_token_cache",
    "sign_warrant",
    "sign_request",
}


def test_the_extension_exports_the_release_v2_client():
    for name in RELEASE_V2:
        assert hasattr(calimero_client_py, name), name
    assert isinstance(calimero_client_py.VERSION, str)


def test_the_package_reexports_exactly_the_release_v2_client():
    assert set(calimero.__all__) == RELEASE_V2
    for name in RELEASE_V2:
        assert getattr(calimero, name) is getattr(calimero_client_py, name)


def test_no_v1_name_is_left():
    for module in (calimero, calimero_client_py):
        left = sorted(n for n in V1 if hasattr(module, n))
        assert not left, f"{module.__name__} still exports {left}"


def test_cloud_error_is_an_exception():
    assert issubclass(calimero_client_py.CloudError, Exception)
