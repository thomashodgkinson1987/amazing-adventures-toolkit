//! The actions behind both the CLI subcommands and the interactive menu.

use anyhow::{Context, Result, bail};
use console::style;
use inquire::{Confirm, InquireError};
use std::path::{Path, PathBuf};

use crate::{cncddraw, hashes, splash, state};

/// Shared settings for every action.
#[derive(Debug, Clone)]
pub struct Ctx {
    /// The game folder.
    pub dir: PathBuf,
    /// Assume "yes" and never prompt.
    pub yes: bool,
    /// Never contact the network (affects `status` only).
    pub offline: bool,
}

fn ok(s: &str) -> String {
    format!("{}", style(s).green())
}
fn warn(s: &str) -> String {
    format!("{}", style(s).yellow())
}
fn bad(s: &str) -> String {
    format!("{}", style(s).red())
}
fn dim(s: &str) -> String {
    format!("{}", style(s).dim())
}
fn bold(s: &str) -> String {
    format!("{}", style(s).bold())
}

fn rel(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/")
}

fn confirm(ctx: &Ctx, prompt: &str) -> Result<bool> {
    if ctx.yes {
        return Ok(true);
    }
    if !console::user_attended() {
        bail!("confirmation required in a non-interactive session; re-run with --yes");
    }
    match Confirm::new(prompt).with_default(false).prompt() {
        Ok(answer) => Ok(answer),
        Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Describe the current state of the game folder.
pub fn print_status(ctx: &Ctx) -> Result<()> {
    let dir = &ctx.dir;
    let marker = state::load(dir);

    println!("{}", bold("Amazing Adventures Toolkit"));
    println!("Game folder: {}", dir.display());
    println!();

    // --- Splash patch ---------------------------------------------------------------
    println!("{}", bold("Splash-screen skip (AmazingAdventures2.exe)"));
    match splash::status(dir) {
        Ok(status) => {
            let label = match &status.state {
                splash::State::Stock => ok("not applied (stock EXE)"),
                splash::State::Patched => ok("applied (patched EXE)"),
                splash::State::Partial => warn("partially applied"),
                splash::State::Unknown { .. } => bad("unrecognised executable"),
            };
            println!("  state  : {label}");
            println!("  sha256 : {}", status.hash);
            if let splash::State::Unknown { .. } = status.state {
                println!(
                    "  {} this is not a game build the toolkit knows; patching will be refused.",
                    warn("note:")
                );
            }
            let backup = status
                .backup
                .as_ref()
                .map(|p| rel(p, dir))
                .unwrap_or_else(|| "none".to_string());
            println!("  backup : {backup}");
        }
        Err(e) => println!("  {} {e:#}", bad("error:")),
    }
    println!();

    // --- cnc-ddraw ------------------------------------------------------------------
    println!("{}", bold("Graphics improvement (cnc-ddraw)"));
    let dll = dir.join(cncddraw::DDRAW_DLL);
    let ini = dir.join(cncddraw::DDRAW_INI);
    if dll.is_file() || ini.is_file() {
        println!("  installed : {}", ok("yes"));
        let dll_hash = dll
            .is_file()
            .then(|| hashes::sha256_file(&dll).ok())
            .flatten();
        match &marker.cncddraw {
            Some(recorded) => {
                println!("  version   : {}", recorded.version);
                if let (Some(current), Some(known)) = (&dll_hash, &recorded.ddraw_dll_sha256)
                    && current != known
                {
                    println!(
                        "  {} ddraw.dll no longer matches the recorded install.",
                        warn("note:")
                    );
                }
            }
            None => println!("  version   : {} (no marker file)", warn("unknown")),
        }
        if let Some(hash) = &dll_hash {
            println!("  ddraw.dll : {hash}");
        }
    } else {
        println!("  installed : {}", dim("no"));
    }

    if ctx.offline {
        println!("  latest    : {} (--offline)", dim("skipped"));
    } else {
        match cncddraw::fetch_latest() {
            Ok(release) => {
                let newer = marker
                    .cncddraw
                    .as_ref()
                    .map(|c| c.version != release.tag)
                    .unwrap_or(true);
                if newer {
                    println!("  latest    : {} ({})", release.tag, warn("available"));
                } else {
                    println!("  latest    : {} ({})", release.tag, ok("up to date"));
                }
            }
            Err(e) => println!("  latest    : {} {e:#}", warn("could not check:")),
        }
    }
    println!();
    launch_option_reminder();
    Ok(())
}

fn launch_option_reminder() {
    println!(
        "{}",
        dim("Reminder: under Proton/Wine cnc-ddraw needs the Steam launch option")
    );
    println!("  {}", dim("WINEDLLOVERRIDES=\"ddraw=n,b\" %command%"));
}

/// Print the Steam launch-option instructions in full.
pub fn print_launch_option_help() {
    println!("{}", bold("Steam launch option"));
    println!();
    println!("Under Proton/Wine, cnc-ddraw is only used if Wine is told to prefer the local");
    println!("DLL. Add this to the game's Steam launch options:");
    println!();
    println!("  {}", ok("WINEDLLOVERRIDES=\"ddraw=n,b\" %command%"));
    println!();
    println!("Steam -> right-click the game -> Properties -> Launch Options -> paste it in.");
    println!();
    println!(
        "{}",
        dim(
            "On native Windows this is normally unnecessary: the local ddraw.dll is loaded automatically."
        )
    );
}

/// Install (or update) cnc-ddraw.
pub fn graphics_apply(ctx: &Ctx) -> Result<()> {
    println!("{}", bold("cnc-ddraw - install"));

    let release =
        cncddraw::fetch_latest().context("looking up the latest cnc-ddraw release on GitHub")?;
    println!("Latest release: {}", release.tag);

    let mut marker = state::load(&ctx.dir);
    if let Some(existing) = &marker.cncddraw {
        if existing.version == release.tag {
            println!(
                "Version {} is already installed; reinstalling.",
                existing.version
            );
        } else {
            println!("Updating {} -> {}.", existing.version, release.tag);
        }
    }

    if !confirm(ctx, "Download and extract cnc-ddraw into the game folder?")? {
        println!("Cancelled.");
        return Ok(());
    }

    let bytes = cncddraw::download(&release)?;

    let ini = ctx.dir.join(cncddraw::DDRAW_INI);
    let backup = ctx.dir.join(cncddraw::INI_BACKUP);
    // Only capture the *first* original ini; a later re-install must not clobber it with
    // the release default we wrote previously.
    if ini.is_file() && !backup.is_file() {
        std::fs::copy(&ini, &backup).with_context(|| format!("backing up {}", ini.display()))?;
        println!(
            "Backed up the existing ddraw.ini to {}.",
            cncddraw::INI_BACKUP
        );
    }

    let files = cncddraw::extract_zip(&bytes, &ctx.dir)?;
    let ddraw_dll_sha256 = hashes::sha256_file(&ctx.dir.join(cncddraw::DDRAW_DLL)).ok();

    marker.cncddraw = Some(state::CncddrawMarker {
        version: release.tag.clone(),
        asset: release.asset_name.clone(),
        files,
        ddraw_dll_sha256,
        installed_at: state::now(),
    });
    state::save(&ctx.dir, &marker)?;

    println!("{} installed cnc-ddraw {}.", ok("Done:"), release.tag);
    println!();
    launch_option_reminder();
    Ok(())
}

/// Remove cnc-ddraw.
pub fn graphics_remove(ctx: &Ctx) -> Result<()> {
    println!("{}", bold("cnc-ddraw - remove"));

    let mut marker = state::load(&ctx.dir);
    let installed = marker.cncddraw.as_ref().map(|c| c.files.clone());
    let plan = cncddraw::plan_remove(&ctx.dir, installed.as_deref());

    if plan.is_empty() {
        println!("Nothing to remove - cnc-ddraw does not appear to be installed here.");
        if marker.cncddraw.is_some() {
            marker.cncddraw = None;
            state::save(&ctx.dir, &marker)?;
        }
        return Ok(());
    }

    println!("The following will be removed:");
    for file in &plan.files {
        println!("  {}", rel(file, &ctx.dir));
    }
    for dir in &plan.dirs {
        println!("  {}/", rel(dir, &ctx.dir));
    }
    if plan.restore_ini {
        println!(
            "  (restore the original ddraw.ini from {})",
            cncddraw::INI_BACKUP
        );
    }

    if !confirm(ctx, "Remove these files?")? {
        println!("Cancelled.");
        return Ok(());
    }

    let removed = cncddraw::perform_remove(&plan)?;
    println!("{} removed {} item(s).", ok("Done:"), removed);

    marker.cncddraw = None;
    state::save(&ctx.dir, &marker)?;
    Ok(())
}

/// Apply the splash-screen skip.
pub fn splash_apply(ctx: &Ctx) -> Result<()> {
    println!("{}", bold("Splash-screen skip - apply"));

    let status = splash::status(&ctx.dir)?;
    match &status.state {
        splash::State::Patched => {
            println!("Already applied - the executable matches the patched hash.");
            return Ok(());
        }
        splash::State::Unknown { .. } => {
            bail!(
                "unrecognised {} (sha256 {}); refusing to patch.",
                hashes::EXE_NAME,
                status.hash
            );
        }
        _ => {}
    }

    if !confirm(
        ctx,
        "Zero the three splash timing constants inside AmazingAdventures2.exe?",
    )? {
        println!("Cancelled.");
        return Ok(());
    }

    match splash::apply(&ctx.dir)? {
        splash::Outcome::Applied {
            before,
            after,
            backup_created,
        } => {
            println!("{} splash patch applied.", ok("Done:"));
            println!("  before: {before}");
            println!("  after : {after}");
            if backup_created {
                println!(
                    "  pristine backup written alongside the executable ({}{}).",
                    hashes::EXE_NAME,
                    splash::BACKUP_SUFFIX
                );
            }
        }
        splash::Outcome::AlreadyPatched => println!("Already applied."),
        splash::Outcome::Removed { .. } | splash::Outcome::AlreadyStock => {}
    }

    record_splash(&ctx.dir, true)?;
    Ok(())
}

/// Remove the splash-screen skip.
pub fn splash_remove(ctx: &Ctx) -> Result<()> {
    println!("{}", bold("Splash-screen skip - remove"));

    let status = splash::status(&ctx.dir)?;
    if status.state == splash::State::Stock {
        println!("Nothing to do - the executable is already stock (splashes are present).");
        record_splash(&ctx.dir, false)?;
        return Ok(());
    }

    if !confirm(ctx, "Restore the pristine AmazingAdventures2.exe?")? {
        println!("Cancelled.");
        return Ok(());
    }

    match splash::remove(&ctx.dir)? {
        splash::Outcome::Removed {
            before,
            after,
            used_backup,
        } => {
            println!("{} splash patch removed.", ok("Done:"));
            println!("  before: {before}");
            println!("  after : {after}");
            if !used_backup {
                println!(
                    "  {} no backup was present; the pristine bytes were reconstructed.",
                    dim("note:")
                );
            }
        }
        splash::Outcome::AlreadyStock => println!("Already stock."),
        splash::Outcome::Applied { .. } | splash::Outcome::AlreadyPatched => {}
    }

    record_splash(&ctx.dir, false)?;
    Ok(())
}

fn record_splash(dir: &Path, patched: bool) -> Result<()> {
    let mut marker = state::load(dir);
    let exe_sha256 = hashes::sha256_file(&splash::exe_path(dir))?;
    marker.splash = Some(state::SplashMarker {
        patched,
        exe_sha256,
        updated_at: state::now(),
    });
    state::save(dir, &marker)
}
