use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, MutexGuard};

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
}

#[derive(Debug)]
pub struct Env {
    pub client_id: String,
    pub client_secret: String,
    pub access_token: Option<String>,
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

pub fn load_env() -> Result<Env, String> {
    // First try environment variables
    let mut client_id = std::env::var("TICKTICK_CLIENT_ID").ok();
    let mut client_secret = std::env::var("TICKTICK_CLIENT_SECRET").ok();
    let mut access_token = std::env::var("TICKTICK_ACCESS_TOKEN").ok();

    // Fall back to .env file for any missing values
    if (client_id.is_none() || client_secret.is_none())
        && let Some(content) = find_env_file()
    {
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, value)) = line.split_once('=') {
                let value = value.trim().trim_matches('"').trim_matches('\'');
                match key.trim() {
                    "TICKTICK_CLIENT_ID" if client_id.is_none() => {
                        client_id = Some(value.to_string())
                    }
                    "TICKTICK_CLIENT_SECRET" if client_secret.is_none() => {
                        client_secret = Some(value.to_string())
                    }
                    "TICKTICK_ACCESS_TOKEN" if access_token.is_none() => {
                        access_token = Some(value.to_string())
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(Env {
        client_id: client_id.ok_or(
            "TICKTICK_CLIENT_ID not set\n\n  hint: Set it as an environment variable or in $XDG_CONFIG_HOME/ticktick-cli/.env"
        )?,
        client_secret: client_secret.ok_or(
            "TICKTICK_CLIENT_SECRET not set\n\n  hint: Set it as an environment variable or in $XDG_CONFIG_HOME/ticktick-cli/.env"
        )?,
        access_token,
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

/// Held by every write to config.json, so concurrent load-modify-save
/// sequences don't drop each other's changes.
static CONFIG_LOCK: Mutex<()> = Mutex::new(());

fn config_lock() -> MutexGuard<'static, ()> {
    CONFIG_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Change config.json in one locked load-modify-save. `f` must not call `update`.
pub fn update<R>(f: impl FnOnce(&mut Config) -> R) -> Result<R, String> {
    update_at(&config_path(), f)
}

fn update_at<R>(path: &Path, f: impl FnOnce(&mut Config) -> R) -> Result<R, String> {
    let _lock = config_lock();
    let mut config = load_from(path);
    let out = f(&mut config);
    save_to(path, &config)?;
    Ok(out)
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
    let result = write_private(&tmp, &json)
        .and_then(|()| fs::rename(&tmp, path).map_err(|e| format!("failed to rename config: {e}")));
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

fn write_private(path: &Path, contents: &str) -> Result<(), String> {
    fs::write(path, contents).map_err(|e| format!("failed to write config: {e}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("failed to set config permissions: {e}"))?;
    }
    Ok(())
}

/// Tokens from a refresh in this process. The env and files still hold the
/// expired access token, so these win over them.
#[derive(Clone)]
pub struct RefreshedTokens {
    pub access: String,
    pub refresh: Option<String>,
    /// Whether rotations are written back to config.json.
    pub persist: bool,
}

static REFRESHED: Mutex<Option<RefreshedTokens>> = Mutex::new(None);

pub fn refreshed_tokens() -> Option<RefreshedTokens> {
    REFRESHED.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

pub fn set_refreshed_tokens(tokens: RefreshedTokens) {
    *REFRESHED.lock().unwrap_or_else(|e| e.into_inner()) = Some(tokens);
}

/// `TICKTICK_REFRESH_TOKEN`, else config.json's (true when from config.json).
pub fn get_refresh_token() -> Option<(String, bool)> {
    if let Some(token) = env_nonempty("TICKTICK_REFRESH_TOKEN") {
        return Some((token, false));
    }
    load().refresh_token.map(|t| (t, true))
}

pub fn get_access_token() -> Result<String, String> {
    if let Some(tokens) = refreshed_tokens() {
        return Ok(tokens.access);
    }

    // Needs no client ID/secret; only `login` and token refresh use those.
    if let Some(token) = env_nonempty("TICKTICK_ACCESS_TOKEN") {
        return Ok(token);
    }

    if let Ok(env) = load_env()
        && let Some(token) = env.access_token
    {
        return Ok(token);
    }

    // Then check stored config
    let config = load();
    config.access_token.ok_or_else(|| {
        "not authenticated\n\n  hint: Run 'ticktick-cli login' to authenticate".to_string()
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

pub fn init(local: bool, force: bool) -> Result<(), String> {
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
    crate::output::success(&serde_json::json!({"path": path.display().to_string()}));
    Ok(())
}

pub fn get_account_timezone() -> Option<String> {
    load().account_timezone
}

pub fn save_account_timezone(tz: &str) -> Result<(), String> {
    update(|c| c.account_timezone = Some(tz.to_string()))
}

pub fn get_inbox_project_id() -> Option<String> {
    load().inbox_project_id
}

pub fn save_inbox_project_id(id: &str) -> Result<(), String> {
    update(|c| c.inbox_project_id = Some(id.to_string()))
}

pub fn logout() -> Result<(), String> {
    {
        let _lock = config_lock();
        let path = config_path();
        if path.exists() {
            fs::remove_file(&path).map_err(|e| format!("failed to remove config: {e}"))?;
        }
    }
    eprintln!("Logged out");
    crate::output::success(&serde_json::json!({"status": "ok"}));
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

/// An unsaved ID is still returned when config.json can't be written.
pub fn get_or_create_device_id() -> String {
    if let Some(id) = load().v2_device_id {
        return id;
    }
    update(|c| {
        c.v2_device_id
            .get_or_insert_with(generate_device_id)
            .clone()
    })
    .unwrap_or_else(|_| generate_device_id())
}

/// Env wins so the long-running MCP server can be fed a fresh cookie.
pub fn get_v2_session_token() -> Option<String> {
    env_nonempty("TICKTICK_V2_SESSION_TOKEN").or_else(|| load().v2_session_token)
}

fn env_nonempty(var: &str) -> Option<String> {
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
            .filter(|n| n != "config.json")
            .collect();
        assert!(leftovers.is_empty(), "temp files left: {leftovers:?}");
    }
}
