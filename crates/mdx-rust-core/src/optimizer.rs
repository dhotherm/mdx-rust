//! The core optimization loop (Phase 3).
//!
//! High-level flow (per the approved plan):
//! 1. Run the agent on the dataset while collecting rich traces.
//! 2. Score outputs (mechanical rules + optional LLM-as-Judge).
//! 3. Diagnose failures using a strong model + policy + traces + code bundle.
//! 4. Generate N targeted candidate fixes (different focus areas).
//! 5. Validate candidates safely (cargo check + clippy + smoke tests in worktree).
//! 6. Evaluate survivors on the full dataset.
//! 7. Accept only net-positive changes with regression guards + holdout set.
//!
//! This module is currently a structural skeleton. Real implementations of
//! the individual steps will be filled in as the analysis crate and LLM
//! client mature.

use crate::registry::RegisteredAgent;
use crate::runner::AgentRunResult;
use serde::{Deserialize, Serialize};

/// Configuration for a single optimization run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizeConfig {
    pub max_iterations: u32,
    pub candidates_per_iteration: u32,
    pub use_llm_judge: bool,
}

/// A single optimization experiment / iteration result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRun {
    pub iteration: u32,
    pub scores: Vec<f32>,
    pub accepted_changes: u32,
    pub notes: String,
    pub candidates: Vec<Candidate>,
}

/// A proposed improvement generated during an optimization iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub focus: String,           // e.g. "system_prompt", "tool_descriptions", "reasoning_step"
    pub description: String,
    pub expected_improvement: String,
}

/// Placeholder for the full optimization engine.
/// In a real implementation this would orchestrate:
/// - the runner
/// - the analysis crate (for bundling + editing)
/// - an LLM client (for diagnosis + candidate generation)
/// - the safe editing/validation pipeline
pub async fn run_optimization(
    agent: &RegisteredAgent,
    config: &OptimizeConfig,
) -> anyhow::Result<Vec<OptimizationRun>> {
    let mut runs = vec![];

    // Simple synthetic dataset for the example agent
    let test_inputs: Vec<serde_json::Value> = (0..5)
        .map(|i| serde_json::json!({"query": format!("What is {} + {}?", i, i+1), "context": null}))
        .collect();

    let mut current_score = 0.55f32;

    for iteration in 0..config.max_iterations {
        let mut scores_this_iter = vec![];

        for input in &test_inputs {
            let run_result = crate::runner::run_agent(agent, input.clone()).await?;
            let score = mechanical_score(&run_result);
            scores_this_iter.push(score);
        }

        let avg_score: f32 = scores_this_iter.iter().sum::<f32>() / scores_this_iter.len() as f32;

        // Diagnosis step now uses the actual bundle scope from the analysis crate
        let bundle = mdx_rust_analysis::build_bundle_scope(&agent.path, None).ok();
        let file_count = bundle.as_ref().map(|b| b.optimizable_paths.len()).unwrap_or(0);

        // Generate concrete candidates based on diagnosis
        let candidates = if avg_score <= current_score {
            vec![
                Candidate {
                    focus: "system_prompt".to_string(),
                    description: "Strengthen the system prompt with explicit reasoning instructions and output format.".to_string(),
                    expected_improvement: "Reduce echo fallback, increase answer quality.".to_string(),
                },
                Candidate {
                    focus: "reasoning_step".to_string(),
                    description: "Add an internal 'think step' before producing the final answer.".to_string(),
                    expected_improvement: "Better calibration and less shallow responses.".to_string(),
                },
            ]
        } else {
            vec![]
        };

        let mut accepted = 0;
        let mut notes = format!("Avg score this iter: {:.2} ({} files in bundle, {} candidates)", avg_score, file_count, candidates.len());

        if !candidates.is_empty() {
            let top = &candidates[0];
            notes.push_str(&format!(" → Top candidate: {} (patch would be applied via worktree)", top.focus));

            // Simulate the editing pipeline output
            if top.focus == "system_prompt" {
                println!("     [Editing] Would create git worktree → apply patch → cargo check → accept if green.");
            }
        } else {
            current_score = avg_score;
            accepted = 1;
            notes.push_str(" → No new candidates — keeping current behavior");
        }

        runs.push(OptimizationRun {
            iteration,
            scores: scores_this_iter,
            accepted_changes: accepted,
            notes,
            candidates,
        });

        if accepted > 0 && iteration > 0 {
            // In real version we'd apply a safe edit here
        }
    }

    // Persist this optimization experiment under the agent's directory
    let experiment_dir = std::env::current_dir()?
        .join(".mdx-rust")
        .join("agents")
        .join(&agent.name)
        .join("experiments");

    std::fs::create_dir_all(&experiment_dir).ok();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    let experiment_file = experiment_dir.join(format!("run-{}.json", timestamp));
    if let Ok(content) = serde_json::to_string_pretty(&runs) {
        let _ = std::fs::write(experiment_file, content);
    }

    Ok(runs)
}

/// Very rough mechanical scorer for the example agent.
/// Gives higher score if the output is not the echo fallback.
pub fn mechanical_score(result: &AgentRunResult) -> f32 {
    if let Some(answer) = result.output.get("answer").and_then(|v| v.as_str()) {
        if answer.starts_with("Echo:") {
            return 0.45;
        } else {
            return 0.85; // "real" response
        }
    }
    0.3
}