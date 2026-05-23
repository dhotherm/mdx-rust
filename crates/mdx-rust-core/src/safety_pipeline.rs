//! Candidate safety pipeline.
//!
//! This module owns the acceptance-critical path:
//! hook checks, isolated validation, patched scoring, final landing,
//! final validation, and rollback.

use crate::hooks::{evaluate_builtin_hook, HookContext, HookDecision, HookPolicy, HookStage};
use crate::registry::{AgentContract, RegisteredAgent};
use crate::runner::AgentRunResult;
use mdx_rust_analysis::editing::ProposedEdit;

#[derive(Debug, Clone, Copy)]
pub struct CandidateExecutionConfig<'a> {
    pub hook_policy: &'a HookPolicy,
    pub review_before_apply: bool,
    pub quiet: bool,
}

#[derive(Debug, Clone)]
pub struct CandidateExecutionOutcome {
    pub validated: u32,
    pub landed: u32,
    pub accepted: u32,
    pub accepted_diff: Option<String>,
    pub patched_score: Option<f32>,
    pub holdout_score: Option<f32>,
    pub delta: Option<f32>,
    pub note: String,
    pub hook_decisions: Vec<HookDecision>,
}

impl CandidateExecutionOutcome {
    fn empty(note: impl Into<String>, hook_decisions: Vec<HookDecision>) -> Self {
        Self {
            validated: 0,
            landed: 0,
            accepted: 0,
            accepted_diff: None,
            patched_score: None,
            holdout_score: None,
            delta: None,
            note: note.into(),
            hook_decisions,
        }
    }
}

pub struct CandidateExecutionContext<'a> {
    pub agent: &'a RegisteredAgent,
    pub config: CandidateExecutionConfig<'a>,
    pub iteration: u32,
    pub candidate_index: usize,
    pub edit: &'a ProposedEdit,
    pub test_inputs: &'a [serde_json::Value],
    pub holdout_inputs: &'a [serde_json::Value],
    pub baseline_score: f32,
    pub scorer: fn(&AgentRunResult) -> f32,
}

pub async fn execute_candidate_edit(
    context: CandidateExecutionContext<'_>,
) -> CandidateExecutionOutcome {
    let agent = context.agent;
    let edit = context.edit;
    let mut hook_decisions = Vec::new();

    let pre_edit = evaluate_builtin_hook(
        context.config.hook_policy,
        &HookContext {
            stage: HookStage::PreEdit,
            agent_name: agent.name.clone(),
            edit_description: Some(edit.description.clone()),
            patch_bytes: edit.patch.len(),
            command: None,
            validation_passed: None,
            score_delta: None,
        },
    );
    let denied = pre_edit.denied();
    hook_decisions.push(pre_edit);
    if denied {
        return CandidateExecutionOutcome::empty(
            " (pre-edit hook denied candidate)",
            hook_decisions,
        );
    }

    let pre_command = evaluate_builtin_hook(
        context.config.hook_policy,
        &HookContext {
            stage: HookStage::PreCommand,
            agent_name: agent.name.clone(),
            edit_description: Some(edit.description.clone()),
            patch_bytes: edit.patch.len(),
            command: Some("cargo check && cargo clippy -- -D warnings".to_string()),
            validation_passed: None,
            score_delta: None,
        },
    );
    let denied = pre_command.denied();
    hook_decisions.push(pre_command);
    if denied {
        return CandidateExecutionOutcome::empty(
            " (pre-command hook denied validation)",
            hook_decisions,
        );
    }

    let wt_name = format!("opt-{}-{}", context.iteration, context.candidate_index);
    let validation_result =
        mdx_rust_analysis::editing::apply_and_validate(&agent.path, edit, &wt_name);

    let Ok(validation) = validation_result else {
        if !context.config.quiet {
            println!("     [Safe Apply] Validation in isolated workspace failed to run.");
        }
        return CandidateExecutionOutcome::empty(" (validation failed to run)", hook_decisions);
    };

    if !validation.passed {
        let decision = evaluate_builtin_hook(
            context.config.hook_policy,
            &HookContext {
                stage: HookStage::PostValidation,
                agent_name: agent.name.clone(),
                edit_description: Some(edit.description.clone()),
                patch_bytes: edit.patch.len(),
                command: None,
                validation_passed: Some(false),
                score_delta: None,
            },
        );
        hook_decisions.push(decision);
        if !context.config.quiet {
            println!("     [Safe Apply] Validation in isolated workspace failed.");
        }
        return CandidateExecutionOutcome::empty(
            format!(
                " (validation rejected candidate: {})",
                validation
                    .cargo_check_output
                    .lines()
                    .last()
                    .unwrap_or("no output")
            ),
            hook_decisions,
        );
    }

    let post_validation = evaluate_builtin_hook(
        context.config.hook_policy,
        &HookContext {
            stage: HookStage::PostValidation,
            agent_name: agent.name.clone(),
            edit_description: Some(edit.description.clone()),
            patch_bytes: edit.patch.len(),
            command: None,
            validation_passed: Some(true),
            score_delta: None,
        },
    );
    let denied = post_validation.denied();
    hook_decisions.push(post_validation);
    if denied {
        return CandidateExecutionOutcome {
            validated: 1,
            ..CandidateExecutionOutcome::empty(
                " (post-validation hook denied candidate)",
                hook_decisions,
            )
        };
    }

    if !context.config.quiet {
        println!(
            "     [Safe Apply] Edit validated in isolated workspace (cargo check + clippy OK)."
        );
    }

    let patched_score = {
        let score_name = format!("score-{}-{}", context.iteration, context.candidate_index);
        match mdx_rust_analysis::editing::create_isolated_workspace(&agent.path, &score_name) {
            Ok(isolated) => {
                let score = if mdx_rust_analysis::editing::apply_edit(&agent.path, &isolated, edit)
                    .is_ok()
                {
                    evaluate_workspace(&isolated, context.test_inputs, context.scorer)
                        .await
                        .unwrap_or(context.baseline_score)
                } else {
                    context.baseline_score
                };
                mdx_rust_analysis::editing::cleanup_isolated_workspace(&agent.path, &isolated);
                score
            }
            Err(_) => context.baseline_score,
        }
    };

    let delta = patched_score - context.baseline_score;
    let pre_accept = evaluate_builtin_hook(
        context.config.hook_policy,
        &HookContext {
            stage: HookStage::PreAccept,
            agent_name: agent.name.clone(),
            edit_description: Some(edit.description.clone()),
            patch_bytes: edit.patch.len(),
            command: None,
            validation_passed: Some(true),
            score_delta: Some(delta),
        },
    );
    let denied = pre_accept.denied();
    hook_decisions.push(pre_accept);
    if denied {
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            ..CandidateExecutionOutcome::empty(
                format!(" (pre-accept hook denied delta {delta:.2})"),
                hook_decisions,
            )
        };
    }

    if delta <= 0.0 {
        if !context.config.quiet {
            println!(
                "     [Net-Negative] Patched score {:.2} vs baseline {:.2} (delta {:.2}) - change rejected.",
                patched_score, context.baseline_score, delta
            );
        }
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            ..CandidateExecutionOutcome::empty(
                format!(
                    " (net-negative {:.2}->{:.2})",
                    context.baseline_score, patched_score
                ),
                hook_decisions,
            )
        };
    }

    if context.config.review_before_apply {
        if !context.config.quiet {
            println!("     [Review] Change validated in isolation but not applied (--review).");
        }
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            ..CandidateExecutionOutcome::empty(
                " (review mode: validated in isolation, not applied)",
                hook_decisions,
            )
        };
    }

    let snapshot = match mdx_rust_analysis::editing::snapshot_file(&edit.file) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return CandidateExecutionOutcome {
                validated: 1,
                patched_score: Some(patched_score),
                delta: Some(delta),
                ..CandidateExecutionOutcome::empty(
                    format!(" (snapshot failed: {err})"),
                    hook_decisions,
                )
            };
        }
    };

    if let Err(err) = mdx_rust_analysis::editing::apply_edit_to_agent(&agent.path, edit) {
        if !context.config.quiet {
            println!(
                "     [Land Failed] Could not apply validated patch to real source: {}",
                err
            );
        }
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            ..CandidateExecutionOutcome::empty(" (landing failed)", hook_decisions)
        };
    }

    let (final_ok, _final_output) = mdx_rust_analysis::editing::validate_build(&agent.path);

    if final_ok {
        let holdout_score = if context.holdout_inputs.is_empty() {
            None
        } else {
            evaluate_workspace(&agent.path, context.holdout_inputs, context.scorer)
                .await
                .ok()
        };

        if !context.config.quiet {
            println!(
                "     [Accepted] Landed + final validation OK (score {:.2} -> {:.2}, delta {:.2}).",
                context.baseline_score, patched_score, delta
            );
        }

        CandidateExecutionOutcome {
            validated: 1,
            landed: 1,
            accepted: 1,
            accepted_diff: Some(edit.patch.clone()),
            patched_score: Some(patched_score),
            holdout_score,
            delta: Some(delta),
            note: format!(" (accepted +{delta:.2})"),
            hook_decisions,
        }
    } else {
        let _ = mdx_rust_analysis::editing::restore_file(&snapshot);
        let _ = mdx_rust_analysis::editing::validate_build(&agent.path);
        if !context.config.quiet {
            println!(
                "     [Final Validation Failed] Change rolled back after re-validation failed."
            );
        }
        CandidateExecutionOutcome {
            validated: 1,
            landed: 0,
            accepted: 0,
            accepted_diff: None,
            patched_score: Some(patched_score),
            holdout_score: None,
            delta: Some(delta),
            note: " (final validation failed and rolled back)".to_string(),
            hook_decisions,
        }
    }
}

async fn evaluate_workspace(
    dir: &std::path::Path,
    inputs: &[serde_json::Value],
    scorer: fn(&AgentRunResult) -> f32,
) -> anyhow::Result<f32> {
    let temp_agent = RegisteredAgent {
        name: "isolated-eval".to_string(),
        path: dir.to_path_buf(),
        contract: AgentContract::Process,
        registered_at: "".to_string(),
    };

    let mut scores = vec![];
    for input in inputs {
        let res = crate::runner::run_agent(&temp_agent, input.clone()).await?;
        scores.push(scorer(&res));
    }
    if scores.is_empty() {
        return Ok(0.0);
    }
    Ok(scores.iter().sum::<f32>() / scores.len() as f32)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::optimizer::mechanical_score;
    use tempfile::tempdir;

    fn temp_agent_source(answer_suffix: &str) -> String {
        r#"use std::io::BufRead;

fn main() {
    let mut input = String::new();
    std::io::stdin().lock().read_line(&mut input).unwrap();
    println!("{{\"answer\":\"A stable useful answer __SUFFIX__\",\"confidence\":0.70,\"reasoning\":\"Think step by step.\"}}");
}
"#
        .replace("__SUFFIX__", answer_suffix)
    }

    fn write_temp_agent(with_final_failure_marker: bool) -> (tempfile::TempDir, RegisteredAgent) {
        let dir = tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"safety-agent\"\nversion=\"0.1.0\"\nedition=\"2021\"\n",
        )
        .unwrap();
        std::fs::write(dir.path().join("src/main.rs"), temp_agent_source("before")).unwrap();

        if with_final_failure_marker {
            std::fs::write(
                dir.path().join("build.rs"),
                r#"
fn main() {
    if std::path::Path::new(".mdx-rust/fail-final").exists() {
        panic!("intentional final validation failure");
    }
}
"#,
            )
            .unwrap();
            std::fs::create_dir_all(dir.path().join(".mdx-rust")).unwrap();
            std::fs::write(dir.path().join(".mdx-rust/fail-final"), "1").unwrap();
        }

        let agent = RegisteredAgent {
            name: "safety-agent".to_string(),
            path: dir.path().to_path_buf(),
            contract: AgentContract::Process,
            registered_at: "test".to_string(),
        };

        (dir, agent)
    }

    fn comment_patch() -> String {
        "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -1,5 +1,6 @@\n use std::io::BufRead;\n+// mdx safety invariant test\n \n fn main() {\n     let mut input = String::new();\n     std::io::stdin().lock().read_line(&mut input).unwrap();\n"
            .to_string()
    }

    fn improved_patch() -> String {
        "diff --git a/src/main.rs b/src/main.rs\n--- a/src/main.rs\n+++ b/src/main.rs\n@@ -2,6 +2,6 @@ use std::io::BufRead;\n \n fn main() {\n     let mut input = String::new();\n     std::io::stdin().lock().read_line(&mut input).unwrap();\n-    println!(\"{{\\\"answer\\\":\\\"A stable useful answer before\\\",\\\"confidence\\\":0.70,\\\"reasoning\\\":\\\"Think step by step.\\\"}}\");\n+    println!(\"{{\\\"answer\\\":\\\"A stable useful answer after with much more useful detail\\\",\\\"confidence\\\":0.70,\\\"reasoning\\\":\\\"Think step by step.\\\"}}\");\n }\n"
            .to_string()
    }

    fn execution_config<'a>(policy: &'a HookPolicy) -> CandidateExecutionConfig<'a> {
        CandidateExecutionConfig {
            hook_policy: policy,
            review_before_apply: false,
            quiet: true,
        }
    }

    #[tokio::test]
    async fn deny_hook_cannot_accept_or_validate() {
        let (_dir, agent) = write_temp_agent(false);
        let policy = HookPolicy {
            max_patch_bytes: 1,
            require_positive_delta: true,
        };
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "too large".to_string(),
            patch: comment_patch(),
        };

        let outcome = execute_candidate_edit(CandidateExecutionContext {
            agent: &agent,
            config: execution_config(&policy),
            iteration: 0,
            candidate_index: 0,
            edit: &edit,
            test_inputs: &[serde_json::json!({"query":"hi"})],
            holdout_inputs: &[],
            baseline_score: 0.0,
            scorer: mechanical_score,
        })
        .await;

        assert_eq!(outcome.validated, 0);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert!(outcome
            .hook_decisions
            .iter()
            .any(|decision| decision.denied()));
    }

    #[tokio::test]
    async fn net_negative_candidate_is_rejected_before_landing() {
        let (_dir, agent) = write_temp_agent(false);
        let before = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap();
        let policy = HookPolicy::default();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "comment only".to_string(),
            patch: comment_patch(),
        };

        let outcome = execute_candidate_edit(CandidateExecutionContext {
            agent: &agent,
            config: execution_config(&policy),
            iteration: 0,
            candidate_index: 0,
            edit: &edit,
            test_inputs: &[serde_json::json!({"query":"hi"})],
            holdout_inputs: &[],
            baseline_score: 0.95,
            scorer: mechanical_score,
        })
        .await;

        let after = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap();
        assert!(
            outcome.note.is_empty() || !outcome.note.contains("validation rejected"),
            "{}",
            outcome.note
        );
        assert_eq!(outcome.validated, 1, "{}", outcome.note);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert_eq!(before, after);
    }

    #[tokio::test]
    async fn final_validation_failure_rolls_back_and_does_not_accept() {
        let (_dir, agent) = write_temp_agent(true);
        let before = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap();
        let policy = HookPolicy::default();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "improve answer".to_string(),
            patch: improved_patch(),
        };

        let outcome = execute_candidate_edit(CandidateExecutionContext {
            agent: &agent,
            config: execution_config(&policy),
            iteration: 0,
            candidate_index: 0,
            edit: &edit,
            test_inputs: &[serde_json::json!({"query":"hi"})],
            holdout_inputs: &[],
            baseline_score: 0.40,
            scorer: mechanical_score,
        })
        .await;

        let after = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap();
        assert!(
            outcome.note.is_empty() || !outcome.note.contains("validation rejected"),
            "{}",
            outcome.note
        );
        assert_eq!(outcome.validated, 1, "{}", outcome.note);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert_eq!(before, after);
        assert!(outcome.note.contains("rolled back"));
    }
}
