use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ticktick - TickTick CLI

\x1b[33mUSAGE:\x1b[0m
    ticktick <COMMAND> [OPTIONS]

\x1b[33mCOMMANDS:\x1b[0m
    login                    Authenticate with TickTick
    logout                   Remove stored credentials
    projects                 List all projects
    project <name>           Get project by name or ID
    tasks [project]          List tasks (optionally for a project)
    add <title> [-p project] Create a new task
    complete <project> <tid> Complete a task
    delete <project> <tid>   Delete a task
    help                     Show this help message
    version                  Show version

\x1b[33mEXAMPLES:\x1b[0m
    ticktick tasks Work
    ticktick tasks 'My Project'
    ticktick add 'New task' -p Personal

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
            let name = args.get(2).ok_or("usage: ticktick project <name>")?;
            let id = crate::api::project::resolve_id(name)?;
            crate::api::project::get_by_id(&id)
        }
        "tasks" => {
            let project = args.get(2).map(|s| crate::api::project::resolve_id(s)).transpose()?;
            crate::api::task::list_by_project(project.as_deref())
        }
        "add" => {
            let title = args.get(2).ok_or("usage: ticktick add <title> [-p <project>]")?;
            let project_name = args.iter().position(|a| a == "--project" || a == "-p")
                .and_then(|i| args.get(i + 1));
            let project_id = project_name
                .map(|n| crate::api::project::resolve_id(n))
                .transpose()?;
            crate::api::task::create(title, project_id.as_deref())
        }
        "complete" => {
            let project = args.get(2).ok_or("usage: ticktick complete <project> <task_id>")?;
            let task_id = args.get(3).ok_or("usage: ticktick complete <project> <task_id>")?;
            let project_id = crate::api::project::resolve_id(project)?;
            crate::api::task::complete(&project_id, task_id)
        }
        "delete" => {
            let project = args.get(2).ok_or("usage: ticktick delete <project> <task_id>")?;
            let task_id = args.get(3).ok_or("usage: ticktick delete <project> <task_id>")?;
            let project_id = crate::api::project::resolve_id(project)?;
            crate::api::task::delete(&project_id, task_id)
        }
        cmd => Err(format!("unknown command: {cmd}\nRun 'ticktick help' for usage")),
    }
}
