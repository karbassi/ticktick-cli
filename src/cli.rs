use clap::{CommandFactory, Parser, Subcommand};
use clap_complete::CompleteEnv;

/// TickTick CLI - A command-line interface for managing tasks and projects
///
/// Manage your TickTick tasks and projects from the terminal. Supports
/// authentication, listing projects/tasks, creating tasks, and more.
///
/// Configuration is stored in $XDG_CONFIG_HOME/ticktick-cli/config.json
///
/// Environment variables:
///   TICKTICK_CLIENT_ID      OAuth client ID (required)
///   TICKTICK_CLIENT_SECRET  OAuth client secret (required)
///   TICKTICK_ACCESS_TOKEN   Access token (optional, for direct auth)
///   TICKTICK_OAUTH_PORT     OAuth callback port (default: 8080)
#[derive(Parser)]
#[command(
    name = "ticktick-cli",
    version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_SHA"), " ", env!("BUILD_DATE"), ")"),
    about,
    long_about
)]
#[command(after_long_help = "\
Examples:
  ticktick-cli login                          # Authenticate with TickTick
  ticktick-cli projects                       # List all projects
  ticktick-cli tasks                          # List all tasks
  ticktick-cli tasks Personal                 # List tasks in Personal project
  ticktick-cli add 'Buy milk'                 # Add task to inbox
  ticktick-cli add 'Review PR' -p Work        # Add task to Work project
  ticktick-cli complete Personal abc123       # Complete a task
  ticktick-cli delete Personal abc123         # Delete a task
")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Increase verbosity (-v, -vv, -vvv)
    #[arg(short, long, action = clap::ArgAction::Count, global = true)]
    verbose: u8,
}

#[derive(Subcommand)]
enum Commands {
    /// Authenticate with TickTick via OAuth
    ///
    /// Initiates the OAuth authentication flow with TickTick:
    ///
    /// 1. Starts a local web server to receive the OAuth callback
    /// 2. Opens your browser to the TickTick authorization page
    /// 3. Waits for you to authorize the application
    /// 4. Stores the access token in $XDG_CONFIG_HOME/ticktick-cli/config.json
    ///
    /// Prerequisites:
    ///   1. Go to https://developer.ticktick.com/manage
    ///   2. Create a new app with redirect URI: http://127.0.0.1:<port>/callback
    ///   3. Set TICKTICK_CLIENT_ID, TICKTICK_CLIENT_SECRET, and TICKTICK_OAUTH_PORT env vars
    ///      (TICKTICK_OAUTH_PORT defaults to 8080)
    #[command(after_long_help = "\
Examples:
  ticktick-cli login
")]
    Login,

    /// Remove stored credentials from local config
    ///
    /// Removes the stored authentication credentials from the local
    /// configuration file. After logging out, you will need to run
    /// 'ticktick-cli login' again to use commands that require authentication.
    #[command(after_long_help = "\
Examples:
  ticktick-cli logout
")]
    Logout,

    /// List all projects in your TickTick account
    ///
    /// Displays all projects with their names and IDs. The project ID
    /// can be used with other commands like 'tasks', 'complete', and 'delete'.
    #[command(after_long_help = "\
Examples:
  ticktick-cli projects
")]
    Projects,

    /// Get details for a specific project by name or ID
    ///
    /// Project name matching:
    ///   - Case-insensitive: 'personal' matches 'Personal'
    ///   - Supports partial match if unambiguous
    ///   - Use quotes for names with spaces: 'Work Projects'
    #[command(after_long_help = "\
Examples:
  ticktick-cli project Personal
  ticktick-cli project 'Work Projects'
  ticktick-cli project 6789abcd1234ef56
")]
    Project {
        /// Project name (case-insensitive) or ID
        name: String,
    },

    /// List tasks, optionally filtered by project
    ///
    /// Lists tasks from your TickTick account. When a project is specified,
    /// only tasks from that project are shown. Otherwise, all tasks are listed.
    ///
    /// Each task displays:
    ///   - Task title and ID (used for complete/delete commands)
    ///   - Project name, due date, and priority level
    #[command(after_long_help = "\
Examples:
  ticktick-cli tasks                    # List all tasks
  ticktick-cli tasks Personal           # List tasks in Personal project
  ticktick-cli tasks 'Work Projects'    # List tasks in project with spaces
")]
    Tasks {
        /// Filter by project name or ID
        project: Option<String>,
    },

    /// Create a new task with the given title
    ///
    /// Creates a new task in TickTick. Optionally assign it to a specific
    /// project using the -p/--project flag. If no project is specified,
    /// the task is added to the inbox/default project.
    #[command(visible_alias = "new")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli add 'Buy groceries'
  ticktick-cli add 'Review pull request' -p Work
  ticktick-cli add 'Call mom' --project Personal
  ticktick-cli add 'Team meeting' -p 'Work Projects'
")]
    Add {
        /// Task title (use quotes for titles with spaces)
        title: String,

        /// Project name or ID to add the task to
        #[arg(short, long)]
        project: Option<String>,

        /// Preview without creating the task
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Mark a task as complete
    ///
    /// Marks a task as complete in TickTick. The task will be moved to
    /// the completed tasks section. This action can be undone in the
    /// TickTick app or web interface.
    ///
    /// To find the task ID, run 'ticktick-cli tasks <project>' first.
    #[command(visible_alias = "done")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli complete Personal abc123def456
  ticktick-cli complete 'Work Projects' 789xyz123
  ticktick-cli done Personal abc123def456        # Using alias
")]
    Complete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli tasks')
        task_id: String,
    },

    /// Permanently delete a task
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes a task from TickTick. Use 'ticktick-cli complete'
    /// if you want to mark a task as done without removing it.
    ///
    /// To find the task ID, run 'ticktick-cli tasks <project>' first.
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli delete Personal abc123def456
  ticktick-cli delete 'Work Projects' 789xyz123
  ticktick-cli delete --force Personal abc123def456
  ticktick-cli rm Personal abc123def456          # Using alias
")]
    Delete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli tasks')
        task_id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Initialize a .env configuration file with required variables
    ///
    /// Creates a .env template with the TickTick API credentials fields.
    /// By default, writes to $XDG_CONFIG_HOME/ticktick-cli/.env.
    /// Use --local to write to the current directory instead.
    #[command(after_long_help = "\
Examples:
  ticktick-cli init                 # Create .env in config directory
  ticktick-cli init --local         # Create .env in current directory
  ticktick-cli init --local --force # Overwrite existing .env
")]
    Init {
        /// Create .env in the current directory instead of the config directory
        #[arg(short, long)]
        local: bool,

        /// Overwrite existing .env file
        #[arg(short, long)]
        force: bool,
    },

    /// Generate shell completions
    #[command(after_long_help = "\
Examples:
  ticktick-cli completions bash >> ~/.bashrc
  ticktick-cli completions zsh > ~/.zfunc/_ticktick-cli
  ticktick-cli completions fish > ~/.config/fish/completions/ticktick-cli.fish
")]
    Completions {
        /// Shell to generate completions for
        shell: clap_complete::Shell,
    },

    /// Print concise help for all commands
    Usage,
}

pub fn run() -> Result<(), String> {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    crate::output::init(cli.verbose);

    match cli.command {
        Commands::Login => crate::api::auth::login(),
        Commands::Logout => crate::config::logout(),
        Commands::Projects => crate::api::project::list(),
        Commands::Project { name } => {
            let id = crate::api::project::resolve_id(&name)?;
            crate::api::project::get_by_id(&id)
        }
        Commands::Tasks { project } => {
            let project_id = project
                .map(|s| crate::api::project::resolve_id(&s))
                .transpose()?;
            crate::api::task::list_by_project(project_id.as_deref())
        }
        Commands::Add { title, project, dry_run } => {
            let project_id = project
                .map(|n| crate::api::project::resolve_id(&n))
                .transpose()?;
            crate::api::task::create(&title, project_id.as_deref(), dry_run)
        }
        Commands::Complete { project, task_id } => {
            let project_id = crate::api::project::resolve_id(&project)?;
            crate::api::task::complete(&project_id, &task_id)
        }
        Commands::Delete {
            project,
            task_id,
            force,
        } => {
            let project_id = crate::api::project::resolve_id(&project)?;
            crate::api::task::delete(&project_id, &task_id, force)
        }
        Commands::Init { local, force } => crate::config::init(local, force),
        Commands::Completions { shell } => {
            clap_complete::generate(shell, &mut Cli::command(), "ticktick-cli", &mut std::io::stdout());
            Ok(())
        }
        Commands::Usage => {
            print_usage();
            Ok(())
        }
    }
}

fn print_usage() {
    let cmd = Cli::command();
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        if name == "usage" || name == "help" {
            continue;
        }
        println!("{name}");
        if let Some(about) = sub.get_about() {
            println!("  {about}");
        }
        for arg in sub.get_arguments() {
            if arg.is_hide_set() || arg.get_id() == "help" || arg.get_id() == "version" {
                continue;
            }
            let long = arg
                .get_long()
                .map(|l| format!("--{l}"))
                .unwrap_or_default();
            let short = arg
                .get_short()
                .map(|s| format!("-{s}"))
                .unwrap_or_default();
            let flag = match (short.is_empty(), long.is_empty()) {
                (false, false) => format!("{short}, {long}"),
                (false, true) => short,
                (true, false) => long,
                _ => String::new(),
            };
            let help = arg
                .get_help()
                .map(|h| h.to_string())
                .unwrap_or_default();
            if flag.is_empty() {
                let id = arg.get_id().to_string();
                if id == "verbose" {
                    continue;
                }
                println!("  <{id}>  {help}");
            } else {
                println!("  {flag}  {help}");
            }
        }
        println!("---");
    }
}
