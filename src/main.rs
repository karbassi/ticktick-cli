mod api;
mod cli;
mod config;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so piping to `head`, `grep`, etc.
    // silently terminates instead of printing a broken pipe error.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
    }
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("\x1b[31merror:\x1b[0m {e}");
            ExitCode::FAILURE
        }
    }
}
