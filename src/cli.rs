use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ticktick - TickTick CLI

\x1b[33mUSAGE:\x1b[0m
    ticktick <COMMAND> [OPTIONS]

\x1b[33mCOMMANDS:\x1b[0m
    login       Authenticate with TickTick
    logout      Remove stored credentials
    projects    List all projects
    tasks       List tasks
    help        Show this help message
    version     Show version

\x1b[33mOPTIONS:\x1b[0m
    -h, --help      Show help
    -v, --version   Show version
";

pub fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print!("{HELP}");
        return Ok(());
    }

    match args[1].as_str() {
        "help" | "-h" | "--help" => {
            print!("{HELP}");
            Ok(())
        }
        "version" | "-v" | "--version" => {
            println!("ticktick {VERSION}");
            Ok(())
        }
        "login" => crate::api::auth::login(),
        "logout" => crate::config::logout(),
        "projects" => crate::api::project::list(),
        "tasks" => crate::api::task::list(&args[2..]),
        cmd => Err(format!("unknown command: {cmd}\nRun 'ticktick help' for usage")),
    }
}
