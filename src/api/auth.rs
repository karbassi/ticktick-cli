use crate::config;
use std::io::{BufRead, BufReader, Write};
use std::net::TcpListener;

const AUTH_URL: &str = "https://ticktick.com/oauth/authorize";
const TOKEN_URL: &str = "https://ticktick.com/oauth/token";
const REDIRECT_URI: &str = "http://127.0.0.1:8585/callback";
const SCOPE: &str = "tasks:read tasks:write";

pub fn login() -> Result<(), String> {
    let env = config::load_env()?;

    let auth_url = format!(
        "{}?client_id={}&scope={}&redirect_uri={}&response_type=code",
        AUTH_URL,
        urlencoding(&env.client_id),
        urlencoding(SCOPE),
        urlencoding(REDIRECT_URI),
    );

    println!("\x1b[33mOpen this URL in your browser:\x1b[0m\n");
    println!("{auth_url}\n");

    println!("\x1b[90mWaiting for authorization...\x1b[0m");

    let code = wait_for_callback()?;

    println!("\x1b[90mExchanging code for token...\x1b[0m");

    let token = exchange_code(&code, &env.client_id, &env.client_secret)?;

    let mut cfg = config::load();
    cfg.access_token = Some(token.access_token);
    cfg.refresh_token = token.refresh_token;
    config::save(&cfg)?;

    println!("\x1b[32mSuccessfully authenticated!\x1b[0m");
    Ok(())
}

fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for c in s.chars() {
        match c {
            'a'..='z' | 'A'..='Z' | '0'..='9' | '-' | '_' | '.' | '~' => result.push(c),
            ' ' => result.push_str("%20"),
            ':' => result.push_str("%3A"),
            '/' => result.push_str("%2F"),
            _ => {
                for b in c.to_string().as_bytes() {
                    result.push_str(&format!("%{:02X}", b));
                }
            }
        }
    }
    result
}

fn wait_for_callback() -> Result<String, String> {
    let listener = TcpListener::bind("127.0.0.1:8585")
        .map_err(|e| format!("failed to bind to port 8585: {e}"))?;

    let (mut stream, _) = listener
        .accept()
        .map_err(|e| format!("failed to accept connection: {e}"))?;

    let mut reader = BufReader::new(&stream);
    let mut request_line = String::new();
    reader
        .read_line(&mut request_line)
        .map_err(|e| format!("failed to read request: {e}"))?;

    let code = request_line
        .split_whitespace()
        .nth(1)
        .and_then(|path| path.strip_prefix("/callback?code="))
        .map(|s| s.split('&').next().unwrap_or(s))
        .map(|s| s.to_string())
        .ok_or("no authorization code in callback")?;

    let response = "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\n\r\n\
        <html><body><h1>Authorization successful!</h1>\
        <p>You can close this window.</p></body></html>";

    stream
        .write_all(response.as_bytes())
        .map_err(|e| format!("failed to send response: {e}"))?;

    Ok(code)
}

#[derive(serde::Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: Option<String>,
}

fn exchange_code(code: &str, client_id: &str, client_secret: &str) -> Result<TokenResponse, String> {
    let body = format!(
        "grant_type=authorization_code&code={}&redirect_uri={}&scope={}",
        urlencoding(code),
        urlencoding(REDIRECT_URI),
        urlencoding(SCOPE),
    );

    let auth = base64_encode(&format!("{client_id}:{client_secret}"));

    let resp = ureq::post(TOKEN_URL)
        .set("Authorization", &format!("Basic {auth}"))
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| format!("token exchange failed: {e}"))?;

    resp.into_json::<TokenResponse>()
        .map_err(|e| format!("failed to parse token response: {e}"))
}

pub fn refresh_token(refresh_token: &str) -> Result<(String, Option<String>), String> {
    let env = config::load_env()?;

    let body = format!(
        "grant_type=refresh_token&refresh_token={}",
        urlencoding(refresh_token),
    );

    let auth = base64_encode(&format!("{}:{}", env.client_id, env.client_secret));

    let resp = ureq::post(TOKEN_URL)
        .set("Authorization", &format!("Basic {auth}"))
        .set("Content-Type", "application/x-www-form-urlencoded")
        .send_string(&body)
        .map_err(|e| format!("token refresh failed: {e}"))?;

    let token: TokenResponse = resp
        .into_json()
        .map_err(|e| format!("failed to parse token response: {e}"))?;

    Ok((token.access_token, token.refresh_token))
}

fn base64_encode(s: &str) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let bytes = s.as_bytes();
    let mut result = String::new();

    for chunk in bytes.chunks(3) {
        let mut n = 0u32;
        for (i, &b) in chunk.iter().enumerate() {
            n |= (b as u32) << (16 - 8 * i);
        }

        let chars = match chunk.len() {
            3 => 4,
            2 => 3,
            1 => 2,
            _ => 0,
        };

        for i in 0..chars {
            let idx = ((n >> (18 - 6 * i)) & 0x3F) as usize;
            result.push(ALPHABET[idx] as char);
        }

        for _ in 0..(4 - chars) {
            result.push('=');
        }
    }

    result
}
