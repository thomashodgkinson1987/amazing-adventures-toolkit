//! Known-answer SHA-256 hashes and hashing helpers.
//!
//! The splash patch is validated against the exact game build it was derived from, so a
//! wrong or unrecognised executable is refused rather than corrupted.

use anyhow::{Context, Result};
use sha2::{Digest, Sha256};
use std::path::Path;

/// The Steam executable that this toolkit patches.
pub const EXE_NAME: &str = "AmazingAdventures2.exe";

/// SHA-256 of the pristine (stock) `AmazingAdventures2.exe`.
pub const STOCK_EXE_SHA256: &str =
    "275b8b278b3ec38657635bca3cab02ac411e85e63d24f06139eb5f9d5d1c00e3";

/// SHA-256 of `AmazingAdventures2.exe` after the splash patch is applied.
pub const PATCHED_EXE_SHA256: &str =
    "d920cde4b531d61b377b5feead659ac13e8cb11a88cd9622761c2ed4ea4eb833";

/// Hex-encoded SHA-256 of a byte slice.
pub fn sha256_bytes(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    hex::encode(hasher.finalize())
}

/// Hex-encoded SHA-256 of a file's contents.
pub fn sha256_file(path: &Path) -> Result<String> {
    let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(sha256_bytes(&data))
}
