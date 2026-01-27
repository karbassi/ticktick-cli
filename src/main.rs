mod cli;
mod config;
mod api;

use std::process::ExitCode;

fn main() -> ExitCode {
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m {e}");
            ExitCode::FAILURE
        }
    }
}
