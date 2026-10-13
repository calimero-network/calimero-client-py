import pytest

from check_workflow_hardening import violations

SHA = "a" * 40

WORKFLOW = """
{permissions}
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
    - uses: {uses}
"""


def check(
    tmp_path,
    permissions="permissions:\n  contents: read",
    uses=f"actions/checkout@{SHA}",
):
    workflow = tmp_path / "ci.yml"
    workflow.write_text(WORKFLOW.format(permissions=permissions, uses=uses))
    return violations(workflow)


def test_a_workflow_without_permissions_is_refused(tmp_path):
    assert check(tmp_path, permissions="")


@pytest.mark.parametrize(
    "uses", ["actions/checkout@v7", "dtolnay/rust-toolchain@stable", "a/b@" + SHA[:7]]
)
def test_a_mutable_ref_is_refused(tmp_path, uses):
    assert check(tmp_path, uses=uses)


@pytest.mark.parametrize("uses", [f"actions/checkout@{SHA} # v7", "./local-action"])
def test_a_pinned_or_local_action_passes(tmp_path, uses):
    assert check(tmp_path, uses=uses) == []
