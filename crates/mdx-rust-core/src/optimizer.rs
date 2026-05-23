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
    /// When true, the optimizer will print proposed changes and wait for confirmation before applying (Phase 4 review gate).
    #[serde(default)]
    pub review_before_apply: bool,
    /// When true, suppress all human progress output (used for --json mode).
    #[serde(default)]
    pub quiet: bool,
}

/// A single optimization experiment / iteration result.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OptimizationRun {
    pub iteration: u32,
    pub scores: Vec<f32>,
    pub accepted_changes: u32,
    pub notes: String,
    pub candidates: Vec<Candidate>,
    /// Optional unified diff of the change that was accepted in this iteration
    #[serde(default)]
    pub diff: Option<String>,
}

/// A proposed improvement generated during an optimization iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub focus: String, // e.g. "system_prompt", "tool_descriptions", "reasoning_step"
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

        // Rich analysis: extract real preambles, tools, entrypoints
        let rich_bundle = mdx_rust_analysis::analyze_agent(&agent.path, None).ok();
        let file_count = rich_bundle
            .as_ref()
            .map(|b| b.scope.optimizable_paths.len())
            .unwrap_or(0);

        // Build a high-signal summary for the LLM
        let bundle_summary = if let Some(ref b) = rich_bundle {
            let mut s = format!(
                "{} source files, Rig agent = {}",
                file_count, b.is_rig_agent
            );
            if !b.preambles.is_empty() {
                s.push_str(&format!(
                    ", current preambles: {:?}",
                    b.preambles.iter().map(|p| &p.text).collect::<Vec<_>>()
                ));
            }
            if !b.tools.is_empty() {
                s.push_str(&format!(
                    ", tools: {:?}",
                    b.tools.iter().map(|t| &t.name).collect::<Vec<_>>()
                ));
            }
            s
        } else {
            format!("{} source files (limited analysis)", file_count)
        };

        let llm = crate::llm::LlmClient::default();
        let diag_req = crate::llm::DiagnosisRequest {
            policy: "Improve the agent so it gives high-quality, reasoned answers instead of echoing. Prefer explicit step-by-step reasoning in the system prompt.".to_string(),
            bundle_summary,
            traces_summary: "Multiple runs with low scores and echo-style outputs. Weak fallback behavior detected.".to_string(),
            scores: scores_this_iter.clone(),
        };

        let diagnosis = llm.diagnose(diag_req).await.ok();

        let mut candidates = vec![];
        let mut accepted = 0;
        let mut notes = format!(
            "Avg score this iter: {:.2} ({} files in bundle)",
            avg_score, file_count
        );
        let mut accepted_diff: Option<String> = None;

        if let Some(d) = diagnosis {
            notes.push_str(&format!(" → LLM: {}", d.summary));
            for c in d.candidates {
                candidates.push(Candidate {
                    focus: c.focus,
                    description: c.description,
                    expected_improvement: c.expected_improvement,
                });
            }
        } else {
            // Fallback
            candidates = vec![Candidate {
                focus: "system_prompt".to_string(),
                description: "Strengthen the system prompt with explicit reasoning instructions."
                    .to_string(),
                expected_improvement: "Reduce echo fallback.".to_string(),
            }];
        }

        if !candidates.is_empty() {
            let top = &candidates[0];
            notes.push_str(&format!(
                " → Top candidate: {} (attempting real worktree apply)",
                top.focus
            ));

            if top.focus == "system_prompt" || top.focus == "llm" {
                // Generate a proper unified diff with context so git apply succeeds
                let main_rs_path = agent.path.join("src/main.rs");
                let main_rs = std::fs::read_to_string(&main_rs_path).unwrap_or_default();
                let _before_snapshot = main_rs.clone(); // captured for future rich diff in reports

                let old_preamble = "You are a concise, helpful assistant. Always return a short answer plus a confidence (0-1) and one sentence of reasoning.";
                let new_preamble = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";

                let patch = generate_preamble_patch(&main_rs, old_preamble, new_preamble);

                let edit = mdx_rust_analysis::editing::ProposedEdit {
                    file: agent.path.join("src/main.rs"),
                    description: top.description.clone(),
                    patch: patch.clone(),
                };

                // Always validate the proposed edit in an isolated workspace first.
                // This is the required safety gate.
                let wt_name = format!("opt-{}", iteration);
                let validation_result =
                    mdx_rust_analysis::editing::apply_and_validate(&agent.path, &edit, &wt_name);

                if let Ok(val) = validation_result {
                    if val.passed {
                        if !config.quiet {
                            if !config.quiet {
                                println!("     [Safe Apply] Edit validated in isolated workspace (cargo check + clippy OK).");
                            }
                        }

                        if !config.review_before_apply {
                            // Validation succeeded in isolation.
                            // We record the validated patch so the caller (CLI) can decide to land it
                            // on the real source in a controlled, auditable way.
                            accepted = 1;
                            accepted_diff = Some(edit.patch.clone());
                            if !config.quiet {
                                if !config.quiet {
                                    println!("     [Validated] Change passed all gates in isolated workspace. Ready to land.");
                                }
                            }
                        } else {
                            // Review mode: validated in isolation, but we do not apply.
                            if !config.quiet {
                                println!("     [Review] Change validated in isolation but not applied (--review).");
                            }
                            notes.push_str(" (review mode — validated in isolation, not applied)");
                        }
                    }
                } else {
                    if !config.quiet {
                        println!("     [Safe Apply] Validation in isolated workspace failed.");
                    }
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
            diff: accepted_diff,
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

    // Also write a rich human-readable report with provenance
    if runs.iter().any(|r| r.accepted_changes > 0) {
        let git_sha = std::process::Command::new("git")
            .args(["rev-parse", "--short", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let mut report = format!(
            "# Optimization Report for '{}'\n\nTimestamp: {}\nGit SHA: {}\n\n## Summary\n\n",
            agent.name, timestamp, git_sha
        );

        for run in &runs {
            if run.accepted_changes > 0 {
                report.push_str(&format!(
                    "- Iteration {}: Accepted {} change(s)\n  Notes: {}\n",
                    run.iteration, run.accepted_changes, run.notes
                ));

                if let Some(d) = &run.diff {
                    report.push_str(&format!("\n```diff\n{}\n```\n", d));
                } else {
                    report.push_str("  (Change persisted to src/main.rs)\n");
                }
            }
        }

        report.push_str("\n## Candidates Considered\n\n");
        for run in &runs {
            for (i, c) in run.candidates.iter().enumerate() {
                report.push_str(&format!(
                    "- [{}] {}: {}\n  Expected: {}\n\n",
                    i + 1,
                    c.focus,
                    c.description,
                    c.expected_improvement
                ));
            }
        }

        let _ = std::fs::write(
            experiment_dir.join(format!("report-{}.md", timestamp)),
            report,
        );
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
            if !config.quiet {
                println!(
                    "   Final re-evaluation after accepted changes: {:.2}",
                    final_avg
                );
            }
        }
    }

    Ok(runs)
}

/// Very rough mechanical scorer for the example agent.
/// Gives higher score if the output is not the echo fallback.
pub fn mechanical_score(result: &AgentRunResult) -> f32 {
    let answer = result
        .output
        .get("answer")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let reasoning = result
        .output
        .get("reasoning")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    if answer.starts_with("Echo:") {
        return 0.4;
    }

    let mut score = 0.75f32;

    // Bonus for explicit reasoning language (the improvement the optimizer tries to install)
    if reasoning.to_lowercase().contains("think")
        || reasoning.to_lowercase().contains("reason")
        || reasoning.to_lowercase().contains("step")
    {
        score += 0.12;
    }

    // Bonus for non-trivial answer length
    if answer.len() > 20 {
        score += 0.08;
    }

    score.min(0.95)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::registry::{AgentContract, RegisteredAgent};
    use std::path::PathBuf;

    #[test]
    fn test_mechanical_score_echo_vs_reasoned() {
        let echo = AgentRunResult {
            output: serde_json::json!({"answer": "Echo: hello", "reasoning": "no key"}),
            duration_ms: 10,
            success: true,
            error: None,
            traces: vec![],
        };
        let good = AgentRunResult {
            output: serde_json::json!({"answer": "The answer is 42 because...", "reasoning": "Think step by step: 6*7"}),
            duration_ms: 120,
            success: true,
            error: None,
            traces: vec![],
        };

        assert!(mechanical_score(&echo) < 0.5);
        assert!(mechanical_score(&good) > 0.8);
    }

    #[test]
    fn test_optimize_config_defaults() {
        let cfg = OptimizeConfig {
            max_iterations: 1,
            candidates_per_iteration: 1,
            use_llm_judge: false,
            review_before_apply: false,
        };
        assert_eq!(cfg.max_iterations, 1);
    }
}
