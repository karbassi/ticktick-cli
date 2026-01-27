use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ticktick - TickTick CLI

\x1b[33mUSAGE:\x1b[0m
    ticktick <COMMAND> [OPTIONS]

\x1b[33mCOMMANDS:\x1b[0m
    login                 Authenticate with TickTick
    logout                Remove stored credentials
    projects              List all projects
    project <id>          Get project by ID
    tasks [project_id]    List tasks (optionally for a project)
    add <title>           Create a new task
    complete <pid> <tid>  Complete a task
    delete <pid> <tid>    Delete a task
    help                  Show this help message
    version               Show version

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
        "project" => {
            let id = args.get(2).ok_or("usage: ticktick project <id>")?;
            crate::api::project::get_by_id(id)
        }
        "tasks" => crate::api::task::list(&args[2..]),
        "add" => {
            let title = args.get(2).ok_or("usage: ticktick add <title> [--project <id>]")?;
            let project_id = args.iter().position(|a| a == "--project" || a == "-p")
                .and_then(|i| args.get(i + 1).map(|s| s.as_str()));
            crate::api::task::create(title, project_id)
        }
        "complete" => {
            let project_id = args.get(2).ok_or("usage: ticktick complete <project_id> <task_id>")?;
            let task_id = args.get(3).ok_or("usage: ticktick complete <project_id> <task_id>")?;
            crate::api::task::complete(project_id, task_id)
        }
        "delete" => {
            let project_id = args.get(2).ok_or("usage: ticktick delete <project_id> <task_id>")?;
            let task_id = args.get(3).ok_or("usage: ticktick delete <project_id> <task_id>")?;
            crate::api::task::delete(project_id, task_id)
        }
        cmd => Err(format!("unknown command: {cmd}\nRun 'ticktick help' for usage")),
    }
}
