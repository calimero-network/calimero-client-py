//! Disk-backed storage implementation for JWT tokens.
//!
//! Stores tokens in `~/.merobox/auth_cache/` with atomic writes and secure permissions.
//!
//! ## Features
//! - Atomic writes using temp file + rename pattern
//! - Secure permissions (0700 for directory, 0600 for files on Unix)
//! - Human-readable + collision-resistant filenames
//! - Proper error handling with context

use std::fs;
use std::io::Write;

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

use calimero_client::traits::ClientStorage;
use calimero_client::JwtToken;
use eyre::WrapErr;

use crate::cache::{get_cache_base_dir, get_token_cache_path_internal};

/// Disk-backed storage implementation for JWT tokens.
#[derive(Clone)]
pub struct MeroboxFileStorage;

impl MeroboxFileStorage {
    pub fn new() -> Self {
        Self
    }

    /// Ensure the cache directory exists with secure permissions (0700 on Unix).
    fn ensure_cache_dir_exists(&self) -> eyre::Result<()> {
        let cache_dir = get_cache_base_dir();
        if !cache_dir.exists() {
            fs::create_dir_all(&cache_dir)
                .wrap_err_with(|| format!("Failed to create cache directory: {:?}", cache_dir))?;

            #[cfg(unix)]
            {
                let permissions = fs::Permissions::from_mode(0o700);
                fs::set_permissions(&cache_dir, permissions).wrap_err_with(|| {
                    format!(
                        "Failed to set permissions on cache directory: {:?}",
                        cache_dir
                    )
                })?;
            }
        }
        Ok(())
    }
}

impl Default for MeroboxFileStorage {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait::async_trait]
impl ClientStorage for MeroboxFileStorage {
    /// Save JWT tokens to disk with atomic write and secure permissions.
    ///
    /// This method:
    /// 1. Ensures the cache directory exists (creating with 0700 permissions if needed)
    /// 2. Writes tokens to a temporary file
    /// 3. Sets file permissions to 0600 (Unix only)
    /// 4. Atomically renames temp file to final path
    async fn save_tokens(&self, node_name: &str, tokens: &JwtToken) -> eyre::Result<()> {
        // Ensure directory exists with proper permissions
        self.ensure_cache_dir_exists()?;

        let cache_path = get_token_cache_path_internal(node_name);
        let temp_path = cache_path.with_extension("json.tmp");

        // Serialize tokens to JSON
        let json = serde_json::to_string_pretty(tokens)
            .wrap_err("Failed to serialize JWT tokens to JSON")?;

        // Write to temp file first (atomic-ish write)
        {
            let mut file = fs::File::create(&temp_path)
                .wrap_err_with(|| format!("Failed to create temp file: {:?}", temp_path))?;
            file.write_all(json.as_bytes())
                .wrap_err_with(|| format!("Failed to write to temp file: {:?}", temp_path))?;
            file.sync_all()
                .wrap_err_with(|| format!("Failed to sync temp file: {:?}", temp_path))?;
        }

        // Set secure permissions on temp file (0600 on Unix)
        #[cfg(unix)]
        {
            let permissions = fs::Permissions::from_mode(0o600);
            fs::set_permissions(&temp_path, permissions).wrap_err_with(|| {
                format!("Failed to set permissions on temp file: {:?}", temp_path)
            })?;
        }

        // Rename temp file to final path (atomic on most filesystems)
        fs::rename(&temp_path, &cache_path).wrap_err_with(|| {
            format!(
                "Failed to rename temp file {:?} to {:?}",
                temp_path, cache_path
            )
        })?;

        Ok(())
    }

    /// Load JWT tokens from disk.
    ///
    /// Returns:
    /// - `Ok(Some(tokens))` if file exists and is valid JSON
    /// - `Ok(None)` if file does not exist
    /// - `Err(...)` if file exists but cannot be read or parsed
    async fn load_tokens(&self, node_name: &str) -> eyre::Result<Option<JwtToken>> {
        let cache_path = get_token_cache_path_internal(node_name);

        // If file doesn't exist, return None (not an error)
        if !cache_path.exists() {
            return Ok(None);
        }

        // Read and deserialize
        let json = fs::read_to_string(&cache_path).wrap_err_with(|| {
            format!(
                "Failed to read token file: {:?} for node: {}",
                cache_path, node_name
            )
        })?;

        let tokens: JwtToken = serde_json::from_str(&json).wrap_err_with(|| {
            format!(
                "Failed to parse token JSON from file: {:?} for node: {}",
                cache_path, node_name
            )
        })?;

        Ok(Some(tokens))
    }

    /// Remove the token file for a given node.
    ///
    /// This overrides the default trait implementation which would save an "empty token".
    /// Instead, we delete the file entirely.
    async fn remove_tokens(&self, node_name: &str) -> eyre::Result<()> {
        let cache_path = get_token_cache_path_internal(node_name);

        // Only try to remove if file exists
        if cache_path.exists() {
            fs::remove_file(&cache_path).wrap_err_with(|| {
                format!(
                    "Failed to remove token file: {:?} for node: {}",
                    cache_path, node_name
                )
            })?;
        }

        Ok(())
    }
}
