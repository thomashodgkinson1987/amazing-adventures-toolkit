//! End-to-end apply/remove round-trip against a real (stock) game executable.
//!
//! This needs the actual `AmazingAdventures2.exe`, which is not in the repository, so the
//! test is skipped unless `AAATW_STOCK_EXE` points at a pristine copy. Run it against the
//! wiki's backup, for example:
//!
//! ```sh
//! AAATW_STOCK_EXE=/workspace/llm-wiki/backups/AmazingAdventures2.exe.orig \
//!     cargo test --test real_exe -- --nocapture
//! ```

use std::path::PathBuf;
use std::process::Command;

use amazing_adventures_toolkit::hashes::{PATCHED_EXE_SHA256, STOCK_EXE_SHA256, sha256_bytes};

fn stock_exe() -> Option<PathBuf> {
    std::env::var_os("AAATW_STOCK_EXE").map(PathBuf::from)
}

fn run_tool(dir: &std::path::Path, args: &[&str]) {
    let status = Command::new(env!("CARGO_BIN_EXE_amazing-adventures-toolkit"))
        .arg("--dir")
        .arg(dir)
        .arg("--yes")
        .args(args)
        .status()
        .expect("running the toolkit binary");
    assert!(
        status.success(),
        "toolkit exited with {status} for {args:?}"
    );
}

#[test]
fn apply_then_remove_roundtrip() {
    let Some(stock) = stock_exe() else {
        eprintln!("skipping: set AAATW_STOCK_EXE to a pristine AmazingAdventures2.exe");
        return;
    };
    assert!(stock.is_file(), "{} is not a file", stock.display());

    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("AmazingAdventures2.exe");
    std::fs::copy(&stock, &exe).unwrap();
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        STOCK_EXE_SHA256
    );

    // Apply.
    run_tool(dir.path(), &["splash", "apply"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        PATCHED_EXE_SHA256
    );
    assert!(
        dir.path()
            .join("AmazingAdventures2.exe.aaatw-orig")
            .is_file()
    );

    // Applying again is idempotent.
    run_tool(dir.path(), &["splash", "apply"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        PATCHED_EXE_SHA256
    );

    // Remove restores the pristine bytes.
    run_tool(dir.path(), &["splash", "remove"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        STOCK_EXE_SHA256
    );

    // Removing again is a no-op.
    run_tool(dir.path(), &["splash", "remove"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        STOCK_EXE_SHA256
    );
}

#[test]
fn remove_without_backup_reconstructs_pristine() {
    let Some(stock) = stock_exe() else {
        eprintln!("skipping: set AAATW_STOCK_EXE to a pristine AmazingAdventures2.exe");
        return;
    };

    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("AmazingAdventures2.exe");
    std::fs::copy(&stock, &exe).unwrap();

    run_tool(dir.path(), &["splash", "apply"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        PATCHED_EXE_SHA256
    );

    // Delete the backup so removal has to reconstruct the pristine bytes.
    std::fs::remove_file(dir.path().join("AmazingAdventures2.exe.aaatw-orig")).unwrap();

    run_tool(dir.path(), &["splash", "remove"]);
    assert_eq!(
        sha256_bytes(&std::fs::read(&exe).unwrap()),
        STOCK_EXE_SHA256
    );
}
