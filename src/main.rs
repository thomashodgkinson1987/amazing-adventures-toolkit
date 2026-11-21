//! Binary entry point: parse arguments and dispatch to the library.

use std::process::ExitCode;

use clap::Parser;

use amazing_adventures_toolkit::app::{self, Ctx};
use amazing_adventures_toolkit::cli::{Cli, Command, GraphicsAction, SplashAction};
use amazing_adventures_toolkit::{game_dir, menu};

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.no_color || std::env::var_os("NO_COLOR").is_some() {
        console::set_colors_enabled(false);
    }

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{} {e:#}", console::style("error:").red().bold());
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> anyhow::Result<()> {
    let dir = game_dir::resolve(cli.dir.clone())?;
    let ctx = Ctx {
        dir: dir.clone(),
        yes: cli.yes,
        offline: cli.offline,
    };

    match cli.command {
        Some(command) => {
            game_dir::validate(&dir)?;
            match command {
                Command::Status => app::print_status(&ctx)?,
                Command::Graphics { action } => match action {
                    GraphicsAction::Apply => app::graphics_apply(&ctx)?,
                    GraphicsAction::Remove => app::graphics_remove(&ctx)?,
                },
                Command::Splash { action } => match action {
                    SplashAction::Apply => app::splash_apply(&ctx)?,
                    SplashAction::Remove => app::splash_remove(&ctx)?,
                },
            }
            Ok(())
        }
        None => {
            if !console::user_attended() {
                anyhow::bail!(
                    "no subcommand given and no interactive terminal is available; run --help or pass a subcommand"
                );
            }
            game_dir::validate(&dir)?;
            menu::run(&ctx)
        }
    }
}
