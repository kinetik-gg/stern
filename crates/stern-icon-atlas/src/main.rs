//! Command-line entry point for deterministic Phosphor generation and checks.

mod generate;
mod linkage;

use std::{env, process::ExitCode};

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let command = arguments.next();
    if arguments.next().is_some() {
        eprintln!("error: expected exactly one command");
        return ExitCode::from(2);
    }
    let result = match command.as_deref() {
        Some("generate") => generate::generate_workspace(),
        Some("check") => generate::check_workspace(),
        Some("linkage-check") => linkage::check_workspace(),
        _ => {
            eprintln!("usage: stern-icon-atlas <generate|check|linkage-check>");
            return ExitCode::from(2);
        }
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}
