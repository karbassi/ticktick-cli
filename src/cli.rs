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
  ticktick-cli task add 'Submit report' -d 2026-03-01  # Add with due date
  ticktick-cli task complete Personal abc123      # Complete a task
  ticktick-cli task delete Personal abc123        # Delete a task
  ticktick-cli task move inbox abc123 --to Work   # Move a task to Work project
  ticktick-cli tag list                           # List all tags
  ticktick-cli tag add urgent                     # Create a tag
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
    Task(Box<TaskCommands>),

    /// Manage tags
    #[command(subcommand)]
    Tag(TagCommands),

    /// Manage projects
    #[command(subcommand)]
    Project(ProjectCommands),

    /// Manage project folders/groups
    #[command(subcommand)]
    Folder(FolderCommands),

    /// Manage habits
    #[command(subcommand)]
    Habit(HabitCommands),

    /// Manage saved filters (smart views)
    #[command(subcommand)]
    Filter(FilterCommands),

    /// View calendar accounts and events (read-only)
    #[command(subcommand)]
    Calendar(CalendarCommands),

    /// Focus/pomodoro timer control and statistics
    #[command(subcommand)]
    Focus(FocusCommands),

    /// Show user profile and account status
    ///
    /// Displays the user's profile information merged with account status.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli profile
")]
    Profile,

    /// Show user preference settings
    ///
    /// Displays the user's preference settings (including web-specific settings).
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli settings
")]
    Settings,

    /// Dump full account state from v2 batch/check endpoint
    ///
    /// Fetches the complete account state (projects, tasks, tags, habits, etc.)
    /// from the TickTick v2 API and outputs raw JSON to stdout.
    /// Requires v2 API authentication (session token).
    ///
    /// Pipe to jq for filtering: ticktick-cli sync | jq '.inboxId'
    #[command(after_long_help = "\
Examples:
  ticktick-cli sync                     # Dump full account state
  ticktick-cli sync | jq '.inboxId'     # Extract inbox ID
  ticktick-cli sync | jq '.projectGroups'  # Extract project groups
")]
    Sync,

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
    /// Use --completed to list completed tasks instead of active ones.
    ///
    /// Each task displays:
    ///   - Task title and ID (used for complete/delete commands)
    ///   - Project name, due date, and priority level
    #[command(after_long_help = "\
Examples:
  ticktick-cli task list                    # List all tasks
  ticktick-cli task list Personal           # List tasks in Personal project
  ticktick-cli task list 'Work Projects'    # List tasks in project with spaces
  ticktick-cli task list --completed        # List completed tasks
  ticktick-cli task list Personal --completed  # Completed tasks in project
  ticktick-cli task list --completed --limit 100
")]
    List {
        /// Filter by project name or ID
        project: Option<String>,

        /// List completed tasks instead of active tasks (requires v2 auth)
        #[arg(long)]
        completed: bool,

        /// Maximum number of completed tasks to return (default: 50)
        #[arg(long, default_value = "50")]
        limit: u32,
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
  ticktick-cli task add 'Submit report' --due 2026-03-01
  ticktick-cli task add 'Call dentist' -d tomorrow
  ticktick-cli task add 'Urgent fix' --priority high
  ticktick-cli task add 'Focus block' --start 2026-02-16T14:00 --duration 2h
  ticktick-cli task add 'Meeting' --start 2026-02-16T14:00 --due 2026-02-16T15:30 --tz America/Los_Angeles
  echo -e 'Task A\\nTask B' | ticktick-cli task add --stdin -p Work
")]
    Add {
        /// Task titles (use quotes for titles with spaces)
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        titles: Vec<String>,

        /// Project name or ID to add the task to
        #[arg(short, long)]
        project: Option<String>,

        /// Due date/time: YYYY-MM-DD or YYYY-MM-DDTHH:MM, 'today', 'tomorrow'
        #[arg(short, long)]
        due: Option<String>,

        /// Priority: none, low, medium, high
        #[arg(short = 'P', long)]
        priority: Option<Priority>,

        /// Start date/time: YYYY-MM-DD or YYYY-MM-DDTHH:MM, 'today', 'tomorrow'
        #[arg(short, long)]
        start: Option<String>,

        /// Duration (e.g. 1h, 30m, 1h30m). Computes due = start + duration
        #[arg(long, conflicts_with = "due")]
        duration: Option<String>,

        /// Force all-day event even with time inputs
        #[arg(long)]
        all_day: bool,

        /// IANA timezone override (e.g. America/New_York). Controls both the UTC offset and display timezone. Defaults to local system timezone.
        #[arg(long = "timezone", visible_alias = "tz")]
        timezone: Option<String>,

        /// Task content/notes
        #[arg(long)]
        content: Option<String>,

        /// Task description
        #[arg(long)]
        desc: Option<String>,

        /// Add a tag (repeatable)
        #[arg(long = "tag", action = clap::ArgAction::Append)]
        tags: Vec<String>,

        /// Add a subtask/checklist item (repeatable)
        #[arg(long = "item", action = clap::ArgAction::Append)]
        items: Vec<String>,

        /// Add a reminder trigger (repeatable, e.g. TRIGGER:P0DT9H0M0S)
        #[arg(long = "reminder", action = clap::ArgAction::Append)]
        reminders: Vec<String>,

        /// Recurrence rule (e.g. RRULE:FREQ=DAILY;INTERVAL=1)
        #[arg(long = "repeat")]
        repeat: Option<String>,

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
  ticktick-cli task edit Personal abc123 --start 2026-02-16T14:00 --duration 1h30m
  ticktick-cli task edit Personal abc123 --clear-start
  echo -e 'id1\\nid2' | ticktick-cli task edit Personal --stdin --due tomorrow
")]
    Edit {
        /// Project name or ID containing the task
        project: String,

        /// Task IDs (find via 'ticktick-cli task list')
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Set due date/time: YYYY-MM-DD or YYYY-MM-DDTHH:MM, 'today', 'tomorrow'
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

        /// Start date/time: YYYY-MM-DD or YYYY-MM-DDTHH:MM, 'today', 'tomorrow'
        #[arg(short, long)]
        start: Option<String>,

        /// Remove the start date
        #[arg(long, conflicts_with = "start")]
        clear_start: bool,

        /// Duration (e.g. 1h, 30m, 1h30m). Computes due = start + duration
        #[arg(long, conflicts_with = "due", conflicts_with = "clear_due")]
        duration: Option<String>,

        /// Force all-day event even with time inputs
        #[arg(long)]
        all_day: bool,

        /// IANA timezone override (e.g. America/New_York). Controls both the UTC offset and display timezone. Defaults to local system timezone.
        #[arg(long = "timezone", visible_alias = "tz")]
        timezone: Option<String>,

        /// Set task content/notes
        #[arg(long)]
        content: Option<String>,

        /// Clear task content
        #[arg(long, conflicts_with = "content")]
        clear_content: bool,

        /// Set task description
        #[arg(long)]
        desc: Option<String>,

        /// Clear task description
        #[arg(long, conflicts_with = "desc")]
        clear_desc: bool,

        /// Set tags (repeatable)
        #[arg(long = "tag", action = clap::ArgAction::Append)]
        tags: Vec<String>,

        /// Remove all tags
        #[arg(long, conflicts_with = "tags")]
        clear_tags: bool,

        /// Add a subtask/checklist item (repeatable)
        #[arg(long = "item", action = clap::ArgAction::Append)]
        items: Vec<String>,

        /// Set a reminder trigger (repeatable, e.g. TRIGGER:P0DT9H0M0S)
        #[arg(long = "reminder", action = clap::ArgAction::Append)]
        reminders: Vec<String>,

        /// Remove all reminders
        #[arg(long, conflicts_with = "reminders")]
        clear_reminders: bool,

        /// Set recurrence rule (e.g. RRULE:FREQ=DAILY;INTERVAL=1)
        #[arg(long = "repeat")]
        repeat: Option<String>,

        /// Remove recurrence rule
        #[arg(long, conflicts_with = "repeat")]
        clear_repeat: bool,

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

    /// Move one or more tasks to a different project
    ///
    /// Moves tasks from one project to another by updating their project ID.
    /// The source project and destination project can be specified by name or ID.
    ///
    /// To find task IDs, run 'ticktick-cli task list <project>' first.
    #[command(visible_alias = "mv")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli task move Personal abc123 --to Work
  ticktick-cli task move inbox id1 id2 id3 --to 'Work Projects'
  ticktick-cli task mv inbox abc123 -t Work
  ticktick-cli task list inbox | jq -r '.[].id' | ticktick-cli task move inbox --stdin --to Work
")]
    Move {
        /// Source project name or ID containing the tasks
        project: String,

        /// Task IDs (find via 'ticktick-cli task list')
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Destination project name or ID
        #[arg(short = 't', long = "to")]
        to: String,

        /// Read task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// Set tasks as subtasks of a parent task
    ///
    /// Makes one or more tasks children of a parent task within the same project.
    /// This uses the v2 API and requires a session token.
    #[command(after_long_help = "\
Examples:
  ticktick-cli task subtask Personal parent123 child456
  ticktick-cli task subtask Personal parent123 child1 child2 child3
  echo -e 'child1\\nchild2' | ticktick-cli task subtask Personal parent123 --stdin
")]
    Subtask {
        /// Project name or ID containing the tasks
        project: String,

        /// Parent task ID
        parent_id: String,

        /// Child task IDs to make subtasks
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        child_ids: Vec<String>,

        /// Read child task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// Remove subtask relationships (make tasks top-level)
    ///
    /// Removes the parent relationship from one or more tasks, making them
    /// top-level tasks again. Uses the v1 API to clear the parentId field.
    #[command(after_long_help = "\
Examples:
  ticktick-cli task unparent Personal child456
  ticktick-cli task unparent Personal child1 child2 child3
  echo -e 'child1\\nchild2' | ticktick-cli task unparent Personal --stdin
")]
    Unparent {
        /// Project name or ID containing the tasks
        project: String,

        /// Task IDs to remove parent relationship from
        #[arg(num_args = 1.., required_unless_present = "stdin")]
        task_ids: Vec<String>,

        /// Read task IDs from stdin (one per line)
        #[arg(long)]
        stdin: bool,
    },

    /// List tasks in the trash
    ///
    /// Shows tasks that have been deleted but not yet permanently removed.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli task trash
")]
    Trash,
}

#[derive(Subcommand)]
enum TagCommands {
    /// List all tags
    ///
    /// Lists all tags from your TickTick account.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag list
")]
    List,

    /// Create one or more tags
    ///
    /// Creates tags in TickTick. Multiple tag names can be provided at once.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag add work
  ticktick-cli tag add urgent important later
")]
    Add {
        /// Tag names to create
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,
    },

    /// Delete one or more tags
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes tags from TickTick. Tags will be removed from
    /// all tasks that have them.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag delete --force old-tag
  ticktick-cli tag delete --force tag1 tag2 tag3
")]
    Delete {
        /// Tag names to delete
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Rename a tag
    ///
    /// Renames a tag across all tasks that use it.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag rename old-name new-name
")]
    Rename {
        /// Current tag name
        old: String,

        /// New tag name
        new: String,
    },

    /// Edit a tag's properties
    ///
    /// Update properties of an existing tag such as color, parent, or sort settings.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "update")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag edit work --color '#4A90E2'
  ticktick-cli tag edit child-tag --parent parent-tag
  ticktick-cli tag edit child-tag --clear-parent
  ticktick-cli tag edit work --sort-type dueDate
")]
    Edit {
        /// Tag name to edit
        name: String,

        /// Set tag color (hex string, e.g. '#4A90E2')
        #[arg(long)]
        color: Option<String>,

        /// Set parent tag (for nesting)
        #[arg(long, conflicts_with = "clear_parent")]
        parent: Option<String>,

        /// Remove parent tag (un-nest)
        #[arg(long)]
        clear_parent: bool,

        /// Set sort order (integer)
        #[arg(long)]
        sort_order: Option<i64>,

        /// Set sort type (e.g. project, dueDate, tag)
        #[arg(long)]
        sort_type: Option<String>,
    },

    /// Merge a tag into another tag
    ///
    /// All tasks tagged with the source tag are re-tagged with the target tag,
    /// and the source tag is deleted.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli tag merge old-tag new-tag
")]
    Merge {
        /// Source tag name (will be deleted)
        source: String,

        /// Target tag name (tasks will be re-tagged with this)
        target: String,
    },
}

#[derive(Subcommand)]
enum FolderCommands {
    /// List all project folders/groups
    ///
    /// Lists all project folders (groups) from your TickTick account.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli folder list
")]
    List,

    /// Create a new folder
    ///
    /// Creates a new project folder (group) in TickTick.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli folder add 'Work Projects'
")]
    Add {
        /// Folder name
        name: String,
    },

    /// Permanently delete one or more folders
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes project folders from TickTick.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli folder delete --force 'Old Folder'
  ticktick-cli folder delete --force folder1 folder2
")]
    Delete {
        /// Folder names or IDs to delete
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Rename a folder
    ///
    /// Renames a project folder (group).
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli folder rename 'Old Name' --name 'New Name'
")]
    Rename {
        /// Current folder name or ID
        folder: String,

        /// New folder name
        #[arg(short, long)]
        name: String,
    },
}

#[derive(Subcommand)]
enum HabitCommands {
    /// List all habits
    ///
    /// Lists all habits from your TickTick account.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit list
")]
    List,

    /// Create a new habit
    ///
    /// Creates a new habit in TickTick.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit add 'Morning Run'
  ticktick-cli habit add 'Read' --goal 30 --unit minutes
  ticktick-cli habit add 'Drink Water' --type real --goal 8 --unit cups
")]
    Add {
        /// Habit name
        name: String,

        /// Habit type: boolean or real
        #[arg(long = "type", default_value = "boolean")]
        habit_type: HabitType,

        /// Goal value (default: 1 for boolean)
        #[arg(long)]
        goal: Option<f64>,

        /// Unit label (e.g. minutes, cups, pages)
        #[arg(long)]
        unit: Option<String>,

        /// Section ID to place the habit in
        #[arg(long)]
        section: Option<String>,

        /// Repeat rule (e.g. each_day, each_week)
        #[arg(long)]
        repeat: Option<String>,

        /// Color (hex string, e.g. '#FF0000')
        #[arg(long)]
        color: Option<String>,
    },

    /// Permanently delete one or more habits
    ///
    /// WARNING: This action cannot be undone!
    ///
    /// Permanently deletes habits from TickTick.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit delete --force 'Old Habit'
  ticktick-cli habit delete --force habit1 habit2
")]
    Delete {
        /// Habit names or IDs to delete
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Edit an existing habit
    ///
    /// Update properties of an existing habit.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "update")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit edit 'Morning Run' --name 'Evening Run'
  ticktick-cli habit edit 'Read' --goal 60 --unit minutes
  ticktick-cli habit edit 'Drink Water' --color '#00FF00'
")]
    Edit {
        /// Current habit name or ID
        habit: String,

        /// New habit name
        #[arg(short, long)]
        name: Option<String>,

        /// New color (hex string)
        #[arg(long)]
        color: Option<String>,

        /// New goal value
        #[arg(long)]
        goal: Option<f64>,

        /// New unit label
        #[arg(long)]
        unit: Option<String>,

        /// New section ID
        #[arg(long)]
        section: Option<String>,

        /// New repeat rule
        #[arg(long)]
        repeat: Option<String>,
    },

    /// Record a habit check-in
    ///
    /// Records a check-in for a habit. Defaults to today with value 1.0.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit checkin 'Morning Run'
  ticktick-cli habit checkin 'Drink Water' --value 3
  ticktick-cli habit checkin 'Read' --date 2026-02-17 --value 30
  ticktick-cli habit checkin 'Meditate' --date yesterday
")]
    Checkin {
        /// Habit name or ID
        habit: String,

        /// Date: YYYY-MM-DD, 'today', or 'yesterday' (default: today)
        #[arg(short, long)]
        date: Option<String>,

        /// Check-in value (default: 1.0)
        #[arg(long, default_value = "1.0")]
        value: f64,
    },

    /// Query habit check-in history
    ///
    /// Queries check-in records for one or more habits.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit log 'Morning Run'
  ticktick-cli habit log 'Read' 'Meditate' --after 2026-01-01
")]
    Log {
        /// Habit names or IDs
        #[arg(num_args = 1.., required = true)]
        habits: Vec<String>,

        /// Only show check-ins after this date (YYYY-MM-DD, default: 30 days ago)
        #[arg(long)]
        after: Option<String>,
    },

    /// Archive one or more habits
    ///
    /// Sets the habit status to archived (status 1).
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit archive 'Old Habit'
  ticktick-cli habit archive habit1 habit2
")]
    Archive {
        /// Habit names or IDs to archive
        #[arg(num_args = 1.., required = true)]
        habits: Vec<String>,
    },

    /// Manage habit sections (grouping)
    #[command(subcommand)]
    Section(HabitSectionCommands),
}

#[derive(Subcommand)]
enum HabitSectionCommands {
    /// List all habit sections
    ///
    /// Lists all habit sections (groups) from your TickTick account.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit section list
")]
    List,

    /// Create a new habit section
    ///
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "new")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit section add 'Morning Routine'
")]
    Add {
        /// Section name
        name: String,
    },

    /// Delete one or more habit sections
    ///
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit section delete 'Old Section' --force
")]
    Delete {
        /// Section names or IDs to delete
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },

    /// Rename a habit section
    ///
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli habit section rename 'Old Name' --name 'New Name'
")]
    Rename {
        /// Section name or ID
        section: String,

        /// New section name
        #[arg(long)]
        name: String,
    },
}

/// Habit type
#[derive(Clone, Copy, ValueEnum)]
pub enum HabitType {
    /// Boolean check-in (done/not done)
    Boolean,
    /// Real-valued check-in (e.g. 30 minutes)
    Real,
}

impl HabitType {
    pub fn to_api_value(self) -> &'static str {
        match self {
            HabitType::Boolean => "Boolean",
            HabitType::Real => "Real",
        }
    }
}

#[derive(Subcommand)]
enum FilterCommands {
    /// List all saved filters
    ///
    /// Lists all saved filters (smart views) from your TickTick account.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli filter list
")]
    List,

    /// Create a new filter
    ///
    /// Creates a saved filter with a name and rule.
    /// The rule is a JSON string defining the filter conditions.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "new")]
    #[command(after_long_help = r#"
Examples:
  ticktick-cli filter add 'High Priority' --rule '{"and":[{"conditionName":"priority","or":[5],"conditionType":1}],"type":0,"version":1}'
  ticktick-cli filter add 'Overdue' --rule '{"and":[{"conditionName":"dueDate","or":["overdue"],"conditionType":1}],"type":0,"version":1}' --sort-type dueDate
"#)]
    Add {
        /// Filter name
        name: String,

        /// Filter rule (JSON string)
        #[arg(long)]
        rule: String,

        /// Sort type (e.g. dueDate, project, priority)
        #[arg(long)]
        sort_type: Option<String>,
    },

    /// Edit an existing filter
    ///
    /// Update properties of a saved filter.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "update")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli filter edit 'High Priority' --name 'Very High Priority'
  ticktick-cli filter edit myfilter --rule '{...}' --sort-type priority
")]
    Edit {
        /// Filter name or ID
        filter: String,

        /// New filter name
        #[arg(long)]
        name: Option<String>,

        /// New filter rule (JSON string)
        #[arg(long)]
        rule: Option<String>,

        /// New sort type (e.g. dueDate, project, priority)
        #[arg(long)]
        sort_type: Option<String>,
    },

    /// Delete one or more filters
    ///
    /// Permanently deletes saved filters.
    /// Requires v2 API authentication (session token).
    #[command(visible_alias = "rm")]
    #[command(after_long_help = "\
Examples:
  ticktick-cli filter delete 'Old Filter' --force
  ticktick-cli filter delete filter1 filter2 --force
")]
    Delete {
        /// Filter names or IDs to delete
        #[arg(num_args = 1.., required = true)]
        names: Vec<String>,

        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum CalendarCommands {
    /// List connected calendar accounts
    ///
    /// Lists third-party calendar accounts (Google, Outlook, etc.) connected to TickTick.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli calendar list
")]
    List,

    /// Query calendar events by date range
    ///
    /// Fetches events from all connected calendars within a date range.
    /// Defaults to 7 days ago through 7 days ahead if no range specified.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli calendar events
  ticktick-cli calendar events --from 2026-02-01 --to 2026-03-01
")]
    Events {
        /// Start date (YYYY-MM-DD, default: 7 days ago)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD, default: 7 days ahead)
        #[arg(long)]
        to: Option<String>,
    },
}

/// Focus timer mode
#[derive(Clone, Copy, ValueEnum)]
pub enum FocusMode {
    /// Pomodoro timer (default)
    Pomo,
    /// Stopwatch (count up)
    Stopwatch,
}

#[derive(Subcommand)]
enum FocusCommands {
    /// Show current timer state
    ///
    /// Returns the current focus/pomodoro timer status.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus status
")]
    Status,

    /// Show focus statistics (today/total)
    ///
    /// Returns pomodoro statistics including today's count and all-time totals.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus stats
")]
    Stats,

    /// Show focus session log
    ///
    /// Returns focus session history for a date range.
    /// Defaults to the last 30 days if no range specified.
    /// Dates are YYYY-MM-DD format, converted to epoch milliseconds internally.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus log
  ticktick-cli focus log --from 2026-02-01 --to 2026-02-18
")]
    Log {
        /// Start date (YYYY-MM-DD, default: 30 days ago)
        #[arg(long)]
        from: Option<String>,

        /// End date (YYYY-MM-DD, default: today)
        #[arg(long)]
        to: Option<String>,
    },

    /// Show full focus session timeline
    ///
    /// Returns the complete focus session timeline.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus timeline
")]
    Timeline,

    /// Start a focus session
    ///
    /// Starts a new pomodoro or stopwatch session. Optionally associate with a task.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus start
  ticktick-cli focus start --mode pomo --duration 25
  ticktick-cli focus start --mode stopwatch
  ticktick-cli focus start --task abc123
")]
    Start {
        /// Task ID to associate with the session
        #[arg(long)]
        task: Option<String>,

        /// Timer mode (pomo or stopwatch, default: pomo)
        #[arg(long, default_value = "pomo")]
        mode: FocusMode,

        /// Duration in minutes (pomodoro only, default: 25)
        #[arg(long, default_value = "25")]
        duration: u32,
    },

    /// Pause the current focus session
    ///
    /// Pauses the currently running focus timer.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus pause
")]
    Pause,

    /// Resume a paused focus session
    ///
    /// Resumes a previously paused focus timer.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus resume
")]
    Resume,

    /// Stop the current focus session
    ///
    /// Stops and saves the current focus session.
    /// Requires v2 API authentication (session token).
    #[command(after_long_help = "\
Examples:
  ticktick-cli focus stop
")]
    Stop,
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
  ticktick-cli project add Work --folder 'Work Projects'
")]
    Add {
        /// Project name
        name: String,

        /// Project color (hex string, e.g. '#FF0000')
        #[arg(long)]
        color: Option<String>,

        /// View mode: list, kanban, timeline
        #[arg(long)]
        view_mode: Option<String>,

        /// Project kind: TASK, NOTE
        #[arg(long)]
        kind: Option<String>,

        /// Assign to a folder (name or ID); use 'none' to remove from folder
        #[arg(long)]
        folder: Option<String>,
    },

    /// Edit an existing project
    ///
    /// Update properties of an existing project. Identify the project
    /// by its current name or ID.
    #[command(after_long_help = "\
Examples:
  ticktick-cli project edit Personal --name 'My Tasks'
  ticktick-cli project edit Work --name 'Work Tasks'
  ticktick-cli project edit Work --color '#FF0000'
  ticktick-cli project edit Work --view-mode kanban
  ticktick-cli project edit Work --folder 'Work Projects'
  ticktick-cli project edit Work --folder none
")]
    Edit {
        /// Current project name or ID
        project: String,

        /// New project name
        #[arg(short, long)]
        name: Option<String>,

        /// Project color (hex string, e.g. '#FF0000')
        #[arg(long)]
        color: Option<String>,

        /// View mode: list, kanban, timeline
        #[arg(long)]
        view_mode: Option<String>,

        /// Project kind: TASK, NOTE
        #[arg(long)]
        kind: Option<String>,

        /// Assign to a folder (name or ID); use 'none' to remove from folder
        #[arg(long)]
        folder: Option<String>,
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

/// Prompt for destructive action confirmation.
fn confirm_destructive(count: usize, noun: &str) -> Result<(), String> {
    use std::io::IsTerminal;

    let is_ci = std::env::var("CI").ok().as_deref() == Some("true");

    if is_ci || !std::io::stdin().is_terminal() {
        return Err(
            "refusing to delete without confirmation in non-interactive mode\n\n  hint: Use --force to skip confirmation"
                .to_string(),
        );
    }

    eprint!("Are you sure you want to delete {count} {noun}(s)? [y/N] ");
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

/// Convert days since Unix epoch to (year, month, day).
fn days_to_ymd(days: u64) -> (i32, u32, u32) {
    // Civil days algorithm from Howard Hinnant
    let z = days as i64 + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = (z - era * 146097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    (y as i32, m as u32, d as u32)
}

/// Parse a YYYY-MM-DD date string to epoch milliseconds (UTC midnight).
fn date_to_epoch_ms(date: &str) -> Result<i64, String> {
    let parts: Vec<&str> = date.split('-').collect();
    if parts.len() != 3 {
        return Err(format!("invalid date format '{date}', expected YYYY-MM-DD"));
    }
    let y: i64 = parts[0]
        .parse()
        .map_err(|_| format!("invalid year in '{date}'"))?;
    let m: i64 = parts[1]
        .parse()
        .map_err(|_| format!("invalid month in '{date}'"))?;
    let d: i64 = parts[2]
        .parse()
        .map_err(|_| format!("invalid day in '{date}'"))?;

    // Inverse of the civil days algorithm — compute days since epoch
    let (y, m) = if m <= 2 { (y - 1, m + 9) } else { (y, m - 3) };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = (y - era * 400) as u64;
    let doy = (153 * (m as u64) + 2) / 5 + (d as u64) - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146097 + doe as i64 - 719468;

    Ok(days * 86400 * 1000)
}

type TimeblockResult = (
    Option<crate::api::task::DateField>,
    Option<crate::api::task::DateField>,
    Option<bool>,
    Option<String>,
);

/// If `--tz` was given, use that. Otherwise, compare the local system timezone
/// with the stored TickTick account timezone.  When they differ and stdin is a
/// terminal, prompt the user to choose.
///
/// Returns `(tz_for_offset, tz_field)`:
/// - `tz_for_offset`: IANA name to compute the UTC offset (None = local)
/// - `tz_field`: value to send as the `timeZone` JSON field
fn resolve_timezone(
    explicit_tz: Option<String>,
    has_datetime: bool,
) -> (Option<String>, Option<String>) {
    use std::io::IsTerminal;

    // --tz always wins
    if let Some(tz) = explicit_tz {
        return (Some(tz.clone()), Some(tz));
    }

    // No datetime flags → nothing to resolve
    if !has_datetime {
        return (None, None);
    }

    let local = crate::api::task::local_iana_timezone();
    let account = crate::config::get_account_timezone();

    // If we don't know the account timezone yet, or they match, use local
    let (Some(local_tz), Some(account_tz)) = (&local, &account) else {
        return (None, None);
    };

    if local_tz == account_tz {
        return (None, None);
    }

    // They differ — prompt if interactive, otherwise default to local
    if !std::io::stdin().is_terminal() {
        return (None, local);
    }

    eprint!(
        "Local timezone ({local_tz}) differs from TickTick account ({account_tz}).\n\
         Use which timezone? [l]ocal / [a]ccount (default: local): "
    );

    let mut input = String::new();
    if std::io::stdin().read_line(&mut input).is_ok()
        && matches!(input.trim().to_lowercase().as_str(), "a" | "account")
    {
        (Some(account_tz.clone()), Some(account_tz.clone()))
    } else {
        (None, local)
    }
}

/// Resolve timeblocking flags into TaskFields components.
/// Returns (due_date, start_date, is_all_day, time_zone).
fn resolve_timeblock(
    due: Option<&str>,
    start: Option<&str>,
    duration: Option<&str>,
    all_day: bool,
    timezone: Option<String>,
    tz_for_offset: Option<&str>,
) -> Result<TimeblockResult, String> {
    use crate::api::task::{DateField, parse_datetime, parse_duration};

    // --duration requires --start
    if duration.is_some() && start.is_none() {
        return Err("--duration requires --start".to_string());
    }

    let parsed_start = start.map(parse_datetime).transpose()?;
    let parsed_due = due.map(parse_datetime).transpose()?;

    // --duration with date-only --start is an error
    if duration.is_some()
        && let Some(ref ps) = parsed_start
        && ps.is_all_day()
    {
        return Err("--duration requires --start with a time (YYYY-MM-DDTHH:MM)".to_string());
    }

    // Compute due from start + duration if --duration given
    let (resolved_due, resolved_start) = if let Some(dur_str) = duration {
        let dur = parse_duration(dur_str)?;
        let ps = parsed_start.as_ref().unwrap(); // guaranteed by check above
        let end = ps.add_duration(&dur)?;
        (
            Some(DateField::Set(end.to_api_string(tz_for_offset))),
            Some(DateField::Set(ps.to_api_string(tz_for_offset))),
        )
    } else {
        (
            parsed_due
                .as_ref()
                .map(|d| DateField::Set(d.to_api_string(tz_for_offset))),
            parsed_start
                .as_ref()
                .map(|d| DateField::Set(d.to_api_string(tz_for_offset))),
        )
    };

    // Derive isAllDay
    let is_all_day = if all_day {
        Some(true)
    } else if parsed_start.is_some() || parsed_due.is_some() || duration.is_some() {
        let has_time = parsed_start.as_ref().is_some_and(|d| !d.is_all_day())
            || parsed_due.as_ref().is_some_and(|d| !d.is_all_day())
            || duration.is_some();
        Some(!has_time)
    } else {
        None
    };

    Ok((resolved_due, resolved_start, is_all_day, timezone))
}

/// Resolve the --folder flag value to a group_id option.
/// "none" (case-insensitive) → clear, otherwise resolve to folder ID.
fn resolve_folder_flag(folder: Option<String>) -> Result<Option<Option<String>>, String> {
    match folder {
        None => Ok(None),
        Some(f) if f.eq_ignore_ascii_case("none") => Ok(Some(None)),
        Some(f) => {
            let (id, _etag) = crate::api::project_group::resolve_id(&f)?;
            Ok(Some(Some(id)))
        }
    }
}

/// After a task create/update, detect the inbox project ID from the response
/// and store it in config if not already known.
fn detect_inbox_id(task: &crate::api::task::Task) {
    if crate::config::get_inbox_project_id().is_some() {
        return;
    }
    if let Some(ref pid) = task.project_id
        && pid.starts_with("inbox")
    {
        let _ = crate::config::save_inbox_project_id(pid);
    }
}

/// After a task create/update, detect the TickTick account timezone from the
/// response and store it in config if not already known.
fn detect_account_timezone(task: &crate::api::task::Task) {
    if crate::config::get_account_timezone().is_some() {
        return;
    }
    if let Some(ref tz) = task.time_zone {
        let _ = crate::config::save_account_timezone(tz);
    }
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
        Commands::Task(subcmd) => match *subcmd {
            TaskCommands::List {
                project,
                completed,
                limit,
            } => {
                if completed {
                    let project_id = project
                        .map(|s| crate::api::project::resolve_id(&s))
                        .transpose()?;
                    if let Some(pid) = project_id {
                        let tasks = crate::api::v2::list_completed_by_project(&pid)?;
                        crate::output::success(&tasks);
                    } else {
                        let tasks = crate::api::v2::list_completed_in_all(limit)?;
                        crate::output::success(&tasks);
                    }
                } else {
                    let token = crate::config::get_access_token()?;
                    let project_id = project
                        .map(|s| crate::api::project::resolve_id(&s))
                        .transpose()?;
                    let tasks = crate::api::task::list_by_project(&token, project_id.as_deref())?;
                    crate::output::success(&tasks);
                }
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
                start,
                duration,
                all_day,
                timezone,
                content,
                desc,
                tags,
                items,
                reminders,
                repeat,
                dry_run,
                stdin,
            } => {
                let inputs = collect_inputs(titles, stdin)?;
                let project_id = project
                    .map(|n| crate::api::project::resolve_id(&n))
                    .transpose()?;
                let priority = priority.map(|p| p.to_api_value());

                let has_datetime = due.is_some() || start.is_some();
                let (tz_for_offset, tz_field) = resolve_timezone(timezone, has_datetime);
                let (due_date, start_date, is_all_day, time_zone) = resolve_timeblock(
                    due.as_deref(),
                    start.as_deref(),
                    duration.as_deref(),
                    all_day,
                    tz_field,
                    tz_for_offset.as_deref(),
                )?;

                let content = content.map(Some);
                let desc = desc.map(Some);
                let tags_field = if tags.is_empty() { None } else { Some(tags) };
                let items_field = if items.is_empty() {
                    None
                } else {
                    Some(
                        items
                            .into_iter()
                            .map(|t| crate::api::task::ChecklistItem {
                                title: t,
                                status: 0,
                                ..Default::default()
                            })
                            .collect(),
                    )
                };
                let reminders_field = if reminders.is_empty() {
                    None
                } else {
                    Some(reminders)
                };
                let repeat_flag = repeat.map(Some);

                if dry_run {
                    let previews: Vec<serde_json::Value> = inputs
                        .iter()
                        .map(|title| {
                            let fields = crate::api::task::TaskFields {
                                title: Some(title.clone()),
                                project_id: project_id.clone(),
                                due_date: due_date.clone(),
                                start_date: start_date.clone(),
                                priority,
                                is_all_day,
                                time_zone: time_zone.clone(),
                                content: content.clone(),
                                desc: desc.clone(),
                                tags: tags_field.clone(),
                                items: items_field.clone(),
                                reminders: reminders_field.clone(),
                                repeat_flag: repeat_flag.clone(),
                                parent_id: None,
                            };
                            let mut body = serde_json::json!({ "dryRun": true });
                            fields.apply_to(&mut body);
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
                        let fields = crate::api::task::TaskFields {
                            title: Some(title.clone()),
                            project_id: project_id.clone(),
                            due_date: due_date.clone(),
                            start_date: start_date.clone(),
                            priority,
                            is_all_day,
                            time_zone: time_zone.clone(),
                            content: content.clone(),
                            desc: desc.clone(),
                            tags: tags_field.clone(),
                            items: items_field.clone(),
                            reminders: reminders_field.clone(),
                            repeat_flag: repeat_flag.clone(),
                            parent_id: None,
                        };
                        match crate::api::task::create(&token, &fields) {
                            Ok(task) => {
                                detect_account_timezone(&task);
                                detect_inbox_id(&task);
                                BulkResult {
                                    id: title.clone(),
                                    status: "ok".into(),
                                    data: Some(serde_json::to_value(&task).unwrap_or_default()),
                                    error: None,
                                }
                            }
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
                start,
                clear_start,
                duration,
                all_day,
                timezone,
                content,
                clear_content,
                desc,
                clear_desc,
                tags,
                clear_tags,
                items,
                reminders,
                clear_reminders,
                repeat,
                clear_repeat,
                stdin,
            } => {
                let inputs = collect_inputs(task_ids, stdin)?;

                if title.is_some() && inputs.len() > 1 {
                    return Err("--title can only be used with a single task ID".to_string());
                }

                let token = crate::config::get_access_token()?;
                let project_id = crate::api::project::resolve_id(&project)?;
                let priority = priority.map(|p| p.to_api_value());

                let has_datetime = due.is_some() || start.is_some();
                let (tz_for_offset, tz_field) = resolve_timezone(timezone, has_datetime);
                let (mut resolved_due, mut resolved_start, is_all_day, time_zone) =
                    resolve_timeblock(
                        due.as_deref(),
                        start.as_deref(),
                        duration.as_deref(),
                        all_day,
                        tz_field,
                        tz_for_offset.as_deref(),
                    )?;

                if clear_due {
                    resolved_due = Some(crate::api::task::DateField::Clear);
                }
                if clear_start {
                    resolved_start = Some(crate::api::task::DateField::Clear);
                }

                let content = if clear_content {
                    Some(None)
                } else {
                    content.map(Some)
                };
                let desc = if clear_desc {
                    Some(None)
                } else {
                    desc.map(Some)
                };
                let tags_field = if clear_tags {
                    Some(vec![])
                } else if tags.is_empty() {
                    None
                } else {
                    Some(tags)
                };
                let items_field = if items.is_empty() {
                    None
                } else {
                    Some(
                        items
                            .into_iter()
                            .map(|t| crate::api::task::ChecklistItem {
                                title: t,
                                status: 0,
                                ..Default::default()
                            })
                            .collect(),
                    )
                };
                let reminders_field = if clear_reminders {
                    Some(vec![])
                } else if reminders.is_empty() {
                    None
                } else {
                    Some(reminders)
                };
                let repeat_flag = if clear_repeat {
                    Some(None)
                } else {
                    repeat.map(Some)
                };

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|task_id| {
                        let fields = crate::api::task::TaskFields {
                            title: title.clone(),
                            project_id: None,
                            due_date: resolved_due.clone(),
                            start_date: resolved_start.clone(),
                            priority,
                            is_all_day,
                            time_zone: time_zone.clone(),
                            content: content.clone(),
                            desc: desc.clone(),
                            tags: tags_field.clone(),
                            items: items_field.clone(),
                            reminders: reminders_field.clone(),
                            repeat_flag: repeat_flag.clone(),
                            parent_id: None,
                        };
                        match crate::api::task::update(&token, &project_id, task_id, &fields) {
                            Ok(task) => {
                                detect_account_timezone(&task);
                                detect_inbox_id(&task);
                                BulkResult {
                                    id: task_id.clone(),
                                    status: "ok".into(),
                                    data: Some(serde_json::to_value(&task).unwrap_or_default()),
                                    error: None,
                                }
                            }
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
                    .map(
                        |task_id| match crate::api::task::complete(&token, &project_id, task_id) {
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
                        },
                    )
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
                    confirm_destructive(inputs.len(), "task")?;
                }

                let token = crate::config::get_access_token()?;
                let project_id = crate::api::project::resolve_id(&project)?;

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(
                        |task_id| match crate::api::task::delete(&token, &project_id, task_id) {
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
                        },
                    )
                    .collect();

                output_results(&results)
            }
            TaskCommands::Move {
                project,
                task_ids,
                to,
                stdin,
            } => {
                let inputs = collect_inputs(task_ids, stdin)?;
                let token = crate::config::get_access_token()?;
                let source_project_id = crate::api::project::resolve_id(&project)?;
                let dest_project_id = crate::api::project::resolve_id(&to)?;

                // Build batch move request
                let moves: Vec<crate::api::v2::TaskMove> = inputs
                    .iter()
                    .map(|task_id| crate::api::v2::TaskMove {
                        task_id: task_id.clone(),
                        from_project_id: source_project_id.clone(),
                        to_project_id: dest_project_id.clone(),
                    })
                    .collect();

                // Single batch v2 API call for all moves
                crate::api::v2::move_tasks(&moves)?;

                // Fetch each task individually for output
                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|task_id| {
                        match crate::api::task::get_by_id(&token, &dest_project_id, task_id) {
                            Ok(task) => {
                                detect_account_timezone(&task);
                                detect_inbox_id(&task);
                                BulkResult {
                                    id: task_id.clone(),
                                    status: "ok".into(),
                                    data: Some(serde_json::to_value(&task).unwrap_or_default()),
                                    error: None,
                                }
                            }
                            Err(_) => {
                                // v1 API may lag; return minimal confirmation
                                BulkResult {
                                    id: task_id.clone(),
                                    status: "ok".into(),
                                    data: Some(serde_json::json!({
                                        "id": task_id,
                                        "projectId": dest_project_id,
                                    })),
                                    error: None,
                                }
                            }
                        }
                    })
                    .collect();

                output_results(&results)
            }
            TaskCommands::Subtask {
                project,
                parent_id,
                child_ids,
                stdin,
            } => {
                let inputs = collect_inputs(child_ids, stdin)?;
                let project_id = crate::api::project::resolve_id(&project)?;

                let parents: Vec<crate::api::v2::TaskParent> = inputs
                    .iter()
                    .map(|child_id| crate::api::v2::TaskParent {
                        parent_id: parent_id.clone(),
                        project_id: project_id.clone(),
                        task_id: child_id.clone(),
                    })
                    .collect();

                crate::api::v2::set_task_parents(&parents)?;

                let results: Vec<BulkResult> = inputs
                    .iter()
                    .map(|child_id| BulkResult {
                        id: child_id.clone(),
                        status: "ok".into(),
                        data: Some(serde_json::json!({
                            "taskId": child_id,
                            "parentId": parent_id,
                            "projectId": project_id,
                        })),
                        error: None,
                    })
                    .collect();

                output_results(&results)
            }
            TaskCommands::Unparent {
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
                        let fields = crate::api::task::TaskFields {
                            parent_id: Some(None), // Clear parent
                            ..Default::default()
                        };
                        match crate::api::task::update(&token, &project_id, task_id, &fields) {
                            Ok(task) => BulkResult {
                                id: task_id.clone(),
                                status: "ok".into(),
                                data: Some(serde_json::to_value(&task).unwrap_or_default()),
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
            TaskCommands::Trash => {
                let tasks = crate::api::v2::list_trash()?;
                crate::output::success(&tasks);
                Ok(())
            }
        },
        Commands::Tag(subcmd) => match subcmd {
            TagCommands::List => {
                let tags = crate::api::tag::list()?;
                crate::output::success(&tags);
                Ok(())
            }
            TagCommands::Add { names } => {
                let tags = crate::api::tag::create(&names)?;
                crate::output::success(&tags);
                Ok(())
            }
            TagCommands::Delete { names, force } => {
                if !force {
                    confirm_destructive(names.len(), "tag")?;
                }

                let results: Vec<BulkResult> = names
                    .iter()
                    .map(|name| match crate::api::tag::delete(name) {
                        Ok(()) => BulkResult {
                            id: name.clone(),
                            status: "ok".into(),
                            data: None,
                            error: None,
                        },
                        Err(e) => BulkResult {
                            id: name.clone(),
                            status: "error".into(),
                            data: None,
                            error: Some(e),
                        },
                    })
                    .collect();

                output_results(&results)
            }
            TagCommands::Edit {
                name,
                color,
                parent,
                clear_parent,
                sort_order,
                sort_type,
            } => {
                let parent_field = if clear_parent {
                    Some(None)
                } else {
                    parent.map(Some)
                };
                let fields = crate::api::tag::TagFields {
                    color,
                    parent: parent_field,
                    sort_order,
                    sort_type,
                };
                let result = crate::api::tag::update(&name, &fields)?;
                crate::output::success(&result);
                Ok(())
            }
            TagCommands::Rename { old, new } => {
                crate::api::tag::rename(&old, &new)?;
                crate::output::success(&serde_json::json!({
                    "status": "ok",
                    "oldName": old,
                    "newName": new,
                }));
                Ok(())
            }
            TagCommands::Merge { source, target } => {
                crate::api::tag::merge(&source, &target)?;
                crate::output::success(&serde_json::json!({
                    "status": "ok",
                    "source": source,
                    "target": target,
                }));
                Ok(())
            }
        },
        Commands::Folder(subcmd) => match subcmd {
            FolderCommands::List => {
                let groups = crate::api::project_group::list()?;
                crate::output::success(&groups);
                Ok(())
            }
            FolderCommands::Add { name } => {
                let result = crate::api::project_group::create(&name)?;
                crate::output::success(&result);
                Ok(())
            }
            FolderCommands::Delete { names, force } => {
                if !force {
                    confirm_destructive(names.len(), "folder")?;
                }

                let results: Vec<BulkResult> = names
                    .iter()
                    .map(|name| match crate::api::project_group::resolve_id(name) {
                        Ok((id, _etag)) => match crate::api::project_group::delete(&[id]) {
                            Ok(()) => BulkResult {
                                id: name.clone(),
                                status: "ok".into(),
                                data: None,
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: name.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        },
                        Err(e) => BulkResult {
                            id: name.clone(),
                            status: "error".into(),
                            data: None,
                            error: Some(e),
                        },
                    })
                    .collect();

                output_results(&results)
            }
            FolderCommands::Rename { folder, name } => {
                let (id, etag) = crate::api::project_group::resolve_id(&folder)?;
                let result = crate::api::project_group::rename(&id, &etag, &name)?;
                crate::output::success(&result);
                Ok(())
            }
        },
        Commands::Habit(subcmd) => match subcmd {
            HabitCommands::List => {
                let habits = crate::api::habit::list()?;
                crate::output::success(&habits);
                Ok(())
            }
            HabitCommands::Add {
                name,
                habit_type,
                goal,
                unit,
                section,
                repeat,
                color,
            } => {
                let fields = crate::api::habit::HabitFields {
                    name: Some(name),
                    habit_type: Some(habit_type.to_api_value().to_string()),
                    goal,
                    unit,
                    section_id: section,
                    repeat_rule: repeat,
                    color,
                    status: None,
                };
                let result = crate::api::habit::create(&fields)?;
                crate::output::success(&result);
                Ok(())
            }
            HabitCommands::Delete { names, force } => {
                if !force {
                    confirm_destructive(names.len(), "habit")?;
                }

                let results: Vec<BulkResult> = names
                    .iter()
                    .map(|name| match crate::api::habit::resolve_id(name) {
                        Ok((id, _etag)) => match crate::api::habit::delete(&[id]) {
                            Ok(()) => BulkResult {
                                id: name.clone(),
                                status: "ok".into(),
                                data: None,
                                error: None,
                            },
                            Err(e) => BulkResult {
                                id: name.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        },
                        Err(e) => BulkResult {
                            id: name.clone(),
                            status: "error".into(),
                            data: None,
                            error: Some(e),
                        },
                    })
                    .collect();

                output_results(&results)
            }
            HabitCommands::Edit {
                habit,
                name,
                color,
                goal,
                unit,
                section,
                repeat,
            } => {
                let (id, etag) = crate::api::habit::resolve_id(&habit)?;
                let fields = crate::api::habit::HabitFields {
                    name,
                    habit_type: None,
                    goal,
                    unit,
                    section_id: section,
                    repeat_rule: repeat,
                    color,
                    status: None,
                };
                let result = crate::api::habit::update(&id, &etag, &fields)?;
                crate::output::success(&result);
                Ok(())
            }
            HabitCommands::Checkin { habit, date, value } => {
                let (habit_id, _etag) = crate::api::habit::resolve_id(&habit)?;
                let stamp = match date {
                    Some(d) => crate::api::habit::date_to_stamp(&d)?,
                    None => crate::api::habit::date_to_stamp("today")?,
                };
                let result = crate::api::habit::checkin(&habit_id, stamp, value)?;
                crate::output::success(&result);
                Ok(())
            }
            HabitCommands::Log { habits, after } => {
                let habit_ids: Vec<String> = habits
                    .iter()
                    .map(|h| crate::api::habit::resolve_id(h).map(|(id, _)| id))
                    .collect::<Result<Vec<_>, _>>()?;

                let after_stamp = match after {
                    Some(d) => crate::api::habit::date_to_stamp(&d)?,
                    None => {
                        // Default to 30 days ago
                        use std::time::{SystemTime, UNIX_EPOCH};
                        let now_secs = SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        let days_ago = now_secs / 86400 - 30;
                        let (y, m, d) = days_to_ymd(days_ago);
                        y as i64 * 10000 + m as i64 * 100 + d as i64
                    }
                };

                let result = crate::api::habit::query_checkins(&habit_ids, after_stamp)?;
                crate::output::success(&result);
                Ok(())
            }
            HabitCommands::Archive { habits } => {
                let results: Vec<BulkResult> = habits
                    .iter()
                    .map(|h| match crate::api::habit::resolve_id(h) {
                        Ok((id, etag)) => {
                            let fields = crate::api::habit::HabitFields {
                                status: Some(1),
                                ..Default::default()
                            };
                            match crate::api::habit::update(&id, &etag, &fields) {
                                Ok(result) => BulkResult {
                                    id: h.clone(),
                                    status: "ok".into(),
                                    data: Some(result),
                                    error: None,
                                },
                                Err(e) => BulkResult {
                                    id: h.clone(),
                                    status: "error".into(),
                                    data: None,
                                    error: Some(e),
                                },
                            }
                        }
                        Err(e) => BulkResult {
                            id: h.clone(),
                            status: "error".into(),
                            data: None,
                            error: Some(e),
                        },
                    })
                    .collect();

                output_results(&results)
            }
            HabitCommands::Section(subcmd) => match subcmd {
                HabitSectionCommands::List => {
                    let sections = crate::api::habit::list_sections()?;
                    crate::output::success(&sections);
                    Ok(())
                }
                HabitSectionCommands::Add { name } => {
                    let result = crate::api::habit::create_section(&name)?;
                    crate::output::success(&result);
                    Ok(())
                }
                HabitSectionCommands::Delete { names, force } => {
                    if !force {
                        confirm_destructive(names.len(), "habit section")?;
                    }

                    let results: Vec<BulkResult> = names
                        .iter()
                        .map(|name| match crate::api::habit::resolve_section_id(name) {
                            Ok((id, _name)) => {
                                match crate::api::habit::delete_sections(std::slice::from_ref(&id))
                                {
                                    Ok(()) => BulkResult {
                                        id: name.clone(),
                                        status: "ok".into(),
                                        data: None,
                                        error: None,
                                    },
                                    Err(e) => BulkResult {
                                        id: name.clone(),
                                        status: "error".into(),
                                        data: None,
                                        error: Some(e),
                                    },
                                }
                            }
                            Err(e) => BulkResult {
                                id: name.clone(),
                                status: "error".into(),
                                data: None,
                                error: Some(e),
                            },
                        })
                        .collect();

                    output_results(&results)
                }
                HabitSectionCommands::Rename { section, name } => {
                    let (id, _old_name) = crate::api::habit::resolve_section_id(&section)?;
                    let result = crate::api::habit::rename_section(&id, &name)?;
                    crate::output::success(&result);
                    Ok(())
                }
            },
        },
        Commands::Filter(subcmd) => match subcmd {
            FilterCommands::List => {
                let filters = crate::api::filter::list()?;
                crate::output::success(&filters);
                Ok(())
            }
            FilterCommands::Add {
                name,
                rule,
                sort_type,
            } => {
                let fields = crate::api::filter::FilterFields {
                    name: Some(name),
                    rule: Some(rule),
                    sort_type,
                };
                let result = crate::api::filter::create(&fields)?;
                crate::output::success(&result);
                Ok(())
            }
            FilterCommands::Edit {
                filter,
                name,
                rule,
                sort_type,
            } => {
                let (id, etag) = crate::api::filter::resolve_id(&filter)?;
                let fields = crate::api::filter::FilterFields {
                    name,
                    rule,
                    sort_type,
                };
                let result = crate::api::filter::update(&id, &etag, &fields)?;
                crate::output::success(&result);
                Ok(())
            }
            FilterCommands::Delete { names, force } => {
                if !force {
                    confirm_destructive(names.len(), "filter")?;
                }

                let results: Vec<BulkResult> = names
                    .iter()
                    .map(|name| match crate::api::filter::resolve_id(name) {
                        Ok((id, _etag)) => {
                            match crate::api::filter::delete(std::slice::from_ref(&id)) {
                                Ok(()) => BulkResult {
                                    id: name.clone(),
                                    status: "ok".into(),
                                    data: None,
                                    error: None,
                                },
                                Err(e) => BulkResult {
                                    id: name.clone(),
                                    status: "error".into(),
                                    data: None,
                                    error: Some(e),
                                },
                            }
                        }
                        Err(e) => BulkResult {
                            id: name.clone(),
                            status: "error".into(),
                            data: None,
                            error: Some(e),
                        },
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
            ProjectCommands::Add {
                name,
                color,
                view_mode,
                kind,
                folder,
            } => {
                let group_id = resolve_folder_flag(folder)?;
                let fields = crate::api::project::ProjectFields {
                    name: Some(name),
                    color,
                    view_mode,
                    kind,
                    group_id,
                };
                crate::api::project::create(&fields)
            }
            ProjectCommands::Edit {
                project,
                name,
                color,
                view_mode,
                kind,
                folder,
            } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                let group_id = resolve_folder_flag(folder)?;
                let fields = crate::api::project::ProjectFields {
                    name,
                    color,
                    view_mode,
                    kind,
                    group_id,
                };
                crate::api::project::update(&project_id, &fields)
            }
            ProjectCommands::Delete { project, force } => {
                let project_id = crate::api::project::resolve_id(&project)?;
                crate::api::project::delete(&project_id, force)
            }
        },
        Commands::Calendar(subcmd) => match subcmd {
            CalendarCommands::List => {
                let accounts = crate::api::calendar::list_accounts()?;
                crate::output::success(&accounts);
                Ok(())
            }
            CalendarCommands::Events { from, to } => {
                use std::time::{SystemTime, UNIX_EPOCH};

                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let begin = if let Some(f) = from {
                    format!("{f}T00:00:00.000+0000")
                } else {
                    let days = now_secs / 86400 - 7;
                    let (y, m, d) = days_to_ymd(days);
                    format!("{y:04}-{m:02}-{d:02}T00:00:00.000+0000")
                };

                let end = if let Some(t) = to {
                    format!("{t}T00:00:00.000+0000")
                } else {
                    let days = now_secs / 86400 + 7;
                    let (y, m, d) = days_to_ymd(days);
                    format!("{y:04}-{m:02}-{d:02}T00:00:00.000+0000")
                };

                let events = crate::api::calendar::query_events(&begin, &end)?;
                crate::output::success(&events);
                Ok(())
            }
        },
        Commands::Focus(subcmd) => match subcmd {
            FocusCommands::Status => {
                let status = crate::api::focus::get_timer_status()?;
                crate::output::success(&status);
                Ok(())
            }
            FocusCommands::Stats => {
                let stats = crate::api::focus::get_stats()?;
                crate::output::success(&stats);
                Ok(())
            }
            FocusCommands::Log { from, to } => {
                use std::time::{SystemTime, UNIX_EPOCH};

                let now_secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap()
                    .as_secs();

                let from_ms = if let Some(f) = from {
                    date_to_epoch_ms(&f)?
                } else {
                    // 30 days ago
                    ((now_secs - 30 * 86400) as i64) * 1000
                };

                let to_ms = if let Some(t) = to {
                    date_to_epoch_ms(&t)? + 86400 * 1000 // end of day
                } else {
                    (now_secs as i64) * 1000
                };

                let log = crate::api::focus::get_log(from_ms, to_ms)?;
                crate::output::success(&log);
                Ok(())
            }
            FocusCommands::Timeline => {
                let timeline = crate::api::focus::get_timeline()?;
                crate::output::success(&timeline);
                Ok(())
            }
            FocusCommands::Start {
                task,
                mode,
                duration,
            } => {
                let session_id = crate::api::focus::generate_id();
                let now = crate::api::focus::now_iso();

                let (focus_type, duration_ms) = match mode {
                    FocusMode::Pomo => (0, (duration as i64) * 60 * 1000),
                    FocusMode::Stopwatch => (1, 0),
                };

                let mut op = serde_json::json!({
                    "op": "start",
                    "sid": session_id,
                    "type": focus_type,
                    "startTime": now,
                });

                if focus_type == 0 {
                    op["duration"] = serde_json::json!(duration_ms);
                }

                if let Some(ref tid) = task {
                    op["taskId"] = serde_json::json!(tid);
                }

                let result = crate::api::focus::focus_op(&[op])?;
                crate::output::success(&result);
                Ok(())
            }
            FocusCommands::Pause => {
                let status = crate::api::focus::get_timer_status()?;
                let sid = status["sid"]
                    .as_str()
                    .ok_or("no active focus session found")?;

                let now = crate::api::focus::now_iso();
                let op = serde_json::json!({
                    "op": "pause",
                    "sid": sid,
                    "pauseTime": now,
                });

                let result = crate::api::focus::focus_op(&[op])?;
                crate::output::success(&result);
                Ok(())
            }
            FocusCommands::Resume => {
                let status = crate::api::focus::get_timer_status()?;
                let sid = status["sid"]
                    .as_str()
                    .ok_or("no active focus session found")?;

                let now = crate::api::focus::now_iso();
                let op = serde_json::json!({
                    "op": "resume",
                    "sid": sid,
                    "resumeTime": now,
                });

                let result = crate::api::focus::focus_op(&[op])?;
                crate::output::success(&result);
                Ok(())
            }
            FocusCommands::Stop => {
                let status = crate::api::focus::get_timer_status()?;
                let sid = status["sid"]
                    .as_str()
                    .ok_or("no active focus session found")?;

                let now = crate::api::focus::now_iso();
                let ops = vec![
                    serde_json::json!({
                        "op": "drop",
                        "sid": sid,
                        "endTime": now,
                    }),
                    serde_json::json!({
                        "op": "exit",
                        "sid": sid,
                    }),
                ];

                let result = crate::api::focus::focus_op(&ops)?;
                crate::output::success(&result);
                Ok(())
            }
        },
        Commands::Profile => {
            let mut profile = crate::api::v2::get_profile()?;
            let status = crate::api::v2::get_status()?;
            // Merge status into profile object
            if let (Some(profile_obj), Some(status_obj)) =
                (profile.as_object_mut(), status.as_object())
            {
                for (k, v) in status_obj {
                    profile_obj.entry(k.clone()).or_insert(v.clone());
                }
            }
            crate::output::success(&profile);
            Ok(())
        }
        Commands::Settings => {
            let settings = crate::api::v2::get_settings()?;
            crate::output::success(&settings);
            Ok(())
        }
        Commands::Sync => {
            let data = crate::api::v2::batch_check()?;
            crate::output::success(&data);
            Ok(())
        }
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
    print_subcommands("", &cmd);
}

fn print_subcommands(prefix: &str, cmd: &clap::Command) {
    for sub in cmd.get_subcommands() {
        let name = sub.get_name();
        if name == "usage" || name == "help" {
            continue;
        }

        let full_name = if prefix.is_empty() {
            name.to_string()
        } else {
            format!("{prefix} {name}")
        };

        let nested: Vec<_> = sub.get_subcommands().collect();
        if nested.is_empty() {
            print_command(&full_name, sub);
        } else {
            print_subcommands(&full_name, sub);
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
