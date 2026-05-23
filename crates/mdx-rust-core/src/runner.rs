//! Basic agent execution harness.
//!
//! This will evolve into the full tracing + invocation layer.

use crate::registry::{AgentContract, RegisteredAgent};
use std::path::Path;

/// Result of running an agent on one input.
#[derive(Debug, Clone)]
pub struct AgentRunResult {
    pub output: serde_json::Value,
    pub duration_ms: u64,
    pub success: bool,
    pub error: Option<String>,
}

/// Very early stub for running a registered agent.
/// In later phases this will:
/// - Support NativeRust (via harness or dynamic loading)
/// - Support Process (stdin/stdout JSON)
/// - Collect rich traces
pub async fn run_agent(
    agent: &RegisteredAgent,
    input: serde_json::Value,
) -> anyhow::Result<AgentRunResult> {
    match agent.contract {
        AgentContract::Process => {
            // For now, just call the binary if it exists, or error.
            // Full implementation coming in Phase 1.
            Err(anyhow::anyhow!(
                "Process contract execution not yet implemented (Phase 1)"
            ))
        }
        AgentContract::NativeRust => {
            // For Rig agents, we will eventually compile a thin harness
            // or use lib loading. Stub for now.
            Err(anyhow::anyhow!(
                "NativeRust contract execution not yet implemented (Phase 1)"
            ))
        }
    }
}