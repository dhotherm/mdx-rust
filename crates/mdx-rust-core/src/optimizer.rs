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
use crate::{diagnose_run, EvaluationDataset, ScorerMetadata, TraceDiagnosis};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// Generate a proper unified diff with surrounding context for a preamble string change.
/// This produces something `git apply` can reliably use.
fn generate_preamble_patch(file_path: &Path, source: &str, old: &str, new: &str) -> String {
    let diff_path = file_path.to_string_lossy();

    if !source.contains(old) {
        // Fallback: still produce something the later fallback in apply_patch can use
        return format!(
            "diff --git a/{diff_path} b/{diff_path}\n--- a/{diff_path}\n+++ b/{diff_path}\n@@ -1,1 +1,1 @@\n-{old}\n+{new}\n"
        );
    }

    let lines: Vec<&str> = source.lines().collect();
    let mut patch_lines = Vec::new();
    patch_lines.push(format!("diff --git a/{diff_path} b/{diff_path}"));
    patch_lines.push(format!("--- a/{diff_path}"));
    patch_lines.push(format!("+++ b/{diff_path}"));

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
    /// Number of changes that were fully validated in isolation
    pub validated_changes: u32,
    /// Number of changes that were successfully landed on the real agent tree
    pub landed_changes: u32,
    /// Number of changes that were accepted (landed + final validation + net-positive)
    pub accepted_changes: u32,
    pub notes: String,
    pub candidates: Vec<Candidate>,
    /// Optional unified diff of the last accepted change
    #[serde(default)]
    pub diff: Option<String>,
    #[serde(default)]
    pub policy_hash: Option<String>,
    #[serde(default)]
    pub dataset_version: Option<String>,
    #[serde(default)]
    pub dataset_hash: Option<String>,
    // Net-positive evaluation (P1 stabilization)
    #[serde(default)]
    pub baseline_score: Option<f32>,
    #[serde(default)]
    pub patched_score: Option<f32>,
    #[serde(default)]
    pub score_delta: Option<f32>,

    // Real provenance (P1 requirement) — populated when a change is accepted
    #[serde(default)]
    pub git_sha_before: Option<String>,
    #[serde(default)]
    pub git_sha_after: Option<String>,
    #[serde(default)]
    pub diff_hash: Option<String>,
    #[serde(default)]
    pub working_tree_dirty_after: Option<bool>,
    #[serde(default)]
    pub scorer: Option<String>,
    #[serde(default)]
    pub validation_commands: Option<Vec<String>>,
    #[serde(default)]
    pub trace_diagnosis: Vec<TraceDiagnosis>,
}

/// A proposed improvement generated during an optimization iteration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Candidate {
    pub focus: String, // e.g. "system_prompt", "tool_descriptions", "reasoning_step"
    pub description: String,
    pub expected_improvement: String,
    #[serde(default)]
    pub strategy: Option<EditStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum EditStrategy {
    SystemPrompt,
    ToolDescription,
    FallbackLogic,
    OutputSchema,
    ModelConfig,
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

    let dataset = EvaluationDataset::synthetic_v1();
    let dataset_hash = dataset.content_hash();
    let scorer = ScorerMetadata::mechanical_v1();
    let test_inputs: Vec<serde_json::Value> = dataset
        .samples
        .iter()
        .map(|sample| sample.input.clone())
        .collect();

    // Baseline evaluation (computed once for net-positive comparison)
    let baseline_score: f32 = {
        let mut total = 0.0f32;
        for input in &test_inputs {
            if let Ok(res) = crate::runner::run_agent(agent, input.clone()).await {
                total += mechanical_score(&res);
            }
        }
        if test_inputs.is_empty() {
            0.0
        } else {
            total / test_inputs.len() as f32
        }
    };

    // Provenance: git sha before any optimization changes (P1 requirement)
    let git_sha_before: Option<String> = std::process::Command::new("git")
        .current_dir(&agent.path)
        .args(["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
            } else {
                None
            }
        });
    let policy_hash = load_policy_hash(&agent.name);

    for iteration in 0..config.max_iterations {
        let mut scores_this_iter = vec![];
        let mut accepted_patched: Option<f32> = None;
        let mut accepted_delta: Option<f32> = None;
        let mut validated = 0;
        let mut landed = 0;
        let mut trace_diagnoses = Vec::new();

        for input in &test_inputs {
            let run_result = crate::runner::run_agent(agent, input.clone()).await?;
            trace_diagnoses.push(diagnose_run(&run_result));
            let score = mechanical_score(&run_result);
            scores_this_iter.push(score);
        }

        let avg_score: f32 = if scores_this_iter.is_empty() {
            0.0
        } else {
            scores_this_iter.iter().sum::<f32>() / scores_this_iter.len() as f32
        };

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
            traces_summary: summarize_trace_diagnoses(&trace_diagnoses),
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
                let strategy = strategy_for_focus(&c.focus);
                candidates.push(Candidate {
                    focus: c.focus,
                    description: c.description,
                    expected_improvement: c.expected_improvement,
                    strategy: Some(strategy),
                });
            }
        } else {
            // Fallback
            candidates = vec![Candidate {
                focus: "system_prompt".to_string(),
                description: "Strengthen the system prompt with explicit reasoning instructions."
                    .to_string(),
                expected_improvement: "Reduce echo fallback.".to_string(),
                strategy: Some(EditStrategy::SystemPrompt),
            }];
        }

        if !candidates.is_empty() {
            let top = &candidates[0];
            notes.push_str(&format!(
                " → Top candidate: {} (attempting real worktree apply)",
                top.focus
            ));

            if top.focus == "system_prompt" || top.focus == "llm" {
                // Prefer a real file from the analysis bundle instead of hard-coded src/main.rs
                let target_file = rich_bundle
                    .as_ref()
                    .and_then(|b| {
                        b.scope.optimizable_paths.iter().find(|p| {
                            let name = p.file_name().unwrap_or_default().to_string_lossy();
                            name.ends_with(".rs") && (name == "main.rs" || name.contains("agent"))
                        })
                    })
                    .cloned()
                    .unwrap_or_else(|| agent.path.join("src/main.rs"));

                let content = std::fs::read_to_string(&target_file).unwrap_or_default();
                let _before_snapshot = content.clone();

                // Drive old preamble from actual analysis when available (less demo-specific)
                let old_preamble = rich_bundle
                    .as_ref()
                    .and_then(|b| b.preambles.first())
                    .map(|p| p.text.clone())
                    .unwrap_or_else(|| {
                        "You are a concise, helpful assistant. Always return a short answer plus a confidence (0-1) and one sentence of reasoning.".to_string()
                    });

                let new_preamble = "You are a concise, helpful assistant. Think step-by-step before answering. Always explain your reasoning in one sentence, then give the final answer.";

                let relative_target = target_file
                    .strip_prefix(&agent.path)
                    .unwrap_or(&target_file)
                    .to_path_buf();
                let patch = generate_preamble_patch(
                    &relative_target,
                    &content,
                    &old_preamble,
                    new_preamble,
                );

                let edit = mdx_rust_analysis::editing::ProposedEdit {
                    file: target_file,
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
                        validated = 1;
                        if !config.quiet {
                            println!("     [Safe Apply] Edit validated in isolated workspace (cargo check + clippy OK).");
                        }

                        // === Net-positive evaluation gate (P1 Codex requirement) ===
                        // Apply the candidate in a fresh isolated workspace and score it.
                        let patched_score: f32 = {
                            let score_name = format!("score-{}", iteration);
                            match mdx_rust_analysis::editing::create_isolated_workspace(
                                &agent.path,
                                &score_name,
                            ) {
                                Ok(isolated) => {
                                    let s = if mdx_rust_analysis::editing::apply_edit(
                                        &agent.path,
                                        &isolated,
                                        &edit,
                                    )
                                    .is_ok()
                                    {
                                        evaluate_workspace(&isolated, &test_inputs)
                                            .await
                                            .unwrap_or(baseline_score)
                                    } else {
                                        baseline_score
                                    };
                                    mdx_rust_analysis::editing::cleanup_isolated_workspace(
                                        &agent.path,
                                        &isolated,
                                    );
                                    s
                                }
                                Err(_) => baseline_score,
                            }
                        };

                        let delta = patched_score - baseline_score;
                        let passes_net_positive = delta > 0.0;

                        if !passes_net_positive {
                            if !config.quiet {
                                println!(
                                    "     [Net-Negative] Patched score {:.2} vs baseline {:.2} (delta {:.2}) — change rejected.",
                                    patched_score, baseline_score, delta
                                );
                            }
                            notes.push_str(&format!(
                                " (net-negative {:.2}→{:.2})",
                                baseline_score, patched_score
                            ));
                        } else if !config.review_before_apply {
                            // Full safety pipeline: land + final validation only for net-positive changes
                            let snapshot = mdx_rust_analysis::editing::snapshot_file(&edit.file)?;

                            if let Err(e) =
                                mdx_rust_analysis::editing::apply_edit_to_agent(&agent.path, &edit)
                            {
                                if !config.quiet {
                                    println!("     [Land Failed] Could not apply validated patch to real source: {}", e);
                                }
                            } else {
                                landed = 1;
                                let (final_ok, _final_output) =
                                    mdx_rust_analysis::editing::validate_build(&agent.path);

                                if final_ok {
                                    accepted = 1;
                                    accepted_diff = Some(edit.patch.clone());
                                    accepted_patched = Some(patched_score);
                                    accepted_delta = Some(delta);

                                    if !config.quiet {
                                        println!(
                                            "     [Accepted] Landed + final validation OK (score {:.2} → {:.2}, Δ{:.2}).",
                                            baseline_score, patched_score, delta
                                        );
                                    }
                                } else {
                                    let _ = mdx_rust_analysis::editing::restore_file(&snapshot);
                                    let _ = mdx_rust_analysis::editing::validate_build(&agent.path);
                                    landed = 0;
                                    if !config.quiet {
                                        println!("     [Final Validation Failed] Change rolled back after re-validation failed.");
                                    }
                                }
                            }
                        } else {
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
            accepted = 0; // No change was proposed or needed
            notes.push_str(" → No new candidates — current behavior is good (no change applied)");
        }

        let (run_baseline, run_patched, run_delta) = if accepted > 0 {
            (Some(baseline_score), accepted_patched, accepted_delta)
        } else {
            (None, None, None)
        };

        // Populate real provenance when we accepted a change (P1)
        let (prov_before, prov_after, prov_diff_hash, prov_dirty, prov_scorer, prov_cmds) =
            if accepted > 0 {
                let after = std::process::Command::new("git")
                    .current_dir(&agent.path)
                    .args(["rev-parse", "--short", "HEAD"])
                    .output()
                    .ok()
                    .and_then(|o| {
                        if o.status.success() {
                            Some(String::from_utf8_lossy(&o.stdout).trim().to_string())
                        } else {
                            None
                        }
                    });
                let dirty_after = std::process::Command::new("git")
                    .current_dir(&agent.path)
                    .args(["status", "--porcelain"])
                    .output()
                    .ok()
                    .filter(|output| output.status.success())
                    .map(|output| !output.stdout.is_empty());

                (
                    git_sha_before.clone(),
                    after,
                    accepted_diff
                        .as_ref()
                        .map(|diff| stable_hash_hex(diff.as_bytes())),
                    dirty_after,
                    Some(scorer.label()),
                    Some(vec![
                        "cargo check (isolated)".to_string(),
                        "cargo clippy -D warnings (isolated)".to_string(),
                        "final validate_build after land (real tree)".to_string(),
                    ]),
                )
            } else {
                (None, None, None, None, None, None)
            };

        runs.push(OptimizationRun {
            iteration,
            scores: scores_this_iter,
            validated_changes: validated,
            landed_changes: landed,
            accepted_changes: accepted,
            notes,
            candidates,
            diff: accepted_diff,
            policy_hash: policy_hash.clone(),
            dataset_version: Some(dataset.version.clone()),
            dataset_hash: Some(dataset_hash.clone()),
            baseline_score: run_baseline,
            patched_score: run_patched,
            score_delta: run_delta,
            git_sha_before: prov_before,
            git_sha_after: prov_after,
            diff_hash: prov_diff_hash,
            working_tree_dirty_after: prov_dirty,
            scorer: prov_scorer,
            validation_commands: prov_cmds,
            trace_diagnosis: trace_diagnoses,
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

                if let Some(h) = &run.policy_hash {
                    report.push_str(&format!("  Policy hash: {}\n", h));
                }
                if let Some(v) = &run.dataset_version {
                    report.push_str(&format!("  Dataset version: {}\n", v));
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

fn load_policy_hash(agent_name: &str) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let candidates = [
        cwd.join(".mdx-rust")
            .join("agents")
            .join(agent_name)
            .join("policies.md"),
        cwd.join(".mdx-rust").join("policies.md"),
    ];

    candidates
        .iter()
        .find_map(|path| std::fs::read(path).ok())
        .map(|content| stable_hash_hex(&content))
}

fn stable_hash_hex(bytes: &[u8]) -> String {
    crate::eval::stable_hash_hex(bytes)
}

fn strategy_for_focus(focus: &str) -> EditStrategy {
    let normalized = focus.to_lowercase();

    if normalized.contains("tool") {
        EditStrategy::ToolDescription
    } else if normalized.contains("fallback") || normalized.contains("logic") {
        EditStrategy::FallbackLogic
    } else if normalized.contains("schema") || normalized.contains("output") {
        EditStrategy::OutputSchema
    } else if normalized.contains("model") || normalized.contains("temperature") {
        EditStrategy::ModelConfig
    } else {
        EditStrategy::SystemPrompt
    }
}

fn summarize_trace_diagnoses(diagnoses: &[TraceDiagnosis]) -> String {
    let mut summaries = Vec::new();

    for diagnosis in diagnoses {
        if diagnosis.has_failures() {
            summaries.push(diagnosis.compact_summary());
        }
    }

    if summaries.is_empty() {
        "No obvious trace failures detected.".to_string()
    } else {
        format!("Trace failures: {}", summaries.join(" | "))
    }
}

/// Evaluate an arbitrary workspace dir (used for isolated patched scoring).
async fn evaluate_workspace(
    dir: &std::path::Path,
    inputs: &[serde_json::Value],
) -> anyhow::Result<f32> {
    use crate::registry::{AgentContract, RegisteredAgent};

    let temp_agent = RegisteredAgent {
        name: "isolated-eval".to_string(),
        path: dir.to_path_buf(),
        contract: AgentContract::Process,
        registered_at: "".to_string(),
    };

    let mut scores = vec![];
    for input in inputs {
        let res = crate::runner::run_agent(&temp_agent, input.clone()).await?;
        scores.push(mechanical_score(&res));
    }
    if scores.is_empty() {
        return Ok(0.0);
    }
    Ok(scores.iter().sum::<f32>() / scores.len() as f32)
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
            quiet: false,
        };
        assert_eq!(cfg.max_iterations, 1);
    }
}
