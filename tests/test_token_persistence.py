#!/usr/bin/env python3
"""
Tests for token persistence functionality.

These tests verify:
1. Token cache path utilities work correctly
2. Filename derivation is stable (same input → same output)
3. Different node names produce different cache paths
4. Cache paths are valid filesystem paths
"""

import os
import json
import tempfile

from calimero_client_py import (
    get_token_cache_path,
    get_token_cache_dir,
)


class TestTokenCachePathUtilities:
    """Tests for get_token_cache_path and get_token_cache_dir functions."""

    def test_get_token_cache_dir_returns_string(self):
        """Test that get_token_cache_dir returns a string path."""
        cache_dir = get_token_cache_dir()
        assert isinstance(cache_dir, str)
        assert len(cache_dir) > 0

    def test_get_token_cache_dir_contains_merobox(self):
        """Test that cache dir is under .merobox/auth_cache."""
        cache_dir = get_token_cache_dir()
        assert ".merobox" in cache_dir
        assert "auth_cache" in cache_dir

    def test_get_token_cache_path_returns_string(self):
        """Test that get_token_cache_path returns a string path."""
        path = get_token_cache_path("test-node")
        assert isinstance(path, str)
        assert len(path) > 0

    def test_get_token_cache_path_ends_with_json(self):
        """Test that cache path has .json extension."""
        path = get_token_cache_path("test-node")
        assert path.endswith(".json")

    def test_get_token_cache_path_under_cache_dir(self):
        """Test that cache path is under the cache directory."""
        cache_dir = get_token_cache_dir()
        cache_path = get_token_cache_path("test-node")
        assert cache_path.startswith(cache_dir)


class TestFilenameDerivationStability:
    """Tests for filename derivation stability."""

    def test_same_input_same_output(self):
        """Test that the same node_name always produces the same path."""
        node_name = "my-stable-node"
        path1 = get_token_cache_path(node_name)
        path2 = get_token_cache_path(node_name)
        path3 = get_token_cache_path(node_name)

        assert path1 == path2 == path3, "Same input should always produce same output"

    def test_different_inputs_different_outputs(self):
        """Test that different node names produce different paths."""
        path1 = get_token_cache_path("node-alpha")
        path2 = get_token_cache_path("node-beta")
        path3 = get_token_cache_path("node-gamma")

        paths = {path1, path2, path3}
        assert len(paths) == 3, "Different inputs should produce different outputs"

    def test_url_like_node_names(self):
        """Test that URL-like node names work and are stable."""
        node_name = "https://my-node.example.com:8080/api"
        path1 = get_token_cache_path(node_name)
        path2 = get_token_cache_path(node_name)

        assert path1 == path2, "URL-like names should be stable"
        assert path1.endswith(".json")

    def test_special_characters_sanitized(self):
        """Test that special characters in node names are handled."""
        # These should all work without errors
        special_names = [
            "node with spaces",
            "node:with:colons",
            "node/with/slashes",
            "node?with=query&params",
            "node#with#hashes",
            "emoji-node-🚀",
            "../../../etc/passwd",  # Path traversal attempt
            "very" * 100,  # Very long name
        ]

        paths = []
        for name in special_names:
            path = get_token_cache_path(name)
            assert isinstance(path, str)
            assert path.endswith(".json")
            # Ensure no path traversal - path should be under cache dir
            cache_dir = get_token_cache_dir()
            assert path.startswith(cache_dir)
            paths.append(path)

        # All paths should be unique
        assert len(set(paths)) == len(
            paths
        ), "All special names should produce unique paths"

    def test_empty_string_node_name(self):
        """Test that empty string node name works."""
        path = get_token_cache_path("")
        assert isinstance(path, str)
        assert path.endswith(".json")


class TestFilenameFormat:
    """Tests for the expected filename format: {slug}-{hash}.json"""

    def test_filename_contains_hash_suffix(self):
        """Test that filename contains a hash-like suffix."""
        path = get_token_cache_path("test-node")
        filename = os.path.basename(path)

        # Remove .json extension
        name_without_ext = filename[:-5]

        # Should contain a dash separating slug from hash
        assert "-" in name_without_ext

    def test_filename_has_reasonable_length(self):
        """Test that filename length is reasonable."""
        # Test with a very long node name
        long_name = "a" * 1000
        path = get_token_cache_path(long_name)
        filename = os.path.basename(path)

        # Filename should be truncated to a reasonable length
        # slug (max 64) + dash + hash (12) + .json (5) = max ~82 chars
        assert len(filename) < 100, f"Filename too long: {len(filename)}"

    def test_filename_is_valid_for_filesystem(self):
        """Test that generated filenames are valid for the filesystem."""
        test_names = [
            "simple",
            "with-dashes",
            "with_underscores",
            "with.dots",
            "https://example.com:8080",
            "node name with spaces",
        ]

        for name in test_names:
            path = get_token_cache_path(name)
            filename = os.path.basename(path)

            # Check for invalid filesystem characters (Windows-focused)
            invalid_chars = '<>:"|?*'
            for char in invalid_chars:
                assert (
                    char not in filename
                ), f"Invalid char '{char}' in filename: {filename}"


class TestTokenCacheIntegration:
    """Integration tests for token caching with actual file operations."""

    def test_can_write_to_cache_path(self):
        """Test that we can actually write to a generated cache path."""
        with tempfile.TemporaryDirectory() as temp_dir:
            # We can't change the cache directory, but we can verify
            # the path format is valid by creating a similar structure
            node_name = "integration-test-node"
            path = get_token_cache_path(node_name)
            filename = os.path.basename(path)

            # Create a test file with the same filename in temp dir
            test_path = os.path.join(temp_dir, filename)

            # Write a mock token
            token_data = {
                "access_token": "test_access_token_12345",
                "refresh_token": "test_refresh_token_67890",
                "expires_at": 1234567890,
            }

            with open(test_path, "w", encoding="utf-8") as f:
                json.dump(token_data, f)

            # Read it back
            with open(test_path, "r", encoding="utf-8") as f:
                loaded = json.load(f)

            assert loaded == token_data

    def test_json_roundtrip(self):
        """Test that token JSON can be round-tripped through a file."""
        with tempfile.TemporaryDirectory() as temp_dir:
            filename = os.path.basename(get_token_cache_path("roundtrip-test"))
            test_path = os.path.join(temp_dir, filename)

            # Create token data matching JwtToken structure
            original_token = {
                "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.test",
                "refresh_token": "refresh_token_value",
                "expires_at": 1735689600,  # Some future timestamp
            }

            # Write
            with open(test_path, "w", encoding="utf-8") as f:
                json.dump(original_token, f, indent=2)

            # Read
            with open(test_path, "r", encoding="utf-8") as f:
                loaded_token = json.load(f)

            # Verify
            assert loaded_token["access_token"] == original_token["access_token"]
            assert loaded_token["refresh_token"] == original_token["refresh_token"]
            assert loaded_token["expires_at"] == original_token["expires_at"]

    def test_remove_token_file(self):
        """Test that token files can be removed."""
        with tempfile.TemporaryDirectory() as temp_dir:
            filename = os.path.basename(get_token_cache_path("remove-test"))
            test_path = os.path.join(temp_dir, filename)

            # Create file
            with open(test_path, "w", encoding="utf-8") as f:
                json.dump({"access_token": "test"}, f)

            assert os.path.exists(test_path)

            # Remove file
            os.remove(test_path)

            assert not os.path.exists(test_path)


class TestNodeNameBestPractices:
    """Tests demonstrating node_name best practices."""

    def test_stable_node_name_example(self):
        """Demonstrate that stable node names are important."""
        # Good: Use the same node_name across sessions
        session1_path = get_token_cache_path("my-production-server")
        session2_path = get_token_cache_path("my-production-server")

        assert (
            session1_path == session2_path
        ), "Stable node_name ensures same token file is used across sessions"

    def test_unique_node_names_for_different_servers(self):
        """Demonstrate using unique names for different servers."""
        # Good: Different servers get different names
        prod_path = get_token_cache_path("prod-api.example.com")
        dev_path = get_token_cache_path("dev-api.example.com")
        test_path = get_token_cache_path("test-dev-node")

        paths = {prod_path, dev_path, test_path}
        assert (
            len(paths) == 3
        ), "Different servers should have different node_names to avoid token collision"
