"""
Calimero Client Python Library

A Python client for Calimero Cloud (release-v2) clusters, over core's Rust
`calimero-cloud-client`, built with PyO3.
"""

from importlib.metadata import version as _installed_version

# Declared once, in Cargo.toml.
__version__ = _installed_version("calimero-client-py")
__author__ = "Calimero Network"
__email__ = "team@calimero.network"

from calimero_client_py import (
    CloudClient,
    CloudError,
    SoftPasskey,
)

__all__ = [
    "CloudClient",
    "CloudError",
    "SoftPasskey",
]
