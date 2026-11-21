//! The interactive menu shown when no subcommand is given.

use anyhow::Result;
use inquire::{InquireError, Select};

use crate::app::{self, Ctx};

const OPTIONS: [&str; 6] = [
    "Show status",
    "Apply graphics improvement (cnc-ddraw)",
    "Remove graphics improvement (cnc-ddraw)",
    "Apply splash-screen skip",
    "Remove splash-screen skip",
    "Steam launch-option help",
];

/// Run the menu loop until the user quits.
pub fn run(ctx: &Ctx) -> Result<()> {
    loop {
        let selected = Select::new(
            "Amazing Adventures Toolkit - choose an action",
            OPTIONS.to_vec(),
        )
        .with_help_message("Arrow keys to move, Enter to select, Esc to quit")
        .prompt();

        let selected = match selected {
            Ok(choice) => choice,
            Err(InquireError::OperationCanceled | InquireError::OperationInterrupted) => {
                println!("Bye.");
                break;
            }
            Err(e) => return Err(e.into()),
        };

        let result = match selected {
            "Show status" => app::print_status(ctx),
            "Apply graphics improvement (cnc-ddraw)" => app::graphics_apply(ctx),
            "Remove graphics improvement (cnc-ddraw)" => app::graphics_remove(ctx),
            "Apply splash-screen skip" => app::splash_apply(ctx),
            "Remove splash-screen skip" => app::splash_remove(ctx),
            "Steam launch-option help" => {
                app::print_launch_option_help();
                Ok(())
            }
            _ => Ok(()),
        };

        if let Err(e) = result {
            eprintln!("{} {e:#}", console::style("error:").red().bold());
        }
        pause();
    }
    Ok(())
}

fn pause() {
    use std::io::Write;
    print!("\nPress Enter to return to the menu...");
    let _ = std::io::stdout().flush();
    let mut line = String::new();
    let _ = std::io::stdin().read_line(&mut line);
    println!();
}
