"""The three places this package states its version must agree.

`pyproject.toml` is the one that matters operationally: the publish workflow
compares it against the previous commit's and skips PyPI when it has not changed.
So a release that bumps `Cargo.toml` and `calimero/__init__.py` but not this one
builds, passes CI, reports success — and publishes nothing. That happened to
0.6.20, and a stale `__version__` (0.3.0 against a 0.6.x package) had to be fixed
in 0.6.19 for the same reason: nothing was checking.
"""

import pathlib
import re

ROOT = pathlib.Path(__file__).resolve().parent.parent


def _pyproject_version() -> str:
    text = (ROOT / "pyproject.toml").read_text()
    # The FIRST top-level `version = ` under [project]; dependency pins and the
    # [tool.*] sections carry their own and must not be picked up.
    match = re.search(r"^version = \"([^\"]+)\"", text, re.M)
    assert match, "no top-level version in pyproject.toml"
    return match.group(1)


def _cargo_version() -> str:
    text = (ROOT / "Cargo.toml").read_text()
    match = re.search(r"^version = \"([^\"]+)\"", text, re.M)
    assert match, "no top-level version in Cargo.toml"
    return match.group(1)


def _dunder_version() -> str:
    text = (ROOT / "calimero" / "__init__.py").read_text()
    match = re.search(r"^__version__ = \"([^\"]+)\"", text, re.M)
    assert match, "no __version__ in calimero/__init__.py"
    return match.group(1)


def test_all_three_version_declarations_agree():
    pyproject, cargo, dunder = (
        _pyproject_version(),
        _cargo_version(),
        _dunder_version(),
    )
    assert pyproject == cargo == dunder, (
        "version drift: "
        f"pyproject.toml={pyproject}, Cargo.toml={cargo}, __init__.py={dunder}. "
        "The publish workflow gates on pyproject.toml, so a bump that misses it "
        "ships nothing while every check stays green."
    )
