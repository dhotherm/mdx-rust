//! Integration test for machine-pure --json mode (P1 Codex Stabilization requirement).
//!
//! Runs the built `mdx-rust` binary with `--json` and verifies stdout is valid JSON
//! (no human text leakage). Uses the already-built binary from the test profile when possible.

use std::path::PathBuf;
use std::process::Command;
use std::str;
use tempfile::tempdir;

fn find_built_binary() -> Option<PathBuf> {
    if let Ok(bin) = std::env::var("CARGO_BIN_EXE_mdx-rust") {
        let path = PathBuf::from(bin);
        if path.exists() {
            return Some(path);
        }
    }

    // Best-effort: look for a previously built debug binary
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = manifest_dir
        .parent()
        .and_then(|path| path.parent())
        .map(PathBuf::from);
    let candidates = [
        PathBuf::from("target/debug/mdx-rust"),
        PathBuf::from("../target/debug/mdx-rust"),
        workspace_root
            .unwrap_or_else(|| PathBuf::from("."))
            .join("target/debug/mdx-rust"),
    ];
    for c in &candidates {
        if c.exists() {
            return Some(c.clone());
        }
    }
    None
}

fn mdx_command(args: &[&str]) -> Command {
    let mut cmd = if let Some(bin) = find_built_binary() {
        Command::new(bin)
    } else {
        let mut c = Command::new("cargo");
        c.args(["run", "-q", "--"]);
        c
    };
    cmd.args(args);
    cmd
}

fn mdx_command_in(args: &[&str], dir: &std::path::Path) -> Command {
    let mut cmd = mdx_command(args);
    cmd.current_dir(dir);
    cmd
}

fn assert_machine_pure_json(args: &[&str]) -> serde_json::Value {
    let mut cmd = mdx_command(args);

    let output = cmd.output().expect("failed to invoke mdx-rust binary");

    let stdout = str::from_utf8(&output.stdout).expect("stdout must be valid UTF-8");
    let stderr = str::from_utf8(&output.stderr).expect("stderr must be valid UTF-8");

    assert!(stderr.trim().is_empty(), "stderr must be empty: {stderr}");
    assert!(
        serde_json::from_str::<serde_json::Value>(stdout.trim()).is_ok(),
        "stdout must be exactly parseable JSON. Got:\n{stdout}"
    );
    serde_json::from_str(stdout.trim()).unwrap()
}

#[test]
fn optimize_json_mode_produces_parseable_json() {
    let value = assert_machine_pure_json(&[
        "optimize",
        "nonexistent-for-json-test",
        "--iterations",
        "1",
        "--json",
    ]);
    assert_eq!(value["status"], "error");
}

#[test]
fn other_json_commands_are_machine_pure_on_errors() {
    for args in [
        &["doctor", "missing-json-agent", "--json"][..],
        &["spec", "missing-json-agent", "--json"][..],
        &["invoke", "missing-json-agent", "--json"][..],
        &["eval", "missing-json-agent", "--json"][..],
        &["audit", "missing-json-agent", "--json"][..],
    ] {
        let value = assert_machine_pure_json(args);
        if args[0] == "doctor" {
            assert_eq!(value["registered"], false);
        } else {
            assert_eq!(value["status"], "error");
        }
    }
}

#[test]
fn init_json_writes_artifact_dir_at_config_root() {
    let dir = tempdir().expect("temp dir");
    let output = mdx_command_in(&["init", "--json"], dir.path())
        .output()
        .expect("failed to invoke mdx-rust init");

    let stdout = str::from_utf8(&output.stdout).expect("stdout utf8");
    let stderr = str::from_utf8(&output.stderr).expect("stderr utf8");
    assert!(output.status.success(), "init failed: {stderr}");
    assert!(stderr.trim().is_empty(), "stderr must be empty: {stderr}");

    let value: serde_json::Value =
        serde_json::from_str(stdout.trim()).expect("init stdout must be json");
    assert_eq!(value["status"], "initialized");

    let config_path = dir.path().join(".mdx-rust/config.toml");
    let config = std::fs::read_to_string(config_path).expect("generated config");
    let artifact_pos = config
        .find("artifact_dir = \".mdx-rust\"")
        .expect("artifact_dir key should exist");
    let models_pos = config.find("[models]").expect("models table should exist");
    assert!(
        artifact_pos < models_pos,
        "artifact_dir must live at config root, not inside [models]"
    );
}
