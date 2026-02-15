use std::sync::OnceLock;

static VERBOSE_LEVEL: OnceLock<u8> = OnceLock::new();

pub fn init(verbose: u8) {
    let _ = VERBOSE_LEVEL.set(verbose);
}

pub fn verbose() -> u8 {
    VERBOSE_LEVEL.get().copied().unwrap_or(0)
}

pub fn success(data: &impl serde::Serialize) {
    println!(
        "{}",
        serde_json::to_string_pretty(data).expect("failed to serialize output")
    );
}

pub fn error(msg: &str) {
    eprintln!("{}", serde_json::json!({"error": msg}));
}
