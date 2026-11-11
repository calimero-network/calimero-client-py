#!/usr/bin/env python3
"""Generate calimero_client_py/openapi and calimero_client_py/facade.py.

`openapi` is openapi-python-client's output for core's OpenAPI document
(vendored by scripts/refresh_openapi.py); `facade.py`, one method per
operation on `Client` (`client.orgs.get(org=...)`), is
scripts/generate_facade.py's from the document and that output. Both are
formatted with black and checked in, and nothing in them is edited by hand:
change the document (in core), these scripts or openapi/generator.yaml, and
run this again. CI runs it with `--check`.

    pip install openapi-python-client==0.26.2 black==26.10.1
    python scripts/generate_openapi_client.py           # regenerate
    python scripts/generate_openapi_client.py --check   # fail if it differs

Before generating, the document is read with three changes, none of which
alters a route, a parameter or a field:

- an error answer's schema is the shared `Error` (`{status, error, message,
  details}`) rather than each operation's `allOf` of it with the codes that
  operation raises (those codes stay in the answer's description). Each
  `allOf` made a model per operation and status, twice the code for the same
  four fields; the sealed transport raises errors from the shared shape.
- a string schema with a `contentMediaType` other than JSON (an upload, a
  blob) is marked `format: binary`, OpenAPI 3.0's spelling, which this
  generator reads; with only 3.1's `contentMediaType` it types the bytes as
  `str` and fails on them.
- `application/wasm` (`POST /bundles`) is read as bytes
  (openapi/generator.yaml).
"""

import argparse
import copy
import filecmp
import json
import os
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
DOCUMENT = ROOT / "openapi" / "openapi.json"
SOURCE = ROOT / "openapi" / "source.json"
CONFIG = ROOT / "openapi" / "generator.yaml"
GENERATED = ROOT / "calimero_client_py" / "openapi"
FACADE = ROOT / "calimero_client_py" / "facade.py"

# Exact: the output is checked in, so a generator release that changes it is
# a change to review. 0.26.2 is the last release whose code supports Python
# 3.9 (`Union`/`Optional`; 0.27 on needs 3.10, 0.29 3.11), which this package
# still does.
GENERATOR = "0.26.2"
ERROR = {"$ref": "#/components/schemas/Error"}
JSON_TYPES = ("application/json",)


def prepared(document: dict) -> dict:
    """The document as the generator is given it (see the module's doc)."""
    document = copy.deepcopy(document)

    def mark_binary(node):
        if isinstance(node, dict):
            media = node.get("contentMediaType")
            if (
                node.get("type") == "string"
                and media
                and media not in JSON_TYPES
                and "format" not in node
            ):
                node["format"] = "binary"
            for value in node.values():
                mark_binary(value)
        elif isinstance(node, list):
            for value in node:
                mark_binary(value)

    mark_binary(document)
    for item in document.get("paths", {}).values():
        for operation in item.values():
            if not isinstance(operation, dict):
                continue
            for status, answer in operation.get("responses", {}).items():
                if status.startswith("2") or "content" not in answer:
                    continue
                answer["content"] = {"application/json": {"schema": dict(ERROR)}}
    return document


def generator_version() -> str:
    try:
        from openapi_python_client import __version__
    except ImportError:
        return ""
    return __version__


def generate(tree: Path) -> None:
    """Generate `tree/calimero_client_py/openapi` and
    `tree/calimero_client_py/facade.py` (scripts/generate_facade.py, which
    reads the generated calls from `tree`)."""
    source = json.loads(SOURCE.read_text())
    package = tree / "calimero_client_py"
    package.mkdir(parents=True)
    # Only so the facade generator can import the generated package from here.
    (package / "__init__.py").write_text("")
    with tempfile.TemporaryDirectory() as tmp:
        spec = Path(tmp) / "openapi.json"
        spec.write_text(json.dumps(prepared(json.loads(DOCUMENT.read_text()))))
        out = package / "openapi"
        subprocess.run(
            [
                sys.executable,
                "-m",
                "openapi_python_client",
                "generate",
                "--path",
                str(spec),
                "--config",
                str(CONFIG),
                "--meta",
                "none",
                "--output-path",
                str(out),
                "--fail-on-warning",
            ],
            check=True,
            # The generator orders some imports by iterating a set: a fixed
            # hash seed makes its output the same from run to run.
            env={**os.environ, "PYTHONHASHSEED": "0"},
        )
        init = out / "__init__.py"
        init.write_text(
            f'"""Core\'s node API, generated: do not edit.\n\n'
            f"Generated by openapi-python-client {GENERATOR} from openapi/openapi.json\n"
            f"({source['repository']} at {source['commit']}, {source['path']})\n"
            f"by scripts/generate_openapi_client.py. Every route but `/health` and\n"
            f"`/attestation` is sealed: run these calls on a client from\n"
            f"`calimero_client_py.Client(...).api()`, whose transport seals them,\n"
            f"or call them as `Client`'s methods (`client.orgs.get(org=...)`,\n"
            f"`calimero_client_py.facade`).\n"
            f'"""\n\n' + init.read_text().split('"""', 2)[2].lstrip()
        )
        subprocess.run(
            [
                sys.executable,
                str(ROOT / "scripts" / "generate_facade.py"),
                "--document",
                str(DOCUMENT),
                "--tree",
                str(tree),
                "--out",
                str(package / "facade.py"),
            ],
            check=True,
        )
        subprocess.run(
            [
                sys.executable,
                "-m",
                "black",
                "--quiet",
                "--config",
                str(ROOT / "pyproject.toml"),
                str(out),
                str(package / "facade.py"),
            ],
            check=True,
        )


def same_tree(a: Path, b: Path) -> bool:
    cmp = filecmp.dircmp(a, b, ignore=["__pycache__"])
    if cmp.left_only or cmp.right_only or cmp.funny_files:
        return False
    _, mismatch, errors = filecmp.cmpfiles(a, b, cmp.common_files, shallow=False)
    if mismatch or errors:
        return False
    return all(same_tree(a / d, b / d) for d in cmp.common_dirs)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument(
        "--check",
        action="store_true",
        help="generate into a temporary directory and fail if it differs",
    )
    args = parser.parse_args()

    found = generator_version()
    if found != GENERATOR:
        print(
            f"needs openapi-python-client=={GENERATOR} (found {found or 'none'}): "
            f"pip install openapi-python-client=={GENERATOR}",
            file=sys.stderr,
        )
        return 1

    with tempfile.TemporaryDirectory() as tmp:
        fresh = Path(tmp) / "tree" / "calimero_client_py"
        generate(fresh.parent)
        if not args.check:
            if GENERATED.exists():
                shutil.rmtree(GENERATED)
            shutil.copytree(fresh / "openapi", GENERATED)
            shutil.copyfile(fresh / "facade.py", FACADE)
            print(
                f"generated {GENERATED.relative_to(ROOT)} and {FACADE.relative_to(ROOT)}"
            )
            return 0
        if not same_tree(fresh / "openapi", GENERATED) or not filecmp.cmp(
            fresh / "facade.py", FACADE, shallow=False
        ):
            print(
                f"{GENERATED.relative_to(ROOT)} or {FACADE.relative_to(ROOT)} is not "
                "what openapi/openapi.json generates: run "
                "scripts/generate_openapi_client.py",
                file=sys.stderr,
            )
            return 1
    print(
        f"{GENERATED.relative_to(ROOT)} and {FACADE.relative_to(ROOT)} are up to date"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
