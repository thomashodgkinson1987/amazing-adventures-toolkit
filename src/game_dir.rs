//! Locating and validating the game folder.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::hashes;

/// Resolve the game folder from an explicit `--dir` or the current directory.
pub fn resolve(dir: Option<PathBuf>) -> Result<PathBuf> {
    let path = match dir {
        Some(d) => d,
        None => std::env::current_dir().context("determining the current directory")?,
    };
    // Best-effort canonicalisation so printed paths are absolute and stable.
    let path = path.canonicalize().unwrap_or(path);
    if !path.is_dir() {
        bail!("{} is not a directory", path.display());
    }
    Ok(path)
}

/// Confirm the folder looks like the game (contains the Steam executable).
pub fn validate(dir: &Path) -> Result<()> {
    if !dir.join(hashes::EXE_NAME).is_file() {
        bail!(
            "{} was not found in {}.\nRun the toolkit from the game folder, or pass --dir <path>.",
            hashes::EXE_NAME,
            dir.display()
        );
    }
    if !dir.join("Resource.dll").is_file() {
        eprintln!(
            "warning: Resource.dll is missing from {} - is this really the game folder?",
            dir.display()
        );
    }
    Ok(())
}
