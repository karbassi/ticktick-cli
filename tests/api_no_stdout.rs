//! `src/api` is shared with the MCP server, whose stdio transport owns stdout.

use std::fs;
use std::path::{Path, PathBuf};

const FORBIDDEN: &[&str] = &["println!", "print!", "output::success", "stdout()"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(dir).expect("read src/api") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

/// Matches `needle` only as a whole identifier, so `eprintln!` doesn't match `println!`.
fn has_token(line: &str, needle: &str) -> bool {
    line.match_indices(needle).any(|(i, _)| {
        !line[..i]
            .chars()
            .next_back()
            .is_some_and(|c| c.is_alphanumeric() || c == '_')
    })
}

#[test]
fn has_token_ignores_stderr_macros() {
    assert!(has_token("    println!(\"x\");", "println!"));
    assert!(has_token("std::print!(\"x\")", "print!"));
    assert!(!has_token("    eprintln!(\"x\");", "println!"));
    assert!(!has_token("    eprint!(\"x\");", "print!"));
}

#[test]
fn api_layer_never_writes_to_stdout() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/api");
    let mut files = Vec::new();
    rust_files(&root, &mut files);
    assert!(!files.is_empty(), "no files found under {}", root.display());

    let mut hits = Vec::new();
    for file in &files {
        let src = fs::read_to_string(file).expect("read source");
        for (n, line) in src.lines().enumerate() {
            if line.trim_start().starts_with("//") {
                continue;
            }
            for needle in FORBIDDEN {
                if has_token(line, needle) {
                    hits.push(format!("{}:{}: {}", file.display(), n + 1, line.trim()));
                }
            }
        }
    }
    assert!(
        hits.is_empty(),
        "src/api must not write to stdout:\n{}",
        hits.join("\n")
    );
}
