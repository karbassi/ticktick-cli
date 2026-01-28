use clap::{Parser, Subcommand};

/// TickTick CLI - A command-line interface for managing tasks and projects
///
/// Manage your TickTick tasks and projects from the terminal. Supports
/// authentication, listing projects/tasks, creating tasks, and more.
///
/// Configuration is stored in ~/.config/ticktick-cli/config.json
///
/// Environment variables:
///   TICKTICK_CLIENT_ID      OAuth client ID
///   TICKTICK_CLIENT_SECRET  OAuth client secret
///   TICKTICK_ACCESS_TOKEN   Access token (optional, for direct auth)
#[derive(Parser)]
#[command(name = "ticktick", version, about, long_about)]
#[command(after_long_help = "\
Examples:
  ticktick login                          # Authenticate with TickTick
  ticktick projects                       # List all projects
  ticktick tasks                          # List all tasks
  ticktick tasks Personal                 # List tasks in Personal project
  ticktick add 'Buy milk'                 # Add task to inbox
  ticktick add 'Review PR' -p Work        # Add task to Work project
  ticktick complete Personal abc123       # Complete a task
  ticktick delete Personal abc123         # Delete a task
")]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
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
    /// 4. Stores the access token in ~/.config/ticktick-cli/config.json
    ///
    /// Prerequisites:
    ///   1. Go to https://developer.ticktick.com/manage
    ///   2. Create a new app with redirect URI: http://127.0.0.1:8585/callback
    ///   3. Set TICKTICK_CLIENT_ID and TICKTICK_CLIENT_SECRET env vars
    #[command(after_long_help = "\
Examples:
  ticktick login
")]
    Login,

    /// Remove stored credentials from local config
    ///
    /// Removes the stored authentication credentials from the local
    /// configuration file. After logging out, you will need to run
    /// 'ticktick login' again to use commands that require authentication.
    #[command(after_long_help = "\
Examples:
  ticktick logout
")]
    Logout,

    /// List all projects in your TickTick account
    ///
    /// Displays all projects with their names and IDs. The project ID
    /// can be used with other commands like 'tasks', 'complete', and 'delete'.
    #[command(after_long_help = "\
Examples:
  ticktick projects
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
  ticktick project Personal
  ticktick project 'Work Projects'
  ticktick project 6789abcd1234ef56
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
  ticktick tasks                    # List all tasks
  ticktick tasks Personal           # List tasks in Personal project
  ticktick tasks 'Work Projects'    # List tasks in project with spaces
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
  ticktick add 'Buy groceries'
  ticktick add 'Review pull request' -p Work
  ticktick add 'Call mom' --project Personal
  ticktick add 'Team meeting' -p 'Work Projects'
")]
    Add {
        /// Task title (use quotes for titles with spaces)
        title: String,

        /// Project name or ID to add the task to
        #[arg(short, long)]
        project: Option<String>,
    },

    /// Mark a task as complete
    ///
    /// Marks a task as complete in TickTick. The task will be moved to
    /// the completed tasks section. This action can be undone in the
    /// TickTick app or web interface.
    ///
    /// To find the task ID, run 'ticktick tasks <project>' first.
    #[command(visible_alias = "done")]
    #[command(after_long_help = "\
Examples:
  ticktick complete Personal abc123def456
  ticktick complete 'Work Projects' 789xyz123
  ticktick done Personal abc123def456        # Using alias
")]
    Complete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick tasks')
        task_id: String,
    },

    /// Permanently delete a task
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes a task from TickTick. Use 'ticktick complete'
    /// if you want to mark a task as done without removing it.
    ///
    /// To find the task ID, run 'ticktick tasks <project>' first.
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick delete Personal abc123def456
  ticktick delete 'Work Projects' 789xyz123
  ticktick rm Personal abc123def456          # Using alias
")]
    Delete {
        /// Project name or ID containing the task
        project: String,

        /// Task ID (find via 'ticktick tasks')
        task_id: String,
    },
}

pub fn run() -> Result<(), String> {
    let cli = Cli::parse();

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
        Commands::Add { title, project } => {
            let project_id = project
                .map(|n| crate::api::project::resolve_id(&n))
                .transpose()?;
            crate::api::task::create(&title, project_id.as_deref())
        }
        Commands::Complete { project, task_id } => {
            let project_id = crate::api::project::resolve_id(&project)?;
            crate::api::task::complete(&project_id, &task_id)
        }
        Commands::Delete { project, task_id } => {
            let project_id = crate::api::project::resolve_id(&project)?;
            crate::api::task::delete(&project_id, &task_id)
        }
    }
}
