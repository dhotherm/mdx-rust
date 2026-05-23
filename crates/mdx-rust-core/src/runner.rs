//! Agent execution harness with tracing support.
//!
//! This module is responsible for actually invoking registered agents
//! (either as separate processes or, later, as native Rust entrypoints)
//! while collecting rich traces for diagnosis and optimization.

use crate::registry::{AgentContract, RegisteredAgent};
use serde::{Deserialize, Serialize};
use std::time::Instant;
use tracing::{info, warn};

/// A single trace event captured during an agent run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp_ms: u64,
    pub event_type: String, // "llm_call", "tool_call", "error", "decision", etc.
    pub data: serde_json::Value,
}

/// The result of running an agent on a single input, including traces.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentRunResult {
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
    pub traces: Vec<TraceEvent>,
}

/// Run a registered agent with the given input.
/// Currently supports Process contracts (spawns the agent binary and pipes JSON).
/// NativeRust support will be added when we generate harnesses.
pub async fn run_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    let start = Instant::now();
    let mut traces = vec![];

    info!(agent = %agent.name, "starting agent run");

    match agent.contract {
        AgentContract::Process => {
            // For now we re-use the simple process runner logic.
            // In a real implementation we would stream output and emit
            // fine-grained trace events (LLM calls, tool calls, etc.).
            let result = run_process_agent(agent, input).await?;

            traces.push(TraceEvent {
                timestamp_ms: start.elapsed().as_millis() as u64,
                event_type: "run_completed".to_string(),
                data: serde_json::json!({
                    "success": result.success,
                    "duration_ms": result.duration_ms
                }),
            });

            if !result.success {
                warn!(agent = %agent.name, error = ?result.error, "agent run failed");
            }

            Ok(AgentRunResult {
                output: result.output,
                duration_ms: result.duration_ms,
                success: result.success,
                error: result.error,
                traces,
            })
        }
        AgentContract::NativeRust => {
            // Placeholder – we will generate a small harness binary during
            // register that links against the agent's crate and exposes the
            // run_agent function over stdin/stdout.
            Err(anyhow::anyhow!(
                "NativeRust contract execution not yet implemented (will use generated harness)"
            ))
        }
    }
}

// Internal helper – the actual process invocation.
// In the future this will parse stdout for structured trace events.
async fn run_process_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    use std::io::Write;
    use std::process::{Command, Stdio};

    let start = Instant::now();

    let manifest = agent.path.join("Cargo.toml");
    if !manifest.exists() {
        return Err(anyhow::anyhow!("Cannot find Cargo.toml for Process agent"));
    }

    let mut child = Command::new("cargo")
        .current_dir(&agent.path)
        .args(["run", "-q", "--manifest-path", manifest.to_str().unwrap(), "--"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        stdin.write_all(serde_json::to_string(&input)?.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    let output = child.wait_with_output()?;
    let duration = start.elapsed().as_millis() as u64;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        return Ok(AgentRunResult {
            output: serde_json::json!({"error": stderr}),
            duration_ms: duration,
            success: false,
            error: Some(stderr),
            traces: vec![],
        });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value =
        serde_json::from_str(stdout.trim()).unwrap_or_else(|_| serde_json::json!({"raw": stdout.to_string()}));

    Ok(AgentRunResult {
        output: parsed,
        duration_ms: duration,
        success: true,
        error: None,
        traces: vec![],
    })
}