//! Safe scoped hardening for ordinary Rust modules.
//!
//! This is the v0.3 path for non-agent Rust code. It reuses the same boring
//! safety philosophy as the optimizer: analyze, propose, validate in isolation,
//! and only touch the real tree when explicitly requested.

use crate::eval::stable_hash_hex;
use mdx_rust_analysis::editing::{
    cleanup_isolated_workspace, create_isolated_workspace, restore_transaction,
    snapshot_transaction, validate_build_detailed_with_budget, ValidationCommandRecord,
};
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
    pub apply: bool,
    pub max_files: usize,
    pub validation_timeout: Duration,
}

impl Default for HardeningConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            apply: false,
            max_files: 100,
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
    pub policy: Option<HardeningPolicyRecord>,
    pub files_scanned: usize,
    pub findings: Vec<HardeningFinding>,
    pub changes: Vec<HardeningChangeSummary>,
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
pub struct HardeningPolicyRecord {
    pub path: String,
    pub hash: String,
    pub rules: Vec<String>,
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
        },
    )?;
    let workspace = workspace_summary(&root);
    let policy = load_policy(&root, config.policy_path.as_deref())?;
    let mode = if config.apply {
        HardeningMode::Apply
    } else {
        HardeningMode::Review
    };
    let changes = summarize_changes(&analysis.changes);

    let outcome = if analysis.changes.is_empty() {
        HardeningOutcome {
            status: HardeningStatus::NoChanges,
            isolated_validation_passed: false,
            applied: false,
            final_validation_passed: false,
            validation_commands: Vec::new(),
            final_validation_commands: Vec::new(),
            rollback_succeeded: None,
            rollback_error: None,
            note: "no high-confidence hardening changes were available".to_string(),
        }
    } else {
        execute_hardening_changes(&root, &analysis.changes, config)?
    };

    let mut run = HardeningRun {
        schema_version: "0.3".to_string(),
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        mode,
        workspace,
        policy,
        files_scanned: analysis.files_scanned,
        findings: analysis.findings,
        changes,
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

    let isolated = create_isolated_workspace(root, "hardening-v0-3")?;
    write_changes(&isolated, changes)?;
    let validation = validate_build_detailed_with_budget(&isolated, config.validation_timeout);
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
        });
    }

    let real_paths: Vec<PathBuf> = changes
        .iter()
        .map(|change| root.join(&change.file))
        .collect();
    let snapshot = snapshot_transaction(&real_paths)?;
    write_changes(root, changes)?;
    let final_validation = validate_build_detailed_with_budget(root, config.validation_timeout);

    if final_validation.passed {
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
        note: "final validation failed; transaction rollback attempted".to_string(),
    })
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

fn load_policy(
    root: &Path,
    policy_path: Option<&Path>,
) -> anyhow::Result<Option<HardeningPolicyRecord>> {
    let Some(policy_path) = policy_path else {
        return Ok(None);
    };
    let path = if policy_path.is_absolute() {
        policy_path.to_path_buf()
    } else {
        root.join(policy_path)
    };
    let content = std::fs::read(&path)?;
    let rules = String::from_utf8_lossy(&content)
        .lines()
        .map(str::trim)
        .filter(|line| {
            line.starts_with("- ") || line.chars().next().is_some_and(|ch| ch.is_ascii_digit())
        })
        .take(20)
        .map(|line| {
            line.trim_start_matches("- ")
                .trim_start_matches(|ch: char| ch.is_ascii_digit() || ch == '.' || ch == ')')
                .trim()
                .to_string()
        })
        .filter(|line| !line.is_empty())
        .collect();
    Ok(Some(HardeningPolicyRecord {
        path: path.display().to_string(),
        hash: stable_hash_hex(&content),
        rules,
    }))
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

fn workspace_summary(root: &Path) -> WorkspaceSummary {
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
