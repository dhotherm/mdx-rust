//! Plan-first guardrailed refactoring.
//!
//! v0.5 deliberately starts with auditable plans instead of autonomous broad
//! rewrites. Plans summarize impact and point high-confidence changes back
//! through the existing hardening transaction path.

use crate::eval::stable_hash_hex;
use crate::hardening::{
    run_hardening, workspace_summary, HardeningConfig, HardeningRun, WorkspaceSummary,
};
use crate::policy::{load_project_policy, ProjectPolicy};
use mdx_rust_analysis::{
    analyze_hardening, analyze_refactor, HardeningAnalyzeConfig, HardeningFinding, ModuleEdge,
    RefactorAnalyzeConfig, RefactorFileSummary,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
pub struct RefactorPlanConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for RefactorPlanConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            max_files: 100,
        }
    }
}

#[derive(Debug, Clone)]
pub struct RefactorApplyConfig {
    pub plan_path: PathBuf,
    pub candidate_id: String,
    pub apply: bool,
    pub allow_public_api_impact: bool,
    pub validation_timeout: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorPlan {
    pub schema_version: String,
    pub plan_id: String,
    pub plan_hash: String,
    pub root: String,
    pub target: Option<String>,
    pub workspace: WorkspaceSummary,
    pub policy: Option<ProjectPolicy>,
    pub behavior_spec: Option<String>,
    pub impact: RefactorImpactSummary,
    pub source_snapshots: Vec<SourceSnapshot>,
    pub files: Vec<RefactorFileSummary>,
    pub module_edges: Vec<ModuleEdge>,
    pub candidates: Vec<RefactorCandidate>,
    pub required_gates: Vec<String>,
    pub non_goals: Vec<String>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SourceSnapshot {
    pub file: String,
    pub hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorImpactSummary {
    pub files_scanned: usize,
    pub public_item_count: usize,
    pub public_files: usize,
    pub module_edge_count: usize,
    pub patchable_hardening_changes: usize,
    pub review_only_findings: usize,
    pub oversized_files: usize,
    pub oversized_functions: usize,
    pub risk_level: RefactorRiskLevel,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorRiskLevel {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorCandidate {
    pub id: String,
    pub candidate_hash: String,
    pub recipe: RefactorRecipe,
    pub title: String,
    pub rationale: String,
    pub file: String,
    pub line: usize,
    pub risk: RefactorRiskLevel,
    pub status: RefactorCandidateStatus,
    pub public_api_impact: bool,
    pub apply_command: Option<String>,
    pub required_gates: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorCandidateStatus {
    ApplyViaImprove,
    PlanOnly,
    NeedsHumanDesign,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorRecipe {
    ContextualErrorHardening,
    ExtractFunctionCandidate,
    SplitModuleCandidate,
    BoundaryValidationReview,
    PublicApiReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorApplyRun {
    pub schema_version: String,
    pub root: String,
    pub plan_path: String,
    pub plan_id: String,
    pub plan_hash: String,
    pub candidate_id: String,
    pub candidate_hash: Option<String>,
    pub mode: RefactorApplyMode,
    pub status: RefactorApplyStatus,
    pub public_api_impact_allowed: bool,
    pub stale_files: Vec<StaleSourceFile>,
    pub hardening_run: Option<HardeningRun>,
    pub note: String,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorApplyMode {
    Review,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorApplyStatus {
    Reviewed,
    Applied,
    Rejected,
    StalePlan,
    Unsupported,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StaleSourceFile {
    pub file: String,
    pub expected_hash: String,
    pub actual_hash: String,
}

pub fn build_refactor_plan(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &RefactorPlanConfig,
) -> anyhow::Result<RefactorPlan> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let refactor = analyze_refactor(
        &root,
        RefactorAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
        },
    )?;
    let hardening = analyze_hardening(
        &root,
        HardeningAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
        },
    )?;
    let policy = load_project_policy(&root, config.policy_path.as_deref())?;
    let workspace = workspace_summary(&root);
    let behavior_spec = config
        .behavior_spec_path
        .as_ref()
        .map(|path| path.display().to_string());
    let impact = summarize_impact(
        &refactor.files,
        refactor.module_edges.len(),
        &hardening.findings,
        hardening.changes.len(),
    );
    let mut candidates = Vec::new();
    candidates.extend(hardening_candidates(&hardening.findings, config));
    candidates.extend(structural_candidates(&refactor.files));
    for candidate in &mut candidates {
        candidate.candidate_hash = candidate_hash(candidate);
    }
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let source_snapshots = source_snapshots(&root, &refactor.files)?;

    let required_gates = required_gates(config.behavior_spec_path.is_some());
    let non_goals = vec![
        "No autonomous broad multi-file refactors in v0.5.".to_string(),
        "No public API changes without explicit human review.".to_string(),
        "No plan candidate may bypass improve/apply validation gates.".to_string(),
    ];

    let plan_id = plan_id(&root, config, &impact, &candidates);
    let mut plan = RefactorPlan {
        schema_version: "0.5".to_string(),
        plan_id,
        plan_hash: String::new(),
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        workspace,
        policy,
        behavior_spec,
        impact,
        source_snapshots,
        files: refactor.files,
        module_edges: refactor.module_edges,
        candidates,
        required_gates,
        non_goals,
        artifact_path: None,
    };
    plan.plan_hash = refactor_plan_hash(&plan);

    if let Some(artifact_root) = artifact_root {
        let path = persist_refactor_plan(artifact_root, &plan)?;
        plan.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&plan)?)?;
    }

    Ok(plan)
}

pub fn apply_refactor_plan_candidate(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &RefactorApplyConfig,
) -> anyhow::Result<RefactorApplyRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let plan_content = std::fs::read_to_string(&config.plan_path)?;
    let plan: RefactorPlan = serde_json::from_str(&plan_content)?;
    let mode = if config.apply {
        RefactorApplyMode::Apply
    } else {
        RefactorApplyMode::Review
    };
    let mut run = RefactorApplyRun {
        schema_version: "0.5".to_string(),
        root: root.display().to_string(),
        plan_path: config.plan_path.display().to_string(),
        plan_id: plan.plan_id.clone(),
        plan_hash: plan.plan_hash.clone(),
        candidate_id: config.candidate_id.clone(),
        candidate_hash: None,
        mode,
        status: RefactorApplyStatus::Rejected,
        public_api_impact_allowed: config.allow_public_api_impact,
        stale_files: Vec::new(),
        hardening_run: None,
        note: String::new(),
        artifact_path: None,
    };

    let actual_plan_hash = refactor_plan_hash(&plan);
    if actual_plan_hash != plan.plan_hash {
        run.status = RefactorApplyStatus::Rejected;
        run.note = format!(
            "plan hash mismatch: expected {} but recomputed {}",
            plan.plan_hash, actual_plan_hash
        );
        return persist_apply_run(artifact_root, run);
    }

    let stale_files = stale_source_files(&root, &plan.source_snapshots)?;
    if !stale_files.is_empty() {
        run.status = RefactorApplyStatus::StalePlan;
        run.stale_files = stale_files;
        run.note =
            "plan source snapshots no longer match the workspace; re-run mdx-rust plan".to_string();
        return persist_apply_run(artifact_root, run);
    }

    let Some(candidate) = plan
        .candidates
        .iter()
        .find(|candidate| candidate.id == config.candidate_id)
    else {
        run.status = RefactorApplyStatus::Rejected;
        run.note = "candidate id was not found in the refactor plan".to_string();
        return persist_apply_run(artifact_root, run);
    };
    run.candidate_hash = Some(candidate.candidate_hash.clone());

    let actual_candidate_hash = candidate_hash(candidate);
    if actual_candidate_hash != candidate.candidate_hash {
        run.status = RefactorApplyStatus::Rejected;
        run.note = format!(
            "candidate hash mismatch: expected {} but recomputed {}",
            candidate.candidate_hash, actual_candidate_hash
        );
        return persist_apply_run(artifact_root, run);
    }

    if candidate.public_api_impact && !config.allow_public_api_impact {
        run.status = RefactorApplyStatus::Rejected;
        run.note = "candidate touches public API impact area; pass --allow-public-api-impact after human review".to_string();
        return persist_apply_run(artifact_root, run);
    }

    if candidate.status != RefactorCandidateStatus::ApplyViaImprove
        || candidate.recipe != RefactorRecipe::ContextualErrorHardening
    {
        run.status = RefactorApplyStatus::Unsupported;
        run.note =
            "candidate is plan-only in v0.5; no executable recipe is available yet".to_string();
        return persist_apply_run(artifact_root, run);
    }

    let hardening = run_hardening(
        &root,
        artifact_root,
        &HardeningConfig {
            target: Some(PathBuf::from(&candidate.file)),
            policy_path: plan
                .policy
                .as_ref()
                .map(|policy| PathBuf::from(policy.path.clone())),
            behavior_spec_path: plan.behavior_spec.as_ref().map(PathBuf::from),
            apply: config.apply,
            max_files: 1,
            validation_timeout: config.validation_timeout,
        },
    )?;

    run.status = if config.apply {
        if hardening.outcome.applied {
            RefactorApplyStatus::Applied
        } else {
            RefactorApplyStatus::Rejected
        }
    } else if hardening.changes.is_empty() {
        RefactorApplyStatus::Rejected
    } else {
        RefactorApplyStatus::Reviewed
    };
    run.note = format!(
        "executed candidate through hardening transaction; hardening status: {:?}",
        hardening.outcome.status
    );
    run.hardening_run = Some(hardening);
    persist_apply_run(artifact_root, run)
}

fn summarize_impact(
    files: &[RefactorFileSummary],
    module_edge_count: usize,
    findings: &[HardeningFinding],
    patchable_hardening_changes: usize,
) -> RefactorImpactSummary {
    let public_item_count = files.iter().map(|file| file.public_item_count).sum();
    let public_files = files
        .iter()
        .filter(|file| file.public_item_count > 0)
        .count();
    let oversized_files = files.iter().filter(|file| file.line_count >= 300).count();
    let oversized_functions = files
        .iter()
        .filter(|file| file.largest_function_lines >= 80)
        .count();
    let review_only_findings = findings.iter().filter(|finding| !finding.patchable).count();
    let risk_level = if public_item_count > 10 || oversized_files > 2 {
        RefactorRiskLevel::High
    } else if public_item_count > 0 || oversized_files > 0 || oversized_functions > 0 {
        RefactorRiskLevel::Medium
    } else {
        RefactorRiskLevel::Low
    };

    RefactorImpactSummary {
        files_scanned: files.len(),
        public_item_count,
        public_files,
        module_edge_count,
        patchable_hardening_changes,
        review_only_findings,
        oversized_files,
        oversized_functions,
        risk_level,
    }
}

fn hardening_candidates(
    findings: &[HardeningFinding],
    config: &RefactorPlanConfig,
) -> Vec<RefactorCandidate> {
    findings
        .iter()
        .filter(|finding| finding.patchable)
        .map(|finding| {
            let file = finding.file.display().to_string();
            RefactorCandidate {
                id: format!("plan-hardening-{}-{}", sanitize_id(&file), finding.line),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::ContextualErrorHardening,
                title: finding.title.clone(),
                rationale: "Patchable contextual error hardening can be applied through the existing isolated validation transaction.".to_string(),
                file: file.clone(),
                line: finding.line,
                risk: RefactorRiskLevel::Low,
                status: RefactorCandidateStatus::ApplyViaImprove,
                public_api_impact: false,
                apply_command: Some(apply_command(&file, config)),
                required_gates: required_gates(config.behavior_spec_path.is_some()),
            }
        })
        .collect()
}

fn structural_candidates(files: &[RefactorFileSummary]) -> Vec<RefactorCandidate> {
    let mut candidates = Vec::new();
    for file in files {
        let file_path = file.file.display().to_string();
        if file.line_count >= 300 {
            candidates.push(RefactorCandidate {
                id: format!("plan-split-module-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::SplitModuleCandidate,
                title: "Split oversized module".to_string(),
                rationale: format!(
                    "{} has {} lines. Split only after reviewing public API and module edges.",
                    file_path, file.line_count
                ),
                file: file_path.clone(),
                line: 1,
                risk: if file.public_item_count > 0 {
                    RefactorRiskLevel::High
                } else {
                    RefactorRiskLevel::Medium
                },
                status: RefactorCandidateStatus::NeedsHumanDesign,
                public_api_impact: file.public_item_count > 0,
                apply_command: None,
                required_gates: vec![
                    "human design review".to_string(),
                    "cargo check".to_string(),
                    "cargo clippy -- -D warnings".to_string(),
                    "behavior evals when configured".to_string(),
                ],
            });
        }

        if file.largest_function_lines >= 80 {
            candidates.push(RefactorCandidate {
                id: format!("plan-extract-function-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::ExtractFunctionCandidate,
                title: "Extract long function".to_string(),
                rationale: format!(
                    "Largest function in {} is {} lines. Extract only with behavior coverage in place.",
                    file_path, file.largest_function_lines
                ),
                file: file_path.clone(),
                line: 1,
                risk: RefactorRiskLevel::Medium,
                status: RefactorCandidateStatus::PlanOnly,
                public_api_impact: file.public_item_count > 0,
                apply_command: None,
                required_gates: vec![
                    "targeted tests or behavior evals".to_string(),
                    "cargo check".to_string(),
                    "cargo clippy -- -D warnings".to_string(),
                ],
            });
        }

        if file.public_item_count > 0 {
            candidates.push(RefactorCandidate {
                id: format!("plan-public-api-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::PublicApiReview,
                title: "Protect public API before refactoring".to_string(),
                rationale: format!(
                    "{} exposes {} public item(s). Treat signature changes as semver-impacting.",
                    file_path, file.public_item_count
                ),
                file: file_path,
                line: 1,
                risk: RefactorRiskLevel::Medium,
                status: RefactorCandidateStatus::PlanOnly,
                public_api_impact: true,
                apply_command: None,
                required_gates: vec![
                    "public API review".to_string(),
                    "docs and changelog review for exported changes".to_string(),
                ],
            });
        }
    }

    candidates
}

fn required_gates(has_behavior_spec: bool) -> Vec<String> {
    let mut gates = vec![
        "cargo check".to_string(),
        "cargo clippy -- -D warnings".to_string(),
        "review plan artifact before applying".to_string(),
    ];
    if has_behavior_spec {
        gates.push("behavior eval spec must pass in isolation and after apply".to_string());
    }
    gates
}

fn apply_command(file: &str, config: &RefactorPlanConfig) -> String {
    let mut command = format!("mdx-rust improve {} --apply", shell_word_str(file));
    if let Some(policy) = &config.policy_path {
        command.push_str(&format!(" --policy {}", shell_word_path(policy)));
    }
    if let Some(eval_spec) = &config.behavior_spec_path {
        command.push_str(&format!(" --eval-spec {}", shell_word_path(eval_spec)));
    }
    command
}

fn shell_word_path(path: &Path) -> String {
    shell_word_str(&path.display().to_string())
}

fn shell_word_str(value: &str) -> String {
    if value
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '/' | '.' | '_' | '-' | ':'))
    {
        value.to_string()
    } else {
        format!("'{}'", value.replace('\'', "'\\''"))
    }
}

fn plan_id(
    root: &Path,
    config: &RefactorPlanConfig,
    impact: &RefactorImpactSummary,
    candidates: &[RefactorCandidate],
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.target).as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.policy_path).as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.behavior_spec_path).as_bytes());
    bytes.extend_from_slice(format!("{impact:?}").as_bytes());
    bytes.extend_from_slice(format!("{candidates:?}").as_bytes());
    stable_hash_hex(&bytes)
}

fn refactor_plan_hash(plan: &RefactorPlan) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(plan.schema_version.as_bytes());
    bytes.extend_from_slice(plan.plan_id.as_bytes());
    bytes.extend_from_slice(plan.root.as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.target).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.impact).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.source_snapshots).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.module_edges).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.candidates).as_bytes());
    stable_hash_hex(&bytes)
}

fn candidate_hash(candidate: &RefactorCandidate) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(candidate.id.as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.recipe).as_bytes());
    bytes.extend_from_slice(candidate.title.as_bytes());
    bytes.extend_from_slice(candidate.rationale.as_bytes());
    bytes.extend_from_slice(candidate.file.as_bytes());
    bytes.extend_from_slice(candidate.line.to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.risk).as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.status).as_bytes());
    bytes.extend_from_slice(candidate.public_api_impact.to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.apply_command).as_bytes());
    stable_hash_hex(&bytes)
}

fn source_snapshots(
    root: &Path,
    files: &[RefactorFileSummary],
) -> anyhow::Result<Vec<SourceSnapshot>> {
    let mut snapshots = Vec::new();
    for file in files {
        let content = std::fs::read(root.join(&file.file))?;
        snapshots.push(SourceSnapshot {
            file: file.file.display().to_string(),
            hash: stable_hash_hex(&content),
        });
    }
    Ok(snapshots)
}

fn stale_source_files(
    root: &Path,
    snapshots: &[SourceSnapshot],
) -> anyhow::Result<Vec<StaleSourceFile>> {
    let mut stale = Vec::new();
    for snapshot in snapshots {
        let rel = safe_relative_path(&snapshot.file)?;
        let actual_hash = std::fs::read(root.join(&rel))
            .map(|content| stable_hash_hex(&content))
            .unwrap_or_else(|_| "<missing>".to_string());
        if actual_hash != snapshot.hash {
            stale.push(StaleSourceFile {
                file: snapshot.file.clone(),
                expected_hash: snapshot.hash.clone(),
                actual_hash,
            });
        }
    }
    Ok(stale)
}

fn safe_relative_path(value: &str) -> anyhow::Result<PathBuf> {
    let path = PathBuf::from(value);
    if path.is_absolute()
        || path.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
    {
        anyhow::bail!("refactor plan contains unscoped path: {value}");
    }
    Ok(path)
}

fn sanitize_id(value: &str) -> String {
    value
        .chars()
        .map(|ch| if ch.is_ascii_alphanumeric() { ch } else { '-' })
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

fn persist_refactor_plan(artifact_root: &Path, plan: &RefactorPlan) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("plans");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!("refactor-plan-{millis}-{}.json", plan.plan_id)))
}

fn persist_apply_run(
    artifact_root: Option<&Path>,
    mut run: RefactorApplyRun,
) -> anyhow::Result<RefactorApplyRun> {
    if let Some(artifact_root) = artifact_root {
        let dir = artifact_root.join("plans");
        std::fs::create_dir_all(&dir)?;
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let path = dir.join(format!(
            "apply-plan-{millis}-{}-{}.json",
            sanitize_id(&run.plan_id),
            sanitize_id(&run.candidate_id)
        ));
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&run)?)?;
    }
    Ok(run)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn refactor_plan_points_patchable_changes_to_improve() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "plan-fixture"
version = "0.1.0"
edition = "2021"

[dependencies]
anyhow = "1"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"pub fn load_config() -> anyhow::Result<String> {
    let content = std::fs::read_to_string("missing.toml").unwrap();
    Ok(content)
}
"#,
        )
        .unwrap();

        let plan = build_refactor_plan(
            dir.path(),
            None,
            &RefactorPlanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                behavior_spec_path: Some(PathBuf::from(".mdx-rust/evals.json")),
                ..RefactorPlanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(plan.schema_version, "0.5");
        assert!(plan.candidates.iter().any(|candidate| candidate.status
            == RefactorCandidateStatus::ApplyViaImprove
            && candidate
                .apply_command
                .as_deref()
                .is_some_and(|command| command.contains("--eval-spec"))));
    }
}
