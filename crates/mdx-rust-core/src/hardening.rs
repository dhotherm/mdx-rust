//! Safe scoped hardening for ordinary Rust modules.
//!
//! This is the safe hardening path for non-agent Rust code. It reuses the same
//! boring safety philosophy as the optimizer: analyze, propose, validate in
//! isolation, and only touch the real tree when explicitly requested.

use crate::eval::{run_behavior_evals, stable_hash_hex, BehaviorEvalReport};
use crate::policy::{load_project_policy, PolicyCategory, ProjectPolicy};
use mdx_rust_analysis::editing::{
    cleanup_isolated_workspace, create_isolated_workspace, restore_transaction,
    snapshot_transaction, validate_build_detailed_with_budget, ValidationCommandRecord,
};
pub use mdx_rust_analysis::HardeningEvidenceDepth;
use mdx_rust_analysis::{
    analyze_hardening, HardeningAnalyzeConfig, HardeningFileChange, HardeningFinding,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct HardeningConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub apply: bool,
    pub max_files: usize,
    pub max_recipe_tier: u8,
    pub evidence_depth: HardeningEvidenceDepth,
    pub validation_timeout: Duration,
}

impl Default for HardeningConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            apply: false,
            max_files: 100,
            max_recipe_tier: 1,
            evidence_depth: HardeningEvidenceDepth::Basic,
            validation_timeout: Duration::from_secs(180),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningRun {
    pub schema_version: String,
    pub root: String,
    pub target: Option<String>,
    pub mode: HardeningMode,
    pub workspace: WorkspaceSummary,
    pub policy: Option<ProjectPolicy>,
    pub policy_matches: Vec<PolicyFindingMatch>,
    pub risk_summary: HardeningRiskSummary,
    pub files_scanned: usize,
    pub findings: Vec<HardeningFinding>,
    pub changes: Vec<HardeningChangeSummary>,
    pub behavior_evaluation: Option<BehaviorEvalReport>,
    pub final_behavior_evaluation: Option<BehaviorEvalReport>,
    pub outcome: HardeningOutcome,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum HardeningMode {
    Review,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct WorkspaceSummary {
    pub cargo_metadata_available: bool,
    pub package_count: usize,
    pub package_names: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PolicyFindingMatch {
    pub finding_id: String,
    pub rule_id: String,
    pub category: PolicyCategory,
    pub reason: String,
}

pub type HardeningPolicyRecord = ProjectPolicy;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningRiskSummary {
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub patchable: usize,
    pub top_recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningChangeSummary {
    pub file: String,
    pub strategy: String,
    pub finding_ids: Vec<String>,
    pub description: String,
    pub old_hash: String,
    pub new_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct HardeningOutcome {
    pub status: HardeningStatus,
    pub isolated_validation_passed: bool,
    pub applied: bool,
    pub final_validation_passed: bool,
    pub validation_commands: Vec<ValidationCommandRecord>,
    pub final_validation_commands: Vec<ValidationCommandRecord>,
    pub behavior_evaluation: Option<BehaviorEvalReport>,
    pub final_behavior_evaluation: Option<BehaviorEvalReport>,
    pub rollback_succeeded: Option<bool>,
    pub rollback_error: Option<String>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum HardeningStatus {
    NoChanges,
    Reviewed,
    Applied,
    ValidationFailed,
    BehaviorValidationFailed,
    FinalValidationFailedRolledBack,
    Rejected,
}

pub fn run_hardening(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &HardeningConfig,
) -> anyhow::Result<HardeningRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let analysis = analyze_hardening(
        &root,
        HardeningAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
            max_recipe_tier: config.max_recipe_tier,
            evidence_depth: config.evidence_depth,
        },
    )?;
    let workspace = workspace_summary(&root);
    let policy = load_project_policy(&root, config.policy_path.as_deref())?;
    let policy_matches = policy
        .as_ref()
        .map(|policy| match_findings_to_policy(&analysis.findings, policy))
        .unwrap_or_default();
    let mode = if config.apply {
        HardeningMode::Apply
    } else {
        HardeningMode::Review
    };
    let changes = summarize_changes(&analysis.changes);
    let risk_summary = summarize_risk(&analysis.findings);

    let outcome = if analysis.changes.is_empty() {
        HardeningOutcome {
            status: HardeningStatus::NoChanges,
            isolated_validation_passed: false,
            applied: false,
            final_validation_passed: false,
            validation_commands: Vec::new(),
            final_validation_commands: Vec::new(),
            behavior_evaluation: None,
            final_behavior_evaluation: None,
            rollback_succeeded: None,
            rollback_error: None,
            note: "no high-confidence hardening changes were available".to_string(),
        }
    } else {
        execute_hardening_changes(&root, &analysis.changes, config)?
    };

    let mut run = HardeningRun {
        schema_version: "1.0".to_string(),
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        mode,
        workspace,
        policy,
        policy_matches,
        risk_summary,
        files_scanned: analysis.files_scanned,
        findings: analysis.findings,
        changes,
        behavior_evaluation: outcome.behavior_evaluation.clone(),
        final_behavior_evaluation: outcome.final_behavior_evaluation.clone(),
        outcome,
        artifact_path: None,
    };

    if let Some(artifact_root) = artifact_root {
        let path = persist_hardening_run(artifact_root, &run)?;
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&run)?)?;
    }

    Ok(run)
}

fn execute_hardening_changes(
    root: &Path,
    changes: &[HardeningFileChange],
    config: &HardeningConfig,
) -> anyhow::Result<HardeningOutcome> {
    ensure_scoped_changes(root, changes)?;
    let behavior_spec_path = config
        .behavior_spec_path
        .as_deref()
        .map(|path| resolve_behavior_spec_path(root, path));

    let isolated = create_isolated_workspace(root, "hardening-v0-8")?;
    write_changes(&isolated, changes)?;
    let validation = validate_build_detailed_with_budget(&isolated, config.validation_timeout);
    let behavior_evaluation = if validation.passed {
        run_optional_behavior_eval(&isolated, behavior_spec_path.as_deref())?
    } else {
        None
    };
    cleanup_isolated_workspace(root, &isolated);

    if !validation.passed {
        return Ok(HardeningOutcome {
            status: HardeningStatus::ValidationFailed,
            isolated_validation_passed: false,
            applied: false,
            final_validation_passed: false,
            validation_commands: validation.command_records,
            final_validation_commands: Vec::new(),
            rollback_succeeded: None,
            rollback_error: None,
            note: "proposed hardening changes failed isolated validation".to_string(),
            behavior_evaluation: None,
            final_behavior_evaluation: None,
        });
    }

    if behavior_evaluation
        .as_ref()
        .is_some_and(|report| !report.passed())
    {
        return Ok(HardeningOutcome {
            status: HardeningStatus::BehaviorValidationFailed,
            isolated_validation_passed: true,
            applied: false,
            final_validation_passed: false,
            validation_commands: validation.command_records,
            final_validation_commands: Vec::new(),
            rollback_succeeded: None,
            rollback_error: None,
            note: "proposed hardening changes failed behavior evaluation".to_string(),
            behavior_evaluation,
            final_behavior_evaluation: None,
        });
    }

    if !config.apply {
        return Ok(HardeningOutcome {
            status: HardeningStatus::Reviewed,
            isolated_validation_passed: true,
            applied: false,
            final_validation_passed: false,
            validation_commands: validation.command_records,
            final_validation_commands: Vec::new(),
            rollback_succeeded: None,
            rollback_error: None,
            note: "changes validated in isolation; rerun with --apply to land them".to_string(),
            behavior_evaluation,
            final_behavior_evaluation: None,
        });
    }

    let real_paths: Vec<PathBuf> = changes
        .iter()
        .map(|change| root.join(&change.file))
        .collect();
    let snapshot = snapshot_transaction(&real_paths)?;
    write_changes(root, changes)?;
    let final_validation = validate_build_detailed_with_budget(root, config.validation_timeout);
    let final_behavior_evaluation = if final_validation.passed {
        run_optional_behavior_eval(root, behavior_spec_path.as_deref())?
    } else {
        None
    };

    if final_validation.passed
        && final_behavior_evaluation
            .as_ref()
            .map(BehaviorEvalReport::passed)
            .unwrap_or(true)
    {
        return Ok(HardeningOutcome {
            status: HardeningStatus::Applied,
            isolated_validation_passed: true,
            applied: true,
            final_validation_passed: true,
            validation_commands: validation.command_records,
            final_validation_commands: final_validation.command_records,
            rollback_succeeded: None,
            rollback_error: None,
            note: "hardening changes applied and final validation passed".to_string(),
            behavior_evaluation,
            final_behavior_evaluation,
        });
    }

    let rollback = restore_transaction(&snapshot);
    let rollback_error = rollback.as_ref().err().map(ToString::to_string);
    Ok(HardeningOutcome {
        status: HardeningStatus::FinalValidationFailedRolledBack,
        isolated_validation_passed: true,
        applied: false,
        final_validation_passed: false,
        validation_commands: validation.command_records,
        final_validation_commands: final_validation.command_records,
        rollback_succeeded: Some(rollback.is_ok()),
        rollback_error,
        note: if final_validation.passed {
            "final behavior evaluation failed; transaction rollback attempted".to_string()
        } else {
            "final validation failed; transaction rollback attempted".to_string()
        },
        behavior_evaluation,
        final_behavior_evaluation,
    })
}

fn run_optional_behavior_eval(
    root: &Path,
    spec_path: Option<&Path>,
) -> anyhow::Result<Option<BehaviorEvalReport>> {
    let Some(spec_path) = spec_path else {
        return Ok(None);
    };
    Ok(Some(run_behavior_evals(root, spec_path)?))
}

fn resolve_behavior_spec_path(root: &Path, spec_path: &Path) -> PathBuf {
    if spec_path.is_absolute() {
        spec_path.to_path_buf()
    } else {
        root.join(spec_path)
    }
}

fn ensure_scoped_changes(root: &Path, changes: &[HardeningFileChange]) -> anyhow::Result<()> {
    if changes.is_empty() {
        anyhow::bail!("hardening transaction has no changes");
    }
    for change in changes {
        if change.file.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            anyhow::bail!("unscoped hardening path: {}", change.file.display());
        }
        let target = root.join(&change.file);
        if !target.starts_with(root) {
            anyhow::bail!("hardening path escapes root: {}", change.file.display());
        }
    }
    Ok(())
}

fn write_changes(root: &Path, changes: &[HardeningFileChange]) -> anyhow::Result<()> {
    for change in changes {
        let path = root.join(&change.file);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, &change.new_content)?;
    }
    Ok(())
}

fn summarize_changes(changes: &[HardeningFileChange]) -> Vec<HardeningChangeSummary> {
    changes
        .iter()
        .map(|change| HardeningChangeSummary {
            file: change.file.display().to_string(),
            strategy: format!("{:?}", change.strategy),
            finding_ids: change.finding_ids.clone(),
            description: change.description.clone(),
            old_hash: stable_hash_hex(change.old_content.as_bytes()),
            new_hash: stable_hash_hex(change.new_content.as_bytes()),
        })
        .collect()
}

fn match_findings_to_policy(
    findings: &[HardeningFinding],
    policy: &ProjectPolicy,
) -> Vec<PolicyFindingMatch> {
    findings
        .iter()
        .filter_map(|finding| {
            let category = category_for_finding(finding);
            let rule = policy
                .rules
                .iter()
                .find(|rule| rule.category == category)
                .or_else(|| {
                    policy
                        .rules
                        .iter()
                        .find(|rule| rule.category == PolicyCategory::General)
                })?;
            Some(PolicyFindingMatch {
                finding_id: finding.id.clone(),
                rule_id: rule.id.clone(),
                category: rule.category.clone(),
                reason: format!("finding maps to policy rule on line {}", rule.line),
            })
        })
        .collect()
}

fn category_for_finding(finding: &HardeningFinding) -> PolicyCategory {
    match finding.strategy {
        mdx_rust_analysis::HardeningStrategy::BorrowParameterTightening
        | mdx_rust_analysis::HardeningStrategy::LenCheckIsEmpty
        | mdx_rust_analysis::HardeningStrategy::IteratorCloned
        | mdx_rust_analysis::HardeningStrategy::MechanicalTier1Cleanup
        | mdx_rust_analysis::HardeningStrategy::MustUsePublicReturn
        | mdx_rust_analysis::HardeningStrategy::OptionContextPropagation
        | mdx_rust_analysis::HardeningStrategy::RepeatedStringLiteralConst => {
            PolicyCategory::General
        }
        mdx_rust_analysis::HardeningStrategy::ErrorContextPropagation
        | mdx_rust_analysis::HardeningStrategy::OptionMatchContextPropagation => {
            PolicyCategory::ErrorContext
        }
        mdx_rust_analysis::HardeningStrategy::ClonePressureReview
        | mdx_rust_analysis::HardeningStrategy::LongFunctionReview => PolicyCategory::General,
        mdx_rust_analysis::HardeningStrategy::ResultUnwrapContext => PolicyCategory::PanicSafety,
        mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview => {
            PolicyCategory::ProcessExecution
        }
        mdx_rust_analysis::HardeningStrategy::UnsafeReview => PolicyCategory::UnsafeCode,
        mdx_rust_analysis::HardeningStrategy::EnvAccessReview => PolicyCategory::Environment,
        mdx_rust_analysis::HardeningStrategy::FileIoReview => PolicyCategory::ErrorContext,
        mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview => PolicyCategory::InputValidation,
    }
}

fn summarize_risk(findings: &[HardeningFinding]) -> HardeningRiskSummary {
    let mut summary = HardeningRiskSummary {
        high: 0,
        medium: 0,
        low: 0,
        patchable: findings.iter().filter(|finding| finding.patchable).count(),
        top_recommendations: Vec::new(),
    };

    let mut saw_patchable = false;
    let mut saw_process = false;
    let mut saw_http = false;
    let mut saw_file = false;

    for finding in findings {
        match finding.strategy {
            mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview
            | mdx_rust_analysis::HardeningStrategy::UnsafeReview => summary.high += 1,
            mdx_rust_analysis::HardeningStrategy::BorrowParameterTightening
            | mdx_rust_analysis::HardeningStrategy::LenCheckIsEmpty
            | mdx_rust_analysis::HardeningStrategy::IteratorCloned
            | mdx_rust_analysis::HardeningStrategy::MechanicalTier1Cleanup
            | mdx_rust_analysis::HardeningStrategy::MustUsePublicReturn
            | mdx_rust_analysis::HardeningStrategy::OptionMatchContextPropagation
            | mdx_rust_analysis::HardeningStrategy::OptionContextPropagation
            | mdx_rust_analysis::HardeningStrategy::RepeatedStringLiteralConst => summary.low += 1,
            mdx_rust_analysis::HardeningStrategy::ErrorContextPropagation
            | mdx_rust_analysis::HardeningStrategy::ClonePressureReview
            | mdx_rust_analysis::HardeningStrategy::LongFunctionReview
            | mdx_rust_analysis::HardeningStrategy::ResultUnwrapContext
            | mdx_rust_analysis::HardeningStrategy::EnvAccessReview
            | mdx_rust_analysis::HardeningStrategy::FileIoReview
            | mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview => summary.medium += 1,
        }

        saw_patchable |= finding.patchable;
        saw_process |= matches!(
            finding.strategy,
            mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview
        );
        saw_http |= matches!(
            finding.strategy,
            mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview
        );
        saw_file |= matches!(
            finding.strategy,
            mdx_rust_analysis::HardeningStrategy::FileIoReview
        );
    }

    if saw_patchable {
        summary
            .top_recommendations
            .push("Run mdx-rust improve <target> --apply after reviewing the proposed mechanical changes.".to_string());
    }
    if saw_process {
        summary.top_recommendations.push(
            "Review process execution callsites for allowlisted commands and validated arguments."
                .to_string(),
        );
    }
    if saw_http {
        summary.top_recommendations.push(
            "Review HTTP route surfaces for request validation and typed error handling."
                .to_string(),
        );
    }
    if saw_file {
        summary.top_recommendations.push(
            "Review filesystem boundaries for contextual errors and path validation.".to_string(),
        );
    }

    summary
}

fn persist_hardening_run(artifact_root: &Path, run: &HardeningRun) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("hardening");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    let mode = match run.mode {
        HardeningMode::Review => "review",
        HardeningMode::Apply => "apply",
    };
    Ok(dir.join(format!("hardening-{mode}-{millis}.json")))
}

pub(crate) fn workspace_summary(root: &Path) -> WorkspaceSummary {
    let mut command = std::process::Command::new("cargo");
    command
        .current_dir(root)
        .args(["metadata", "--no-deps", "--format-version", "1"]);
    let Some(output) =
        mdx_rust_analysis::editing::run_command_with_timeout(&mut command, Duration::from_secs(20))
    else {
        return WorkspaceSummary {
            cargo_metadata_available: false,
            package_count: 0,
            package_names: Vec::new(),
        };
    };

    if !output.success() {
        return WorkspaceSummary {
            cargo_metadata_available: false,
            package_count: 0,
            package_names: Vec::new(),
        };
    }

    let value: serde_json::Value = match serde_json::from_str(&output.stdout) {
        Ok(value) => value,
        Err(_) => {
            return WorkspaceSummary {
                cargo_metadata_available: false,
                package_count: 0,
                package_names: Vec::new(),
            }
        }
    };
    let package_names: Vec<String> = value
        .get("packages")
        .and_then(|packages| packages.as_array())
        .map(|packages| {
            packages
                .iter()
                .filter_map(|package| package.get("name").and_then(|name| name.as_str()))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default();
    WorkspaceSummary {
        cargo_metadata_available: true,
        package_count: package_names.len(),
        package_names,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn write_fixture(root: &Path) {
        std::fs::write(
            root.join("Cargo.toml"),
            r#"[package]
name = "hardening-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
        )
        .unwrap();
        let src = root.join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(
            src.join("lib.rs"),
            r#"pub fn load_config() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
        )
        .unwrap();
    }

    fn write_behavior_spec(root: &Path, expected: &str) -> PathBuf {
        let path = root.join("evals.json");
        std::fs::write(
            &path,
            format!(
                r#"{{
  "version": "v1",
  "commands": [
    {{
      "id": "cargo-check",
      "command": "cargo",
      "args": ["check"],
      "expect_success": true,
      "expect_stderr_contains": ["{expected}"],
      "timeout_seconds": 120
    }}
  ]
}}"#
            ),
        )
        .unwrap();
        path
    }

    fn write_artifact_behavior_spec(root: &Path) -> PathBuf {
        let dir = root.join(".mdx-rust");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("evals.json");
        std::fs::write(
            &path,
            r#"{
  "version": "v1",
  "commands": [
    {
      "id": "cargo-check",
      "command": "cargo",
      "args": ["check"],
      "expect_success": true,
      "timeout_seconds": 120
    }
  ]
}"#,
        )
        .unwrap();
        PathBuf::from(".mdx-rust/evals.json")
    }

    #[test]
    fn hardening_review_validates_without_touching_real_tree() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());
        let before = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();

        let run = run_hardening(
            dir.path(),
            None,
            &HardeningConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                validation_timeout: Duration::from_secs(120),
                ..HardeningConfig::default()
            },
        )
        .unwrap();

        let after = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();
        assert_eq!(before, after);
        assert_eq!(run.outcome.status, HardeningStatus::Reviewed);
        assert_eq!(run.schema_version, "1.0");
        assert!(run.outcome.isolated_validation_passed);
        assert!(!run.changes.is_empty());
    }

    #[test]
    fn hardening_apply_lands_validated_transaction() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());

        let run = run_hardening(
            dir.path(),
            None,
            &HardeningConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                apply: true,
                validation_timeout: Duration::from_secs(120),
                ..HardeningConfig::default()
            },
        )
        .unwrap();

        let after = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();
        assert_eq!(run.outcome.status, HardeningStatus::Applied);
        assert!(run.outcome.final_validation_passed);
        assert!(after.contains("use anyhow::Context;"));
        assert!(after.contains(".context(\"load_config failed instead of panicking\")?"));
    }

    #[test]
    fn hardening_maps_findings_to_structured_policy_rules() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());
        std::fs::create_dir_all(dir.path().join(".mdx-rust")).unwrap();
        std::fs::write(
            dir.path().join(".mdx-rust/policies.md"),
            "1. Avoid unwrap in service boundaries.",
        )
        .unwrap();

        let run = run_hardening(
            dir.path(),
            None,
            &HardeningConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                validation_timeout: Duration::from_secs(120),
                ..HardeningConfig::default()
            },
        )
        .unwrap();

        assert!(run.policy.is_some());
        assert!(!run.policy_matches.is_empty());
        assert_eq!(run.policy_matches[0].category, PolicyCategory::PanicSafety);
    }

    #[test]
    fn hardening_behavior_eval_failure_blocks_apply() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());
        let spec = write_behavior_spec(dir.path(), "definitely-not-in-cargo-output");
        let before = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();

        let run = run_hardening(
            dir.path(),
            None,
            &HardeningConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                behavior_spec_path: Some(spec),
                apply: true,
                validation_timeout: Duration::from_secs(120),
                ..HardeningConfig::default()
            },
        )
        .unwrap();

        let after = std::fs::read_to_string(dir.path().join("src/lib.rs")).unwrap();
        assert_eq!(before, after);
        assert_eq!(
            run.outcome.status,
            HardeningStatus::BehaviorValidationFailed
        );
        assert!(run.outcome.behavior_evaluation.is_some());
    }

    #[test]
    fn hardening_behavior_eval_spec_in_artifact_dir_runs_in_isolation() {
        let dir = tempdir().unwrap();
        write_fixture(dir.path());
        let spec = write_artifact_behavior_spec(dir.path());

        let run = run_hardening(
            dir.path(),
            None,
            &HardeningConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                behavior_spec_path: Some(spec),
                apply: true,
                validation_timeout: Duration::from_secs(120),
                ..HardeningConfig::default()
            },
        )
        .unwrap();

        assert_eq!(run.outcome.status, HardeningStatus::Applied);
        assert!(run
            .outcome
            .behavior_evaluation
            .as_ref()
            .is_some_and(BehaviorEvalReport::passed));
        assert!(run
            .outcome
            .final_behavior_evaluation
            .as_ref()
            .is_some_and(BehaviorEvalReport::passed));
    }

    #[test]
    fn hardening_rejects_unscoped_transaction_paths() {
        let dir = tempdir().unwrap();
        let changes = vec![HardeningFileChange {
            file: PathBuf::from("../escape.rs"),
            old_content: String::new(),
            new_content: String::new(),
            strategy: mdx_rust_analysis::HardeningStrategy::ResultUnwrapContext,
            finding_ids: vec!["escape".to_string()],
            description: "bad path".to_string(),
        }];

        let err = ensure_scoped_changes(dir.path(), &changes).unwrap_err();
        assert!(err.to_string().contains("unscoped hardening path"));
    }
}
