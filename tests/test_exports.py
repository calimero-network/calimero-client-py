"""The package exports the Calimero 1.0 client and nothing of v1's.

The v1 client (`create_connection`, `create_client`, v1's `Client`, warrants,
request proofs, `TeePolicy`, the token cache) was deleted with v1's peer-to-peer
node; CHANGELOG.md says what replaces each name. This keeps one from coming back
half-wired. `Client` is a name again, for the 1.0 client: the same class as
`CloudClient`.
"""

import calimero
import calimero_client_py

CLIENT = {
    "Client",
    "CloudClient",
    "CloudError",
    "LoginRequired",
    "SoftPasskey",
    "WrongNode",
}

V1 = {
    "create_connection",
    "create_client",
    "ConnectionInfo",
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


def test_the_extension_exports_the_client():
    for name in CLIENT:
        assert hasattr(calimero_client_py, name), name
    assert isinstance(calimero_client_py.VERSION, str)


def test_the_package_reexports_exactly_the_client():
    assert set(calimero.__all__) == CLIENT
    for name in CLIENT:
        assert getattr(calimero, name) is getattr(calimero_client_py, name)


def test_no_v1_name_is_left():
    for module in (calimero, calimero_client_py):
        left = sorted(n for n in V1 if hasattr(module, n))
        assert not left, f"{module.__name__} still exports {left}"


def test_cloud_error_is_an_exception():
    assert issubclass(calimero_client_py.CloudError, Exception)


def test_login_required_and_wrong_node_are_cloud_errors():
    # Code that catches `CloudError` keeps catching both.
    for name in ("LoginRequired", "WrongNode"):
        assert issubclass(
            getattr(calimero_client_py, name), calimero_client_py.CloudError
        )


def test_the_client_names_the_node_in_use():
    # A property: `base_url` until a `421 WRONG_NODE` moved the client on.
    assert hasattr(calimero_client_py.Client, "node_url")


def test_client_is_the_cloud_client():
    assert calimero_client_py.Client is calimero_client_py.CloudClient
    assert calimero.Client is calimero.CloudClient


# The Ops API calls mdma makes through `CloudClient` (mdma_common.cloud_v2), and
# the rest of core's `calimero-client`: a missing one is a missing binding.
METHODS = {
    # a cluster by name, from the node directory
    "connect",
    # session and account
    "sign_up",
    "sign_in",
    "refresh",
    "sign_out",
    "sign_out_everywhere",
    "use_session",
    "use_api_key",
    "session",
    "me",
    "usage",
    "add_passkey",
    "remove_passkey",
    "label_passkey",
    "create_api_key",
    "list_api_keys",
    "revoke_api_key",
    "set_alias",
    "alias",
    "aliases",
    "delete_alias",
    # packages
    "upload_package",
    "install_package",
    "packages",
    "package",
    "uninstall_package",
    # apps
    "upload_bundle",
    "create_app",
    "create_app_from_package",
    "list_apps",
    "get_app",
    "delete_app",
    "set_access",
    "set_app_metadata",
    "app_storage",
    "app_abi",
    "upgrade_app",
    "transfer_app",
    "verify_app_cluster",
    "call",
    "call_confirmed",
    "view",
    "events",
    "publish_presence",
    "presence",
    "presence_stream",
    "upload_blob",
    "download_blob",
    "list_blobs",
    "blob_info",
    "delete_blob",
    "approve_release",
    "approve_outbound",
    # organizations and teams
    "create_org",
    "get_org",
    "delete_org",
    "set_org_metadata",
    "set_member",
    "set_member_metadata",
    "remove_member",
    "set_capabilities",
    "invite",
    "redeem_invite",
    "create_team",
    "get_team",
    "update_team",
    "delete_team",
    "set_team_member",
    "remove_team_member",
    "upgrade_org",
    "upgrade_run",
    "retry_upgrade_run",
    # Ops API
    "ops_status",
    "ops_metrics",
    "ops_register_node",
    "ops_remove_node",
    "ops_set_replicas",
    "ops_backup",
    "ops_quotas",
    "ops_set_quotas",
    "ops_approve_release",
    "ops_approve_outbound",
    "ops_upgrade_app",
    "ops_upgrade_org",
    "ops_packages",
    "ops_upload_package",
    "ops_install_package",
    "ops_uninstall_package",
    # any path
    "get",
    "get_text",
    "post",
    "put",
    "delete",
    "post_confirmed",
    "delete_confirmed",
    "attestation",
}


def test_the_client_binds_every_method():
    missing = sorted(
        m for m in METHODS if not callable(getattr(calimero_client_py.Client, m, None))
    )
    assert not missing, f"Client lacks {missing}"
