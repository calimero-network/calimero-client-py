"""
Sealed files (core's `/apps/{app}/files` routes) through Client.

These check the signatures only; the routes themselves need a node with a
files-component app, which the kv-store fixture is not.
"""

import inspect

import pytest

from calimero_client_py import Client


def params(method) -> list:
    return list(inspect.signature(getattr(Client, method)).parameters)


def test_upload_file_takes_the_app_and_a_path_or_bytes():
    assert params("upload_file")[1:] == ["app", "file"]


def test_list_files_takes_the_app():
    assert params("list_files")[1:] == ["app"]


def test_get_file_takes_the_app_and_the_file_id():
    assert params("get_file")[1:] == ["app", "id"]


def test_upload_file_documents_the_answer():
    doc = Client.upload_file.__doc__ or ""
    for field in ("id", "blobId", "size", "sha256", "index"):
        assert field in doc, field


def test_upload_file_refuses_anything_but_a_path_or_bytes():
    client = Client("http://127.0.0.1:1")
    with pytest.raises(TypeError):
        Client.upload_file(client, "app", 42)
