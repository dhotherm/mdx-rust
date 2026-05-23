//! Integration test for machine-pure --json mode (P1 Codex Stabilization requirement).
//!
//! Runs the built `mdx-rust` binary with `--json` and verifies stdout is valid JSON
//! (no human text leakage). Uses the already-built binary from the test profile when possible.

use std::path::PathBuf;
use std::process::Command;
use std::str;

fn find_built_binary() -> Option<PathBuf> {
    // Best-effort: look for a previously built debug binary
    let candidates = ["target/debug/mdx-rust", "../target/debug/mdx-rust"];
    for c in &candidates {
        let p = PathBuf::from(c);
        if p.exists() {
            return Some(p);
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
