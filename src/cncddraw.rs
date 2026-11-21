//! Installing and removing the cnc-ddraw graphics improvement.
//!
//! cnc-ddraw is a native reimplementation of DirectDraw. Dropping its `ddraw.dll` (plus
//! `ddraw.ini` and its shaders) into the game folder replaces Wine's slow
//! `ddraw -> wined3d` path. Under Proton it additionally needs the Steam launch option
//! `WINEDLLOVERRIDES="ddraw=n,b"`; on native Windows the local DLL is picked up
//! automatically. This module handles the files only - the launch option is printed as
//! instructions by the caller.

use anyhow::{Context, Result, anyhow};
use serde::Deserialize;
use std::collections::BTreeSet;
use std::io::Read;
use std::path::{Path, PathBuf};

use indicatif::{ProgressBar, ProgressStyle};

/// GitHub API endpoint for the newest non-prerelease, non-draft release.
const LATEST_RELEASE_API: &str =
    "https://api.github.com/repos/FunkyFr3sh/cnc-ddraw/releases/latest";

/// GitHub asks for a descriptive User-Agent.
const USER_AGENT: &str = concat!("amazing-adventures-toolkit/", env!("CARGO_PKG_VERSION"));

/// The primary zip asset published by cnc-ddraw.
pub const ASSET_NAME: &str = "cnc-ddraw.zip";

pub const DDRAW_DLL: &str = "ddraw.dll";
pub const DDRAW_INI: &str = "ddraw.ini";
/// Backup of a pre-existing `ddraw.ini`, taken before we overwrite it.
pub const INI_BACKUP: &str = "ddraw.ini.bak";
pub const CONFIG_EXE: &str = "cnc-ddraw config.exe";
pub const SHADERS_DIR: &str = "Shaders";

#[derive(Debug, Deserialize)]
struct GhRelease {
    tag_name: String,
    assets: Vec<GhAsset>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GhAsset {
    pub name: String,
    pub browser_download_url: String,
    pub size: u64,
}

/// Details of the release asset to install.
#[derive(Debug, Clone)]
pub struct Release {
    pub tag: String,
    pub asset_name: String,
    pub download_url: String,
    pub size: u64,
}

/// Choose the release asset to install.
///
/// Prefers exactly [`ASSET_NAME`], then any `cnc-ddraw*.zip` that is not the
/// Windows-ME "legacy" build.
pub fn pick_asset(assets: &[GhAsset]) -> Option<&GhAsset> {
    assets.iter().find(|a| a.name == ASSET_NAME).or_else(|| {
        assets.iter().find(|a| {
            a.name.starts_with("cnc-ddraw")
                && a.name.ends_with(".zip")
                && !a.name.contains("legacy")
        })
    })
}

/// Query GitHub for the latest cnc-ddraw release.
pub fn fetch_latest() -> Result<Release> {
    let release: GhRelease = ureq::get(LATEST_RELEASE_API)
        .header("User-Agent", USER_AGENT)
        .header("Accept", "application/vnd.github+json")
        .call()
        .context("querying GitHub for the latest cnc-ddraw release")?
        .body_mut()
        .read_json()
        .context("parsing the GitHub release response")?;

    let asset = pick_asset(&release.assets)
        .ok_or_else(|| anyhow!("release {} has no {ASSET_NAME} asset", release.tag_name))?;

    Ok(Release {
        tag: release.tag_name,
        asset_name: asset.name.clone(),
        download_url: asset.browser_download_url.clone(),
        size: asset.size,
    })
}

/// Download a release asset into memory, showing progress.
pub fn download(release: &Release) -> Result<Vec<u8>> {
    let mut response = ureq::get(&release.download_url)
        .header("User-Agent", USER_AGENT)
        .call()
        .with_context(|| format!("downloading {}", release.asset_name))?;

    let total = response.body().content_length().unwrap_or(release.size);
    let bar = ProgressBar::new(total);
    bar.set_style(
        ProgressStyle::with_template("{msg} [{bar:40}] {bytes}/{total_bytes} ({eta})")
            .expect("valid progress template")
            .progress_chars("=> "),
    );
    bar.set_message(release.asset_name.clone());

    let mut bytes: Vec<u8> = Vec::with_capacity(total as usize);
    let mut reader = response.body_mut().as_reader();
    let mut chunk = vec![0u8; 64 * 1024];
    loop {
        let n = reader
            .read(&mut chunk)
            .with_context(|| format!("reading {}", release.asset_name))?;
        if n == 0 {
            break;
        }
        bytes.extend_from_slice(&chunk[..n]);
        bar.inc(n as u64);
    }
    bar.finish_and_clear();

    if bytes.is_empty() {
        return Err(anyhow!("downloaded an empty {} file", release.asset_name));
    }
    Ok(bytes)
}

/// Extract every entry of an in-memory zip into `dest`, returning the file names written.
///
/// `ZipArchive::extract` sanitises paths, so this is safe against zip-slip.
pub fn extract_zip(bytes: &[u8], dest: &Path) -> Result<Vec<String>> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(bytes))
        .context("opening the downloaded cnc-ddraw archive")?;

    let mut files = Vec::new();
    for name in archive.file_names() {
        let name = name.context("reading a zip entry name")?.to_string();
        if !name.ends_with('/') {
            files.push(name);
        }
    }

    archive
        .extract(dest)
        .with_context(|| format!("extracting the archive into {}", dest.display()))?;
    Ok(files)
}

/// A pre-computed list of files/directories to delete.
#[derive(Debug, Default, Clone)]
pub struct RemovalPlan {
    /// The game folder the plan applies to.
    pub dir: PathBuf,
    pub files: Vec<PathBuf>,
    pub dirs: Vec<PathBuf>,
    /// Whether an original `ddraw.ini` should be restored from [`INI_BACKUP`].
    pub restore_ini: bool,
}

impl RemovalPlan {
    pub fn is_empty(&self) -> bool {
        self.files.is_empty() && self.dirs.is_empty() && !self.restore_ini
    }
}

/// Work out what would be removed.
///
/// If `installed` (the file list recorded in the marker) is given, it is used; otherwise a
/// sensible set of known cnc-ddraw paths is assumed.
pub fn plan_remove(dir: &Path, installed: Option<&[String]>) -> RemovalPlan {
    let mut candidates: BTreeSet<String> = BTreeSet::new();
    match installed {
        Some(list) => candidates.extend(list.iter().cloned()),
        None => {
            candidates.insert(DDRAW_DLL.to_string());
            candidates.insert(DDRAW_INI.to_string());
            candidates.insert(CONFIG_EXE.to_string());
        }
    }
    // Always consider the two core files, even if the marker was incomplete.
    candidates.insert(DDRAW_DLL.to_string());
    candidates.insert(DDRAW_INI.to_string());

    let mut files = Vec::new();
    let mut top_dirs: BTreeSet<String> = BTreeSet::new();
    for name in candidates {
        let path = dir.join(&name);
        if path.is_file() {
            files.push(path);
        }
        // A path with no '.' in its first component is a directory (e.g. `Shaders`).
        if let Some(first) = name.split(['/', '\\']).next()
            && !first.is_empty()
            && !first.contains('.')
        {
            let dir_path = dir.join(first);
            if dir_path.is_dir() {
                top_dirs.insert(first.to_string());
            }
        }
    }
    // The shaders directory is part of every install.
    let shaders = dir.join(SHADERS_DIR);
    if shaders.is_dir() {
        top_dirs.insert(SHADERS_DIR.to_string());
    }

    RemovalPlan {
        dir: dir.to_path_buf(),
        files,
        dirs: top_dirs.into_iter().map(|d| dir.join(d)).collect(),
        restore_ini: dir.join(INI_BACKUP).is_file(),
    }
}

/// Execute a [`RemovalPlan`]; returns the number of files/directories removed.
pub fn perform_remove(plan: &RemovalPlan) -> Result<usize> {
    let mut count = 0;
    for file in &plan.files {
        std::fs::remove_file(file).with_context(|| format!("removing {}", file.display()))?;
        count += 1;
    }
    for dir in &plan.dirs {
        std::fs::remove_dir_all(dir).with_context(|| format!("removing {}", dir.display()))?;
        count += 1;
    }
    if plan.restore_ini {
        let backup = plan.dir.join(INI_BACKUP);
        let target = plan.dir.join(DDRAW_INI);
        std::fs::copy(&backup, &target)
            .with_context(|| format!("restoring {}", target.display()))?;
        std::fs::remove_file(&backup).with_context(|| format!("removing {}", backup.display()))?;
        count += 1;
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn asset(name: &str) -> GhAsset {
        GhAsset {
            name: name.to_string(),
            browser_download_url: "https://example.invalid/x".to_string(),
            size: 1,
        }
    }

    #[test]
    fn prefers_the_canonical_asset() {
        let assets = vec![
            asset("ddraw.pdb"),
            asset(ASSET_NAME),
            asset("proxy-dlls.zip"),
        ];
        assert_eq!(pick_asset(&assets).unwrap().name, ASSET_NAME);
    }

    #[test]
    fn falls_back_to_a_non_legacy_zip() {
        let assets = vec![asset("ddraw.pdb"), asset("cnc-ddraw-foo.zip")];
        assert_eq!(pick_asset(&assets).unwrap().name, "cnc-ddraw-foo.zip");
    }

    #[test]
    fn ignores_legacy_and_unrelated_assets() {
        assert!(pick_asset(&[asset("cnc-ddraw_legacy_WindowsME.zip")]).is_none());
        assert!(pick_asset(&[asset("proxy-dlls.zip")]).is_none());
        assert!(pick_asset(&[]).is_none());
    }

    fn make_zip() -> Vec<u8> {
        let mut cursor = std::io::Cursor::new(Vec::new());
        {
            let mut writer = zip::ZipWriter::new(&mut cursor);
            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored);
            writer.start_file(DDRAW_DLL, options).unwrap();
            writer.write_all(b"fake dll").unwrap();
            writer
                .start_file("Shaders/interpolation/x.glsl", options)
                .unwrap();
            writer.write_all(b"shader").unwrap();
            writer.finish().unwrap();
        }
        cursor.into_inner()
    }

    #[test]
    fn extracts_all_entries_and_lists_files() {
        let dir = tempfile::tempdir().unwrap();
        let bytes = make_zip();
        let files = extract_zip(&bytes, dir.path()).unwrap();

        assert!(files.contains(&DDRAW_DLL.to_string()));
        assert!(files.contains(&"Shaders/interpolation/x.glsl".to_string()));
        assert!(dir.path().join(DDRAW_DLL).is_file());
        assert!(dir.path().join("Shaders/interpolation/x.glsl").is_file());
    }

    #[test]
    fn remove_restores_the_original_ini_from_the_correct_folder() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(DDRAW_DLL), b"dll").unwrap();
        std::fs::write(dir.path().join(DDRAW_INI), b"release-default").unwrap();
        std::fs::create_dir_all(dir.path().join("Shaders/crt")).unwrap();
        std::fs::write(dir.path().join("Shaders/crt/x.glsl"), b"shader").unwrap();
        std::fs::write(dir.path().join(INI_BACKUP), b"user's original").unwrap();

        let plan = plan_remove(dir.path(), None);
        assert!(plan.restore_ini);
        assert!(plan.files.contains(&dir.path().join(DDRAW_DLL)));
        assert!(plan.dirs.contains(&dir.path().join(SHADERS_DIR)));

        perform_remove(&plan).unwrap();

        // The user's original ini is restored (not nested under Shaders/), and the
        // installed files/dirs are gone.
        assert_eq!(
            std::fs::read(dir.path().join(DDRAW_INI)).unwrap(),
            b"user's original"
        );
        assert!(!dir.path().join(INI_BACKUP).exists());
        assert!(!dir.path().join(DDRAW_DLL).exists());
        assert!(!dir.path().join(SHADERS_DIR).exists());
    }
}
