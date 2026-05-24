//! Integration test for machine-pure --json mode.
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

fn assert_machine_pure_json_in(args: &[&str], dir: &std::path::Path) -> serde_json::Value {
    let output = mdx_command_in(args, dir)
        .output()
        .expect("failed to invoke mdx-rust binary");

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
fn schema_json_mode_outputs_machine_parseable_schema() {
    let agent_contract = assert_machine_pure_json(&["schema", "agent-contract", "--json"]);
    assert_eq!(agent_contract["title"], "MdxAgentContract");
    assert!(agent_contract["properties"]["commands"].is_object());

    let runtime = assert_machine_pure_json(&["schema", "agent-runtime-manifest", "--json"]);
    assert_eq!(runtime["title"], "AgentRuntimeManifest");
    assert!(runtime["properties"]["tools"].is_object());

    let agent_pack = assert_machine_pure_json(&["schema", "agent-pack", "--json"]);
    assert_eq!(agent_pack["title"], "AgentPack");

    let artifact_explanation =
        assert_machine_pure_json(&["schema", "artifact-explanation", "--json"]);
    assert_eq!(artifact_explanation["title"], "ArtifactExplanation");

    let recipe_catalog = assert_machine_pure_json(&["schema", "recipe-catalog", "--json"]);
    assert_eq!(recipe_catalog["title"], "RecipeCatalog");

    let scorecard = assert_machine_pure_json(&["schema", "evolution-scorecard", "--json"]);
    assert_eq!(scorecard["title"], "EvolutionScorecard");

    let value = assert_machine_pure_json(&["schema", "audit-packet", "--json"]);

    assert_eq!(value["title"], "AuditPacket");
    assert_eq!(value["type"], "object");
    assert!(value["properties"]["schema_version"].is_object());

    let hardening = assert_machine_pure_json(&["schema", "hardening-run", "--json"]);
    assert_eq!(hardening["title"], "HardeningRun");
    assert!(hardening["properties"]["outcome"].is_object());

    let behavior = assert_machine_pure_json(&["schema", "behavior-eval-report", "--json"]);
    assert_eq!(behavior["title"], "BehaviorEvalReport");

    let policy = assert_machine_pure_json(&["schema", "project-policy", "--json"]);
    assert_eq!(policy["title"], "ProjectPolicy");

    let plan = assert_machine_pure_json(&["schema", "refactor-plan", "--json"]);
    assert_eq!(plan["title"], "RefactorPlan");
    assert!(plan["properties"]["plan_id"].is_object());

    let apply_plan = assert_machine_pure_json(&["schema", "refactor-apply-run", "--json"]);
    assert_eq!(apply_plan["title"], "RefactorApplyRun");
    assert!(apply_plan["properties"]["candidate_id"].is_object());

    let batch_apply_plan =
        assert_machine_pure_json(&["schema", "refactor-batch-apply-run", "--json"]);
    assert_eq!(batch_apply_plan["title"], "RefactorBatchApplyRun");
    assert!(batch_apply_plan["properties"]["executed_candidates"].is_object());

    let codebase_map = assert_machine_pure_json(&["schema", "codebase-map", "--json"]);
    assert_eq!(codebase_map["title"], "CodebaseMap");
    assert!(codebase_map["properties"]["quality"].is_object());

    let autopilot = assert_machine_pure_json(&["schema", "autopilot-run", "--json"]);
    assert_eq!(autopilot["title"], "AutopilotRun");
    assert!(autopilot["properties"]["passes"].is_object());
}

#[test]
fn agent_contract_json_mode_is_machine_parseable() {
    let value = assert_machine_pure_json(&["agent-contract", "--json"]);

    assert_eq!(value["schema_version"], "0.9");
    assert_eq!(
        value["json_mode_contract"],
        "Pass --json for machine-pure stdout. Errors are emitted as structured JSON when --json is set."
    );
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(|command| command["name"] == "evolve"
            && command["mutates_source"] == serde_json::Value::Bool(true)));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(
            |command| command["name"] == "recipes" && command["primary_schema"] == "recipe-catalog"
        ));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(|command| command["name"] == "scorecard"
            && command["primary_schema"] == "evolution-scorecard"));
    assert!(value["artifact_globs"]
        .as_array()
        .expect("artifact globs")
        .iter()
        .any(|glob| glob
            .as_str()
            .is_some_and(|glob| glob.contains("/scorecards/"))));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(|command| command["name"] == "runtime"
            && command["primary_schema"] == "agent-runtime-manifest"));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(|command| command["name"] == "mcp" && command["mutates_source"] == true));
    let mcp = value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .find(|command| command["name"] == "mcp")
        .expect("mcp command");
    assert!(mcp["required_flags_for_mutation"]
        .as_array()
        .expect("mcp required flags")
        .iter()
        .any(|flag| flag == "confirm_mutation=true"));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(
            |command| command["name"] == "agent-pack" && command["primary_schema"] == "agent-pack"
        ));
    assert!(value["commands"]
        .as_array()
        .expect("commands array")
        .iter()
        .any(|command| command["name"] == "explain"
            && command["primary_schema"] == "artifact-explanation"));
    assert!(value["safety_rules"]
        .as_array()
        .expect("safety rules")
        .iter()
        .any(|rule| rule
            .as_str()
            .is_some_and(|rule| rule.contains("Never add --apply"))));
}

#[test]
fn runtime_and_agent_pack_json_mode_are_machine_parseable() {
    let runtime = assert_machine_pure_json(&["runtime", "--json"]);
    assert_eq!(runtime["schema_version"], "0.9");
    assert!(runtime["transports"]
        .as_array()
        .expect("transports")
        .iter()
        .any(|transport| transport["id"] == "mcp-stdio"));
    assert!(runtime["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .any(|tool| tool["name"] == "evolve" && tool["mutation_capable"] == true));
    let evolve_tool = runtime["tools"]
        .as_array()
        .expect("tools")
        .iter()
        .find(|tool| tool["name"] == "evolve")
        .expect("evolve tool");
    assert!(evolve_tool["required_flags_for_mutation"]
        .as_array()
        .expect("required flags")
        .iter()
        .any(|flag| flag == "confirm_mutation=true"));

    let pack = assert_machine_pure_json(&["agent-pack", "codex", "--json"]);
    assert_eq!(pack["schema_version"], "0.9");
    assert_eq!(pack["target_agent"], "codex");
    assert!(pack["files"]
        .as_array()
        .expect("files")
        .iter()
        .any(|file| file["path"]
            .as_str()
            .is_some_and(|path| path.contains(".codex/skills"))));
    assert_eq!(
        pack["written_files"]
            .as_array()
            .expect("written files")
            .len(),
        0
    );
}

#[test]
fn mcp_stdio_lists_tools_and_blocks_unconfirmed_mutation() {
    let mut child = mdx_command(&["mcp", "--stdio"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn mcp");
    {
        use std::io::Write;
        let stdin = child.stdin.as_mut().expect("stdin");
        writeln!(stdin, r#"{{"id":1,"method":"tools/list"}}"#).unwrap();
        writeln!(
            stdin,
            r#"{{"id":2,"method":"tools/call","params":{{"name":"evolve","arguments":{{"apply":true}}}}}}"#
        )
        .unwrap();
    }
    let output = child.wait_with_output().expect("mcp output");
    let stderr = str::from_utf8(&output.stderr).expect("stderr utf8");
    assert!(stderr.trim().is_empty(), "stderr must be empty: {stderr}");
    let stdout = str::from_utf8(&output.stdout).expect("stdout utf8");
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2, "{stdout}");
    let first: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
    assert!(first["result"]
        .as_array()
        .expect("tool list")
        .iter()
        .any(|tool| tool["name"] == "scorecard"));
    let second: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
    assert!(second["error"]["message"]
        .as_str()
        .is_some_and(|message| message.contains("confirm_mutation")));
}

#[test]
fn serve_localhost_runtime_endpoint_and_rejects_remote_bind() {
    let remote = assert_machine_pure_json(&["serve", "--bind", "0.0.0.0:3799", "--once", "--json"]);
    assert_eq!(remote["status"], "error");
    assert!(remote["error"]
        .as_str()
        .is_some_and(|message| message.contains("localhost-only")));

    let dir = tempdir().expect("temp dir");
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("free port");
    let port = listener.local_addr().expect("local addr").port();
    drop(listener);
    let bind = format!("127.0.0.1:{port}");
    let mut child = mdx_command_in(&["serve", "--bind", &bind, "--once", "--json"], dir.path())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn runtime server");

    let mut response = None;
    for _ in 0..60 {
        match std::net::TcpStream::connect(&bind) {
            Ok(mut stream) => {
                use std::io::{Read, Write};
                stream
                    .write_all(b"GET /runtime HTTP/1.1\r\nhost: localhost\r\n\r\n")
                    .expect("write request");
                let mut body = String::new();
                stream.read_to_string(&mut body).expect("read response");
                response = Some(body);
                break;
            }
            Err(_) => std::thread::sleep(std::time::Duration::from_millis(50)),
        }
    }

    if response.is_none() {
        let _ = child.kill();
    }
    let response = response.expect("runtime server should respond");
    assert!(response.contains("HTTP/1.1 200 OK"));
    assert!(response.contains("\"schema_version\": \"0.9\""));
    assert!(response.contains("\"mcp-stdio\""));

    let output = child.wait_with_output().expect("server output");
    let stderr = str::from_utf8(&output.stderr).expect("stderr utf8");
    assert!(stderr.trim().is_empty(), "stderr must be empty: {stderr}");
}

#[test]
fn scorecard_json_mode_briefs_agents_without_applying() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-scorecard-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["scorecard", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.9");
    assert!(value["scorecard_id"].as_str().is_some());
    assert_eq!(value["readiness"]["grade"], "Tier1Ready");
    assert_eq!(value["readiness"]["executable_candidates"], 1);
    assert!(value["map"]["map_id"].as_str().is_some());
    assert!(value["plan"]["plan_id"].as_str().is_some());
    assert!(
        value["recipes"]["recipes"]
            .as_array()
            .expect("recipes")
            .len()
            >= 5
    );
    assert!(value["next_commands"]
        .as_array()
        .expect("next commands")
        .iter()
        .any(|command| command
            .as_str()
            .is_some_and(|command| command.contains("evolve"))));
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
    let artifact_path = value["artifact_path"]
        .as_str()
        .expect("scorecard should persist artifact");
    assert!(std::path::Path::new(artifact_path).exists());
    let explanation =
        assert_machine_pure_json_in(&["explain", artifact_path, "--json"], dir.path());
    assert_eq!(explanation["artifact_kind"], "EvolutionScorecard");
}

#[test]
fn scorecard_json_mode_makes_high_security_review_only() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-scorecard-security-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn run_shell() -> anyhow::Result<String> {
    let output = std::process::Command::new("sh").arg("-c").arg("echo hi").output().unwrap();
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["scorecard", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["readiness"]["grade"], "ReviewOnly");
    assert_eq!(value["readiness"]["executable_candidates"], 0);
    assert!(value["readiness"]["blockers"]
        .as_array()
        .expect("blockers")
        .iter()
        .any(|blocker| blocker
            .as_str()
            .is_some_and(|blocker| blocker.contains("high-severity security"))));
    assert!(value["plan"]["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .any(|candidate| candidate["recipe"] == "SecurityBoundaryReview"
            && candidate["autonomy"]["decision"] == "ReviewOnly"));
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
}

#[test]
fn scorecard_json_mode_scopes_security_to_target() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-scorecard-scoped-security-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    std::fs::write(
        src.join("danger.rs"),
        r#"pub fn run_shell() {
    let _ = std::process::Command::new("sh").arg("-c").arg("echo hi").status();
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["scorecard", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["readiness"]["grade"], "Tier1Ready");
    assert_eq!(value["readiness"]["executable_candidates"], 1);
    assert_eq!(value["map"]["security"]["high"], 0);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
}

#[test]
fn scorecard_json_mode_rejects_missing_target() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-scorecard-missing-target-fixture"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();

    let value = assert_machine_pure_json_in(&["scorecard", "src/missing.rs", "--json"], dir.path());

    assert_eq!(value["status"], "error");
    assert_eq!(value["command"], "scorecard");
    assert!(value["error"]
        .as_str()
        .is_some_and(|message| message.contains("target does not exist")));
}

#[test]
fn recipes_json_mode_lists_evidence_gated_recipes() {
    let value = assert_machine_pure_json(&["recipes", "--json"]);

    assert_eq!(value["schema_version"], "0.9");
    let recipes = value["recipes"].as_array().expect("recipes array");
    assert!(recipes
        .iter()
        .any(|recipe| recipe["id"] == "contextual-error-hardening"
            && recipe["required_evidence"] == "Compiled"
            && recipe["executable"] == true));
    assert!(recipes
        .iter()
        .any(|recipe| recipe["id"] == "security-boundary-review" && recipe["executable"] == false));
}

#[test]
fn evidence_json_mode_profiles_files_for_agents() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-evidence-profile-fixture"
version = "0.1.0"
edition = "2021"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    std::fs::write(
        src.join("lib.rs"),
        r#"pub fn answer() -> usize {
    42
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(super::answer(), 42);
    }
}
"#,
    )
    .unwrap();

    let value = assert_machine_pure_json_in(&["evidence", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.9");
    assert_eq!(value["grade"], "Tested");
    let profiles = value["file_profiles"].as_array().expect("file profiles");
    assert_eq!(profiles.len(), 1);
    assert_eq!(profiles[0]["file"], "src/lib.rs");
    assert!(profiles[0]["function_profiles"]
        .as_array()
        .expect("function profiles")
        .iter()
        .any(|function| function["name"] == "answer"));
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
    assert!(dir.path().join(".mdx-rust/evals.json").exists());
    let artifact_pos = config
        .find("artifact_dir = \".mdx-rust\"")
        .expect("artifact_dir key should exist");
    let models_pos = config.find("[models]").expect("models table should exist");
    assert!(
        artifact_pos < models_pos,
        "artifact_dir must live at config root, not inside [models]"
    );
}

#[test]
fn improve_json_mode_reviews_hardening_without_applying() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-improve-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["improve", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.9");
    assert_eq!(value["outcome"]["status"], "Reviewed");
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
}

#[test]
fn improve_apply_json_mode_lands_validated_hardening() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-improve-apply-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();

    let value = assert_machine_pure_json_in(
        &[
            "improve",
            "src/lib.rs",
            "--apply",
            "--timeout-seconds",
            "90",
            "--json",
        ],
        dir.path(),
    );

    assert_eq!(value["schema_version"], "0.9");
    assert_eq!(value["outcome"]["status"], "Applied");
    assert_eq!(value["outcome"]["isolated_validation_passed"], true);
    assert_eq!(value["outcome"]["final_validation_passed"], true);

    let after = std::fs::read_to_string(&lib).unwrap();
    assert!(after.contains("use anyhow::Context;"));
    assert!(after.contains(".context(\"load failed instead of panicking\")?"));
    assert!(!after.contains(".unwrap()"));
}

#[test]
fn plan_json_mode_writes_refactor_plan_without_applying() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-plan-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["plan", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.9");
    assert!(value["plan_id"].as_str().is_some());
    assert_eq!(value["impact"]["patchable_hardening_changes"], 1);
    assert!(value["security"]["score"].as_u64().is_some());
    assert!(value["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .any(|candidate| candidate["evidence_context"]["grade"] == "Compiled"));
    assert!(value["candidates"]
        .as_array()
        .expect("candidates")
        .iter()
        .any(|candidate| candidate["autonomy"]["decision"] == "Allowed"));
    assert_eq!(value["autonomy"]["grade"], "Tier1Ready");
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
    let artifact_path = value["artifact_path"]
        .as_str()
        .expect("plan should persist artifact");
    assert!(std::path::Path::new(artifact_path).exists());
    let explanation =
        assert_machine_pure_json_in(&["explain", artifact_path, "--json"], dir.path());
    assert_eq!(explanation["schema_version"], "0.9");
    assert_eq!(explanation["artifact_kind"], "RefactorPlan");
    assert!(explanation["recommended_next_actions"]
        .as_array()
        .expect("next actions")
        .iter()
        .any(|action| action
            .as_str()
            .is_some_and(|action| action.contains("apply-plan"))));
}

#[test]
fn map_json_mode_profiles_repo_without_applying() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-map-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let value = assert_machine_pure_json_in(&["map", "src/lib.rs", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.9");
    assert_eq!(value["evidence"]["grade"], "Compiled");
    assert_eq!(value["evidence"]["max_autonomous_tier"], 1);
    assert!(value["security"]["score"].as_u64().is_some());
    assert!(value["quality"]["security_score"].as_u64().is_some());
    assert_eq!(value["autonomy"]["grade"], "Tier1Ready");
    assert_eq!(value["quality"]["patchable_findings"], 1);
    assert!(value["capability_gates"].as_array().unwrap().len() >= 4);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);
    let artifact_path = value["artifact_path"]
        .as_str()
        .expect("map should persist artifact");
    assert!(std::path::Path::new(artifact_path).exists());
}

#[test]
fn autopilot_json_mode_reviews_then_applies_safe_queue() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-autopilot-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    let config = src.join("config.rs");
    std::fs::write(
        &lib,
        r#"mod config;

pub fn load_root() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("root.toml").unwrap();
    Ok(format!("{}{}", content, config::load_config()?))
}
"#,
    )
    .unwrap();
    std::fs::write(
        &config,
        r#"pub fn load_config() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("config.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before_lib = std::fs::read_to_string(&lib).unwrap();
    let before_config = std::fs::read_to_string(&config).unwrap();

    let reviewed = assert_machine_pure_json_in(
        &[
            "autopilot",
            "src",
            "--max-passes",
            "2",
            "--max-candidates",
            "10",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(reviewed["schema_version"], "0.9");
    assert_eq!(reviewed["status"], "Reviewed");
    assert_eq!(reviewed["budget_exhausted"], false);
    assert_eq!(reviewed["total_executed_candidates"], 2);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before_lib);
    assert_eq!(std::fs::read_to_string(&config).unwrap(), before_config);

    let applied = assert_machine_pure_json_in(
        &[
            "autopilot",
            "src",
            "--apply",
            "--max-passes",
            "2",
            "--max-candidates",
            "10",
            "--timeout-seconds",
            "90",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(applied["status"], "Applied");
    assert_eq!(applied["total_executed_candidates"], 2);
    assert_eq!(applied["quality_before"]["patchable_findings"], 2);
    assert_eq!(
        applied["quality_after"]["patchable_findings"],
        serde_json::Value::from(0)
    );
    let after_lib = std::fs::read_to_string(&lib).unwrap();
    let after_config = std::fs::read_to_string(&config).unwrap();
    assert!(after_lib.contains("use anyhow::Context;"));
    assert!(after_config.contains("use anyhow::Context;"));
    assert!(!after_lib.contains(".unwrap()"));
    assert!(!after_config.contains(".unwrap()"));
}

#[test]
fn evolve_json_mode_respects_evidence_and_budget_surface() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-evolve-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let blocked = assert_machine_pure_json_in(
        &[
            "evolve",
            "src/lib.rs",
            "--budget",
            "60s",
            "--min-evidence",
            "tested",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(blocked["status"], "NoExecutableCandidates");
    assert_eq!(blocked["budget_seconds"], 60);
    assert_eq!(blocked["total_executed_candidates"], 0);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);

    let applied = assert_machine_pure_json_in(
        &[
            "evolve",
            "src/lib.rs",
            "--apply",
            "--budget",
            "60s",
            "--tier",
            "1",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(applied["status"], "Applied");
    assert_eq!(applied["budget_seconds"], 60);
    assert_eq!(applied["total_executed_candidates"], 1);
    let after = std::fs::read_to_string(&lib).unwrap();
    assert!(after.contains("use anyhow::Context;"));
    assert!(!after.contains(".unwrap()"));
}

#[test]
fn apply_plan_json_mode_reviews_then_applies_executable_candidate() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-apply-plan-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before = std::fs::read_to_string(&lib).unwrap();

    let plan = assert_machine_pure_json_in(&["plan", "src/lib.rs", "--json"], dir.path());
    let artifact_path = plan["artifact_path"]
        .as_str()
        .expect("plan should persist artifact");
    let candidate_id = plan["candidates"]
        .as_array()
        .expect("candidate array")
        .iter()
        .find(|candidate| candidate["status"] == "ApplyViaImprove")
        .and_then(|candidate| candidate["id"].as_str())
        .expect("executable candidate");

    let reviewed = assert_machine_pure_json_in(
        &[
            "apply-plan",
            artifact_path,
            "--candidate",
            candidate_id,
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(reviewed["schema_version"], "0.9");
    assert_eq!(reviewed["status"], "Reviewed");
    assert_eq!(reviewed["hardening_run"]["outcome"]["status"], "Reviewed");
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before);

    let applied = assert_machine_pure_json_in(
        &[
            "apply-plan",
            artifact_path,
            "--candidate",
            candidate_id,
            "--apply",
            "--timeout-seconds",
            "90",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(applied["status"], "Applied");
    assert_eq!(
        applied["hardening_run"]["outcome"]["final_validation_passed"],
        true
    );
    let after = std::fs::read_to_string(&lib).unwrap();
    assert!(after.contains("use anyhow::Context;"));
    assert!(!after.contains(".unwrap()"));
}

#[test]
fn apply_plan_json_mode_rejects_stale_plan() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-stale-plan-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();

    let plan = assert_machine_pure_json_in(&["plan", "src/lib.rs", "--json"], dir.path());
    let artifact_path = plan["artifact_path"]
        .as_str()
        .expect("plan should persist artifact");
    let candidate_id = plan["candidates"]
        .as_array()
        .expect("candidate array")
        .iter()
        .find(|candidate| candidate["status"] == "ApplyViaImprove")
        .and_then(|candidate| candidate["id"].as_str())
        .expect("executable candidate");

    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    Ok("changed after planning".to_string())
}
"#,
    )
    .unwrap();

    let stale = assert_machine_pure_json_in(
        &[
            "apply-plan",
            artifact_path,
            "--candidate",
            candidate_id,
            "--apply",
            "--json",
        ],
        dir.path(),
    );

    assert_eq!(stale["status"], "StalePlan");
    assert_eq!(stale["stale_files"].as_array().unwrap().len(), 1);
}

#[test]
fn apply_plan_json_mode_rejects_tampered_plan_hash() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-tampered-plan-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    std::fs::write(
        &lib,
        r#"pub fn load() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();

    let plan = assert_machine_pure_json_in(&["plan", "src/lib.rs", "--json"], dir.path());
    let artifact_path = plan["artifact_path"]
        .as_str()
        .expect("plan should persist artifact");
    let candidate_id = plan["candidates"]
        .as_array()
        .expect("candidate array")
        .iter()
        .find(|candidate| candidate["status"] == "ApplyViaImprove")
        .and_then(|candidate| candidate["id"].as_str())
        .expect("executable candidate");

    let mut plan_json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(artifact_path).unwrap()).unwrap();
    plan_json["candidates"][0]["title"] = serde_json::Value::String("tampered".to_string());
    std::fs::write(
        artifact_path,
        serde_json::to_string_pretty(&plan_json).unwrap(),
    )
    .unwrap();

    let rejected = assert_machine_pure_json_in(
        &[
            "apply-plan",
            artifact_path,
            "--candidate",
            candidate_id,
            "--apply",
            "--json",
        ],
        dir.path(),
    );

    assert_eq!(rejected["status"], "Rejected");
    assert!(rejected["note"]
        .as_str()
        .unwrap()
        .contains("plan hash mismatch"));
}

#[test]
fn apply_plan_all_json_mode_reviews_then_applies_executable_queue() {
    let dir = tempdir().expect("temp dir");
    std::fs::write(
        dir.path().join("Cargo.toml"),
        r#"[package]
name = "json-apply-plan-all-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
    )
    .unwrap();
    let src = dir.path().join("src");
    std::fs::create_dir_all(&src).unwrap();
    let lib = src.join("lib.rs");
    let config = src.join("config.rs");
    std::fs::write(
        &lib,
        r#"mod config;

pub fn load_root() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("root.toml").unwrap();
    Ok(format!("{}{}", content, config::load_config()?))
}
"#,
    )
    .unwrap();
    std::fs::write(
        &config,
        r#"pub fn load_config() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("config.toml").unwrap();
    Ok(content)
}
"#,
    )
    .unwrap();
    let before_lib = std::fs::read_to_string(&lib).unwrap();
    let before_config = std::fs::read_to_string(&config).unwrap();

    let plan = assert_machine_pure_json_in(&["plan", "src", "--json"], dir.path());
    let artifact_path = plan["artifact_path"]
        .as_str()
        .expect("plan should persist artifact");
    assert!(
        plan["candidates"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|candidate| candidate["status"] == "ApplyViaImprove")
            .count()
            >= 2
    );

    let reviewed = assert_machine_pure_json_in(
        &["apply-plan", artifact_path, "--all", "--json"],
        dir.path(),
    );
    assert_eq!(reviewed["schema_version"], "0.9");
    assert_eq!(reviewed["status"], "Reviewed");
    assert_eq!(reviewed["executed_candidates"], 2);
    assert_eq!(std::fs::read_to_string(&lib).unwrap(), before_lib);
    assert_eq!(std::fs::read_to_string(&config).unwrap(), before_config);

    let applied = assert_machine_pure_json_in(
        &[
            "apply-plan",
            artifact_path,
            "--all",
            "--apply",
            "--max-candidates",
            "10",
            "--timeout-seconds",
            "90",
            "--json",
        ],
        dir.path(),
    );
    assert_eq!(applied["status"], "Applied");
    assert_eq!(applied["executed_candidates"], 2);
    assert_eq!(
        applied["steps"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|step| step["status"] == "Applied")
            .count(),
        2
    );
    let after_lib = std::fs::read_to_string(&lib).unwrap();
    let after_config = std::fs::read_to_string(&config).unwrap();
    assert!(after_lib.contains("use anyhow::Context;"));
    assert!(after_config.contains("use anyhow::Context;"));
    assert!(!after_lib.contains(".unwrap()"));
    assert!(!after_config.contains(".unwrap()"));
}

#[test]
fn workspace_eval_json_mode_runs_behavior_spec() {
    let dir = tempdir().expect("temp dir");
    let eval_dir = dir.path().join(".mdx-rust");
    std::fs::create_dir_all(&eval_dir).unwrap();
    std::fs::write(
        eval_dir.join("evals.json"),
        r#"{
  "version": "v1",
  "commands": [
    {
      "id": "cargo-version",
      "command": "cargo",
      "args": ["--version"],
      "expect_success": true,
      "expect_stdout_contains": ["cargo"],
      "timeout_seconds": 30
    }
  ]
}"#,
    )
    .unwrap();

    let value = assert_machine_pure_json_in(&["eval", "--json"], dir.path());

    assert_eq!(value["schema_version"], "0.4");
    assert_eq!(value["total"], 1);
    assert_eq!(value["passed"], 1);
    assert_eq!(value["failed"], 0);
}
