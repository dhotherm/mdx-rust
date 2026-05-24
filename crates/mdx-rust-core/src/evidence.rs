//! Measured behavioral evidence for autonomous Rust evolution.
//!
//! v0.7 makes evidence a persisted artifact instead of an inferred hint. The
//! refactor planner and autopilot can consume the latest evidence run to decide
//! how much autonomy is allowed.

use crate::eval::stable_hash_hex;
use crate::refactor::{EvidenceAnalysisDepth, EvidenceGrade};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub struct EvidenceRunConfig {
    pub target: Option<PathBuf>,
    pub include_coverage: bool,
    pub include_mutation: bool,
    pub include_semver: bool,
    pub command_timeout: Duration,
}

impl Default for EvidenceRunConfig {
    fn default() -> Self {
        Self {
            target: None,
            include_coverage: false,
            include_mutation: false,
            include_semver: false,
            command_timeout: Duration::from_secs(180),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceRun {
    pub schema_version: String,
    pub run_id: String,
    pub root: String,
    pub target: Option<String>,
    pub grade: EvidenceGrade,
    pub analysis_depth: EvidenceAnalysisDepth,
    pub metrics: Vec<EvidenceMetric>,
    pub commands: Vec<EvidenceCommandRecord>,
    pub unlocked_recipe_tiers: Vec<String>,
    pub unlock_suggestions: Vec<String>,
    pub note: String,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceMetric {
    pub id: String,
    pub label: String,
    pub value: f64,
    pub unit: String,
    pub source_command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceCommandRecord {
    pub id: String,
    pub command: String,
    pub skipped: bool,
    pub skip_reason: Option<String>,
    pub success: bool,
    pub timed_out: bool,
    pub status_code: Option<i32>,
    pub duration_ms: u128,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceArtifactRef {
    pub run_id: String,
    pub grade: EvidenceGrade,
    pub analysis_depth: EvidenceAnalysisDepth,
    pub artifact_path: Option<String>,
}

impl From<&EvidenceRun> for EvidenceArtifactRef {
    fn from(run: &EvidenceRun) -> Self {
        Self {
            run_id: run.run_id.clone(),
            grade: run.grade,
            analysis_depth: run.analysis_depth.clone(),
            artifact_path: run.artifact_path.clone(),
        }
    }
}

pub fn run_evidence(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &EvidenceRunConfig,
) -> anyhow::Result<EvidenceRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let target = config
        .target
        .as_ref()
        .map(|path| resolve_target(&root, path))
        .transpose()?;

    let mut commands = Vec::new();
    commands.push(run_command(
        &root,
        "cargo-metadata",
        "cargo metadata --no-deps --format-version 1",
        config.command_timeout,
    ));
    commands.push(run_command(
        &root,
        "cargo-test",
        "cargo test",
        config.command_timeout,
    ));

    if config.include_coverage {
        commands.push(run_optional_cargo_subcommand(
            &root,
            "cargo-llvm-cov",
            "coverage",
            "cargo llvm-cov --workspace --summary-only",
            config.command_timeout,
        ));
    } else {
        commands.push(skipped_command(
            "coverage",
            "cargo llvm-cov --workspace --summary-only",
            "coverage evidence was not requested",
        ));
    }

    if config.include_mutation {
        commands.push(run_optional_cargo_subcommand(
            &root,
            "cargo-mutants",
            "mutation",
            "cargo mutants --no-shuffle --timeout 60",
            config.command_timeout,
        ));
    } else {
        commands.push(skipped_command(
            "mutation",
            "cargo mutants --no-shuffle --timeout 60",
            "mutation evidence was not requested",
        ));
    }

    if config.include_semver {
        commands.push(run_optional_cargo_subcommand(
            &root,
            "cargo-semver-checks",
            "semver-checks",
            "cargo semver-checks",
            config.command_timeout,
        ));
    } else {
        commands.push(skipped_command(
            "semver-checks",
            "cargo semver-checks",
            "semver evidence was not requested",
        ));
    }

    let grade = grade_from_commands(&commands);
    let analysis_depth = analysis_depth_for_grade(grade);
    let metrics = evidence_metrics(&commands);
    let mut run = EvidenceRun {
        schema_version: "0.7".to_string(),
        run_id: evidence_run_id(&root, target.as_deref(), &commands),
        root: root.display().to_string(),
        target: target.as_ref().map(|path| path.display().to_string()),
        grade,
        analysis_depth,
        metrics,
        commands,
        unlocked_recipe_tiers: unlocked_recipe_tiers(grade),
        unlock_suggestions: unlock_suggestions(grade, config),
        note: evidence_note(grade),
        artifact_path: None,
    };

    if let Some(artifact_root) = artifact_root {
        let path = persist_evidence_run(artifact_root, &run)?;
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&run)?)?;
    }

    Ok(run)
}

pub fn load_latest_evidence(artifact_root: Option<&Path>) -> anyhow::Result<Option<EvidenceRun>> {
    load_latest_evidence_matching(artifact_root, |_| true)
}

pub fn load_latest_evidence_for_root(
    artifact_root: Option<&Path>,
    root: &Path,
) -> anyhow::Result<Option<EvidenceRun>> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    load_latest_evidence_matching(artifact_root, |run| run.root == root.display().to_string())
}

fn load_latest_evidence_matching(
    artifact_root: Option<&Path>,
    matches_run: impl Fn(&EvidenceRun) -> bool,
) -> anyhow::Result<Option<EvidenceRun>> {
    let Some(artifact_root) = artifact_root else {
        return Ok(None);
    };
    let dir = artifact_root.join("evidence");
    if !dir.exists() {
        return Ok(None);
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(&dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|ext| ext == "json") {
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .ok();
            entries.push((modified, path));
        }
    }
    entries.sort_by(|left, right| left.0.cmp(&right.0).then_with(|| left.1.cmp(&right.1)));

    while let Some((_, path)) = entries.pop() {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let Ok(run) = serde_json::from_str::<EvidenceRun>(&content) else {
            continue;
        };
        if matches_run(&run) {
            return Ok(Some(run));
        }
    }
    Ok(None)
}

fn resolve_target(root: &Path, target: &Path) -> anyhow::Result<PathBuf> {
    if target
        .components()
        .any(|component| matches!(component, Component::ParentDir))
    {
        anyhow::bail!(
            "evidence target must stay inside root: {}",
            target.display()
        );
    }
    let resolved = if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    };
    if !resolved.starts_with(root) {
        anyhow::bail!("evidence target is outside root: {}", target.display());
    }
    Ok(resolved
        .strip_prefix(root)
        .unwrap_or(&resolved)
        .to_path_buf())
}

fn run_optional_cargo_subcommand(
    root: &Path,
    executable: &str,
    id: &str,
    command: &str,
    timeout: Duration,
) -> EvidenceCommandRecord {
    if !executable_exists(executable) {
        return skipped_command(id, command, &format!("{executable} was not found on PATH"));
    }
    run_command(root, id, command, timeout)
}

fn run_command(root: &Path, id: &str, command: &str, timeout: Duration) -> EvidenceCommandRecord {
    let started_at = Instant::now();
    let mut parts = command.split_whitespace();
    let Some(program) = parts.next() else {
        return EvidenceCommandRecord {
            id: id.to_string(),
            command: command.to_string(),
            skipped: false,
            skip_reason: None,
            success: false,
            timed_out: false,
            status_code: None,
            duration_ms: started_at.elapsed().as_millis(),
            stdout: String::new(),
            stderr: "empty evidence command".to_string(),
        };
    };
    let mut child = match Command::new(program)
        .args(parts)
        .current_dir(root)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(error) => {
            return EvidenceCommandRecord {
                id: id.to_string(),
                command: command.to_string(),
                skipped: false,
                skip_reason: None,
                success: false,
                timed_out: false,
                status_code: None,
                duration_ms: started_at.elapsed().as_millis(),
                stdout: String::new(),
                stderr: error.to_string(),
            };
        }
    };

    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if started_at.elapsed() >= timeout => {
                timed_out = true;
                let _ = child.kill();
                break;
            }
            Ok(None) => std::thread::sleep(Duration::from_millis(50)),
            Err(error) => {
                return EvidenceCommandRecord {
                    id: id.to_string(),
                    command: command.to_string(),
                    skipped: false,
                    skip_reason: None,
                    success: false,
                    timed_out: false,
                    status_code: None,
                    duration_ms: started_at.elapsed().as_millis(),
                    stdout: String::new(),
                    stderr: error.to_string(),
                };
            }
        }
    }

    match child.wait_with_output() {
        Ok(output) => EvidenceCommandRecord {
            id: id.to_string(),
            command: command.to_string(),
            skipped: false,
            skip_reason: None,
            success: !timed_out && output.status.success(),
            timed_out,
            status_code: output.status.code(),
            duration_ms: started_at.elapsed().as_millis(),
            stdout: String::from_utf8_lossy(&output.stdout).to_string(),
            stderr: String::from_utf8_lossy(&output.stderr).to_string(),
        },
        Err(error) => EvidenceCommandRecord {
            id: id.to_string(),
            command: command.to_string(),
            skipped: false,
            skip_reason: None,
            success: false,
            timed_out,
            status_code: None,
            duration_ms: started_at.elapsed().as_millis(),
            stdout: String::new(),
            stderr: error.to_string(),
        },
    }
}

fn skipped_command(id: &str, command: &str, reason: &str) -> EvidenceCommandRecord {
    EvidenceCommandRecord {
        id: id.to_string(),
        command: command.to_string(),
        skipped: true,
        skip_reason: Some(reason.to_string()),
        success: false,
        timed_out: false,
        status_code: None,
        duration_ms: 0,
        stdout: String::new(),
        stderr: String::new(),
    }
}

fn evidence_metrics(commands: &[EvidenceCommandRecord]) -> Vec<EvidenceMetric> {
    let mut metrics = Vec::new();
    if let Some(command) = commands.iter().find(|command| command.id == "coverage") {
        if let Some(percent) = last_percent(&format!("{}\n{}", command.stdout, command.stderr)) {
            metrics.push(EvidenceMetric {
                id: "coverage-percent".to_string(),
                label: "Line coverage".to_string(),
                value: percent,
                unit: "percent".to_string(),
                source_command: command.id.clone(),
            });
        }
    }
    if let Some(command) = commands.iter().find(|command| command.id == "mutation") {
        if let Some(percent) = last_percent(&format!("{}\n{}", command.stdout, command.stderr)) {
            metrics.push(EvidenceMetric {
                id: "mutation-score-percent".to_string(),
                label: "Mutation score".to_string(),
                value: percent,
                unit: "percent".to_string(),
                source_command: command.id.clone(),
            });
        }
    }
    metrics
}

fn last_percent(output: &str) -> Option<f64> {
    output
        .split_whitespace()
        .filter_map(|token| token.trim_end_matches('%').parse::<f64>().ok())
        .next_back()
}

fn grade_from_commands(commands: &[EvidenceCommandRecord]) -> EvidenceGrade {
    let metadata_ok = command_success(commands, "cargo-metadata");
    if !metadata_ok {
        return EvidenceGrade::None;
    }
    let tests_ok = command_success(commands, "cargo-test");
    if !tests_ok {
        return EvidenceGrade::Compiled;
    }
    let coverage_ok = command_success(commands, "coverage");
    let mutation_ok = command_success(commands, "mutation");
    let semver_ok = command_success(commands, "semver-checks");
    if coverage_ok && mutation_ok && semver_ok {
        EvidenceGrade::Proven
    } else if coverage_ok && mutation_ok {
        EvidenceGrade::Hardened
    } else if coverage_ok {
        EvidenceGrade::Covered
    } else {
        EvidenceGrade::Tested
    }
}

fn command_success(commands: &[EvidenceCommandRecord], id: &str) -> bool {
    commands
        .iter()
        .any(|command| command.id == id && command.success)
}

fn analysis_depth_for_grade(grade: EvidenceGrade) -> EvidenceAnalysisDepth {
    match grade {
        EvidenceGrade::None => EvidenceAnalysisDepth::None,
        EvidenceGrade::Compiled => EvidenceAnalysisDepth::Mechanical,
        EvidenceGrade::Tested => EvidenceAnalysisDepth::BoundaryAware,
        EvidenceGrade::Covered | EvidenceGrade::Hardened | EvidenceGrade::Proven => {
            EvidenceAnalysisDepth::Structural
        }
    }
}

fn unlocked_recipe_tiers(grade: EvidenceGrade) -> Vec<String> {
    let mut tiers = Vec::new();
    if grade >= EvidenceGrade::Compiled {
        tiers.push("Tier 1 mechanical recipes".to_string());
    }
    if grade >= EvidenceGrade::Covered {
        tiers.push("Tier 2 structural mechanical recipes".to_string());
    }
    if grade >= EvidenceGrade::Hardened {
        tiers.push("Tier 3 semantic planning candidates".to_string());
    }
    tiers
}

fn unlock_suggestions(grade: EvidenceGrade, config: &EvidenceRunConfig) -> Vec<String> {
    let mut suggestions = Vec::new();
    if grade < EvidenceGrade::Tested {
        suggestions.push("Make `cargo test` pass to unlock tested evidence.".to_string());
    }
    if !config.include_coverage {
        suggestions.push(
            "Run `mdx-rust evidence --include-coverage` after installing cargo-llvm-cov to unlock Tier 2 autonomous recipes.".to_string(),
        );
    }
    if !config.include_mutation {
        suggestions.push(
            "Run `mdx-rust evidence --include-mutation` after installing cargo-mutants to unlock hardened autonomy.".to_string(),
        );
    }
    suggestions
}

fn evidence_note(grade: EvidenceGrade) -> String {
    match grade {
        EvidenceGrade::None => "no usable Cargo evidence was collected".to_string(),
        EvidenceGrade::Compiled => {
            "Cargo metadata exists, but tests did not pass during evidence collection".to_string()
        }
        EvidenceGrade::Tested => {
            "tests passed; Tier 1 autonomy is allowed and Tier 2 remains gated by coverage"
                .to_string()
        }
        EvidenceGrade::Covered => {
            "tests and coverage passed; Tier 2 structural mechanical recipes may run".to_string()
        }
        EvidenceGrade::Hardened => {
            "tests, coverage, and mutation evidence passed; hardened autonomy is unlocked"
                .to_string()
        }
        EvidenceGrade::Proven => {
            "tests, coverage, mutation, and semver evidence passed; highest autonomy is unlocked"
                .to_string()
        }
    }
}

fn evidence_run_id(
    root: &Path,
    target: Option<&Path>,
    commands: &[EvidenceCommandRecord],
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{target:?}").as_bytes());
    bytes.extend_from_slice(format!("{commands:?}").as_bytes());
    stable_hash_hex(&bytes)
}

fn persist_evidence_run(artifact_root: &Path, run: &EvidenceRun) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("evidence");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!(
        "evidence-{millis}-{}.json",
        sanitize_id(&run.run_id)
    )))
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn executable_exists(name: &str) -> bool {
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(name).is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evidence_metrics_parse_percentages_from_tool_output() {
        let commands = vec![
            EvidenceCommandRecord {
                id: "coverage".to_string(),
                command: "cargo llvm-cov --workspace --summary-only".to_string(),
                skipped: false,
                skip_reason: None,
                success: true,
                timed_out: false,
                status_code: Some(0),
                duration_ms: 12,
                stdout: "total 91.7%".to_string(),
                stderr: String::new(),
            },
            EvidenceCommandRecord {
                id: "mutation".to_string(),
                command: "cargo mutants --no-shuffle --timeout 60".to_string(),
                skipped: false,
                skip_reason: None,
                success: true,
                timed_out: false,
                status_code: Some(0),
                duration_ms: 12,
                stdout: String::new(),
                stderr: "mutation score 82.5%".to_string(),
            },
        ];

        let metrics = evidence_metrics(&commands);

        assert!(metrics
            .iter()
            .any(|metric| metric.id == "coverage-percent"
                && (metric.value - 91.7).abs() < f64::EPSILON));
        assert!(metrics
            .iter()
            .any(|metric| metric.id == "mutation-score-percent"
                && (metric.value - 82.5).abs() < f64::EPSILON));
    }
}
