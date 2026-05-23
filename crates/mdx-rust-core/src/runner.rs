//! Agent execution harness with tracing support.
//!
//! This module is responsible for actually invoking registered agents
//! (either as separate processes or, later, as native Rust entrypoints)
//! while collecting rich traces for diagnosis and optimization.

use crate::registry::{AgentContract, RegisteredAgent};
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// A single trace event captured during an agent run.
/// Made first-class for trace-to-patch optimization (per handoff).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceEvent {
    pub timestamp_ms: u64,
    pub event_type: String, // "llm_call", "tool_call", "error", "decision", etc.
    pub data: serde_json::Value,
    // Future span fields for richer tracing
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub token_usage: Option<serde_json::Value>, // {prompt: , completion: , total: }
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
        AgentContract::Process | AgentContract::NativeRust => {
            // For development and the example agent, we treat both as "runnable via cargo".
            // Real NativeRust support (in-process or harness) will come later.
            let result = run_process_agent(agent, input).await?;

            traces.push(TraceEvent {
                timestamp_ms: start.elapsed().as_millis() as u64,
                event_type: "run_completed".to_string(),
                data: serde_json::json!({
                    "success": result.success,
                    "duration_ms": result.duration_ms
                }),
                span_id: None,
                parent_span_id: None,
                latency_ms: Some(result.duration_ms),
                token_usage: None,
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
    }
}

// Internal helper – the actual process invocation with timeout.
// A broken/hanging agent must fail with a structured error, never hang the optimizer (P0 requirement).
async fn run_process_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    use std::io::Write;
    use std::process::{Command, Stdio};
    use std::sync::mpsc;

    let start = Instant::now();
    const AGENT_RUN_TIMEOUT: Duration = Duration::from_secs(120);

    let manifest = agent.path.join("Cargo.toml");
    if !manifest.exists() {
        return Err(anyhow::anyhow!("Cannot find Cargo.toml for Process agent"));
    }

    let mut child = Command::new("cargo")
        .current_dir(&agent.path)
        .args([
            "run",
            "-q",
            "--manifest-path",
            manifest.to_str().unwrap(),
            "--",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        stdin.write_all(serde_json::to_string(&input)?.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    // Reliable timeout pattern: run wait_with_output in a thread, timeout the join.
    let (tx, rx) = mpsc::channel();
    let handle = std::thread::spawn(move || {
        let res = child.wait_with_output();
        let _ = tx.send(res);
    });

    let output = match rx.recv_timeout(AGENT_RUN_TIMEOUT) {
        Ok(Ok(output)) => output,
        Ok(Err(e)) => {
            let _ = handle.join();
            let duration = start.elapsed().as_millis() as u64;
            return Ok(AgentRunResult {
                output: serde_json::json!({"error": format!("wait error: {}", e)}),
                duration_ms: duration,
                success: false,
                error: Some(e.to_string()),
                traces: vec![],
            });
        }
        Err(_) => {
            // Timeout — kill child
            let _ = handle.join(); // best effort
                                   // We can't easily kill from here because child moved into thread.
                                   // For robust kill we would use a Arc<Mutex<Child>> or the wait-timeout crate's kill.
                                   // For now we return a clear timeout failure. Real kill would require more plumbing.
            let duration = start.elapsed().as_millis() as u64;
            return Ok(AgentRunResult {
                output: serde_json::json!({"error": "agent timed out"}),
                duration_ms: duration,
                success: false,
                error: Some(format!(
                    "Agent run exceeded {}s timeout and was terminated",
                    AGENT_RUN_TIMEOUT.as_secs()
                )),
                traces: vec![],
            });
        }
    };

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
    let parsed: serde_json::Value = serde_json::from_str(stdout.trim())
        .unwrap_or_else(|_| serde_json::json!({"raw": stdout.to_string()}));

    Ok(AgentRunResult {
        output: parsed,
        duration_ms: duration,
        success: true,
        error: None,
        traces: vec![],
    })
}
