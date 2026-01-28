use std::env;

const VERSION: &str = env!("CARGO_PKG_VERSION");

const HELP: &str = "\
ticktick - TickTick CLI

A command-line interface for managing tasks and projects in TickTick.

\x1b[33mUSAGE:\x1b[0m
    ticktick <COMMAND> [OPTIONS]

\x1b[33mCOMMANDS:\x1b[0m
    login                    Authenticate with TickTick via OAuth
    logout                   Remove stored credentials from local config
    projects                 List all projects in your TickTick account
    project <name>           Get details for a specific project by name or ID
    tasks [project]          List tasks, optionally filtered by project
    add <title> [-p project] Create a new task with the given title
    complete <project> <tid> Mark a task as complete
    delete <project> <tid>   Permanently delete a task
    help                     Show this help message
    version                  Show version information

\x1b[33mEXAMPLES:\x1b[0m
    ticktick login
    ticktick projects
    ticktick tasks
    ticktick tasks Personal
    ticktick tasks 'Work Projects'
    ticktick add 'Buy groceries' -p Personal
    ticktick complete Personal abc123
    ticktick delete Personal abc123

\x1b[33mGLOBAL OPTIONS:\x1b[0m
    -h, --help      Show help for a command
    -v, --version   Show version information

\x1b[33mCONFIGURATION:\x1b[0m
    Config file: ~/.config/ticktick-cli/config.json
    Environment: TICKTICK_CLIENT_ID, TICKTICK_CLIENT_SECRET, TICKTICK_ACCESS_TOKEN

\x1b[33mMORE INFO:\x1b[0m
    Run 'ticktick <command> --help' for detailed help on a specific command.
";

const HELP_LOGIN: &str = "\
ticktick-login - Authenticate with TickTick

\x1b[33mUSAGE:\x1b[0m
    ticktick login

\x1b[33mDESCRIPTION:\x1b[0m
    Initiates the OAuth authentication flow with TickTick. This command will:

    1. Start a local web server to receive the OAuth callback
    2. Open your browser to the TickTick authorization page
    3. Wait for you to authorize the application
    4. Store the access token in ~/.config/ticktick-cli/config.json

\x1b[33mPREREQUISITES:\x1b[0m
    Before running this command, you need to set up a TickTick developer app:

    1. Go to https://developer.ticktick.com/manage
    2. Create a new app with redirect URI: http://127.0.0.1:8080/callback
    3. Set environment variables or create a .env file:
       - TICKTICK_CLIENT_ID=<your_client_id>
       - TICKTICK_CLIENT_SECRET=<your_client_secret>

\x1b[33mEXAMPLES:\x1b[0m
    ticktick login

\x1b[33mSEE ALSO:\x1b[0m
    ticktick logout - Remove stored credentials
";

const HELP_LOGOUT: &str = "\
ticktick-logout - Remove stored credentials

\x1b[33mUSAGE:\x1b[0m
    ticktick logout

\x1b[33mDESCRIPTION:\x1b[0m
    Removes the stored authentication credentials from the local configuration
    file. After logging out, you will need to run 'ticktick login' again to
    use commands that require authentication.

    This command deletes the access_token and refresh_token from:
    ~/.config/ticktick-cli/config.json

\x1b[33mEXAMPLES:\x1b[0m
    ticktick logout

\x1b[33mSEE ALSO:\x1b[0m
    ticktick login - Authenticate with TickTick
";

const HELP_PROJECTS: &str = "\
ticktick-projects - List all projects

\x1b[33mUSAGE:\x1b[0m
    ticktick projects

\x1b[33mDESCRIPTION:\x1b[0m
    Lists all projects in your TickTick account. Each project is displayed
    with its name and ID. The project ID can be used with other commands
    like 'tasks', 'complete', and 'delete'.

\x1b[33mOUTPUT:\x1b[0m
    Displays a list of projects with:
    - Project name
    - Project ID (used for referencing in other commands)

\x1b[33mEXAMPLES:\x1b[0m
    ticktick projects

\x1b[33mSEE ALSO:\x1b[0m
    ticktick project <name> - Get details for a specific project
    ticktick tasks [project] - List tasks in a project
";

const HELP_PROJECT: &str = "\
ticktick-project - Get project details

\x1b[33mUSAGE:\x1b[0m
    ticktick project <name>
    ticktick project <id>

\x1b[33mARGUMENTS:\x1b[0m
    <name>    The name of the project (case-insensitive, supports partial match)
    <id>      The project ID (exact match)

\x1b[33mDESCRIPTION:\x1b[0m
    Retrieves and displays detailed information about a specific project.
    You can specify the project by its name or ID.

    Project name matching:
    - Case-insensitive: 'personal' matches 'Personal'
    - Supports partial match if unambiguous
    - Use quotes for names with spaces: 'Work Projects'

\x1b[33mEXAMPLES:\x1b[0m
    ticktick project Personal
    ticktick project 'Work Projects'
    ticktick project abc123def456

\x1b[33mSEE ALSO:\x1b[0m
    ticktick projects - List all projects
";

const HELP_TASKS: &str = "\
ticktick-tasks - List tasks

\x1b[33mUSAGE:\x1b[0m
    ticktick tasks
    ticktick tasks <project>

\x1b[33mARGUMENTS:\x1b[0m
    [project]    Optional. Filter tasks by project name or ID.
                 If omitted, lists tasks from all projects.

\x1b[33mDESCRIPTION:\x1b[0m
    Lists tasks from your TickTick account. When a project is specified,
    only tasks from that project are shown. Otherwise, all tasks are listed.

    Each task displays:
    - Task title
    - Task ID (used for complete/delete commands)
    - Project name
    - Due date (if set)
    - Priority level

\x1b[33mEXAMPLES:\x1b[0m
    ticktick tasks                    # List all tasks
    ticktick tasks Personal           # List tasks in 'Personal' project
    ticktick tasks 'Work Projects'    # List tasks in project with spaces

\x1b[33mSEE ALSO:\x1b[0m
    ticktick add - Create a new task
    ticktick complete - Mark a task as complete
    ticktick delete - Delete a task
";

const HELP_ADD: &str = "\
ticktick-add - Create a new task

\x1b[33mUSAGE:\x1b[0m
    ticktick add <title>
    ticktick add <title> -p <project>
    ticktick add <title> --project <project>

\x1b[33mARGUMENTS:\x1b[0m
    <title>      The title/description of the task to create.
                 Use quotes for titles with spaces.

\x1b[33mOPTIONS:\x1b[0m
    -p, --project <project>    Add task to a specific project.
                               Can be project name or ID.
                               If omitted, adds to inbox/default project.

\x1b[33mDESCRIPTION:\x1b[0m
    Creates a new task in TickTick with the specified title. Optionally
    assign it to a specific project using the -p flag.

\x1b[33mEXAMPLES:\x1b[0m
    ticktick add 'Buy groceries'
    ticktick add 'Review pull request' -p Work
    ticktick add 'Call mom' --project Personal
    ticktick add 'Team meeting prep' -p 'Work Projects'

\x1b[33mSEE ALSO:\x1b[0m
    ticktick tasks - List tasks
    ticktick complete - Mark a task as complete
";

const HELP_COMPLETE: &str = "\
ticktick-complete - Mark a task as complete

\x1b[33mUSAGE:\x1b[0m
    ticktick complete <project> <task_id>

\x1b[33mARGUMENTS:\x1b[0m
    <project>     The project name or ID containing the task.
    <task_id>     The ID of the task to complete.
                  Find task IDs using 'ticktick tasks'.

\x1b[33mDESCRIPTION:\x1b[0m
    Marks a task as complete in TickTick. The task will be moved to the
    completed tasks section. This action can be undone in the TickTick
    app or web interface.

    To find the task ID, first run 'ticktick tasks <project>' to list
    all tasks with their IDs.

\x1b[33mEXAMPLES:\x1b[0m
    ticktick complete Personal abc123def456
    ticktick complete 'Work Projects' task789xyz

\x1b[33mSEE ALSO:\x1b[0m
    ticktick tasks - List tasks to find task IDs
    ticktick delete - Permanently delete a task
";

const HELP_DELETE: &str = "\
ticktick-delete - Delete a task

\x1b[33mUSAGE:\x1b[0m
    ticktick delete <project> <task_id>

\x1b[33mARGUMENTS:\x1b[0m
    <project>     The project name or ID containing the task.
    <task_id>     The ID of the task to delete.
                  Find task IDs using 'ticktick tasks'.

\x1b[33mDESCRIPTION:\x1b[0m
    Permanently deletes a task from TickTick. This action cannot be undone.
    Use 'ticktick complete' if you want to mark a task as done without
    removing it.

    To find the task ID, first run 'ticktick tasks <project>' to list
    all tasks with their IDs.

\x1b[33mWARNING:\x1b[0m
    This action is permanent and cannot be undone!

\x1b[33mEXAMPLES:\x1b[0m
    ticktick delete Personal abc123def456
    ticktick delete 'Work Projects' task789xyz

\x1b[33mSEE ALSO:\x1b[0m
    ticktick complete - Mark a task as complete (non-destructive)
    ticktick tasks - List tasks to find task IDs
";

const HELP_VERSION: &str = "\
ticktick-version - Show version information

\x1b[33mUSAGE:\x1b[0m
    ticktick version
    ticktick -v
    ticktick --version

\x1b[33mDESCRIPTION:\x1b[0m
    Displays the current version of the ticktick CLI tool.
";

/// Check if the arguments contain a help flag
fn wants_help(args: &[String]) -> bool {
    args.iter().any(|a| a == "-h" || a == "--help" || a == "help")
}

pub fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print!("{HELP}");
        return Ok(());
    }

    match args[1].as_str() {
        "help" | "-h" | "--help" => {
            // Check if help is requested for a specific command
            if let Some(cmd) = args.get(2) {
                return show_command_help(cmd);
            }
            print!("{HELP}");
            Ok(())
        }
        "version" | "-v" | "--version" => {
            if wants_help(&args[2..]) {
                print!("{HELP_VERSION}");
                return Ok(());
            }
            println!("ticktick {VERSION}");
            Ok(())
        }
        "login" => {
            if wants_help(&args[2..]) {
                print!("{HELP_LOGIN}");
                return Ok(());
            }
            crate::api::auth::login()
        }
        "logout" => {
            if wants_help(&args[2..]) {
                print!("{HELP_LOGOUT}");
                return Ok(());
            }
            crate::config::logout()
        }
        "projects" => {
            if wants_help(&args[2..]) {
                print!("{HELP_PROJECTS}");
                return Ok(());
            }
            crate::api::project::list()
        }
        "project" => {
            if wants_help(&args[2..]) {
                print!("{HELP_PROJECT}");
                return Ok(());
            }
            let name = args.get(2).ok_or("usage: ticktick project <name>\n\nRun 'ticktick project --help' for more information.")?;
            let id = crate::api::project::resolve_id(name)?;
            crate::api::project::get_by_id(&id)
        }
        "tasks" => {
            if wants_help(&args[2..]) {
                print!("{HELP_TASKS}");
                return Ok(());
            }
            let project = args.get(2).map(|s| crate::api::project::resolve_id(s)).transpose()?;
            crate::api::task::list_by_project(project.as_deref())
        }
        "add" => {
            if wants_help(&args[2..]) {
                print!("{HELP_ADD}");
                return Ok(());
            }
            let title = args.get(2).ok_or("usage: ticktick add <title> [-p <project>]\n\nRun 'ticktick add --help' for more information.")?;
            let project_name = args.iter().position(|a| a == "--project" || a == "-p")
                .and_then(|i| args.get(i + 1));
            let project_id = project_name
                .map(|n| crate::api::project::resolve_id(n))
                .transpose()?;
            crate::api::task::create(title, project_id.as_deref())
        }
        "complete" => {
            if wants_help(&args[2..]) {
                print!("{HELP_COMPLETE}");
                return Ok(());
            }
            let project = args.get(2).ok_or("usage: ticktick complete <project> <task_id>\n\nRun 'ticktick complete --help' for more information.")?;
            let task_id = args.get(3).ok_or("usage: ticktick complete <project> <task_id>\n\nRun 'ticktick complete --help' for more information.")?;
            let project_id = crate::api::project::resolve_id(project)?;
            crate::api::task::complete(&project_id, task_id)
        }
        "delete" => {
            if wants_help(&args[2..]) {
                print!("{HELP_DELETE}");
                return Ok(());
            }
            let project = args.get(2).ok_or("usage: ticktick delete <project> <task_id>\n\nRun 'ticktick delete --help' for more information.")?;
            let task_id = args.get(3).ok_or("usage: ticktick delete <project> <task_id>\n\nRun 'ticktick delete --help' for more information.")?;
            let project_id = crate::api::project::resolve_id(project)?;
            crate::api::task::delete(&project_id, task_id)
        }
        cmd => Err(format!("unknown command: {cmd}\n\nRun 'ticktick help' for usage information.")),
    }
}

/// Show help for a specific command
fn show_command_help(cmd: &str) -> Result<(), String> {
    match cmd {
        "login" => print!("{HELP_LOGIN}"),
        "logout" => print!("{HELP_LOGOUT}"),
        "projects" => print!("{HELP_PROJECTS}"),
        "project" => print!("{HELP_PROJECT}"),
        "tasks" => print!("{HELP_TASKS}"),
        "add" => print!("{HELP_ADD}"),
        "complete" => print!("{HELP_COMPLETE}"),
        "delete" => print!("{HELP_DELETE}"),
        "version" | "-v" | "--version" => print!("{HELP_VERSION}"),
        "help" | "-h" | "--help" => print!("{HELP}"),
        _ => return Err(format!("unknown command: {cmd}\n\nRun 'ticktick help' for available commands.")),
    }
    Ok(())
}
