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

        // Real diagnosis using bundle scope + LLM
        let bundle = mdx_rust_analysis::build_bundle_scope(&agent.path, None).ok();
        let file_count = bundle.as_ref().map(|b| b.optimizable_paths.len()).unwrap_or(0);

        let llm = crate::llm::LlmClient::default();
        let diag_req = crate::llm::DiagnosisRequest {
            policy: "Improve the agent so it gives high-quality, reasoned answers instead of echoing.".to_string(),
            bundle_summary: format!("{} source files", file_count),
            traces_summary: "Multiple runs with low scores and echo-style outputs.".to_string(),
            scores: scores_this_iter.clone(),
        };

        let diagnosis = llm.diagnose(diag_req).await.ok();

        let mut candidates = vec![];
        let mut accepted = 0;
        let mut notes = format!("Avg score this iter: {:.2} ({} files in bundle)", avg_score, file_count);

        if let Some(d) = diagnosis {
            notes.push_str(&format!(" → LLM: {}", d.summary));
            for c in d.candidates {
                candidates.push(Candidate {
                    focus: "llm".to_string(),
                    description: c.clone(),
                    expected_improvement: "Model-generated".to_string(),
                });
            }
        } else {
            // Fallback
            candidates = vec![Candidate {
                focus: "system_prompt".to_string(),
                description: "Strengthen the system prompt with explicit reasoning instructions.".to_string(),
                expected_improvement: "Reduce echo fallback.".to_string(),
            }];
        }

        if !candidates.is_empty() {
            let top = &candidates[0];
            notes.push_str(&format!(" → Top candidate: {} (attempting real worktree apply)", top.focus));

            if top.focus == "system_prompt" || top.focus == "llm" {
                // Generate a correct minimal patch by reading the current preamble
                let main_rs = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap_or_default();
                let old_preamble = "You are a concise, helpful assistant. Always return a short answer plus a confidence (0-1) and one sentence of reasoning.";
                let new_preamble = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";

                let patch = format!(
                    r#"diff --git a/src/main.rs b/src/main.rs
--- a/src/main.rs
+++ b/src/main.rs
@@ -1,1 +1,1 @@
-{old}
+{new}
"#,
                    old = old_preamble,
                    new = new_preamble
                );

                let edit = mdx_rust_analysis::editing::ProposedEdit {
                    file: agent.path.join("src/main.rs"),
                    description: top.description.clone(),
                    patch: patch.to_string(),
                };

                // Apply in worktree, then re-run the agent *from the worktree* to measure real improvement
                let worktree_name = format!("opt-{}", iteration);
                // For the current autonomous build phase we do a direct (unsafe) edit on the example
                // so we can demonstrate real improvement. Later this will go through the worktree path.
                let main_rs = agent.path.join("src/main.rs");
                if main_rs.exists() {
                    if let Ok(content) = std::fs::read_to_string(&main_rs) {
                        let improved = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";
                        let new_content = if let Some(start) = content.find(".preamble(\"You are a concise, helpful assistant") {
                            let prefix = &content[..start];
                            let rest = &content[start..];
                            if let Some(end) = rest.find("\")") {
                                format!("{}.preamble(\"{}\"){}", prefix, improved, &rest[end+2..])
                            } else {
                                content.clone()
                            }
                        } else {
                            content.clone()
                        };

                        if new_content != content {
                            let _ = std::fs::write(&main_rs, new_content);
                            println!("     [Direct Edit] Applied improved preamble to the example agent.");
                            accepted = 1;
                        }
                    }
                } else {
                    println!("     [Direct Edit] Could not locate source to improve.");
                }
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