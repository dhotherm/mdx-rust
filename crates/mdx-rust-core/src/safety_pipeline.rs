//! Candidate safety pipeline.
//!
//! This module owns the acceptance-critical path:
//! hook checks, isolated validation, patched scoring, final landing,
//! final validation, and rollback.

use crate::hooks::{evaluate_builtin_hook, HookContext, HookDecision, HookPolicy, HookStage};
use crate::registry::{AgentContract, RegisteredAgent};
use crate::runner::AgentRunResult;
use mdx_rust_analysis::editing::{ProposedEdit, ValidationCommandRecord};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy)]
pub struct CandidateExecutionConfig<'a> {
    pub hook_policy: &'a HookPolicy,
    pub review_before_apply: bool,
    pub quiet: bool,
    pub candidate_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, thiserror::Error, PartialEq, Eq)]
pub enum SafetyRejectionKind {
    #[error("edit scope rejected")]
    EditScope,
    #[error("hook denied candidate")]
    HookDenied,
    #[error("validation failed")]
    ValidationFailed,
    #[error("candidate was not net positive")]
    NetNegative,
    #[error("review mode prevented landing")]
    ReviewOnly,
    #[error("snapshot failed")]
    SnapshotFailed,
    #[error("landing failed")]
    LandingFailed,
    #[error("final validation failed")]
    FinalValidationFailed,
    #[error("candidate timed out")]
    Timeout,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SafetyRejection {
    pub kind: SafetyRejectionKind,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub validation_commands: Vec<ValidationCommandRecord>,
    pub final_validation_commands: Vec<ValidationCommandRecord>,
    pub rollback_succeeded: Option<bool>,
    pub rollback_error: Option<String>,
    pub timed_out: bool,
    #[serde(default)]
    pub rejection: Option<SafetyRejection>,
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
            validation_commands: Vec::new(),
            final_validation_commands: Vec::new(),
            rollback_succeeded: None,
            rollback_error: None,
            timed_out: false,
            rejection: None,
        }
    }

    fn rejected(
        kind: SafetyRejectionKind,
        message: impl Into<String>,
        hook_decisions: Vec<HookDecision>,
    ) -> Self {
        let message = message.into();
        Self {
            rejection: Some(SafetyRejection {
                kind,
                message: message.clone(),
            }),
            ..Self::empty(message, hook_decisions)
        }
    }
}

struct ScopedEdit<'a> {
    edit: &'a ProposedEdit,
}

struct IsolatedValidatedEdit<'a> {
    scoped: ScopedEdit<'a>,
    validation_commands: Vec<ValidationCommandRecord>,
}

struct NetPositiveEdit<'a> {
    validated: IsolatedValidatedEdit<'a>,
    patched_score: f32,
    delta: f32,
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
    let timeout = context.config.candidate_timeout;
    match tokio::time::timeout(timeout, execute_candidate_edit_inner(context)).await {
        Ok(outcome) => outcome,
        Err(_) => CandidateExecutionOutcome {
            timed_out: true,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::Timeout,
                format!(" (candidate timed out after {}s)", timeout.as_secs()),
                Vec::new(),
            )
        },
    }
}

async fn execute_candidate_edit_inner(
    context: CandidateExecutionContext<'_>,
) -> CandidateExecutionOutcome {
    let agent = context.agent;
    let edit = context.edit;
    let mut hook_decisions = Vec::new();
    let deadline_start = Instant::now();

    if let Err(err) = ensure_single_file_patch_scope(&agent.path, edit) {
        return CandidateExecutionOutcome::rejected(
            SafetyRejectionKind::EditScope,
            format!(" (edit scope rejected: {err})"),
            hook_decisions,
        );
    }
    let scoped_edit = ScopedEdit { edit };

    if deadline_start.elapsed() >= context.config.candidate_timeout {
        return timed_out_outcome(context.config.candidate_timeout, hook_decisions);
    }

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
        return CandidateExecutionOutcome::rejected(
            SafetyRejectionKind::HookDenied,
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
        return CandidateExecutionOutcome::rejected(
            SafetyRejectionKind::HookDenied,
            " (pre-command hook denied validation)",
            hook_decisions,
        );
    }

    let wt_name = format!("opt-{}-{}", context.iteration, context.candidate_index);
    let Some(validation_budget) =
        remaining_budget(deadline_start, context.config.candidate_timeout)
    else {
        return timed_out_outcome(context.config.candidate_timeout, hook_decisions);
    };
    let validation_result = mdx_rust_analysis::editing::apply_and_validate_with_budget(
        &agent.path,
        edit,
        &wt_name,
        validation_budget,
    );

    let Ok(validation) = validation_result else {
        if !context.config.quiet {
            println!("     [Safe Apply] Validation in isolated workspace failed to run.");
        }
        return CandidateExecutionOutcome::rejected(
            SafetyRejectionKind::ValidationFailed,
            " (validation failed to run)",
            hook_decisions,
        );
    };
    if !validation.passed {
        let validation_commands = validation.command_records;
        let validation_timed_out = validation_commands.iter().any(|record| record.timed_out);
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
        return CandidateExecutionOutcome {
            validation_commands,
            timed_out: validation_timed_out,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::ValidationFailed,
                format!(
                    " (validation rejected candidate: {})",
                    validation
                        .cargo_check_output
                        .lines()
                        .last()
                        .unwrap_or("no output")
                ),
                hook_decisions,
            )
        };
    }
    let validation_commands = validation.command_records;
    let validated_edit = IsolatedValidatedEdit {
        scoped: scoped_edit,
        validation_commands,
    };
    if deadline_start.elapsed() >= context.config.candidate_timeout {
        let validation_commands = validated_edit.validation_commands;
        return CandidateExecutionOutcome {
            validated: 1,
            validation_commands,
            ..timed_out_outcome(context.config.candidate_timeout, hook_decisions)
        };
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
        let validation_commands = validated_edit.validation_commands;
        return CandidateExecutionOutcome {
            validated: 1,
            validation_commands,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::HookDenied,
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
    if deadline_start.elapsed() >= context.config.candidate_timeout {
        let validation_commands = validated_edit.validation_commands;
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(patched_score - context.baseline_score),
            validation_commands,
            ..timed_out_outcome(context.config.candidate_timeout, hook_decisions)
        };
    }

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
        let validation_commands = validated_edit.validation_commands;
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            validation_commands,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::HookDenied,
                format!(" (pre-accept hook denied delta {delta:.2})"),
                hook_decisions,
            )
        };
    }

    if delta <= 0.0 {
        let validation_commands = validated_edit.validation_commands;
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
            validation_commands,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::NetNegative,
                format!(
                    " (net-negative {:.2}->{:.2})",
                    context.baseline_score, patched_score
                ),
                hook_decisions,
            )
        };
    }
    let net_positive_edit = NetPositiveEdit {
        validated: validated_edit,
        patched_score,
        delta,
    };

    if context.config.review_before_apply {
        let validation_commands = net_positive_edit.validated.validation_commands;
        if !context.config.quiet {
            println!("     [Review] Change validated in isolation but not applied (--review).");
        }
        return CandidateExecutionOutcome {
            validated: 1,
            patched_score: Some(patched_score),
            delta: Some(delta),
            validation_commands,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::ReviewOnly,
                " (review mode: validated in isolation, not applied)",
                hook_decisions,
            )
        };
    }

    let edit = net_positive_edit.validated.scoped.edit;
    let validation_commands = net_positive_edit.validated.validation_commands;
    let patched_score = net_positive_edit.patched_score;
    let delta = net_positive_edit.delta;

    let snapshot = match mdx_rust_analysis::editing::snapshot_file(&edit.file) {
        Ok(snapshot) => snapshot,
        Err(err) => {
            return CandidateExecutionOutcome {
                validated: 1,
                patched_score: Some(patched_score),
                delta: Some(delta),
                validation_commands,
                ..CandidateExecutionOutcome::rejected(
                    SafetyRejectionKind::SnapshotFailed,
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
            validation_commands,
            ..CandidateExecutionOutcome::rejected(
                SafetyRejectionKind::LandingFailed,
                " (landing failed)",
                hook_decisions,
            )
        };
    }

    let final_budget = remaining_budget(deadline_start, context.config.candidate_timeout)
        .unwrap_or_else(|| Duration::from_secs(0));
    let final_report =
        mdx_rust_analysis::editing::validate_build_detailed_with_budget(&agent.path, final_budget);
    let final_ok = final_report.passed;
    let final_validation_commands = final_report.command_records;
    let final_validation_timed_out = final_validation_commands
        .iter()
        .any(|record| record.timed_out);
    if deadline_start.elapsed() >= context.config.candidate_timeout || final_validation_timed_out {
        let rollback_result = mdx_rust_analysis::editing::restore_file(&snapshot);
        let rollback_error = rollback_result.as_ref().err().map(ToString::to_string);
        let rollback_succeeded = rollback_result.is_ok();
        return CandidateExecutionOutcome {
            validated: 1,
            landed: 0,
            accepted: 0,
            accepted_diff: None,
            patched_score: Some(patched_score),
            holdout_score: None,
            delta: Some(delta),
            note: format!(
                " (candidate timed out after {}s and was rolled back)",
                context.config.candidate_timeout.as_secs()
            ),
            hook_decisions,
            validation_commands,
            final_validation_commands,
            rollback_succeeded: Some(rollback_succeeded),
            rollback_error,
            timed_out: true,
            rejection: Some(SafetyRejection {
                kind: SafetyRejectionKind::Timeout,
                message: format!(
                    "candidate timed out after {}s and was rolled back",
                    context.config.candidate_timeout.as_secs()
                ),
            }),
        };
    }

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
            validation_commands,
            final_validation_commands,
            rollback_succeeded: None,
            rollback_error: None,
            timed_out: false,
            rejection: None,
        }
    } else {
        let rollback_result = mdx_rust_analysis::editing::restore_file(&snapshot);
        let rollback_error = rollback_result.as_ref().err().map(ToString::to_string);
        let rollback_succeeded = rollback_result.is_ok();
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
            validation_commands,
            final_validation_commands,
            rollback_succeeded: Some(rollback_succeeded),
            rollback_error,
            timed_out: false,
            rejection: Some(SafetyRejection {
                kind: SafetyRejectionKind::FinalValidationFailed,
                message: "final validation failed and rolled back".to_string(),
            }),
        }
    }
}

fn timed_out_outcome(
    timeout: Duration,
    hook_decisions: Vec<HookDecision>,
) -> CandidateExecutionOutcome {
    CandidateExecutionOutcome {
        timed_out: true,
        ..CandidateExecutionOutcome::rejected(
            SafetyRejectionKind::Timeout,
            format!(" (candidate timed out after {}s)", timeout.as_secs()),
            hook_decisions,
        )
    }
}

fn remaining_budget(start: Instant, total: Duration) -> Option<Duration> {
    total
        .checked_sub(start.elapsed())
        .filter(|remaining| !remaining.is_zero())
}

fn ensure_single_file_patch_scope(agent_root: &Path, edit: &ProposedEdit) -> anyhow::Result<()> {
    let expected = if edit.file.is_absolute() {
        edit.file.strip_prefix(agent_root).map_err(|_| {
            anyhow::anyhow!("edit file is outside agent root: {}", edit.file.display())
        })?
    } else {
        edit.file.as_path()
    };

    for line in edit.patch.lines() {
        for path in diff_paths_from_line(line) {
            if path == "/dev/null" {
                continue;
            }

            if Path::new(&path) != expected {
                anyhow::bail!(
                    "patch touches {}, but ProposedEdit.file is {}",
                    path,
                    expected.display()
                );
            }
        }
    }

    Ok(())
}

fn diff_paths_from_line(line: &str) -> Vec<String> {
    if let Some(path) = line
        .strip_prefix("+++ ")
        .or_else(|| line.strip_prefix("--- "))
    {
        return normalize_diff_path(path).into_iter().collect();
    }

    if let Some(rest) = line.strip_prefix("diff --git ") {
        return rest
            .split_whitespace()
            .filter_map(normalize_diff_path)
            .collect();
    }

    for prefix in ["rename from ", "rename to ", "copy from ", "copy to "] {
        if let Some(path) = line.strip_prefix(prefix) {
            return normalize_diff_path(path).into_iter().collect();
        }
    }

    if let Some(rest) = line.strip_prefix("Binary files ") {
        if let Some((left, right_with_suffix)) = rest.split_once(" and ") {
            let right = right_with_suffix
                .strip_suffix(" differ")
                .unwrap_or(right_with_suffix);
            return [left, right]
                .into_iter()
                .filter_map(normalize_diff_path)
                .collect();
        }
    }

    Vec::new()
}

fn normalize_diff_path(raw: &str) -> Option<String> {
    let path = raw.trim().trim_matches('"');
    if path == "/dev/null" {
        return Some(path.to_string());
    }

    path.strip_prefix("a/")
        .or_else(|| path.strip_prefix("b/"))
        .or(Some(path))
        .map(str::to_string)
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
    use proptest::prelude::*;
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
            candidate_timeout: Duration::from_secs(30),
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
    }

    #[tokio::test]
    async fn patch_scope_mismatch_is_rejected_before_validation() {
        let (_dir, agent) = write_temp_agent(false);
        let policy = HookPolicy::default();
        let before = std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "bad multi-file patch".to_string(),
            patch: "diff --git a/src/lib.rs b/src/lib.rs\n--- a/src/lib.rs\n+++ b/src/lib.rs\n@@ -1,1 +1,1 @@\n-a\n+b\n".to_string(),
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

        assert_eq!(outcome.validated, 0);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert!(outcome.note.contains("edit scope rejected"));
        assert_eq!(
            std::fs::read_to_string(agent.path.join("src/main.rs")).unwrap(),
            before
        );
    }

    #[tokio::test]
    async fn diff_git_scope_mismatch_is_rejected_before_validation() {
        let (_dir, agent) = write_temp_agent(false);
        let policy = HookPolicy::default();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "bad diff header".to_string(),
            patch: "diff --git a/src/main.rs b/src/lib.rs\n--- a/src/main.rs\n+++ b/src/lib.rs\n@@ -1,1 +1,1 @@\n-a\n+b\n".to_string(),
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

        assert_eq!(outcome.validated, 0);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert!(outcome.note.contains("edit scope rejected"));
    }

    #[tokio::test]
    async fn rename_scope_mismatch_is_rejected_before_validation() {
        let (_dir, agent) = write_temp_agent(false);
        let policy = HookPolicy::default();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "bad rename".to_string(),
            patch: "diff --git a/src/main.rs b/src/lib.rs\nsimilarity index 100%\nrename from src/main.rs\nrename to src/lib.rs\n".to_string(),
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

        assert_eq!(outcome.validated, 0);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert!(outcome.note.contains("edit scope rejected"));
    }

    #[tokio::test]
    async fn exhausted_candidate_timeout_stops_before_validation() {
        let (_dir, agent) = write_temp_agent(false);
        let policy = HookPolicy::default();
        let edit = ProposedEdit {
            file: agent.path.join("src/main.rs"),
            description: "comment only".to_string(),
            patch: comment_patch(),
        };
        let config = CandidateExecutionConfig {
            hook_policy: &policy,
            review_before_apply: false,
            quiet: true,
            candidate_timeout: Duration::from_secs(0),
        };

        let outcome = execute_candidate_edit(CandidateExecutionContext {
            agent: &agent,
            config,
            iteration: 0,
            candidate_index: 0,
            edit: &edit,
            test_inputs: &[serde_json::json!({"query":"hi"})],
            holdout_inputs: &[],
            baseline_score: 0.40,
            scorer: mechanical_score,
        })
        .await;

        assert!(outcome.timed_out);
        assert_eq!(outcome.validated, 0);
        assert_eq!(outcome.landed, 0);
        assert_eq!(outcome.accepted, 0);
        assert_eq!(
            outcome.rejection.as_ref().map(|rejection| &rejection.kind),
            Some(&SafetyRejectionKind::Timeout)
        );
    }

    proptest! {
        #[test]
        fn normalized_diff_paths_remove_only_diff_side_prefixes(path in "[a-zA-Z0-9_./-]{1,64}") {
            let line = format!("diff --git a/{path} b/{path}");
            let paths = diff_paths_from_line(&line);

            prop_assert_eq!(paths, vec![path.clone(), path]);
        }

        #[test]
        fn pre_accept_policy_denies_all_non_positive_deltas(delta in -10.0f32..=0.0f32) {
            let decision = evaluate_builtin_hook(
                &HookPolicy::default(),
                &HookContext {
                    stage: HookStage::PreAccept,
                    agent_name: "agent".to_string(),
                    edit_description: None,
                    patch_bytes: 0,
                    command: None,
                    validation_passed: Some(true),
                    score_delta: Some(delta),
                },
            );

            prop_assert!(decision.denied());
        }
    }
}
