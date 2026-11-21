//! The startup splash-screen skip (the "Option A" patch).
//!
//! The three startup splashes (PopCap -> SpinTop -> Amazing Adventures) are states of the
//! game's own front-end state machine, driven by `f32` dwell/fade constants in `.rdata`.
//! Zeroing them makes each state advance in about one frame while the state machine still
//! runs, so the main menu is built normally.
//!
//! The real game is **not** `AmazingAdventures2.exe` itself: that file is a PopCap DRM
//! wrapper that extracts the actual game (`popcapgame1.exe`) from a PE overlay at file
//! offset `0x17b1a2` on every launch. The overlay is written out verbatim, so the only
//! durable place to patch is inside `AmazingAdventures2.exe`.
//!
//! Each inner-game constant at file offset `X` is therefore at
//! `0x17b1a2 + X` in the wrapper. See the wiki's `va-file-offset-mapping`.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

use crate::hashes::{self, EXE_NAME};

/// Suffix of the pristine backup written next to the EXE before patching.
pub const BACKUP_SUFFIX: &str = ".aaatw-orig";

/// The float `0.0`, as stored little-endian.
pub const ZEROED: [u8; 4] = [0x00, 0x00, 0x00, 0x00];

/// One timing-constant site inside the overlay embedded in `AmazingAdventures2.exe`.
pub struct Site {
    /// File offset of the 4-byte little-endian `f32` in `AmazingAdventures2.exe`.
    pub offset: usize,
    /// The stock (pristine) bytes at this offset.
    pub stock: [u8; 4],
    /// Human-readable description.
    pub name: &'static str,
}

/// The three main splash timing constants (the two optional extras are deliberately not
/// touched, matching the verified Option A patch).
pub const SITES: [Site; 3] = [
    Site {
        offset: 0x2a1d8a,
        stock: [0x00, 0x00, 0xa0, 0x3f],
        name: "splash dwell (1.25 s)",
    },
    Site {
        offset: 0x2a84ea,
        stock: [0x00, 0x00, 0xe0, 0x3f],
        name: "state 4->5 fade (1.75 s)",
    },
    Site {
        offset: 0x2a8462,
        stock: [0x00, 0x00, 0x20, 0x40],
        name: "state 5 (2.5 s)",
    },
];

/// Per-site classification.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteState {
    /// Still holds the pristine float.
    Stock,
    /// Holds `0.0`.
    Zeroed,
    /// Holds something unexpected (or the offset is out of range).
    Unexpected,
}

/// Overall patch state of the executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    /// Matches the pristine hash.
    Stock,
    /// Matches the patched hash.
    Patched,
    /// Some of the known sites differ, in a way that is still fully explained by them.
    Partial,
    /// The file is not a version this toolkit recognises.
    Unknown { hash: String },
}

/// Result of inspecting the game executable.
#[derive(Debug, Clone)]
pub struct Status {
    pub exe: PathBuf,
    pub hash: String,
    pub state: State,
    pub backup: Option<PathBuf>,
    pub sites: Vec<(usize, SiteState)>,
}

/// What an apply/remove operation did.
#[derive(Debug, Clone)]
pub enum Outcome {
    Applied {
        before: String,
        after: String,
        backup_created: bool,
    },
    Removed {
        before: String,
        after: String,
        used_backup: bool,
    },
    AlreadyPatched,
    AlreadyStock,
}

/// Path of `AmazingAdventures2.exe` in `dir`.
pub fn exe_path(dir: &Path) -> PathBuf {
    dir.join(EXE_NAME)
}

/// Path of the pristine backup next to the executable.
pub fn backup_path(dir: &Path) -> PathBuf {
    dir.join(format!("{EXE_NAME}{BACKUP_SUFFIX}"))
}

/// Classify each known site in `data` (pure; does not verify hashes).
pub fn site_states(data: &[u8]) -> Vec<(usize, SiteState)> {
    SITES
        .iter()
        .enumerate()
        .map(|(i, site)| {
            let state = match data.get(site.offset..site.offset + 4) {
                Some(bytes) if bytes == site.stock.as_slice() => SiteState::Stock,
                Some(bytes) if bytes == ZEROED.as_slice() => SiteState::Zeroed,
                _ => SiteState::Unexpected,
            };
            (i, state)
        })
        .collect()
}

/// True if every known site still holds its pristine bytes.
pub fn is_stock(data: &[u8]) -> bool {
    site_states(data)
        .iter()
        .all(|(_, s)| *s == SiteState::Stock)
}

/// True if every known site holds `0.0`.
pub fn is_patched(data: &[u8]) -> bool {
    site_states(data)
        .iter()
        .all(|(_, s)| *s == SiteState::Zeroed)
}

/// Return a copy of `data` with every known site forced back to its pristine bytes.
///
/// Because the patch only changes these sites, this reconstructs the exact stock file even
/// if `data` is partially or fully patched.
pub fn reconstruct_pristine(data: &[u8]) -> Vec<u8> {
    let mut out = data.to_vec();
    for site in SITES {
        out[site.offset..site.offset + 4].copy_from_slice(&site.stock);
    }
    out
}

/// Return a copy of a pristine image with every known site zeroed.
pub fn patched_bytes(pristine: &[u8]) -> Vec<u8> {
    let mut out = pristine.to_vec();
    for site in SITES {
        out[site.offset..site.offset + 4].copy_from_slice(&ZEROED);
    }
    out
}

/// Inspect the executable in `dir`.
pub fn status(dir: &Path) -> Result<Status> {
    let exe = exe_path(dir);
    let data = std::fs::read(&exe).with_context(|| format!("reading {}", exe.display()))?;
    let hash = hashes::sha256_bytes(&data);
    let state = classify(&data, &hash);
    let backup = backup_path(dir);
    Ok(Status {
        exe,
        hash,
        state,
        backup: backup.is_file().then_some(backup),
        sites: site_states(&data),
    })
}

fn classify(data: &[u8], hash: &str) -> State {
    if hash == hashes::STOCK_EXE_SHA256 {
        State::Stock
    } else if hash == hashes::PATCHED_EXE_SHA256 {
        State::Patched
    } else if hashes::sha256_bytes(&reconstruct_pristine(data)) == hashes::STOCK_EXE_SHA256 {
        // Only the known sites differ -> a valid but non-standard combination.
        State::Partial
    } else {
        State::Unknown {
            hash: hash.to_string(),
        }
    }
}

/// Ensure a valid pristine backup exists next to the executable.
///
/// `pristine` is the reconstructed stock image (already hash-verified by the caller).
fn ensure_backup(dir: &Path, pristine: &[u8]) -> Result<bool> {
    let path = backup_path(dir);
    if path.is_file() {
        let existing =
            std::fs::read(&path).with_context(|| format!("reading {}", path.display()))?;
        if hashes::sha256_bytes(&existing) == hashes::STOCK_EXE_SHA256 {
            return Ok(false); // Already have a good backup.
        }
        bail!(
            "an existing backup at {} is not a recognised pristine copy; move it aside first",
            path.display()
        );
    }
    std::fs::write(&path, pristine)
        .with_context(|| format!("writing backup {}", path.display()))?;
    Ok(true)
}

/// Apply the splash patch.
pub fn apply(dir: &Path) -> Result<Outcome> {
    let exe = exe_path(dir);
    let data = std::fs::read(&exe).with_context(|| format!("reading {}", exe.display()))?;
    let before = hashes::sha256_bytes(&data);

    let pristine = reconstruct_pristine(&data);
    if hashes::sha256_bytes(&pristine) != hashes::STOCK_EXE_SHA256 {
        bail!(
            "unrecognised {EXE_NAME} (sha256 {before}); this toolkit only patches the known game build"
        );
    }

    if before == hashes::PATCHED_EXE_SHA256 {
        ensure_backup(dir, &pristine)?;
        return Ok(Outcome::AlreadyPatched);
    }

    let backup_created = ensure_backup(dir, &pristine)?;
    let patched = patched_bytes(&pristine);
    std::fs::write(&exe, &patched).with_context(|| format!("writing {}", exe.display()))?;

    let after = hashes::sha256_file(&exe)?;
    if after != hashes::PATCHED_EXE_SHA256 {
        bail!("post-patch verification failed: {EXE_NAME} hashes to {after}");
    }
    Ok(Outcome::Applied {
        before,
        after,
        backup_created,
    })
}

/// Remove the splash patch, restoring the pristine executable.
pub fn remove(dir: &Path) -> Result<Outcome> {
    let exe = exe_path(dir);
    let data = std::fs::read(&exe).with_context(|| format!("reading {}", exe.display()))?;
    let before = hashes::sha256_bytes(&data);

    if before == hashes::STOCK_EXE_SHA256 {
        return Ok(Outcome::AlreadyStock);
    }

    let backup = backup_path(dir);
    let (pristine, used_backup) = if backup.is_file() {
        let bytes =
            std::fs::read(&backup).with_context(|| format!("reading {}", backup.display()))?;
        if hashes::sha256_bytes(&bytes) != hashes::STOCK_EXE_SHA256 {
            bail!(
                "backup {} is not a recognised pristine copy; refusing to restore it",
                backup.display()
            );
        }
        (bytes, true)
    } else {
        let pristine = reconstruct_pristine(&data);
        if hashes::sha256_bytes(&pristine) != hashes::STOCK_EXE_SHA256 {
            bail!("cannot reconstruct a pristine {EXE_NAME} from sha256 {before}");
        }
        (pristine, false)
    };

    std::fs::write(&exe, &pristine).with_context(|| format!("writing {}", exe.display()))?;
    let after = hashes::sha256_file(&exe)?;
    if after != hashes::STOCK_EXE_SHA256 {
        bail!("post-remove verification failed: {EXE_NAME} hashes to {after}");
    }
    Ok(Outcome::Removed {
        before,
        after,
        used_backup,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_stock() -> Vec<u8> {
        let size = SITES.iter().map(|s| s.offset).max().unwrap() + 16;
        let mut data = vec![0xAAu8; size];
        for site in SITES {
            data[site.offset..site.offset + 4].copy_from_slice(&site.stock);
        }
        data
    }

    #[test]
    fn stock_and_patched_classification() {
        let stock = synthetic_stock();
        assert!(is_stock(&stock));
        assert!(!is_patched(&stock));

        let patched = patched_bytes(&stock);
        assert!(is_patched(&patched));
        assert!(!is_stock(&patched));
    }

    #[test]
    fn reconstruct_recovers_stock_from_patched() {
        let stock = synthetic_stock();
        let patched = patched_bytes(&stock);
        assert_eq!(reconstruct_pristine(&patched), stock);
    }

    #[test]
    fn reconstruct_recovers_stock_from_partial() {
        let stock = synthetic_stock();
        let mut partial = stock.clone();
        // Zero only the middle site.
        let site = &SITES[1];
        partial[site.offset..site.offset + 4].copy_from_slice(&ZEROED);

        let states = site_states(&partial);
        assert_eq!(states[0].1, SiteState::Stock);
        assert_eq!(states[1].1, SiteState::Zeroed);
        assert_eq!(states[2].1, SiteState::Stock);
        assert_eq!(reconstruct_pristine(&partial), stock);
    }

    #[test]
    fn unexpected_bytes_are_detected() {
        let mut stock = synthetic_stock();
        let site = &SITES[0];
        stock[site.offset..site.offset + 4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        assert_eq!(site_states(&stock)[0].1, SiteState::Unexpected);
    }

    #[test]
    fn out_of_range_sites_are_unexpected() {
        let tiny = vec![0u8; 4];
        assert!(
            site_states(&tiny)
                .iter()
                .all(|(_, s)| *s == SiteState::Unexpected)
        );
    }
}
