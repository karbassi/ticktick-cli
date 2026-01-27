pub mod auth;
pub mod project;
pub mod task;

const BASE_URL: &str = "https://api.ticktick.com/open/v1";

pub fn get(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    ureq::get(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| format!("API request failed: {e}"))
}

#[allow(dead_code)]
pub fn post(endpoint: &str, token: &str, body: &serde_json::Value) -> Result<ureq::Response, String> {
    ureq::post(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .set("Content-Type", "application/json")
        .send_json(body.clone())
        .map_err(|e| format!("API request failed: {e}"))
}

#[allow(dead_code)]
pub fn delete(endpoint: &str, token: &str) -> Result<ureq::Response, String> {
    ureq::delete(&format!("{BASE_URL}{endpoint}"))
        .set("Authorization", &format!("Bearer {token}"))
        .call()
        .map_err(|e| format!("API request failed: {e}"))
}
