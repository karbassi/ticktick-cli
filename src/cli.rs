use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::CompleteEnv;

/// Task priority level
#[derive(Clone, Copy, ValueEnum)]
pub enum Priority {
    /// No priority (default)
    None,
    /// Low priority
    Low,
    /// Medium priority
    Medium,
    /// High priority
    High,
}

impl Priority {
    pub fn to_api_value(self) -> i32 {
        match self {
            Priority::None => 0,
            Priority::Low => 1,
            Priority::Medium => 3,
            Priority::High => 5,
        }
    }
}

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
  ticktick-cli login                              # Authenticate with TickTick
  ticktick-cli project list                       # List all projects
  ticktick-cli task list                          # List all tasks
  ticktick-cli task list Personal                 # List tasks in Personal project
  ticktick-cli task add 'Buy milk'                # Add task to inbox
  ticktick-cli task add 'Review PR' -p Work       # Add task to Work project
  ticktick-cli task add 'Submit report' -d 2025-03-01  # Add with due date
  ticktick-cli task complete Personal abc123      # Complete a task
  ticktick-cli task delete Personal abc123        # Delete a task
")]
pub struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

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

    /// Manage tasks
    #[command(subcommand)]
    Task(TaskCommands),

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

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

#[derive(Subcommand)]
enum TaskCommands {
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
  ticktick-cli task list                    # List all tasks
  ticktick-cli task list Personal           # List tasks in Personal project
  ticktick-cli task list 'Work Projects'    # List tasks in project with spaces
")]
    List {
        /// Filter by project name or ID
        project: Option<String>,
    },

    /// Get details for a specific task by project and task ID
    ///
    /// Displays all fields for a single task including title, due date,
    /// priority, status, tags, and more.
    ///
    /// To find the task ID, run 'ticktick-cli task list <project>' first.
    #[command(after_long_help = "\
Examples:
  ticktick-cli task get Personal abc123def456
  ticktick-cli task get 'Work Projects' 789xyz123
")]
    Get {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli task list')
        task_id: String,
    },

    /// Create a new task with the given title
    ///
    /// Creates a new task in TickTick. Optionally assign it to a specific
    /// project using the -p/--project flag. If no project is specified,
    /// the task is added to the inbox/default project.
    #[command(visible_alias = "new")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task add 'Buy groceries'
  ticktick-cli task add 'Review pull request' -p Work
  ticktick-cli task add 'Call mom' --project Personal
  ticktick-cli task add 'Team meeting' -p 'Work Projects'
  ticktick-cli task add 'Submit report' --due 2025-03-01
  ticktick-cli task add 'Call dentist' -d tomorrow
  ticktick-cli task add 'Urgent fix' --priority high
")]
    Add {
        /// Task title (use quotes for titles with spaces)
        title: String,

        /// Project name or ID to add the task to
        #[arg(short, long)]
        project: Option<String>,

        /// Due date: YYYY-MM-DD, 'today', or 'tomorrow'
        #[arg(short, long)]
        due: Option<String>,

        /// Priority: none, low, medium, high
        #[arg(short = 'P', long)]
        priority: Option<Priority>,

        /// Preview without creating the task
        #[arg(short = 'n', long)]
        dry_run: bool,
    },

    /// Edit an existing task
    ///
    /// Update properties of an existing task such as due date or title.
    /// Requires the project and task ID (find via 'ticktick-cli task list').
    #[command(visible_alias = "update")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task edit Personal abc123 --due 2025-04-01
  ticktick-cli task edit Work xyz789 --due tomorrow
  ticktick-cli task edit Personal abc123 --title 'New title'
  ticktick-cli task edit Personal abc123 --clear-due
  ticktick-cli task edit Personal abc123 --priority high
")]
    Edit {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli task list')
        task_id: String,

        /// Set due date: YYYY-MM-DD, 'today', or 'tomorrow'
        #[arg(short, long)]
        due: Option<String>,

        /// Remove the due date
        #[arg(long, conflicts_with = "due")]
        clear_due: bool,

        /// Set a new title
        #[arg(short, long)]
        title: Option<String>,

        /// Priority: none, low, medium, high
        #[arg(short = 'P', long)]
        priority: Option<Priority>,
    },

    /// Mark a task as complete
    ///
    /// Marks a task as complete in TickTick. The task will be moved to
    /// the completed tasks section. This action can be undone in the
    /// TickTick app or web interface.
    ///
    /// To find the task ID, run 'ticktick-cli task list <project>' first.
    #[command(visible_alias = "done")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task complete Personal abc123def456
  ticktick-cli task complete 'Work Projects' 789xyz123
  ticktick-cli task done Personal abc123def456        # Using alias
")]
    Complete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli task list')
        task_id: String,
    },

    /// Permanently delete a task
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes a task from TickTick. Use 'ticktick-cli task complete'
    /// if you want to mark a task as done without removing it.
    ///
    /// To find the task ID, run 'ticktick-cli task list <project>' first.
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task delete Personal abc123def456
  ticktick-cli task delete 'Work Projects' 789xyz123
  ticktick-cli task delete --force Personal abc123def456
  ticktick-cli task rm Personal abc123def456          # Using alias
")]
    Delete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick-cli task list')
        task_id: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ProjectCommands {
    /// List all projects in your TickTick account
    ///
    /// Displays all projects with their names and IDs. The project ID
    /// can be used with other commands like 'task list', 'task complete', and 'task delete'.
    #[command(after_long_help = "\
Examples:
  ticktick-cli project list
")]
    List,

    /// Get details for a specific project by name or ID
    ///
    /// Project name matching:
    ///   - Case-insensitive: 'personal' matches 'Personal'
    ///   - Supports partial match if unambiguous
    ///   - Use quotes for names with spaces: 'Work Projects'
    #[command(after_long_help = "\
Examples:
  ticktick-cli project get Personal
  ticktick-cli project get 'Work Projects'
  ticktick-cli project get 6789abcd1234ef56
")]
    Get {
        /// Project name (case-insensitive) or ID
        name: String,
    },

    /// Create a new project
    ///
    /// Creates a new project in TickTick with the given name.
    #[command(after_long_help = "\
Examples:
  ticktick-cli project add 'Side Projects'
  ticktick-cli project add Work
")]
    Add {
        /// Project name
        name: String,
    },

    /// Rename an existing project
    ///
    /// Changes the name of an existing project. Identify the project
    /// by its current name or ID.
    #[command(after_long_help = "\
Examples:
  ticktick-cli project edit Personal --name 'My Tasks'
  ticktick-cli project edit Work --name 'Work Tasks'
")]
    Edit {
        /// Current project name or ID
        project: String,

        /// New project name
        #[arg(short, long)]
        name: Option<String>,
    },

    /// Permanently delete a project
    ///
    /// WARNING: This action cannot be undone! All tasks in the project
    /// will also be deleted.
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli project delete 'Old Project'
  ticktick-cli project delete --force 'Old Project'
")]
    Delete {
        /// Project name or ID
        project: String,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub fn run() -> Result<(), String> {
    CompleteEnv::with_factory(Cli::command).complete();

    let cli = Cli::parse();

    crate::output::init(cli.verbose);

    let command = match cli.command {
        Some(cmd) => cmd,
        None => {
            print_usage();
            return Ok(());
        }
    };

    match command {
        Commands::Login => crate::api::auth::login(),
        Commands::Logout => crate::config::logout(),
        Commands::Task(subcmd) => match subcmd {
            TaskCommands::List { project } => {
                let project_id = project
                    .map(|s| crate::api::project::resolve_id(&s))
                    .transpose()?;
                crate::api::task::list_by_project(project_id.as_deref())
            }
            TaskCommands::Get { project, task_id } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::task::get_by_id(&project_id, &task_id)
            }
            TaskCommands::Add {
                title,
                project,
                due,
                priority,
                dry_run,
            } => {
                let project_id = project
                    .map(|n| crate::api::project::resolve_id(&n))
                    .transpose()?;
                let due_date = due
                    .map(|d| crate::api::task::parse_due_date(&d))
                    .transpose()?;
                let priority = priority.map(|p| p.to_api_value());
                crate::api::task::create(
                    &title,
                    project_id.as_deref(),
                    due_date.as_deref(),
                    priority,
                    dry_run,
                )
            }
            TaskCommands::Edit {
                project,
                task_id,
                due,
                clear_due,
                title,
                priority,
            } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                let due_date = if clear_due {
                    Some(crate::api::task::DueDate::Clear)
                } else if let Some(d) = due {
                    Some(crate::api::task::DueDate::Set(
                        crate::api::task::parse_due_date(&d)?,
                    ))
                } else {
                    None
                };
                let priority = priority.map(|p| p.to_api_value());
                crate::api::task::update(
                    &project_id,
                    &task_id,
                    title.as_deref(),
                    due_date,
                    priority,
                )
            }
            TaskCommands::Complete { project, task_id } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::task::complete(&project_id, &task_id)
            }
            TaskCommands::Delete {
                project,
                task_id,
                force,
            } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::task::delete(&project_id, &task_id, force)
            }
        },
        Commands::Project(subcmd) => match subcmd {
            ProjectCommands::List => crate::api::project::list(),
            ProjectCommands::Get { name } => {
                let id = crate::api::project::resolve_id(&name)?;
                crate::api::project::get_by_id(&id)
            }
            ProjectCommands::Add { name } => crate::api::project::create(&name),
            ProjectCommands::Edit { project, name } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::project::update(&project_id, name.as_deref())
            }
            ProjectCommands::Delete { project, force } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::project::delete(&project_id, force)
            }
        },
        Commands::Init { local, force } => crate::config::init(local, force),
        Commands::Completions { shell } => {
            clap_complete::generate(
                shell,
                &mut Cli::command(),
                "ticktick-cli",
                &mut std::io::stdout(),
            );
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

        let nested: Vec<_> = sub.get_subcommands().collect();
        if nested.is_empty() {
            // Top-level command with no subcommands
            print_command(name, sub);
        } else {
            // Nested subcommand group
            for nested_sub in sub.get_subcommands() {
                let nested_name = nested_sub.get_name();
                if nested_name == "help" {
                    continue;
                }
                let full_name = format!("{name} {nested_name}");
                print_command(&full_name, nested_sub);
            }
        }
    }
}

fn print_command(name: &str, cmd: &clap::Command) {
    println!("{name}");
    if let Some(about) = cmd.get_about() {
        println!("  {about}");
    }
    for arg in cmd.get_arguments() {
        if arg.is_hide_set() || arg.get_id() == "help" || arg.get_id() == "version" {
            continue;
        }
        let long = arg.get_long().map(|l| format!("--{l}")).unwrap_or_default();
        let short = arg.get_short().map(|s| format!("-{s}")).unwrap_or_default();
        let flag = match (short.is_empty(), long.is_empty()) {
            (false, false) => format!("{short}, {long}"),
            (false, true) => short,
            (true, false) => long,
            _ => String::new(),
        };
        let help = arg.get_help().map(|h| h.to_string()).unwrap_or_default();
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
