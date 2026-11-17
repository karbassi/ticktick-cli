mod api;
mod cli;
mod config;
mod mcp;
mod output;

use std::process::ExitCode;

fn main() -> ExitCode {
    // Reset SIGPIPE to default behavior so piping to `head`, `grep`, etc.
    // silently terminates instead of printing a broken pipe error. `mcp`
    // ignores it again.
    unsafe {
        libc::signal(libc::SIGPIPE, libc::SIG_DFL);
        libc::signal(libc::SIGINT, libc::SIG_DFL);
    }
    match cli::run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            if !e.is_empty() {
                output::error(&e);
            }
            ExitCode::FAILURE
        }
    }
}
