//! Agent execution harness with tracing support.
//!
//! This module is responsible for actually invoking registered agents
//! (either as separate processes or, later, as native Rust entrypoints)
//! while collecting rich traces for diagnosis and optimization.

use crate::registry::{AgentContract, RegisteredAgent};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tracing::{info, warn};

/// A single trace event captured during an agent run.
/// Made first-class for trace-to-patch optimization.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TraceEvent {
    pub timestamp_ms: u64,
    pub event_type: String, // "llm_call", "tool_call", "error", "decision", etc.
    pub data: serde_json::Value,
    #[serde(default)]
    pub span_id: Option<String>,
    #[serde(default)]
    pub parent_span_id: Option<String>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub token_usage: Option<serde_json::Value>, // {prompt: , completion: , total: }
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub tool_name: Option<String>,
    #[serde(default)]
    pub cost_usd: Option<f64>,
    #[serde(default)]
    pub redacted: bool,
    #[serde(default)]
    pub candidate_id: Option<String>,
}

impl TraceEvent {
    pub fn lifecycle(
        timestamp_ms: u64,
        event_type: impl Into<String>,
        span_id: impl Into<String>,
        parent_span_id: Option<String>,
        latency_ms: Option<u64>,
        data: serde_json::Value,
    ) -> Self {
        Self {
            timestamp_ms,
            event_type: event_type.into(),
            data,
            span_id: Some(span_id.into()),
            parent_span_id,
            latency_ms,
            token_usage: None,
            model: None,
            tool_name: None,
            cost_usd: None,
            redacted: false,
            candidate_id: None,
        }
    }
}

/// The result of running an agent on a single input, including traces.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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

            let run_span_id = format!("run-{}", start.elapsed().as_nanos());
            traces.push(TraceEvent::lifecycle(
                0,
                "run_started",
                run_span_id.clone(),
                None,
                None,
                serde_json::json!({
                    "agent": agent.name,
                    "contract": format!("{:?}", agent.contract)
                }),
            ));
            traces.push(TraceEvent::lifecycle(
                start.elapsed().as_millis() as u64,
                "run_completed",
                format!("{run_span_id}:completed"),
                Some(run_span_id),
                Some(result.duration_ms),
                serde_json::json!({
                    "success": result.success,
                    "duration_ms": result.duration_ms
                }),
            ));

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

    let start = Instant::now();
    const AGENT_RUN_TIMEOUT: Duration = Duration::from_secs(120);

    let manifest = agent.path.join("Cargo.toml");
    if !manifest.exists() {
        return Err(anyhow::anyhow!("Cannot find Cargo.toml for Process agent"));
    }

    let mut command = Command::new("cargo");
    command
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
        .stderr(Stdio::piped());
    configure_process_group(&mut command);

    let mut child = command.spawn()?;

    {
        let mut stdin = child
            .stdin
            .take()
            .ok_or_else(|| anyhow::anyhow!("no stdin"))?;
        stdin.write_all(serde_json::to_string(&input)?.as_bytes())?;
        stdin.write_all(b"\n")?;
    }

    let output = loop {
        match child.try_wait() {
            Ok(Some(_)) => break child.wait_with_output()?,
            Ok(None) if start.elapsed() >= AGENT_RUN_TIMEOUT => {
                terminate_process_group(child.id());
                let _ = child.kill();
                let _ = child.wait();
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
            Ok(None) => std::thread::sleep(Duration::from_millis(20)),
            Err(e) => {
                terminate_process_group(child.id());
                let _ = child.kill();
                let _ = child.wait();
                let duration = start.elapsed().as_millis() as u64;
                return Ok(AgentRunResult {
                    output: serde_json::json!({"error": format!("wait error: {}", e)}),
                    duration_ms: duration,
                    success: false,
                    error: Some(e.to_string()),
                    traces: vec![],
                });
            }
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

#[cfg(unix)]
fn configure_process_group(command: &mut std::process::Command) {
    use std::os::unix::process::CommandExt;
    command.process_group(0);
}

#[cfg(not(unix))]
fn configure_process_group(_command: &mut std::process::Command) {}

#[cfg(unix)]
fn terminate_process_group(pid: u32) {
    for signal in ["-TERM", "-KILL"] {
        let _ = std::process::Command::new("kill")
            .arg(signal)
            .arg(format!("-{pid}"))
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status();
        std::thread::sleep(Duration::from_millis(50));
    }
}

#[cfg(not(unix))]
fn terminate_process_group(_pid: u32) {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{AgentContract, RegisteredAgent};
    use tempfile::tempdir;

    #[tokio::test]
    async fn process_agent_receives_eof_after_json_input() {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"stdin-eof-agent\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        std::fs::write(
            dir.path().join("src/main.rs"),
            r#"
use std::io::Read;

fn main() {
    let mut input = String::new();
    std::io::stdin().read_to_string(&mut input).unwrap();
    assert!(!input.trim().is_empty());
    println!("{{\"answer\":\"read eof\",\"reasoning\":\"stdin closed\",\"confidence\":0.9}}");
}
"#,
        )
        .unwrap();

        let agent = RegisteredAgent {
            name: "stdin-eof-agent".to_string(),
            path: dir.path().to_path_buf(),
            contract: AgentContract::Process,
            registered_at: "test".to_string(),
        };

        let result = run_agent(&agent, serde_json::json!({"query":"hello"}))
            .await
            .unwrap();

        assert!(result.success, "{:?}", result.error);
        assert_eq!(result.output["answer"], "read eof");
    }
}
