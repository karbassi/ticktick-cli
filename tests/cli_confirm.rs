//! Delete confirmation needs a TTY on stdin, so these run the CLI under
//! `script`, which gives it a pseudo-terminal.

mod common;

use std::io::Write;
use std::process::{Command, Stdio};

use httpmock::MockServer;

fn under_pty(dir: &std::path::Path, server: &MockServer, args: &[&str], answer: &str) {
    let cli = common::isolated_command(dir);
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("script");
        c.arg("-q").arg("/dev/null").arg(common::bin()).args(args);
        c
    } else {
        let line = std::iter::once(common::bin())
            .chain(args.iter().copied())
            .map(|a| format!("'{a}'"))
            .collect::<Vec<_>>()
            .join(" ");
        let mut c = Command::new("script");
        c.args(["-qec", &line, "/dev/null"]);
        c
    };
    cmd.current_dir(dir);
    for (k, v) in cli.get_envs() {
        match v {
            Some(v) => cmd.env(k, v),
            None => cmd.env_remove(k),
        };
    }
    cmd.env("TICKTICK_ACCESS_TOKEN", common::ACCESS_TOKEN)
        .env("TICKTICK_API_BASE", server.url("/open/v1"))
        .env("TICKTICK_V2_BASE", server.url("/api/v2"))
        .env("TICKTICK_V2_SESSION_TOKEN", common::SESSION_TOKEN)
        .env_remove("CI")
        .stdin(Stdio::piped())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let mut child = cmd.spawn().expect("script(1) is required for this test");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(answer.as_bytes())
        .unwrap();
    child.wait().unwrap();
}

#[test]
fn declining_a_delete_sends_no_request() {
    let server = MockServer::start();
    let any = server.mock(|_, then| {
        then.status(200).body("[]");
    });
    let dir = tempfile::tempdir().unwrap();

    for args in [
        &["task", "delete", common::HEX_PROJECT, "bbbbbbbbbbbbbbbbbbbbbbbb"][..],
        &["tag", "delete", "work"][..],
        &["folder", "delete", "Home"][..],
        &["habit", "delete", "Read"][..],
        &["filter", "delete", "Today"][..],
    ] {
        under_pty(dir.path(), &server, args, "n\n");
    }

    any.assert_calls(0);
}
