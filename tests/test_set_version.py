"""scripts/set_version.py: the release's version, written for the build."""

import importlib.util
import shutil
from pathlib import Path

import pytest

ROOT = Path(__file__).resolve().parents[1]
spec = importlib.util.spec_from_file_location(
    "set_version", ROOT / "scripts" / "set_version.py"
)
set_version = importlib.util.module_from_spec(spec)
spec.loader.exec_module(set_version)


@pytest.mark.parametrize(
    "semver, pep440",
    [
        ("1.0.0", "1.0.0"),
        ("1.0.0-rc.5", "1.0.0rc5"),
        ("2.1.3-beta.2", "2.1.3b2"),
        ("1.2.0-alpha.1", "1.2.0a1"),
    ],
)
def test_pep440(semver, pep440):
    assert set_version.pep440(semver) == pep440


@pytest.mark.parametrize("bad", ["v1.0.0", "1.0", "1.0.0-rc5", "1.0.0-next.1", ""])
def test_refuses_what_is_not_a_release(bad):
    with pytest.raises(ValueError):
        set_version.pep440(bad)


def test_writes_the_crate_version_only(tmp_path):
    shutil.copy(ROOT / "Cargo.toml", tmp_path / "Cargo.toml")
    shutil.copy(ROOT / "Cargo.lock", tmp_path / "Cargo.lock")
    before_toml = (tmp_path / "Cargo.toml").read_text()
    before_lock = (tmp_path / "Cargo.lock").read_text()
    assert set_version.set_version(tmp_path, "9.8.7-rc.6") == "9.8.7rc6"
    toml = (tmp_path / "Cargo.toml").read_text()
    lock = (tmp_path / "Cargo.lock").read_text()
    assert '\nversion = "9.8.7-rc.6"\n' in toml
    # One line changed in each file: the crate's own version.
    changed = [
        (a, b) for a, b in zip(before_toml.splitlines(), toml.splitlines()) if a != b
    ]
    assert len(changed) == 1 and changed[0][1] == 'version = "9.8.7-rc.6"'
    changed = [
        (a, b) for a, b in zip(before_lock.splitlines(), lock.splitlines()) if a != b
    ]
    assert len(changed) == 1 and changed[0][1] == 'version = "9.8.7-rc.6"'
    assert lock.count('name = "calimero-client-py"\nversion = "9.8.7-rc.6"') == 1
