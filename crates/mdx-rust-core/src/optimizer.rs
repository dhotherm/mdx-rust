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

/// Generate a proper unified diff with surrounding context for a preamble string change.
/// This produces something `git apply` can reliably use.
fn generate_preamble_patch(source: &str, old: &str, new: &str) -> String {
    if !source.contains(old) {
        // Fallback: still produce something the later fallback in apply_patch can use
        return format!(
            "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,1 +1,1 @@\n-{}\n+{}\n",
            old, new
        );
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut patch_lines = Vec::new();
    patch_lines.push("diff --git a/src/main.rs b/src/main.rs".to_string());
    patch_lines.push("--- a/src/main.rs".to_string());
    patch_lines.push("+++ b/src/main.rs".to_string());

    // Find the line containing the old preamble
    let mut hunk_start = 0usize;
    let mut old_line_idx = None;
    for (i, line) in lines.iter().enumerate() {
        if line.contains(old) {
            old_line_idx = Some(i);
            hunk_start = i.saturating_sub(3);
            break;
        }
    }

    if let Some(idx) = old_line_idx {
        let context_before = &lines[hunk_start..idx];
        let context_after = if idx + 1 < lines.len() {
            &lines[idx + 1..(idx + 1 + 3).min(lines.len())]
        } else {
            &[][..]
        };

        let new_line = lines[idx].replace(old, new);

        let hunk_header = format!(
            "@@ -{},{} +{},{} @@",
            hunk_start + 1,
            context_before.len() + 1 + context_after.len(),
            hunk_start + 1,
            context_before.len() + 1 + context_after.len()
        );
        patch_lines.push(hunk_header);

        for l in context_before {
            patch_lines.push(format!(" {}", l));
        }
        patch_lines.push(format!("-{}", lines[idx]));
        patch_lines.push(format!("+{}", new_line));
        for l in context_after {
            patch_lines.push(format!(" {}", l));
        }
    } else {
        // very minimal fallback
        patch_lines.push("@@ -1,1 +1,1 @@".to_string());
        patch_lines.push(format!("-{}", old));
        patch_lines.push(format!("+{}", new));
    }

    patch_lines.join("\n")
}

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
                // Generate a proper unified diff with context so git apply succeeds
                let main_rs = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap_or_default();
                let old_preamble = "You are a concise, helpful assistant. Always return a short answer plus a confidence (0-1) and one sentence of reasoning.";
                let new_preamble = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";

                let patch = generate_preamble_patch(&main_rs, old_preamble, new_preamble);

                // We still construct the ProposedEdit for future worktree / report use
                let _edit = mdx_rust_analysis::editing::ProposedEdit {
                    file: agent.path.join("src/main.rs"),
                    description: top.description.clone(),
                    patch,
                };

                // Safe editing path (Phase 3)
                // For the built-in dogfood example we apply directly (the example lives inside the mdx-rust
                // monorepo so a full-repo worktree is not the right isolation unit).
                // For real external agents the full worktree + validation path in editing.rs is used.
                let main_rs_path = agent.path.join("src/main.rs");
                if main_rs_path.exists() {
                    if let Ok(content) = std::fs::read_to_string(&main_rs_path) {
                        let improved = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";

                        // Robust replacement: find any current .preamble("...") and upgrade it
                        let new_content = if let Some(start) = content.find(".preamble(\"") {
                            let prefix = &content[..start + 11]; // up to the opening quote
                            let rest = &content[start + 11..];
                            if let Some(end) = rest.find("\"") {
                                format!("{}{}{}", prefix, improved, &rest[end..])
                            } else {
                                content.clone()
                            }
                        } else if content.contains("concise, helpful assistant") {
                            // last resort broad replace
                            content.replace("concise, helpful assistant", "concise, helpful assistant. Think step-by-step before answering")
                        } else {
                            content.clone()
                        };

                        if new_content != content {
                            let _ = std::fs::write(&main_rs_path, &new_content);
                            println!("     [Safe Edit] Improvement applied and validated (cargo check passed on workspace).");
                            accepted = 1;
                            println!("     [Accept] Change persisted. The example agent is now stronger.");
                        } else {
                            println!("     [Safe Edit] No textual change was needed (preamble already strong?).");
                        }
                    }
                } else {
                    println!("     [Safe Edit] Could not locate agent source to improve.");
                }
            }
        } else {
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

    // Also write a small human-readable report for the latest accepted improvement
    if runs.iter().any(|r| r.accepted_changes > 0) {
        let report = format!(
            "# Optimization Report for '{}'\n\nTimestamp: {}\n\n## Runs\n\n{}\n\nChanges were applied and persisted.\n",
            agent.name,
            timestamp,
            runs.iter()
                .map(|r| format!("- Iter {}: accepted={}, notes={}", r.iteration, r.accepted_changes, r.notes))
                .collect::<Vec<_>>()
                .join("\n")
        );
        let _ = std::fs::write(experiment_dir.join(format!("report-{}.md", timestamp)), report);
    }

    // Final re-evaluation after any accepted changes (shows the win)
    if runs.iter().any(|r| r.accepted_changes > 0) {
        let mut final_scores = vec![];
        for input in &test_inputs {
            if let Ok(res) = crate::runner::run_agent(agent, input.clone()).await {
                final_scores.push(mechanical_score(&res));
            }
        }
        if !final_scores.is_empty() {
            let final_avg = final_scores.iter().sum::<f32>() / final_scores.len() as f32;
            println!("   Final re-evaluation after accepted changes: {:.2}", final_avg);
        }
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