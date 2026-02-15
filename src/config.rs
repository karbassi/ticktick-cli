use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Config {
    pub access_token: Option<String>,
    pub refresh_token: Option<String>,
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
    let path = config_path();
    fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn save(config: &Config) -> Result<(), String> {
    let dir = config_dir();
    fs::create_dir_all(&dir).map_err(|e| format!("failed to create config dir: {e}"))?;

    let path = config_path();
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("failed to serialize config: {e}"))?;

    let tmp = path.with_extension("tmp");
    if tmp.exists() {
        let _ = fs::remove_file(&tmp);
    }
    fs::write(&tmp, &json).map_err(|e| format!("failed to write config: {e}"))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&tmp, fs::Permissions::from_mode(0o600))
            .map_err(|e| format!("failed to set config permissions: {e}"))?;
    }

    fs::rename(&tmp, &path).map_err(|e| format!("failed to rename config: {e}"))?;
    Ok(())
}

pub fn get_access_token() -> Result<String, String> {
    // Check for access token in env vars / .env file
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

pub fn logout() -> Result<(), String> {
    let path = config_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("failed to remove config: {e}"))?;
    }
    eprintln!("Logged out");
    crate::output::success(&serde_json::json!({"status": "ok"}));
    Ok(())
}
