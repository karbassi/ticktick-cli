use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use clap_complete::CompleteEnv;
use serde::Serialize;

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

    /// Create new tasks with the given titles
    ///
    /// Creates one or more tasks in TickTick. Optionally assign them to a
    /// specific project using the -p/--project flag. If no project is specified,
    /// tasks are added to the inbox/default project.
    #[command(visible_alias = "new")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task add 'Buy groceries'
  ticktick-cli task add 'Task 1' 'Task 2' 'Task 3'  # Add multiple tasks
  ticktick-cli task add 'Review pull request' -p Work
  ticktick-cli task add 'Call mom' --project Personal
  ticktick-cli task add 'Submit report' --due 2025-03-01
  ticktick-cli task add 'Call dentist' -d tomorrow
  ticktick-cli task add 'Urgent fix' --priority high
  echo -e 'Task A\\nTask B' | ticktick-cli task add --stdin -p Work
")]
    Add {
        /// Task titles (use quotes for titles with spaces)
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        titles: Vec<String>,

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

        /// Read titles from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// Edit one or more existing tasks
    ///
    /// Update properties of existing tasks such as due date or title.
    /// Requires the project and task IDs (find via 'ticktick-cli task list').
    /// Note: --title can only be used with a single task ID.
    #[command(visible_alias = "update")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task edit Personal abc123 --due 2025-04-01
  ticktick-cli task edit Work id1 id2 id3 --due tomorrow   # Edit multiple tasks
  ticktick-cli task edit Personal abc123 --title 'New title'
  ticktick-cli task edit Personal abc123 --clear-due
  ticktick-cli task edit Personal abc123 --priority high
  echo -e 'id1\\nid2' | ticktick-cli task edit Personal --stdin --due tomorrow
")]
    Edit {
        /// Project name or ID containing the task
        project: String,

        /// Task IDs (find via 'ticktick-cli task list')
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Set due date: YYYY-MM-DD, 'today', or 'tomorrow'
        #[arg(short, long)]
        due: Option<String>,

        /// Remove the due date
        #[arg(long, conflicts_with = "due")]
        clear_due: bool,

        /// Set a new title (only valid with a single task ID)
        #[arg(short, long)]
        title: Option<String>,

        /// Priority: none, low, medium, high
        #[arg(short = 'P', long)]
        priority: Option<Priority>,

        /// Read task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// Mark one or more tasks as complete
    ///
    /// Marks tasks as complete in TickTick. The tasks will be moved to
    /// the completed tasks section. This action can be undone in the
    /// TickTick app or web interface.
    ///
    /// To find task IDs, run 'ticktick-cli task list <project>' first.
    #[command(visible_alias = "done")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task complete Personal abc123def456
  ticktick-cli task complete Personal id1 id2 id3    # Complete multiple tasks
  ticktick-cli task done Personal abc123def456        # Using alias
  ticktick-cli task list Work | jq -r '.[].id' | ticktick-cli task complete Work --stdin
")]
    Complete {
        /// Project name or ID containing the task
        project: String,

        /// Task IDs (find via 'ticktick-cli task list')
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Read task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// Permanently delete one or more tasks
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes tasks from TickTick. Use 'ticktick-cli task complete'
    /// if you want to mark tasks as done without removing them.
    ///
    /// To find task IDs, run 'ticktick-cli task list <project>' first.
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task delete Personal abc123def456
  ticktick-cli task delete Personal id1 id2 id3      # Delete multiple tasks
  ticktick-cli task delete --force Personal abc123def456
  ticktick-cli task rm Personal abc123def456          # Using alias
  ticktick-cli task list Work | jq -r '.[].id' | ticktick-cli task delete Work --stdin --force
")]
    Delete {
        /// Project name or ID containing the task
        project: String,

        /// Task IDs (find via 'ticktick-cli task list')
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,

        /// Read task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
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

// ---------------------------------------------------------------------------
// Bulk helpers
// ---------------------------------------------------------------------------

/// Merge positional args with stdin lines (one item per line).
fn collect_inputs(mut positional: Vec<String>, from_stdin: bool) -> Result<Vec<String>, String> {
    if from_stdin {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        for line in buf.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() {
                positional.push(trimmed.to_string());
            }
        }
    }
    if positional.is_empty() {
        return Err("no inputs provided".to_string());
    }
    Ok(positional)
}

#[derive(Serialize)]
struct BulkResult {
    id: String,
    status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

/// Output bulk results.
///
/// - 1 item, success → output inner data (backward compat)
/// - 1 item, failure → return Err
/// - N items → output the BulkResult array, then Err summary if any failed
fn output_results(results: &[BulkResult]) -> Result<(), String> {
    if results.len() == 1 {
        let r = &results[0];
        if r.status == "ok" {
            if let Some(data) = &r.data {
                crate::output::success(data);
            } else {
                crate::output::success(&serde_json::json!({"status": "ok"}));
            }
            return Ok(());
        } else {
            return Err(r.error.clone().unwrap_or_else(|| "unknown error".into()));
        }
    }

    // Multiple items — always output the full array
    crate::output::success(&results);

    let failed = results.iter().filter(|r| r.status == "error").count();
    if failed > 0 {
        Err(format!("{failed} of {} operations failed", results.len()))
    } else {
        Ok(())
    }
}

/// Prompt for delete confirmation.
fn confirm_delete(count: usize, from_stdin: bool) -> Result<(), String> {
    use std::io::IsTerminal;

    let is_ci = std::env::var("CI").ok().as_deref() == Some("true");

    if is_ci || from_stdin || !std::io::stdin().is_terminal() {
        return Err(
            "refusing to delete without confirmation in non-interactive mode\n\n  hint: Use --force to skip confirmation"
                .to_string(),
        );
    }

    eprint!("Are you sure you want to delete {count} task(s)? [y/N] ");
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .map_err(|e| format!("failed to read input: {e}"))?;
    if !matches!(input.trim().to_lowercase().as_str(), "y" | "yes") {
        eprintln!("Cancelled.");
        return Ok(());
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Main dispatch
// ---------------------------------------------------------------------------

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
                let token = crate::config::get_access_token()?;
                let project_id = project
                    .map(|s| crate::api::project::resolve_id(&s))
                    .transpose()?;
                let tasks =
                    crate::api::task::list_by_project(&token, project_id.as_deref())?;
                crate::output::success(&tasks);
                Ok(())
            }
            TaskCommands::Get { project, task_id } => {
                let token = crate::config::get_access_token()?;
                let project_id = crate::api::project::resolve_id(&project)?;
                let task = crate::api::task::get_by_id(&token, &project_id, &task_id)?;
                crate::output::success(&task);
                Ok(())
            }
            TaskCommands::Add {
                titles,
                project,
                due,
                priority,
                dry_run,
                stdin,
            } => {
                let inputs = collect_inputs(titles, stdin)?;
                let project_id = project
                    .map(|n| crate::api::project::resolve_id(&n))
                    .transpose()?;
                let due_date = due
                    .map(|d| crate::api::task::parse_due_date(&d))
                    .transpose()?;
                let priority = priority.map(|p| p.to_api_value());

                if dry_run {
                    let previews: Vec<serde_json::Value> = inputs
                        .iter()
                        .map(|title| {
                            let mut body = serde_json::json!({ "title": title, "dryRun": true });
                            if let Some(pid) = &project_id {
                                body["projectId"] =
                                    serde_json::Value::String(pid.clone());
                            }
                            if let Some(d) = &due_date {
                                body["dueDate"] =
                                    serde_json::Value::String(d.clone());
                            }
                            if let Some(p) = priority {
                                body["priority"] = serde_json::Value::Number(p.into());
                            }
                            body
                        })
                        .collect();

                    if previews.len() == 1 {
                        crate::output::success(&previews[0]);
                    } else {
                        crate::output::success(&previews);
                    }
                    return Ok(());
                }

                let token = crate::config::get_access_token()?;
                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|title| {
                        match crate::api::task::create(
                            &token,
                            title,
                            project_id.as_deref(),
                            due_date.as_deref(),
                            priority,
                        ) {
                            Ok(task) => BulkResult {
                                id: title.clone(),
                                status: "ok".into(),
                                data: Some(
                                    serde_json::to_value(&task).unwrap_or_default(),
                                ),
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: title.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        }
                    })
                    .collect();

                output_results(&results)
            }
            TaskCommands::Edit {
                project,
                task_ids,
                due,
                clear_due,
                title,
                priority,
                stdin,
            } => {
                let inputs = collect_inputs(task_ids, stdin)?;

                if title.is_some() && inputs.len() > 1 {
                    return Err(
                        "--title can only be used with a single task ID".to_string()
                    );
                }

                let token = crate::config::get_access_token()?;
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

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|task_id| {
                        match crate::api::task::update(
                            &token,
                            &project_id,
                            task_id,
                            title.as_deref(),
                            due_date.clone(),
                            priority,
                        ) {
                            Ok(task) => BulkResult {
                                id: task_id.clone(),
                                status: "ok".into(),
                                data: Some(
                                    serde_json::to_value(&task).unwrap_or_default(),
                                ),
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: task_id.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        }
                    })
                    .collect();

                output_results(&results)
            }
            TaskCommands::Complete {
                project,
                task_ids,
                stdin,
            } => {
                let inputs = collect_inputs(task_ids, stdin)?;
                let token = crate::config::get_access_token()?;
                let project_id = crate::api::project::resolve_id(&project)?;

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|task_id| {
                        match crate::api::task::complete(&token, &project_id, task_id) {
                            Ok(()) => BulkResult {
                                id: task_id.clone(),
                                status: "ok".into(),
                                data: None,
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: task_id.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        }
                    })
                    .collect();

                output_results(&results)
            }
            TaskCommands::Delete {
                project,
                task_ids,
                force,
                stdin,
            } => {
                let inputs = collect_inputs(task_ids, stdin)?;

                if !force {
                    confirm_delete(inputs.len(), stdin)?;
                }

                let token = crate::config::get_access_token()?;
                let project_id = crate::api::project::resolve_id(&project)?;

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|task_id| {
                        match crate::api::task::delete(&token, &project_id, task_id) {
                            Ok(()) => BulkResult {
                                id: task_id.clone(),
                                status: "ok".into(),
                                data: None,
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: task_id.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        }
                    })
                    .collect();

                output_results(&results)
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
