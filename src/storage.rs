//! Disk-backed storage implementation for JWT tokens.
//!
//! Stores tokens in `~/.merobox/auth_cache/` with atomic writes and secure permissions.
//!
//! ## Features
//! - Atomic writes using temp file + rename pattern
//! - Secure permissions (0700 for directory, 0600 for files on Unix), tightened if wider
//! - Human-readable + collision-resistant filenames, bound to the node's base URL
//! - Proper error handling with context

use std::collections::hash_map::RandomState;
use std::fs;
use std::fs::OpenOptions;
use std::hash::BuildHasher;
use std::io::Write;
use std::path::{Path, PathBuf};

#[cfg(unix)]
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};

use calimero_client::traits::ClientStorage;
use calimero_client::JwtToken;
use eyre::WrapErr;
use url::Url;

use crate::cache::{derive_token_filename, get_cache_base_dir, token_cache_base};

const CACHE_DIR_MODE: u32 = 0o700;
const CACHE_FILE_MODE: u32 = 0o600;

/// Guard that ensures a temp file is cleaned up if the operation fails.
/// The file is only removed if `commit()` is not called before drop.
struct TempFileGuard {
    path: PathBuf,
    committed: bool,
}

impl TempFileGuard {
    fn new(path: PathBuf) -> Self {
        Self {
            path,
            committed: false,
        }
    }

    /// Mark the temp file as successfully committed (renamed to final path).
    /// After calling this, the guard will not remove the file on drop.
    fn commit(mut self) {
        self.committed = true;
        // self is moved, so drop won't be called
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if !self.committed && self.path.exists() {
            // Best effort cleanup - ignore errors since we're in drop
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// Disk-backed storage implementation for JWT tokens, for one node base URL.
#[derive(Clone)]
pub struct MeroboxFileStorage {
    base: String,
    /// `None` is the home cache, resolved per call so connections that never
    /// cache tokens do not need a home directory.
    dir: Option<PathBuf>,
}

impl MeroboxFileStorage {
    pub fn new(api_url: &Url) -> Self {
        Self {
            base: token_cache_base(api_url),
            dir: None,
        }
    }

    fn cache_dir(&self) -> eyre::Result<PathBuf> {
        self.dir.clone().map_or_else(get_cache_base_dir, Ok)
    }

    pub fn path(&self, node_name: &str) -> eyre::Result<PathBuf> {
        Ok(self
            .cache_dir()?
            .join(derive_token_filename(node_name, &self.base)))
    }

    /// Ensure the cache directory exists with secure permissions (0700 on Unix).
    ///
    /// Uses `DirBuilderExt::mode` on Unix to create with correct permissions atomically,
    /// avoiding a TOCTOU race between create and set_permissions.
    fn ensure_cache_dir_exists(&self) -> eyre::Result<()> {
        let cache_dir = self.cache_dir()?;
        if cache_dir.exists() {
            restrict_mode(&cache_dir, CACHE_DIR_MODE)
        } else {
            Self::create_cache_dir(&cache_dir)
        }
    }

    #[cfg(unix)]
    fn create_cache_dir(cache_dir: &std::path::Path) -> eyre::Result<()> {
        use std::os::unix::fs::DirBuilderExt;
        fs::DirBuilder::new()
            .mode(CACHE_DIR_MODE)
            .recursive(true)
            .create(cache_dir)
            .wrap_err_with(|| format!("Failed to create cache directory: {:?}", cache_dir))
    }

    #[cfg(not(unix))]
    fn create_cache_dir(cache_dir: &std::path::Path) -> eyre::Result<()> {
        fs::create_dir_all(cache_dir)
            .wrap_err_with(|| format!("Failed to create cache directory: {:?}", cache_dir))
    }

    /// Save JWT tokens to disk with atomic write and secure permissions.
    ///
    /// This method:
    /// 1. Ensures the cache directory exists (creating with 0700 permissions if needed)
    /// 2. Creates a fresh temp file with 0600 permissions (Unix) via OpenOptionsExt::mode
    /// 3. Writes tokens and syncs
    /// 4. Atomically renames temp file to final path
    pub fn save(&self, node_name: &str, tokens: &JwtToken) -> eyre::Result<()> {
        // Ensure directory exists with proper permissions
        self.ensure_cache_dir_exists()?;

        let cache_path = self.path(node_name)?;
        // Unique per save, so concurrent writers never share or reuse a temp file.
        let temp_path = cache_path.with_extension(format!(
            "json.{}.{:016x}.tmp",
            std::process::id(),
            RandomState::new().hash_one(())
        ));

        // Create guard to ensure temp file is cleaned up on error
        let _guard = TempFileGuard::new(temp_path.clone());

        // Serialize tokens to JSON
        let json = serde_json::to_string_pretty(tokens)
            .wrap_err("Failed to serialize JWT tokens to JSON")?;

        // Create temp file with correct permissions atomically (avoids TOCTOU).
        // On Unix, use OpenOptionsExt::mode(0o600); on other platforms, use default create.
        {
            #[allow(unused_mut)] // mut needed on Unix for mode() call
            let mut opts = OpenOptions::new();
            opts.write(true).create_new(true);
            #[cfg(unix)]
            {
                opts.mode(CACHE_FILE_MODE);
            }
            let mut file = opts
                .open(&temp_path)
                .wrap_err_with(|| format!("Failed to create temp file: {:?}", temp_path))?;
            file.write_all(json.as_bytes())
                .wrap_err_with(|| format!("Failed to write to temp file: {:?}", temp_path))?;
            file.sync_all()
                .wrap_err_with(|| format!("Failed to sync temp file: {:?}", temp_path))?;
        }

        // Rename temp file to final path (atomic on most filesystems)
        fs::rename(&temp_path, &cache_path).wrap_err_with(|| {
            format!(
                "Failed to rename temp file {:?} to {:?}",
                temp_path, cache_path
            )
        })?;

        // Successfully committed - prevent cleanup on drop
        _guard.commit();

        Ok(())
    }

    /// Load JWT tokens from disk, first tightening a directory or file left wider
    /// than owner-only.
    ///
    /// Returns:
    /// - `Ok(Some(tokens))` if file exists and is valid JSON
    /// - `Ok(None)` if file does not exist
    /// - `Err(...)` if file exists but cannot be read or parsed
    fn load(&self, node_name: &str) -> eyre::Result<Option<JwtToken>> {
        let cache_path = self.path(node_name)?;

        // If file doesn't exist, return None (not an error)
        if !cache_path.exists() {
            return Ok(None);
        }
        restrict_mode(&self.cache_dir()?, CACHE_DIR_MODE)?;
        restrict_mode(&cache_path, CACHE_FILE_MODE)?;

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
}

#[async_trait::async_trait]
impl ClientStorage for MeroboxFileStorage {
    async fn save_tokens(&self, node_name: &str, tokens: &JwtToken) -> eyre::Result<()> {
        self.save(node_name, tokens)
    }

    async fn load_tokens(&self, node_name: &str) -> eyre::Result<Option<JwtToken>> {
        self.load(node_name)
    }

    /// Overrides the default, which would save an "empty token": the file is deleted.
    async fn remove_tokens(&self, node_name: &str) -> eyre::Result<()> {
        let cache_path = self.path(node_name)?;

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

/// Clear any group or other bits on `path`, leaving at most `mode`.
#[cfg(unix)]
fn restrict_mode(path: &Path, mode: u32) -> eyre::Result<()> {
    let current = fs::metadata(path)
        .wrap_err_with(|| format!("Failed to stat {:?}", path))?
        .permissions()
        .mode();
    if current & 0o077 != 0 {
        fs::set_permissions(path, fs::Permissions::from_mode(mode))
            .map_err(|e| restrict_error(path, mode, e))?;
    }
    Ok(())
}

#[cfg(unix)]
fn restrict_error(path: &Path, mode: u32, e: std::io::Error) -> eyre::Report {
    let context = if e.kind() == std::io::ErrorKind::PermissionDenied {
        format!(
            "{:?} is accessible to other users but owned by another user, so its \
             permissions cannot be restricted; set it to {:o} or remove it",
            path, mode
        )
    } else {
        format!("Failed to restrict permissions on {:?} to {:o}", path, mode)
    };
    eyre::Report::new(e).wrap_err(context)
}

#[cfg(not(unix))]
fn restrict_mode(_path: &Path, _mode: u32) -> eyre::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A storage for `url` rooted in a fresh directory under the system temp dir.
    fn storage_in(dir: &Path, url: &str) -> MeroboxFileStorage {
        MeroboxFileStorage {
            dir: Some(dir.join("auth_cache")),
            ..MeroboxFileStorage::new(&Url::parse(url).unwrap())
        }
    }

    fn scratch_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "calimero-client-py-{}-{}",
            name,
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn token() -> JwtToken {
        JwtToken::with_refresh("access".to_owned(), "refresh".to_owned())
    }

    #[test]
    fn tokens_for_one_origin_are_not_loaded_for_another() {
        let dir = scratch_dir("origin");
        storage_in(&dir, "https://node-a.example.com:2428")
            .save("node", &token())
            .unwrap();

        let same = storage_in(&dir, "https://node-a.example.com:2428/");
        assert_eq!(same.load("node").unwrap().unwrap().access_token, "access");
        let other = storage_in(&dir, "https://node-b.example.com:2428");
        assert!(other.load("node").unwrap().is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    fn mode(path: &Path) -> u32 {
        fs::metadata(path).unwrap().permissions().mode() & 0o777
    }

    #[cfg(unix)]
    #[test]
    fn load_tightens_a_wider_directory_and_file() {
        let dir = scratch_dir("load-modes");
        let storage = storage_in(&dir, "https://node.example.com");
        let path = storage.path("node").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();
        fs::write(&path, serde_json::to_string(&token()).unwrap()).unwrap();
        fs::set_permissions(&path, fs::Permissions::from_mode(0o644)).unwrap();

        assert!(storage.load("node").unwrap().is_some());
        assert_eq!(mode(path.parent().unwrap()), CACHE_DIR_MODE);
        assert_eq!(mode(&path), CACHE_FILE_MODE);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn save_tightens_the_directory() {
        let dir = scratch_dir("save-modes");
        let storage = storage_in(&dir, "https://node.example.com");
        let path = storage.path("node").unwrap();
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::set_permissions(path.parent().unwrap(), fs::Permissions::from_mode(0o755)).unwrap();

        storage.save("node", &token()).unwrap();
        assert_eq!(mode(path.parent().unwrap()), CACHE_DIR_MODE);
        assert_eq!(mode(&path), CACHE_FILE_MODE);

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn nodes_under_one_gateway_do_not_share_tokens() {
        let dir = scratch_dir("gateway");
        storage_in(&dir, "https://gw.example.com/tenant-a/")
            .save("node", &token())
            .unwrap();

        assert!(storage_in(&dir, "https://gw.example.com/tenant-a")
            .load("node")
            .unwrap()
            .is_some());
        assert!(storage_in(&dir, "https://gw.example.com/tenant-b/")
            .load("node")
            .unwrap()
            .is_none());

        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn concurrent_saves_all_succeed_and_leave_a_whole_file() {
        let dir = scratch_dir("concurrent");
        let storage = storage_in(&dir, "https://node.example.com");
        std::thread::scope(|s| {
            for t in 0..4 {
                let storage = &storage;
                s.spawn(move || {
                    for i in 0..100 {
                        let access = format!("access-{t}-{}", "x".repeat(i));
                        let token = JwtToken::with_refresh(access, "refresh".to_owned());
                        storage.save("node", &token).unwrap();
                    }
                });
            }
        });

        assert!(storage.load("node").unwrap().is_some());
        let left = fs::read_dir(storage.path("node").unwrap().parent().unwrap())
            .unwrap()
            .count();
        assert_eq!(left, 1, "no temp files are left behind");
        fs::remove_dir_all(&dir).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn a_file_that_cannot_be_restricted_names_the_cause() {
        let err = restrict_error(
            Path::new("/cache/node.json"),
            CACHE_FILE_MODE,
            std::io::Error::from(std::io::ErrorKind::PermissionDenied),
        );
        let message = format!("{:#}", err);
        assert!(message.contains("owned by another user"), "{message}");
        assert!(message.contains("600"), "{message}");
    }
}
