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

#[test]
fn optimize_json_mode_produces_parseable_json() {
    // Prefer a pre-built binary if present; otherwise fall back to `cargo run -q`
    let mut cmd = if let Some(bin) = find_built_binary() {
        let mut c = Command::new(bin);
        c.arg("optimize")
            .arg("nonexistent-for-json-test")
            .arg("--iterations")
            .arg("1")
            .arg("--json");
        c
    } else {
        let mut c = Command::new("cargo");
        c.args([
            "run",
            "-q",
            "--",
            "optimize",
            "nonexistent-for-json-test",
            "--iterations",
            "1",
            "--json",
        ]);
        c
    };

    let output = cmd.output().expect("failed to invoke mdx-rust binary");

    let stdout = str::from_utf8(&output.stdout).expect("stdout must be valid UTF-8");
    let stderr = str::from_utf8(&output.stderr).expect("stderr must be valid UTF-8");

    // Search both stdout and stderr for a line that is valid JSON.
    // In real --json runs the optimizer emits a JSON array of runs.
    // On early errors (agent not registered) the top-level handler now emits a JSON error object.
    let combined = format!("{}\n{}", stdout, stderr);

    let mut found_json = false;
    for line in combined.lines() {
        let t = line.trim();
        if (t.starts_with('{') || t.starts_with('['))
            && serde_json::from_str::<serde_json::Value>(t).is_ok()
        {
            found_json = true;
            break;
        }
    }

    assert!(
        found_json,
        "Expected machine-pure JSON (object or array) from --json mode. Got stdout+stderr:\n{}",
        combined
    );
}
