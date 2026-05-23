//! Basic agent execution harness.
//!
//! Phase 1 focus: Support Process contracts reliably (stdin/stdout JSON).

use crate::registry::{AgentContract, RegisteredAgent};
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

/// Result of running an agent on one input.
#[derive(Debug, Clone, serde::Serialize)]
pub struct AgentRunResult {
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Run a registered agent with the given input.
/// Currently supports Process contracts by spawning a command and piping JSON.
pub async fn run_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    match agent.contract {
        AgentContract::Process => run_process_agent(agent, input).await,
        AgentContract::NativeRust => {
            // For now fall back to trying to run via cargo if it's a directory
            // (we'll improve NativeRust support significantly in later phases)
            run_process_agent(agent, input).await
        }
    }
}

async fn run_process_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    tracing::info!(agent = %agent.name, "starting agent run");
    let start = Instant::now();

    let manifest = agent.path.join("Cargo.toml").canonicalize()?;
    if !manifest.exists() {
        return Err(anyhow::anyhow!("Cannot find Cargo.toml for Process agent"));
    }

    let agent_dir = manifest.parent().unwrap().to_path_buf();

    let mut cmd = Command::new("cargo");
    cmd.current_dir(&agent_dir)
        .args(["run", "-q", "--manifest-path", manifest.to_str().unwrap(), "--"]);

    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| anyhow::anyhow!("Failed to open stdin"))?;
        stdin.write_all(serde_json::to_string(&input)?.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    let output = child.wait_with_output()?;
    let duration = start.elapsed().as_millis() as u64;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        tracing::warn!(agent = %agent.name, error = %stderr, "agent run failed");
        return Ok(AgentRunResult {
            output: serde_json::json!({"error": stderr.to_string()}),
            duration_ms: duration,
            success: false,
            error: Some(stderr.to_string()),
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim()).unwrap_or_else(|_| serde_json::json!({"raw": stdout.to_string()}));

    tracing::info!(agent = %agent.name, duration_ms = duration, success = true, "agent run completed");

    Ok(AgentRunResult {
        output: parsed,
        duration_ms: duration,
        success: true,
        error: None,
    })
}