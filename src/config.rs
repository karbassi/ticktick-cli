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
    dirs_home().join(".config").join("ticktick-cli")
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
    let env_path = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .join(".env");

    let content = fs::read_to_string(&env_path)
        .map_err(|_| "failed to read .env file")?;

    let mut client_id = None;
    let mut client_secret = None;
    let mut access_token = None;

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            let value = value.trim().trim_matches('"').trim_matches('\'');
            match key.trim() {
                "TICKTICK_CLIENT_ID" => client_id = Some(value.to_string()),
                "TICKTICK_CLIENT_SECRET" => client_secret = Some(value.to_string()),
                "TICKTICK_ACCESS_TOKEN" => access_token = Some(value.to_string()),
                _ => {}
            }
        }
    }

    Ok(Env {
        client_id: client_id.ok_or("TICKTICK_CLIENT_ID not found in .env")?,
        client_secret: client_secret.ok_or("TICKTICK_CLIENT_SECRET not found in .env")?,
        access_token,
    })
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

    fs::write(&path, json).map_err(|e| format!("failed to write config: {e}"))?;
    Ok(())
}

pub fn get_access_token() -> Result<String, String> {
    // First check .env for testing
    if let Ok(env) = load_env() {
        if let Some(token) = env.access_token {
            return Ok(token);
        }
    }

    // Then check stored config
    let config = load();
    config.access_token.ok_or_else(|| {
        "not authenticated. Run 'ticktick login' first".to_string()
    })
}

pub fn logout() -> Result<(), String> {
    let path = config_path();
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("failed to remove config: {e}"))?;
    }
    println!("\x1b[32mLogged out successfully\x1b[0m");
    Ok(())
}
