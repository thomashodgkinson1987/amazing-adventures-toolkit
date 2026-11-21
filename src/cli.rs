//! Command-line interface definition.

use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// Install cnc-ddraw and apply/remove the startup-splash skip for
/// *Amazing Adventures Around the World*.
#[derive(Debug, Parser)]
#[command(
    name = "amazing-adventures-toolkit",
    version,
    about = "Install cnc-ddraw and patch the startup splashes for Amazing Adventures Around the World.",
    long_about = None,
    subcommand_required = false,
    arg_required_else_help = false,
)]
pub struct Cli {
    /// Game folder (defaults to the current directory).
    #[arg(long, global = true, value_name = "PATH")]
    pub dir: Option<PathBuf>,

    /// Assume "yes" for every confirmation; never prompt.
    #[arg(short = 'y', long, global = true)]
    pub yes: bool,

    /// Do not contact the network (affects `status` only).
    #[arg(long, global = true)]
    pub offline: bool,

    /// Disable coloured output.
    #[arg(long, global = true)]
    pub no_color: bool,

    #[command(subcommand)]
    pub command: Option<Command>,
}

#[derive(Debug, Subcommand)]
pub enum Command {
    /// Show the current state of the game folder.
    Status,
    /// Install or remove the cnc-ddraw graphics improvement.
    Graphics {
        #[command(subcommand)]
        action: GraphicsAction,
    },
    /// Apply or remove the startup splash-screen skip.
    Splash {
        #[command(subcommand)]
        action: SplashAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum GraphicsAction {
    /// Download and install the latest cnc-ddraw release.
    Apply,
    /// Remove the installed cnc-ddraw files.
    Remove,
}

#[derive(Debug, Subcommand)]
pub enum SplashAction {
    /// Zero the three splash timing constants.
    Apply,
    /// Restore the pristine executable.
    Remove,
}
