use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard, OnceLock};

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_timezone: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inbox_project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_session_token: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub v2_device_id: Option<String>,
    /// Inbox IDs of env access tokens, keyed by [`fingerprint`].
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub env_inbox_ids: BTreeMap<String, String>,
}

/// FNV-1a, so the key is stable across builds and the token isn't stored.
fn fingerprint(token: &str) -> String {
    let hash = token.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |h, b| {
        (h ^ u64::from(b)).wrapping_mul(0x0100_0000_01b3)
    });
    format!("{hash:016x}")
}

#[derive(Debug)]
pub struct Env {
    pub client_id: String,
    pub client_secret: String,
}

fn config_dir() -> PathBuf {
    std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| dirs_home().join(".config"))
        .join("ticktick-cli")
}

fn find_env_file() -> Option<String> {
    let candidates = [
        // Current directory
        std::env::current_dir().ok().map(|d| d.join(".env")),
        // XDG config directory
        Some(config_dir().join(".env")),
    ];

    for candidate in candidates.into_iter().flatten() {
        if let Ok(content) = fs::read_to_string(&candidate) {
            return Some(content);
        }
    }
    None
}

fn config_path() -> PathBuf {
    config_dir().join("config.json")
}

fn dirs_home() -> PathBuf {
    std::env::var("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("."))
}

/// Blank values are skipped; the first of a repeated key wins.
fn parse_dotenv(content: &str) -> HashMap<String, String> {
    let mut vars = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').trim_matches('\'').trim();
            if !value.is_empty() {
                vars.entry(key.trim().to_string())
                    .or_insert_with(|| value.to_string());
            }
        }
    }
    vars
}

/// Each key on its own: the process env wins, else the .env file.
struct Settings(HashMap<String, String>);

impl Settings {
    fn load() -> Self {
        Self(
            find_env_file()
                .map(|c| parse_dotenv(&c))
                .unwrap_or_default(),
        )
    }

    fn get(&self, key: &str) -> Option<String> {
        env_nonempty(key).or_else(|| self.0.get(key).cloned())
    }
}

pub fn load_env() -> Result<Env, String> {
    let settings = Settings::load();
    Ok(Env {
        client_id: settings.get("TICKTICK_CLIENT_ID").ok_or(
            "TICKTICK_CLIENT_ID not set\n\n  hint: Set it as an environment variable or in $XDG_CONFIG_HOME/ticktick-cli/.env"
        )?,
        client_secret: settings.get("TICKTICK_CLIENT_SECRET").ok_or(
            "TICKTICK_CLIENT_SECRET not set\n\n  hint: Set it as an environment variable or in $XDG_CONFIG_HOME/ticktick-cli/.env"
        )?,
    })
}

pub fn oauth_port() -> u16 {
    std::env::var("TICKTICK_OAUTH_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8080)
}

pub fn load() -> Config {
    load_from(&config_path())
}

fn load_from(path: &Path) -> Config {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

static CONFIG_LOCK: Mutex<()> = Mutex::new(());

/// Held by every write to config.json, so concurrent load-modify-save
/// sequences don't drop each other's changes: the mutex within this process,
/// an advisory lock on `config.json.lock` across processes.
struct ConfigLock {
    // Declared first so the file lock is released before the mutex.
    _file: fs::File,
    _mutex: MutexGuard<'static, ()>,
}

fn lock_config(path: &Path) -> Result<ConfigLock, String> {
    let mutex = CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let lock_path = path.with_extension("json.lock");
    let file = fs::OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)
        .map_err(|e| format!("failed to open {}: {e}", lock_path.display()))?;
    file.lock()
        .map_err(|e| format!("failed to lock {}: {e}", lock_path.display()))?;
    Ok(ConfigLock {
        _file: file,
        _mutex: mutex,
    })
}

/// Change config.json in one locked load-modify-save. `f` must not call `update`.
pub fn update<R>(f: impl FnOnce(&mut Config) -> R) -> Result<R, String> {
    update_at(&config_path(), f)
}

fn update_at<R>(path: &Path, f: impl FnOnce(&mut Config) -> R) -> Result<R, String> {
    update_if_at(path, |c| (f(c), true))
}

/// Like `update_at`, but saves only when `f` returns true.
fn update_if_at<R>(path: &Path, f: impl FnOnce(&mut Config) -> (R, bool)) -> Result<R, String> {
    let _lock = lock_config(path)?;
    let mut config = load_strict(path)?;
    let (out, save) = f(&mut config);
    if save {
        save_to(path, &config)?;
    }
    Ok(out)
}

/// Unlike `load_from`, refuses a file it can't parse, so a save can't wipe it.
fn load_strict(path: &Path) -> Result<Config, String> {
    match fs::read_to_string(path) {
        Ok(s) => serde_json::from_str(&s).map_err(|e| {
            format!(
                "{} is not valid config JSON: {e}\n\n  hint: Fix it, or run 'ticktick-cli logout' to delete it",
                path.display()
            )
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Config::default()),
        Err(e) => Err(format!("failed to read {}: {e}", path.display())),
    }
}

fn save_to(path: &Path, config: &Config) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(|e| format!("failed to create config dir: {e}"))?;
    }
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;

    static WRITES: AtomicU64 = AtomicU64::new(0);
    let tmp = path.with_extension(format!(
        "{}.{}.tmp",
        std::process::id(),
        WRITES.fetch_add(1, Ordering::Relaxed)
    ));
    let result = write_private(&tmp, &json).and_then(|()| {
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            format!("failed to rename config: {e}")
        })
    });
    remove_stale_temps(path);
    result
}

/// Created 0600, so the tokens are never readable by others, even briefly.
fn write_private(path: &Path, contents: &str) -> Result<(), String> {
    use std::io::Write;
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let mut file = opts
        .open(path)
        .map_err(|e| format!("failed to write config: {e}"))?;
    file.write_all(contents.as_bytes()).map_err(|e| {
        let _ = fs::remove_file(path);
        format!("failed to write config: {e}")
    })
}

const STALE_TEMP_AGE: std::time::Duration = std::time::Duration::from_secs(60);

/// Temp files left by a process killed mid-save. Younger ones may belong to a
/// save still in flight in another process.
fn remove_stale_temps(path: &Path) {
    let (Some(dir), Some(stem)) = (path.parent(), path.file_stem().and_then(|s| s.to_str())) else {
        return;
    };
    let prefix = format!("{stem}.");
    let Ok(entries) = fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(name) = name.to_str() else { continue };
        if !name.starts_with(&prefix) || !name.ends_with(".tmp") {
            continue;
        }
        let stale = entry
            .metadata()
            .and_then(|m| m.modified())
            .is_ok_and(|t| t.elapsed().is_ok_and(|age| age > STALE_TEMP_AGE));
        if stale {
            let _ = fs::remove_file(entry.path());
        }
    }
}

/// Where the access token comes from. It decides every account-scoped value:
/// an env token may belong to a different account than config.json, so the
/// two never mix.
enum Source {
    /// `TICKTICK_ACCESS_TOKEN` from the process env or a .env file. Its refresh
    /// token and v2 cookie come from there too, its refreshes stay in memory,
    /// and config.json's tokens and account fields are never used.
    Env {
        access: String,
        settings: Settings,
    },
    Config(Config),
}

impl Source {
    fn current() -> Self {
        let settings = Settings::load();
        match settings.get("TICKTICK_ACCESS_TOKEN") {
            Some(access) => Source::Env { access, settings },
            None => Source::Config(load()),
        }
    }

    fn is_env(&self) -> bool {
        matches!(self, Source::Env { .. })
    }

    /// Before any refresh.
    fn access_token(&self) -> Option<String> {
        match self {
            Source::Env { access, .. } => Some(access.clone()),
            Source::Config(c) => c.access_token.clone().filter(|t| !t.trim().is_empty()),
        }
    }

    fn refresh_token(&self) -> Option<String> {
        match self {
            Source::Env { settings, .. } => settings.get("TICKTICK_REFRESH_TOKEN"),
            Source::Config(c) => c.refresh_token.clone(),
        }
    }

    fn v2_session_token(&self) -> Option<String> {
        match self {
            Source::Env { settings, .. } => settings.get("TICKTICK_V2_SESSION_TOKEN"),
            // Env wins so the long-running MCP server can be fed a fresh cookie.
            Source::Config(c) => env_nonempty("TICKTICK_V2_SESSION_TOKEN")
                .or_else(|| c.v2_session_token.clone().filter(|t| !t.trim().is_empty())),
        }
    }

    fn account_timezone(&self) -> Option<String> {
        match self {
            Source::Env { .. } => None,
            Source::Config(c) => c.account_timezone.clone(),
        }
    }
}

/// A refresh that config.json can't hold: always for env tokens, and for
/// config.json's tokens only when saving them failed.
struct Refreshed {
    /// Applies only while the source still holds this token, so a new login
    /// in config.json wins.
    replaced: String,
    access: String,
    refresh: String,
}

static REFRESHED: Mutex<Option<Refreshed>> = Mutex::new(None);

fn refreshed() -> MutexGuard<'static, Option<Refreshed>> {
    REFRESHED.lock().unwrap_or_else(|e| e.into_inner())
}

pub fn get_access_token() -> Result<String, String> {
    let source = Source::current().access_token().ok_or_else(|| {
        "not authenticated\n\n  hint: Run 'ticktick-cli login' to authenticate".to_string()
    })?;
    Ok(refreshed()
        .as_ref()
        .filter(|r| r.replaced == source)
        .map_or(source, |r| r.access.clone()))
}

/// The CLI's error for a 401 a refresh couldn't fix. `login` can't override
/// an env token, so for one the hint names the env vars instead.
pub fn expired_token_error() -> String {
    let hint = match Source::current() {
        Source::Env { settings, .. } if settings.get("TICKTICK_REFRESH_TOKEN").is_none() => {
            "TICKTICK_ACCESS_TOKEN (from the environment or .env) is expired. Set \
             TICKTICK_REFRESH_TOKEN, TICKTICK_CLIENT_ID and TICKTICK_CLIENT_SECRET to \
             refresh it automatically, or update or remove it"
        }
        Source::Env { .. } => {
            "TICKTICK_ACCESS_TOKEN (from the environment or .env) is expired and \
             TICKTICK_REFRESH_TOKEN could not refresh it. Update or remove both"
        }
        Source::Config(_) => "Run 'ticktick-cli login' to re-authenticate",
    };
    format!("token expired\n\n  hint: {hint}")
}

/// What a token refresh starts from. Callers serialize refreshes.
pub struct RefreshPlan {
    pub refresh: String,
    replaced: String,
    from_env: bool,
    /// config.json's refresh token when the plan was made.
    file_refresh: Option<String>,
}

/// An env access token refreshes only with `TICKTICK_REFRESH_TOKEN`, a
/// config.json one only with config.json's.
pub fn plan_refresh() -> Option<RefreshPlan> {
    let source = Source::current();
    let replaced = source.access_token()?;
    let file_refresh = match &source {
        Source::Env { .. } => None,
        Source::Config(c) => c.refresh_token.clone(),
    };
    let remembered = refreshed()
        .as_ref()
        .filter(|r| r.replaced == replaced)
        .map(|r| r.refresh.clone());
    let refresh = remembered.or_else(|| source.refresh_token())?;
    Some(RefreshPlan {
        refresh,
        replaced,
        from_env: source.is_env(),
        file_refresh,
    })
}

/// Store a refresh's tokens and return the access token to use. config.json is
/// written only if its refresh token is unchanged since `plan`; otherwise
/// another login happened, and its access token is returned instead.
pub fn store_refresh(plan: RefreshPlan, access: String, refresh: String) -> Option<String> {
    if !plan.from_env {
        match store_refresh_at(&config_path(), &plan, &access, &refresh) {
            Ok(token) => return token,
            Err(e) => eprintln!("warning: refreshed token not saved: {e}"),
        }
    }
    *refreshed() = Some(Refreshed {
        replaced: plan.replaced,
        access: access.clone(),
        refresh,
    });
    Some(access)
}

fn store_refresh_at(
    path: &Path,
    plan: &RefreshPlan,
    access: &str,
    refresh: &str,
) -> Result<Option<String>, String> {
    update_if_at(path, |cfg| {
        if cfg.refresh_token != plan.file_refresh {
            return (cfg.access_token.clone(), false);
        }
        cfg.access_token = Some(access.to_string());
        cfg.refresh_token = Some(refresh.to_string());
        (Some(access.to_string()), true)
    })
}

const ENV_TEMPLATE: &str = "\
# TickTick API credentials
# Get these from https://developer.ticktick.com/manage
TICKTICK_CLIENT_ID=
TICKTICK_CLIENT_SECRET=

# Optional: set an access token directly (skips OAuth flow)
# TICKTICK_ACCESS_TOKEN=

# Optional: OAuth callback port (default: 8080)
# TICKTICK_OAUTH_PORT=8080
";

pub fn init(local: bool, force: bool) -> Result<PathBuf, String> {
    let path = if local {
        std::env::current_dir()
            .map_err(|e| format!("failed to get current directory: {e}"))?
            .join(".env")
    } else {
        let dir = config_dir();
        fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;
        dir.join(".env")
    };

    if path.exists() && !force {
        return Err(format!(
            "{} already exists\n\n  hint: Use --force to overwrite",
            path.display()
        ));
    }

    fs::write(&path, ENV_TEMPLATE).map_err(|e| format!("failed to write .env: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("failed to set .env permissions: {e}"))?;
    }

    eprintln!("Created {}", path.display());
    Ok(path)
}

pub fn get_account_timezone() -> Option<String> {
    Source::current().account_timezone()
}

/// A no-op for an env access token.
pub fn save_account_timezone(tz: &str) -> Result<(), String> {
    if Source::current().is_env() {
        return Ok(());
    }
    update(|c| c.account_timezone = Some(tz.to_string()))
}

pub fn get_inbox_project_id() -> Option<String> {
    match Source::current() {
        Source::Env { access, .. } => load().env_inbox_ids.remove(&fingerprint(&access)),
        Source::Config(c) => c.inbox_project_id,
    }
}

pub fn save_inbox_project_id(id: &str) -> Result<(), String> {
    match Source::current() {
        Source::Env { access, .. } => update(|c| {
            c.env_inbox_ids.insert(fingerprint(&access), id.to_string());
        }),
        Source::Config(_) => update(|c| c.inbox_project_id = Some(id.to_string())),
    }
}

pub fn logout() -> Result<(), String> {
    let path = config_path();
    if path.exists() {
        let _lock = lock_config(&path)?;
        fs::remove_file(&path).map_err(|e| format!("failed to remove config: {e}"))?;
    }
    eprintln!("Logged out");
    Ok(())
}

// ---------------------------------------------------------------------------
// v2 API helpers
// ---------------------------------------------------------------------------

/// Generate a device ID in the format used by the TickTick web client:
/// "6490" prefix + 20 random hex characters.
pub fn generate_device_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};

    let seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64;

    // LCG parameters (Numerical Recipes)
    let mut state = seed;
    let mut hex = String::with_capacity(24);
    hex.push_str("6490");
    for _ in 0..20 {
        state = state
            .wrapping_mul(6364136223846793005)
            .wrapping_add(1442695040888963407);
        let nibble = ((state >> 33) & 0xf) as u8;
        hex.push(char::from_digit(nibble as u32, 16).unwrap());
    }
    hex
}

pub fn get_or_create_device_id() -> String {
    device_id_at(&config_path())
}

/// When config.json can't be written, one unsaved ID serves the whole process.
fn device_id_at(path: &Path) -> String {
    static UNSAVED: OnceLock<String> = OnceLock::new();
    if let Some(id) = load_from(path).v2_device_id {
        return id;
    }
    update_at(path, |c| {
        c.v2_device_id
            .get_or_insert_with(generate_device_id)
            .clone()
    })
    .unwrap_or_else(|_| UNSAVED.get_or_init(generate_device_id).clone())
}

pub fn get_v2_session_token() -> Option<String> {
    Source::current().v2_session_token()
}

pub(crate) fn env_nonempty(var: &str) -> Option<String> {
    std::env::var(var)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn device_id_format() {
        let id = generate_device_id();
        assert_eq!(
            id.len(),
            24,
            "device ID should be 24 chars (4 prefix + 20 hex)"
        );
        assert!(id.starts_with("6490"), "device ID should start with '6490'");
        assert!(
            id[4..].chars().all(|c| c.is_ascii_hexdigit()),
            "device ID suffix should be hex characters, got: {}",
            &id[4..]
        );
    }

    #[test]
    fn device_id_uniqueness() {
        let id1 = generate_device_id();
        // Small sleep to ensure different seed
        std::thread::sleep(std::time::Duration::from_millis(1));
        let id2 = generate_device_id();
        assert_ne!(id1, id2, "two device IDs should be different");
    }

    #[test]
    fn concurrent_updates_to_different_fields_all_survive() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let setters: [fn(&mut Config, String); 6] = [
            |c, v| c.access_token = Some(v),
            |c, v| c.refresh_token = Some(v),
            |c, v| c.account_timezone = Some(v),
            |c, v| c.inbox_project_id = Some(v),
            |c, v| c.v2_session_token = Some(v),
            |c, v| c.v2_device_id = Some(v),
        ];
        std::thread::scope(|s| {
            for (field, set) in setters.into_iter().enumerate() {
                let path = &path;
                s.spawn(move || {
                    for n in 0..25 {
                        update_at(path, |c| set(c, format!("{field}-{n}"))).unwrap();
                    }
                });
            }
        });

        let c = load_from(&path);
        let got = [
            c.access_token,
            c.refresh_token,
            c.account_timezone,
            c.inbox_project_id,
            c.v2_session_token,
            c.v2_device_id,
        ];
        for (field, value) in got.into_iter().enumerate() {
            assert_eq!(value, Some(format!("{field}-24")), "field {field}");
        }
        let leftovers: Vec<_> = fs::read_dir(dir.path())
            .unwrap()
            .map(|e| e.unwrap().file_name())
            .filter(|n| n != "config.json" && n != "config.json.lock")
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    }

    #[test]
    fn update_waits_for_another_processs_file_lock() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let other = fs::File::create(dir.path().join("config.json.lock")).unwrap();
        other.lock().unwrap();
        let held = std::time::Duration::from_millis(300);
        let release = std::thread::spawn(move || {
            std::thread::sleep(held);
            drop(other);
        });

        let started = std::time::Instant::now();
        update_at(&path, |c| c.inbox_project_id = Some("x".into())).unwrap();

        assert!(started.elapsed() >= held, "{:?}", started.elapsed());
        release.join().unwrap();
    }

    #[test]
    fn update_refuses_to_overwrite_unparsable_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let corrupt = r#"{"access_token": "keep-me", "#;
        fs::write(&path, corrupt).unwrap();

        let ran = update_at(&path, |c| c.inbox_project_id = Some("x".into()));

        assert!(ran.is_err());
        assert_eq!(fs::read_to_string(&path).unwrap(), corrupt);
    }

    #[test]
    fn save_removes_only_stale_temp_files() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let stale = dir.path().join("config.111.0.tmp");
        let fresh = dir.path().join("config.222.0.tmp");
        let unrelated = dir.path().join("other.333.0.tmp");
        for p in [&stale, &fresh, &unrelated] {
            fs::write(p, "{}").unwrap();
        }
        let old = std::time::SystemTime::now() - std::time::Duration::from_secs(120);
        for p in [&stale, &unrelated] {
            fs::File::options()
                .write(true)
                .open(p)
                .unwrap()
                .set_modified(old)
                .unwrap();
        }

        save_to(&path, &Config::default()).unwrap();

        assert!(!stale.exists());
        assert!(fresh.exists());
        assert!(unrelated.exists());
    }

    #[cfg(unix)]
    #[test]
    fn saved_config_is_private() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        save_to(&path, &Config::default()).unwrap();
        let mode = fs::metadata(&path).unwrap().permissions().mode();
        assert_eq!(mode & 0o777, 0o600);
    }

    #[test]
    fn unwritable_config_reuses_one_device_id() {
        let dir = tempfile::tempdir().unwrap();
        let blocker = dir.path().join("not-a-dir");
        fs::write(&blocker, "").unwrap();
        let path = blocker.join("config.json");

        let first = device_id_at(&path);
        std::thread::sleep(std::time::Duration::from_millis(1));
        assert_eq!(device_id_at(&path), first);
    }

    #[test]
    fn device_id_is_saved_and_reused() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let first = device_id_at(&path);
        assert_eq!(load_from(&path).v2_device_id.as_deref(), Some(&*first));
        assert_eq!(device_id_at(&path), first);
    }

    fn plan(file_refresh: &str) -> RefreshPlan {
        RefreshPlan {
            refresh: file_refresh.into(),
            replaced: "a0".into(),
            from_env: false,
            file_refresh: Some(file_refresh.into()),
        }
    }

    #[test]
    fn refresh_is_saved_while_config_still_holds_the_used_token() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        fs::write(&path, r#"{"access_token":"a0","refresh_token":"r1"}"#).unwrap();

        let token = store_refresh_at(&path, &plan("r1"), "a1", "r2").unwrap();

        assert_eq!(token.as_deref(), Some("a1"));
        let c = load_from(&path);
        assert_eq!(c.access_token.as_deref(), Some("a1"));
        assert_eq!(c.refresh_token.as_deref(), Some("r2"));
    }

    #[test]
    fn refresh_yields_to_a_login_that_happened_meanwhile() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.json");
        let login = r#"{"access_token":"login","refresh_token":"r9"}"#;
        fs::write(&path, login).unwrap();

        let token = store_refresh_at(&path, &plan("r1"), "a1", "r2").unwrap();

        assert_eq!(token.as_deref(), Some("login"));
        assert_eq!(fs::read_to_string(&path).unwrap(), login);
    }

    #[test]
    fn update_creates_a_missing_config() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("sub/config.json");
        update_at(&path, |c| c.inbox_project_id = Some("inbox1".into())).unwrap();
        assert_eq!(load_from(&path).inbox_project_id.as_deref(), Some("inbox1"));
    }

    #[test]
    fn fingerprint_is_fnv1a_64() {
        // Published FNV-1a 64-bit test vectors.
        assert_eq!(fingerprint(""), "cbf29ce484222325");
        assert_eq!(fingerprint("a"), "af63dc4c8601ec8c");
        assert_eq!(fingerprint("foobar"), "85944171f73967e8");
    }
}
