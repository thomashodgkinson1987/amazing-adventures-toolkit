//! The `.aaatw-toolkit.json` marker file kept in the game folder.
//!
//! The marker records what the toolkit installed so that `status` can report a version and
//! `remove` knows exactly which files to delete. It is a convenience only: remove falls
//! back to the known file set, and the splash patch is fully reversible without it.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Name of the marker file in the game folder.
pub const MARKER_FILE: &str = ".aaatw-toolkit.json";

/// Current marker schema version.
pub const SCHEMA: u32 = 1;

/// Top-level marker document.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Marker {
    #[serde(default = "default_schema")]
    pub schema: u32,
    #[serde(default)]
    pub cncddraw: Option<CncddrawMarker>,
    #[serde(default)]
    pub splash: Option<SplashMarker>,
}

fn default_schema() -> u32 {
    SCHEMA
}

impl Default for Marker {
    fn default() -> Self {
        Marker {
            schema: SCHEMA,
            cncddraw: None,
            splash: None,
        }
    }
}

/// What was recorded about a cnc-ddraw install.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CncddrawMarker {
    pub version: String,
    pub asset: String,
    #[serde(default)]
    pub files: Vec<String>,
    #[serde(default)]
    pub ddraw_dll_sha256: Option<String>,
    pub installed_at: String,
}

/// What was recorded about the splash patch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SplashMarker {
    pub patched: bool,
    pub exe_sha256: String,
    pub updated_at: String,
}

/// Path of the marker file in `dir`.
pub fn marker_path(dir: &Path) -> PathBuf {
    dir.join(MARKER_FILE)
}

/// Load the marker, tolerating a missing or malformed file.
pub fn load(dir: &Path) -> Marker {
    let path = marker_path(dir);
    match std::fs::read_to_string(&path) {
        Ok(text) => match serde_json::from_str(&text) {
            Ok(marker) => marker,
            Err(e) => {
                eprintln!("warning: ignoring malformed {}: {e}", path.display());
                Marker::default()
            }
        },
        Err(_) => Marker::default(),
    }
}

/// Write the marker file.
pub fn save(dir: &Path, marker: &Marker) -> Result<()> {
    let path = marker_path(dir);
    let text = serde_json::to_string_pretty(marker).context("serialising the marker file")?;
    std::fs::write(&path, text).with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

/// Current UTC time as an RFC 3339 string.
pub fn now() -> String {
    time::OffsetDateTime::now_utc()
        .format(&time::format_description::well_known::Rfc3339)
        .unwrap_or_else(|_| "unknown".to_string())
}
