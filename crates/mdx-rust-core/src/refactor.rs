//! Plan-first guardrailed refactoring.
//!
//! v1.0 beta keeps auditable plans as the mutation boundary and adds file/function
//! evidence plus security posture to the safe executable subset.

use crate::contracts::{scan_contracts, ContractRecommendation, ContractRun, ContractScanConfig};
use crate::eval::stable_hash_hex;
use crate::evidence::{load_latest_evidence_for_root, EvidenceArtifactRef, EvidenceRun};
use crate::hardening::{
    run_hardening, workspace_summary, HardeningConfig, HardeningRun, WorkspaceSummary,
};
use crate::performance::{
    scan_performance, PerformanceFinding, PerformanceRun, PerformanceScanConfig,
};
use crate::policy::{load_project_policy, ProjectPolicy};
use crate::repo_context::{
    build_noise_filter, build_repo_map, NoiseFilter, RepoMap, RepoMapConfig,
};
use crate::security::{audit_agent, AuditFinding, AuditSeverity};
use mdx_rust_analysis::{
    analyze_hardening, analyze_refactor, HardeningAnalyzeConfig, HardeningEvidenceDepth,
    HardeningFinding, ModuleEdge, RefactorAnalyzeConfig, RefactorFileSummary,
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

#[derive(Debug, Clone)]
pub struct RefactorBatchApplyConfig {
    pub plan_path: PathBuf,
    pub apply: bool,
    pub allow_public_api_impact: bool,
    pub validation_timeout: Duration,
    pub max_candidates: usize,
    pub max_tier: RecipeTier,
    pub min_evidence: EvidenceGrade,
}

#[derive(Debug, Clone)]
pub struct CodebaseMapConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for CodebaseMapConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            max_files: 250,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AutopilotConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub apply: bool,
    pub max_files: usize,
    pub max_passes: usize,
    pub max_candidates: usize,
    pub validation_timeout: Duration,
    pub allow_public_api_impact: bool,
    pub max_tier: RecipeTier,
    pub min_evidence: EvidenceGrade,
    pub budget: Option<Duration>,
}

impl Default for AutopilotConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            apply: false,
            max_files: 250,
            max_passes: 3,
            max_candidates: 25,
            validation_timeout: Duration::from_secs(180),
            allow_public_api_impact: false,
            max_tier: RecipeTier::Tier1,
            min_evidence: EvidenceGrade::Compiled,
            budget: None,
        }
    }
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
    pub evidence: EvidenceSummary,
    pub measured_evidence: Option<EvidenceArtifactRef>,
    #[serde(default)]
    pub security: SecurityPostureSummary,
    #[serde(default)]
    pub contracts: ContractPostureSummary,
    #[serde(default)]
    pub performance: PerformancePostureSummary,
    #[serde(default)]
    pub autonomy: AutonomyReadiness,
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
pub struct CodebaseMap {
    pub schema_version: String,
    pub map_id: String,
    pub map_hash: String,
    pub root: String,
    pub target: Option<String>,
    pub workspace: WorkspaceSummary,
    pub policy: Option<ProjectPolicy>,
    pub behavior_spec: Option<String>,
    pub evidence: EvidenceSummary,
    pub measured_evidence: Option<EvidenceArtifactRef>,
    #[serde(default)]
    pub security: SecurityPostureSummary,
    #[serde(default)]
    pub contracts: ContractPostureSummary,
    #[serde(default)]
    pub performance: PerformancePostureSummary,
    #[serde(default)]
    pub autonomy: AutonomyReadiness,
    pub quality: CodebaseQualitySummary,
    pub capability_gates: Vec<CapabilityGate>,
    pub impact: RefactorImpactSummary,
    pub files: Vec<RefactorFileSummary>,
    pub module_edges: Vec<ModuleEdge>,
    pub findings: Vec<HardeningFinding>,
    pub recommended_actions: Vec<String>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CodebaseQualitySummary {
    pub grade: CodebaseQualityGrade,
    pub debt_score: u8,
    #[serde(default)]
    pub security_score: u8,
    pub patchable_findings: usize,
    pub review_only_findings: usize,
    pub public_api_pressure: usize,
    pub oversized_files: usize,
    pub oversized_functions: usize,
    pub test_coverage_signal: TestCoverageSignal,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceSummary {
    pub grade: EvidenceGrade,
    pub max_autonomous_tier: u8,
    pub analysis_depth: EvidenceAnalysisDepth,
    pub signals: Vec<EvidenceSignal>,
    #[serde(default)]
    pub profiled_files: usize,
    pub unlocked_recipe_tiers: Vec<String>,
    pub unlock_suggestions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum EvidenceAnalysisDepth {
    None,
    Mechanical,
    BoundaryAware,
    Structural,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvidenceSignal {
    pub id: String,
    pub label: String,
    pub present: bool,
    pub detail: String,
}

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum EvidenceGrade {
    None,
    Compiled,
    Tested,
    Covered,
    Hardened,
    Proven,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum CodebaseQualityGrade {
    Excellent,
    Good,
    NeedsWork,
    HighRisk,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum TestCoverageSignal {
    Present,
    Sparse,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CapabilityGate {
    pub id: String,
    pub label: String,
    pub available: bool,
    pub command: String,
    pub purpose: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CandidateEvidenceContext {
    pub grade: EvidenceGrade,
    #[serde(default)]
    pub status: CandidateEvidenceStatus,
    pub source: String,
    pub profiled_file: Option<String>,
    pub signals: Vec<String>,
}

impl Default for CandidateEvidenceContext {
    fn default() -> Self {
        Self {
            grade: EvidenceGrade::None,
            status: CandidateEvidenceStatus::Unmeasured,
            source: "legacy artifact without candidate evidence context".to_string(),
            profiled_file: None,
            signals: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq, Default)]
pub enum CandidateEvidenceStatus {
    #[default]
    Unmeasured,
    Compiled,
    Tested,
    Covered,
    MutationBacked,
    Proven,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SecurityPostureSummary {
    pub score: u8,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub info: usize,
    pub top_findings: Vec<String>,
}

impl Default for SecurityPostureSummary {
    fn default() -> Self {
        Self {
            score: 100,
            high: 0,
            medium: 0,
            low: 0,
            info: 0,
            top_findings: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ContractPostureSummary {
    pub grade: ContractPostureGrade,
    pub function_count: usize,
    pub public_function_count: usize,
    pub functions_with_contracts: usize,
    pub public_functions_missing_contracts: usize,
    pub assertion_count: usize,
    pub top_recommendations: Vec<String>,
}

impl Default for ContractPostureSummary {
    fn default() -> Self {
        Self {
            grade: ContractPostureGrade::Unknown,
            function_count: 0,
            public_function_count: 0,
            functions_with_contracts: 0,
            public_functions_missing_contracts: 0,
            assertion_count: 0,
            top_recommendations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ContractPostureGrade {
    Strong,
    Partial,
    Weak,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PerformancePostureSummary {
    pub score: u8,
    pub finding_count: usize,
    pub high: usize,
    pub medium: usize,
    pub low: usize,
    pub top_findings: Vec<String>,
    pub recommendations: Vec<String>,
}

impl Default for PerformancePostureSummary {
    fn default() -> Self {
        Self {
            score: 100,
            finding_count: 0,
            high: 0,
            medium: 0,
            low: 0,
            top_findings: Vec::new(),
            recommendations: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutonomyReadiness {
    pub grade: AutonomyReadinessGrade,
    pub max_safe_tier: RecipeTier,
    pub executable_candidates: usize,
    pub review_only_candidates: usize,
    pub blocked_candidates: usize,
    pub blockers: Vec<String>,
    pub recommended_command: Option<String>,
}

impl Default for AutonomyReadiness {
    fn default() -> Self {
        Self {
            grade: AutonomyReadinessGrade::Blocked,
            max_safe_tier: RecipeTier::Tier1,
            executable_candidates: 0,
            review_only_candidates: 0,
            blocked_candidates: 0,
            blockers: Vec::new(),
            recommended_command: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AutonomyReadinessGrade {
    Blocked,
    ReviewOnly,
    Tier1Ready,
    Tier2Ready,
    Tier3Planning,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CandidateAutonomyDecision {
    pub decision: AutonomyDecision,
    pub reasons: Vec<String>,
}

impl Default for CandidateAutonomyDecision {
    fn default() -> Self {
        Self {
            decision: AutonomyDecision::ReviewOnly,
            reasons: vec!["legacy artifact without explicit autonomy decision".to_string()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AutonomyDecision {
    Allowed,
    Blocked,
    ReviewOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutopilotRun {
    pub schema_version: String,
    pub run_id: String,
    pub root: String,
    pub target: Option<String>,
    pub mode: RefactorApplyMode,
    pub status: AutopilotStatus,
    pub budget_seconds: Option<u64>,
    pub max_passes: usize,
    pub max_candidates_per_pass: usize,
    pub quality_before: CodebaseQualitySummary,
    pub quality_after: Option<CodebaseQualitySummary>,
    pub evidence: EvidenceSummary,
    pub measured_evidence: Option<EvidenceArtifactRef>,
    pub execution_summary: AutopilotExecutionSummary,
    pub passes: Vec<AutopilotPass>,
    pub total_planned_candidates: usize,
    pub total_executed_candidates: usize,
    pub total_skipped_candidates: usize,
    pub budget_exhausted: bool,
    pub note: String,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutopilotExecutionSummary {
    pub plans_created: usize,
    pub executable_candidates_seen: usize,
    pub validated_transactions: usize,
    pub applied_transactions: usize,
    pub blocked_or_plan_only_candidates: usize,
    pub evidence_grade: EvidenceGrade,
    pub analysis_depth: EvidenceAnalysisDepth,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AutopilotPass {
    pub pass_index: usize,
    pub plan_id: String,
    pub plan_hash: String,
    pub plan_artifact_path: Option<String>,
    pub planned_candidates: usize,
    pub executable_candidates: usize,
    pub batch: Option<RefactorBatchApplyRun>,
    pub status: AutopilotPassStatus,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AutopilotStatus {
    Reviewed,
    Applied,
    PartiallyApplied,
    NoExecutableCandidates,
    Rejected,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AutopilotPassStatus {
    Planned,
    Reviewed,
    Applied,
    PartiallyApplied,
    NoExecutableCandidates,
    Rejected,
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
    pub tier: RecipeTier,
    pub required_evidence: EvidenceGrade,
    pub evidence_satisfied: bool,
    #[serde(default)]
    pub evidence_context: CandidateEvidenceContext,
    #[serde(default)]
    pub autonomy: CandidateAutonomyDecision,
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

#[derive(
    Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq, PartialOrd, Ord,
)]
pub enum RecipeTier {
    Tier1,
    Tier2,
    Tier3,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorRecipe {
    BorrowParameterTightening,
    ClonePressureReview,
    ContractCoverageReview,
    ContextualErrorHardening,
    ErrorContextPropagation,
    ExtractFunctionCandidate,
    IteratorCloned,
    LenCheckIsEmpty,
    LongFunctionReview,
    MustUsePublicReturn,
    OptionMatchContextPropagation,
    OptionContextPropagation,
    RepeatedStringLiteralConst,
    SecurityBoundaryReview,
    SplitModuleCandidate,
    BoundaryValidationReview,
    PublicApiReview,
    PerformanceHotspotReview,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeCatalog {
    pub schema_version: String,
    pub recipes: Vec<RecipeSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RecipeSpec {
    pub id: String,
    pub recipe: RefactorRecipe,
    pub tier: RecipeTier,
    pub required_evidence: EvidenceGrade,
    pub executable: bool,
    pub risk: RefactorRiskLevel,
    pub mutation_path: String,
    pub description: String,
}

#[derive(Debug, Clone)]
pub struct EvolutionScorecardConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for EvolutionScorecardConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            max_files: 250,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvolutionScorecard {
    pub schema_version: String,
    pub scorecard_id: String,
    pub root: String,
    pub target: Option<String>,
    pub readiness: AutonomyReadiness,
    pub map: CodebaseMap,
    pub plan: RefactorPlan,
    pub recipes: RecipeCatalog,
    pub next_commands: Vec<String>,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentReadyReport {
    pub schema_version: String,
    pub product_version: String,
    pub status: AgentReadyStatus,
    pub target: Option<String>,
    pub readiness: AutonomyReadiness,
    pub evidence: EvidenceSummary,
    pub quality: CodebaseQualitySummary,
    pub security: SecurityPostureSummary,
    pub contracts: ContractPostureSummary,
    pub performance: PerformancePostureSummary,
    pub agent_contract: AgentReadyContractRefs,
    pub next_commands: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum AgentReadyStatus {
    Ready,
    Review,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentReadyContractRefs {
    pub discovery: String,
    pub runtime: String,
    pub scorecard_artifact: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvolutionBriefConfig {
    pub target: Option<PathBuf>,
    pub policy_path: Option<PathBuf>,
    pub behavior_spec_path: Option<PathBuf>,
    pub max_files: usize,
}

impl Default for EvolutionBriefConfig {
    fn default() -> Self {
        Self {
            target: None,
            policy_path: None,
            behavior_spec_path: None,
            max_files: 250,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EvolutionBrief {
    pub schema_version: String,
    pub brief_id: String,
    pub root: String,
    pub target: Option<String>,
    pub repo_map: RepoMap,
    pub noise_filter: NoiseFilter,
    pub contracts: ContractPostureSummary,
    pub performance: PerformancePostureSummary,
    pub scorecard: EvolutionScorecard,
    pub recommended_sequence: Vec<String>,
    pub artifact_path: Option<String>,
}

pub fn recipe_catalog() -> RecipeCatalog {
    macro_rules! spec {
        (
            $id:expr,
            $recipe:expr,
            $tier:expr,
            $required_evidence:expr,
            $executable:expr,
            $risk:expr,
            $mutation_path:expr,
            $description:expr $(,)?
        ) => {
            RecipeSpec {
                id: $id.to_string(),
                recipe: $recipe,
                tier: $tier,
                required_evidence: $required_evidence,
                executable: $executable,
                risk: $risk,
                mutation_path: $mutation_path.to_string(),
                description: $description.to_string(),
            }
        };
    }

    RecipeCatalog {
        schema_version: "1.0".to_string(),
        recipes: vec![
            spec!(
                "contextual-error-hardening",
                RefactorRecipe::ContextualErrorHardening,
                RecipeTier::Tier1,
                EvidenceGrade::Compiled,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction",
                "Replace panic-prone Result unwraps with contextual errors.",
            ),
            spec!(
                "error-context-propagation",
                RefactorRecipe::ErrorContextPropagation,
                RecipeTier::Tier1,
                EvidenceGrade::Compiled,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction",
                "Add context to boundary errors without changing public behavior.",
            ),
            spec!(
                "borrow-parameter-tightening",
                RefactorRecipe::BorrowParameterTightening,
                RecipeTier::Tier1,
                EvidenceGrade::Compiled,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction",
                "Prefer borrowed slice/string parameters in private functions.",
            ),
            spec!(
                "iterator-cloned-cleanup",
                RefactorRecipe::IteratorCloned,
                RecipeTier::Tier1,
                EvidenceGrade::Compiled,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction",
                "Move cloned calls to the narrower iterator position when mechanical.",
            ),
            spec!(
                "option-context-propagation",
                RefactorRecipe::OptionContextPropagation,
                RecipeTier::Tier2,
                EvidenceGrade::Covered,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction with covered evidence",
                "Convert Option ok_or string boundaries to anyhow Context under coverage gates.",
            ),
            spec!(
                "len-check-is-empty",
                RefactorRecipe::LenCheckIsEmpty,
                RecipeTier::Tier2,
                EvidenceGrade::Covered,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction with covered evidence",
                "Convert zero-length comparisons to is_empty for clarity.",
            ),
            spec!(
                "repeated-string-literal-const",
                RefactorRecipe::RepeatedStringLiteralConst,
                RecipeTier::Tier2,
                EvidenceGrade::Covered,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction with covered evidence",
                "Extract repeated local string literals into a constant.",
            ),
            spec!(
                "clone-pressure-review",
                RefactorRecipe::ClonePressureReview,
                RecipeTier::Tier3,
                EvidenceGrade::Hardened,
                false,
                RefactorRiskLevel::Medium,
                "plan only",
                "Identify clone-heavy code that needs semantic review before rewriting.",
            ),
            spec!(
                "option-match-context-propagation",
                RefactorRecipe::OptionMatchContextPropagation,
                RecipeTier::Tier3,
                EvidenceGrade::Hardened,
                true,
                RefactorRiskLevel::Low,
                "hardening transaction with hardened evidence",
                "Collapse simple Option match error boundaries into anyhow Context under hardened evidence gates.",
            ),
            spec!(
                "contract-coverage-review",
                RefactorRecipe::ContractCoverageReview,
                RecipeTier::Tier2,
                EvidenceGrade::Tested,
                false,
                RefactorRiskLevel::Medium,
                "plan only",
                "Surface public functions that need explicit behavior contracts before semantic changes.",
            ),
            spec!(
                "performance-hotspot-review",
                RefactorRecipe::PerformanceHotspotReview,
                RecipeTier::Tier2,
                EvidenceGrade::Tested,
                false,
                RefactorRiskLevel::Medium,
                "plan only",
                "Surface static performance pressure that needs benchmarks before rewrites.",
            ),
            spec!(
                "extract-function",
                RefactorRecipe::ExtractFunctionCandidate,
                RecipeTier::Tier2,
                EvidenceGrade::Covered,
                false,
                RefactorRiskLevel::Medium,
                "plan only",
                "Stage long functions for behavior-gated extraction.",
            ),
            spec!(
                "split-module",
                RefactorRecipe::SplitModuleCandidate,
                RecipeTier::Tier2,
                EvidenceGrade::Covered,
                false,
                RefactorRiskLevel::Medium,
                "plan only",
                "Stage oversized modules for human-reviewed decomposition.",
            ),
            spec!(
                "security-boundary-review",
                RefactorRecipe::SecurityBoundaryReview,
                RecipeTier::Tier2,
                EvidenceGrade::Tested,
                false,
                RefactorRiskLevel::High,
                "plan only",
                "Surface process, unsafe, and boundary risks before autonomous work expands.",
            ),
        ],
    }
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorBatchApplyRun {
    pub schema_version: String,
    pub root: String,
    pub plan_path: String,
    pub plan_id: String,
    pub plan_hash: String,
    pub mode: RefactorApplyMode,
    pub status: RefactorBatchApplyStatus,
    pub public_api_impact_allowed: bool,
    pub max_candidates: usize,
    pub requested_candidates: usize,
    pub executed_candidates: usize,
    pub skipped_candidates: usize,
    pub steps: Vec<RefactorBatchCandidateRun>,
    pub note: String,
    pub artifact_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct RefactorBatchCandidateRun {
    pub candidate_id: String,
    pub candidate_hash: Option<String>,
    pub file: String,
    pub status: RefactorApplyStatus,
    pub stale_file: Option<StaleSourceFile>,
    pub hardening_run: Option<HardeningRun>,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorApplyMode {
    Review,
    Apply,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum RefactorBatchApplyStatus {
    Reviewed,
    Applied,
    PartiallyApplied,
    Rejected,
    StalePlan,
    NoExecutableCandidates,
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
    let measured_evidence = load_latest_evidence_for_root(artifact_root, &root)?;
    let hardening = analyze_hardening(
        &root,
        HardeningAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
            max_recipe_tier: measured_hardening_tier(measured_evidence.as_ref()),
            evidence_depth: hardening_depth_for_evidence(measured_evidence.as_ref()),
        },
    )?;
    let policy = load_project_policy(&root, config.policy_path.as_deref())?;
    let audit_scope = audit_scope_path(&root, config.target.as_deref());
    let audit = audit_agent(&audit_scope)?;
    let security = security_posture_summary(&audit.findings);
    let contract_run = scan_contracts(
        &root,
        &ContractScanConfig {
            target: config.target.clone(),
            max_files: config.max_files,
        },
    )?;
    let contracts = contract_posture_summary(&contract_run);
    let performance_run = scan_performance(
        &root,
        &PerformanceScanConfig {
            target: config.target.clone(),
            max_files: config.max_files,
        },
    )?;
    let performance = performance_posture_summary(&performance_run);
    let workspace = workspace_summary(&root);
    let behavior_spec = config
        .behavior_spec_path
        .as_ref()
        .map(|path| path.display().to_string());
    let capability_gates = capability_gates();
    let evidence = summarize_evidence(
        &workspace,
        &refactor.files,
        &capability_gates,
        config.behavior_spec_path.is_some(),
        measured_evidence.as_ref(),
    );
    let impact = summarize_impact(
        &refactor.files,
        refactor.module_edges.len(),
        &hardening.findings,
        hardening.changes.len(),
    );
    let mut candidates = Vec::new();
    candidates.extend(hardening_candidates(
        &hardening.findings,
        config,
        &evidence,
        measured_evidence.as_ref(),
    ));
    candidates.extend(structural_candidates(
        &refactor.files,
        &evidence,
        measured_evidence.as_ref(),
    ));
    candidates.extend(security_candidates(
        &audit.findings,
        &evidence,
        measured_evidence.as_ref(),
    ));
    candidates.extend(contract_candidates(
        &contract_run,
        &evidence,
        measured_evidence.as_ref(),
    ));
    candidates.extend(performance_candidates(
        &performance_run,
        &evidence,
        measured_evidence.as_ref(),
    ));
    annotate_candidate_autonomy(&mut candidates, &evidence, &security);
    let autonomy = autonomy_readiness(&evidence, &security, &candidates);
    for candidate in &mut candidates {
        candidate.candidate_hash = candidate_hash(candidate);
    }
    candidates.sort_by(|left, right| left.id.cmp(&right.id));
    let source_snapshots = source_snapshots(&root, &refactor.files)?;

    let required_gates = required_gates(config.behavior_spec_path.is_some());
    let non_goals = vec![
        "No broad API-changing refactors without explicit human allowance.".to_string(),
        "No public API changes without explicit human review.".to_string(),
        "No plan candidate may bypass improve/apply validation gates.".to_string(),
    ];

    let plan_id = plan_id(&root, config, &impact, &candidates);
    let mut plan = RefactorPlan {
        schema_version: "1.0".to_string(),
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
        evidence,
        measured_evidence: measured_evidence.as_ref().map(EvidenceArtifactRef::from),
        security,
        contracts,
        performance,
        autonomy,
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

pub fn build_codebase_map(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &CodebaseMapConfig,
) -> anyhow::Result<CodebaseMap> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let refactor = analyze_refactor(
        &root,
        RefactorAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
        },
    )?;
    let measured_evidence = load_latest_evidence_for_root(artifact_root, &root)?;
    let hardening = analyze_hardening(
        &root,
        HardeningAnalyzeConfig {
            target: config.target.as_deref(),
            max_files: config.max_files,
            max_recipe_tier: measured_hardening_tier(measured_evidence.as_ref()),
            evidence_depth: hardening_depth_for_evidence(measured_evidence.as_ref()),
        },
    )?;
    let policy = load_project_policy(&root, config.policy_path.as_deref())?;
    let audit_scope = audit_scope_path(&root, config.target.as_deref());
    let audit = audit_agent(&audit_scope)?;
    let security = security_posture_summary(&audit.findings);
    let contract_run = scan_contracts(
        &root,
        &ContractScanConfig {
            target: config.target.clone(),
            max_files: config.max_files,
        },
    )?;
    let contracts = contract_posture_summary(&contract_run);
    let performance_run = scan_performance(
        &root,
        &PerformanceScanConfig {
            target: config.target.clone(),
            max_files: config.max_files,
        },
    )?;
    let performance = performance_posture_summary(&performance_run);
    let workspace = workspace_summary(&root);
    let behavior_spec = config
        .behavior_spec_path
        .as_ref()
        .map(|path| path.display().to_string());
    let capability_gates = capability_gates();
    let evidence = summarize_evidence(
        &workspace,
        &refactor.files,
        &capability_gates,
        config.behavior_spec_path.is_some(),
        measured_evidence.as_ref(),
    );
    let impact = summarize_impact(
        &refactor.files,
        refactor.module_edges.len(),
        &hardening.findings,
        hardening.changes.len(),
    );
    let mut readiness_candidates = Vec::new();
    readiness_candidates.extend(hardening_candidates(
        &hardening.findings,
        &RefactorPlanConfig {
            target: config.target.clone(),
            policy_path: config.policy_path.clone(),
            behavior_spec_path: config.behavior_spec_path.clone(),
            max_files: config.max_files,
        },
        &evidence,
        measured_evidence.as_ref(),
    ));
    readiness_candidates.extend(structural_candidates(
        &refactor.files,
        &evidence,
        measured_evidence.as_ref(),
    ));
    readiness_candidates.extend(security_candidates(
        &audit.findings,
        &evidence,
        measured_evidence.as_ref(),
    ));
    readiness_candidates.extend(contract_candidates(
        &contract_run,
        &evidence,
        measured_evidence.as_ref(),
    ));
    readiness_candidates.extend(performance_candidates(
        &performance_run,
        &evidence,
        measured_evidence.as_ref(),
    ));
    annotate_candidate_autonomy(&mut readiness_candidates, &evidence, &security);
    let autonomy = autonomy_readiness(&evidence, &security, &readiness_candidates);
    let quality = summarize_quality(&refactor.files, &hardening.findings, &impact, &security);
    let recommended_actions = recommended_actions(
        &quality,
        &impact,
        &capability_gates,
        &evidence,
        &security,
        &contracts,
        &performance,
    );
    let map_id = codebase_map_id(&root, config, &quality, &impact);
    let mut map = CodebaseMap {
        schema_version: "1.0".to_string(),
        map_id,
        map_hash: String::new(),
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        workspace,
        policy,
        behavior_spec,
        evidence,
        measured_evidence: measured_evidence.as_ref().map(EvidenceArtifactRef::from),
        security,
        contracts,
        performance,
        autonomy,
        quality,
        capability_gates,
        impact,
        files: refactor.files,
        module_edges: refactor.module_edges,
        findings: hardening.findings,
        recommended_actions,
        artifact_path: None,
    };
    map.map_hash = codebase_map_hash(&map);

    if let Some(artifact_root) = artifact_root {
        let path = persist_codebase_map(artifact_root, &map)?;
        map.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&map)?)?;
    }

    Ok(map)
}

pub fn build_evolution_scorecard(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &EvolutionScorecardConfig,
) -> anyhow::Result<EvolutionScorecard> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let map = build_codebase_map(
        &root,
        artifact_root,
        &CodebaseMapConfig {
            target: config.target.clone(),
            policy_path: config.policy_path.clone(),
            behavior_spec_path: config.behavior_spec_path.clone(),
            max_files: config.max_files,
        },
    )?;
    let plan = build_refactor_plan(
        &root,
        artifact_root,
        &RefactorPlanConfig {
            target: config.target.clone(),
            policy_path: config.policy_path.clone(),
            behavior_spec_path: config.behavior_spec_path.clone(),
            max_files: config.max_files,
        },
    )?;
    let recipes = recipe_catalog();
    let readiness = plan.autonomy.clone();
    let next_commands = scorecard_next_commands(&readiness, &plan);
    let scorecard_id = evolution_scorecard_id(&root, config, &map, &plan);
    let mut scorecard = EvolutionScorecard {
        schema_version: "1.0".to_string(),
        scorecard_id,
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        readiness,
        map,
        plan,
        recipes,
        next_commands,
        artifact_path: None,
    };

    if let Some(artifact_root) = artifact_root {
        let path = persist_evolution_scorecard(artifact_root, &scorecard)?;
        scorecard.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&scorecard)?)?;
    }

    Ok(scorecard)
}

pub fn build_evolution_brief(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &EvolutionBriefConfig,
) -> anyhow::Result<EvolutionBrief> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let repo_map = build_repo_map(
        &root,
        &RepoMapConfig {
            target: config.target.clone(),
            max_depth: 3,
            max_dirs: config.max_files.clamp(20, 250),
        },
    )?;
    let noise_filter = build_noise_filter(&root);
    let scorecard = build_evolution_scorecard(
        &root,
        artifact_root,
        &EvolutionScorecardConfig {
            target: config.target.clone(),
            policy_path: config.policy_path.clone(),
            behavior_spec_path: config.behavior_spec_path.clone(),
            max_files: config.max_files,
        },
    )?;
    let contracts = scorecard.map.contracts.clone();
    let performance = scorecard.map.performance.clone();
    let recommended_sequence = evolution_brief_sequence(config.target.as_deref(), &scorecard);
    let brief_id = evolution_brief_id(&root, config, &scorecard);
    let mut brief = EvolutionBrief {
        schema_version: "1.0".to_string(),
        brief_id,
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        repo_map,
        noise_filter,
        contracts,
        performance,
        scorecard,
        recommended_sequence,
        artifact_path: None,
    };

    if let Some(artifact_root) = artifact_root {
        let path = persist_evolution_brief(artifact_root, &brief)?;
        brief.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&brief)?)?;
    }

    Ok(brief)
}

pub fn run_autopilot(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &AutopilotConfig,
) -> anyhow::Result<AutopilotRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let map_config = CodebaseMapConfig {
        target: config.target.clone(),
        policy_path: config.policy_path.clone(),
        behavior_spec_path: config.behavior_spec_path.clone(),
        max_files: config.max_files,
    };
    let before_map = build_codebase_map(&root, artifact_root, &map_config)?;
    let evidence = before_map.evidence.clone();
    let quality_before = before_map.quality.clone();
    let mode = if config.apply {
        RefactorApplyMode::Apply
    } else {
        RefactorApplyMode::Review
    };
    let mut run = AutopilotRun {
        schema_version: "1.0".to_string(),
        run_id: autopilot_run_id(&root, config, &before_map),
        root: root.display().to_string(),
        target: config
            .target
            .as_ref()
            .map(|path| path.display().to_string()),
        mode,
        status: AutopilotStatus::NoExecutableCandidates,
        budget_seconds: config.budget.map(|duration| duration.as_secs()),
        max_passes: config.max_passes,
        max_candidates_per_pass: config.max_candidates,
        quality_before,
        quality_after: None,
        evidence,
        measured_evidence: before_map.measured_evidence.clone(),
        execution_summary: AutopilotExecutionSummary {
            plans_created: 0,
            executable_candidates_seen: 0,
            validated_transactions: 0,
            applied_transactions: 0,
            blocked_or_plan_only_candidates: 0,
            evidence_grade: before_map.evidence.grade,
            analysis_depth: before_map.evidence.analysis_depth.clone(),
        },
        passes: Vec::new(),
        total_planned_candidates: 0,
        total_executed_candidates: 0,
        total_skipped_candidates: 0,
        budget_exhausted: false,
        note: String::new(),
        artifact_path: None,
    };

    let started_at = std::time::Instant::now();
    let pass_count = config.max_passes.max(1);
    for pass_index in 1..=pass_count {
        if config
            .budget
            .is_some_and(|budget| started_at.elapsed() >= budget)
        {
            run.budget_exhausted = true;
            break;
        }
        let plan = build_refactor_plan(
            &root,
            artifact_root,
            &RefactorPlanConfig {
                target: config.target.clone(),
                policy_path: config.policy_path.clone(),
                behavior_spec_path: config.behavior_spec_path.clone(),
                max_files: config.max_files,
            },
        )?;
        let executable = count_executable_candidates(
            &plan,
            config.allow_public_api_impact,
            config.max_candidates,
            config.max_tier,
            config.min_evidence,
        );
        run.total_planned_candidates += plan.candidates.len();

        let mut pass = AutopilotPass {
            pass_index,
            plan_id: plan.plan_id.clone(),
            plan_hash: plan.plan_hash.clone(),
            plan_artifact_path: plan.artifact_path.clone(),
            planned_candidates: plan.candidates.len(),
            executable_candidates: executable,
            batch: None,
            status: AutopilotPassStatus::Planned,
            note: String::new(),
        };

        if executable == 0 {
            pass.status = AutopilotPassStatus::NoExecutableCandidates;
            pass.note = "no executable low-risk candidates remain for this pass".to_string();
            run.passes.push(pass);
            break;
        }

        let Some(plan_path) = plan.artifact_path.as_ref() else {
            pass.status = AutopilotPassStatus::Rejected;
            pass.note = "autopilot requires persisted plan artifacts before execution".to_string();
            run.passes.push(pass);
            break;
        };

        let mut validation_timeout = config.validation_timeout;
        if let Some(budget) = config.budget {
            let Some(remaining) = budget.checked_sub(started_at.elapsed()) else {
                run.budget_exhausted = true;
                pass.status = AutopilotPassStatus::NoExecutableCandidates;
                pass.note = "budget exhausted before execution could start".to_string();
                run.passes.push(pass);
                break;
            };
            if remaining.is_zero() {
                run.budget_exhausted = true;
                pass.status = AutopilotPassStatus::NoExecutableCandidates;
                pass.note = "budget exhausted before execution could start".to_string();
                run.passes.push(pass);
                break;
            }
            validation_timeout = validation_timeout.min(remaining);
        }

        let batch = apply_refactor_plan_batch(
            &root,
            artifact_root,
            &RefactorBatchApplyConfig {
                plan_path: PathBuf::from(plan_path),
                apply: config.apply,
                allow_public_api_impact: config.allow_public_api_impact,
                validation_timeout,
                max_candidates: config.max_candidates,
                max_tier: config.max_tier,
                min_evidence: config.min_evidence,
            },
        )?;
        if config
            .budget
            .is_some_and(|budget| started_at.elapsed() >= budget)
        {
            run.budget_exhausted = true;
        }
        run.total_executed_candidates += batch.executed_candidates;
        run.total_skipped_candidates += batch.skipped_candidates;
        pass.status = autopilot_pass_status(&batch.status);
        pass.note = batch.note.clone();
        let should_stop = !config.apply
            || matches!(
                batch.status,
                RefactorBatchApplyStatus::Rejected
                    | RefactorBatchApplyStatus::StalePlan
                    | RefactorBatchApplyStatus::NoExecutableCandidates
                    | RefactorBatchApplyStatus::PartiallyApplied
            )
            || batch.executed_candidates == 0;
        pass.batch = Some(batch);
        run.passes.push(pass);
        if should_stop {
            break;
        }
    }

    let after_map = if config.apply && run.total_executed_candidates > 0 {
        Some(build_codebase_map(&root, artifact_root, &map_config)?)
    } else {
        None
    };
    run.quality_after = after_map.map(|map| map.quality);
    run.status = autopilot_status(config.apply, &run.passes, run.total_executed_candidates);
    run.note = autopilot_note(&run);
    run.execution_summary = autopilot_execution_summary(&run);
    persist_autopilot_run(artifact_root, run)
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
        schema_version: "1.0".to_string(),
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

    if !candidate.evidence_satisfied {
        run.status = RefactorApplyStatus::Unsupported;
        run.note = format!(
            "candidate requires {:?} evidence but plan evidence is {:?}",
            candidate.required_evidence, plan.evidence.grade
        );
        return persist_apply_run(artifact_root, run);
    }

    if candidate.autonomy.decision != AutonomyDecision::Allowed {
        run.status = RefactorApplyStatus::Unsupported;
        run.note = format!(
            "candidate autonomy decision is {:?}: {}",
            candidate.autonomy.decision,
            candidate.autonomy.reasons.join("; ")
        );
        return persist_apply_run(artifact_root, run);
    }

    if candidate.status != RefactorCandidateStatus::ApplyViaImprove
        || !is_supported_mechanical_recipe(&candidate.recipe)
    {
        run.status = RefactorApplyStatus::Unsupported;
        run.note = "candidate is plan-only; no executable recipe is available yet".to_string();
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
            max_recipe_tier: recipe_tier_number(candidate.tier),
            evidence_depth: hardening_depth_for_grade(candidate.required_evidence),
            validation_timeout: config.validation_timeout,
        },
    )?;

    run.status = if config.apply {
        if hardening.outcome.applied {
            RefactorApplyStatus::Applied
        } else {
            RefactorApplyStatus::Rejected
        }
    } else if hardening.outcome.isolated_validation_passed {
        RefactorApplyStatus::Reviewed
    } else {
        RefactorApplyStatus::Rejected
    };
    run.note = format!(
        "executed candidate through hardening transaction; hardening status: {:?}",
        hardening.outcome.status
    );
    run.hardening_run = Some(hardening);
    persist_apply_run(artifact_root, run)
}

pub fn apply_refactor_plan_batch(
    root: &Path,
    artifact_root: Option<&Path>,
    config: &RefactorBatchApplyConfig,
) -> anyhow::Result<RefactorBatchApplyRun> {
    let root = root.canonicalize().unwrap_or_else(|_| root.to_path_buf());
    let plan_content = std::fs::read_to_string(&config.plan_path)?;
    let plan: RefactorPlan = serde_json::from_str(&plan_content)?;
    let mode = if config.apply {
        RefactorApplyMode::Apply
    } else {
        RefactorApplyMode::Review
    };
    let mut run = RefactorBatchApplyRun {
        schema_version: "1.0".to_string(),
        root: root.display().to_string(),
        plan_path: config.plan_path.display().to_string(),
        plan_id: plan.plan_id.clone(),
        plan_hash: plan.plan_hash.clone(),
        mode,
        status: RefactorBatchApplyStatus::Rejected,
        public_api_impact_allowed: config.allow_public_api_impact,
        max_candidates: config.max_candidates,
        requested_candidates: 0,
        executed_candidates: 0,
        skipped_candidates: 0,
        steps: Vec::new(),
        note: String::new(),
        artifact_path: None,
    };

    let actual_plan_hash = refactor_plan_hash(&plan);
    if actual_plan_hash != plan.plan_hash {
        run.status = RefactorBatchApplyStatus::Rejected;
        run.note = format!(
            "plan hash mismatch: expected {} but recomputed {}",
            plan.plan_hash, actual_plan_hash
        );
        return persist_batch_apply_run(artifact_root, run);
    }

    let initial_stale_files = stale_source_files(&root, &plan.source_snapshots)?;
    if !initial_stale_files.is_empty() {
        run.status = RefactorBatchApplyStatus::StalePlan;
        run.steps = initial_stale_files
            .into_iter()
            .map(|stale| RefactorBatchCandidateRun {
                candidate_id: String::new(),
                candidate_hash: None,
                file: stale.file.clone(),
                status: RefactorApplyStatus::StalePlan,
                stale_file: Some(stale),
                hardening_run: None,
                note: "source snapshot no longer matches the workspace".to_string(),
            })
            .collect();
        run.note =
            "plan source snapshots no longer match the workspace; re-run mdx-rust plan".to_string();
        return persist_batch_apply_run(artifact_root, run);
    }

    let queue = executable_candidate_queue(&plan, config);
    run.requested_candidates = queue.len();
    if queue.is_empty() {
        run.status = RefactorBatchApplyStatus::NoExecutableCandidates;
        run.note = "no executable low-risk candidates were available in the plan".to_string();
        return persist_batch_apply_run(artifact_root, run);
    }

    for candidate in queue {
        let mut step = RefactorBatchCandidateRun {
            candidate_id: candidate.id.clone(),
            candidate_hash: Some(candidate.candidate_hash.clone()),
            file: candidate.file.clone(),
            status: RefactorApplyStatus::Rejected,
            stale_file: None,
            hardening_run: None,
            note: String::new(),
        };

        let actual_candidate_hash = candidate_hash(candidate);
        if actual_candidate_hash != candidate.candidate_hash {
            step.note = format!(
                "candidate hash mismatch: expected {} but recomputed {}",
                candidate.candidate_hash, actual_candidate_hash
            );
            run.skipped_candidates += 1;
            run.steps.push(step);
            if config.apply {
                break;
            }
            continue;
        }

        if let Some(stale) = stale_file_for_candidate(&root, &plan, &candidate.file)? {
            step.status = RefactorApplyStatus::StalePlan;
            step.stale_file = Some(stale);
            step.note =
                "candidate source file changed after planning; re-run mdx-rust plan".to_string();
            run.skipped_candidates += 1;
            run.steps.push(step);
            if config.apply {
                break;
            }
            continue;
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
                max_recipe_tier: recipe_tier_number(candidate.tier),
                evidence_depth: hardening_depth_for_grade(candidate.required_evidence),
                validation_timeout: config.validation_timeout,
            },
        )?;

        step.status = if config.apply {
            if hardening.outcome.applied {
                RefactorApplyStatus::Applied
            } else {
                RefactorApplyStatus::Rejected
            }
        } else if hardening.outcome.isolated_validation_passed {
            RefactorApplyStatus::Reviewed
        } else {
            RefactorApplyStatus::Rejected
        };
        step.note = format!(
            "executed candidate through hardening transaction; hardening status: {:?}",
            hardening.outcome.status
        );
        step.hardening_run = Some(hardening);

        if matches!(
            step.status,
            RefactorApplyStatus::Reviewed | RefactorApplyStatus::Applied
        ) {
            run.executed_candidates += 1;
        } else {
            run.skipped_candidates += 1;
        }

        let failed_apply_step = config.apply && step.status != RefactorApplyStatus::Applied;
        run.steps.push(step);
        if failed_apply_step {
            break;
        }
    }

    run.status = batch_status(
        config.apply,
        run.executed_candidates,
        run.requested_candidates,
    );
    run.note = format!(
        "processed {} executable candidate(s); executed {}, skipped {}",
        run.requested_candidates, run.executed_candidates, run.skipped_candidates
    );
    persist_batch_apply_run(artifact_root, run)
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

fn summarize_quality(
    files: &[RefactorFileSummary],
    findings: &[HardeningFinding],
    impact: &RefactorImpactSummary,
    security: &SecurityPostureSummary,
) -> CodebaseQualitySummary {
    let patchable_findings = findings.iter().filter(|finding| finding.patchable).count();
    let review_only_findings = findings.len().saturating_sub(patchable_findings);
    let files_with_tests = files.iter().filter(|file| file.has_tests).count();
    let test_coverage_signal = if files.is_empty() {
        TestCoverageSignal::Unknown
    } else if files_with_tests > 0 {
        TestCoverageSignal::Present
    } else {
        TestCoverageSignal::Sparse
    };

    let mut score = 0usize;
    score += patchable_findings.saturating_mul(8);
    score += review_only_findings.saturating_mul(4);
    score += impact.oversized_files.saturating_mul(10);
    score += impact.oversized_functions.saturating_mul(7);
    score += impact.public_files.saturating_mul(2);
    score += (100usize.saturating_sub(security.score as usize)) / 2;
    if test_coverage_signal == TestCoverageSignal::Sparse {
        score += 12;
    }
    let debt_score = score.min(100) as u8;
    let grade = if debt_score >= 70 {
        CodebaseQualityGrade::HighRisk
    } else if debt_score >= 35 {
        CodebaseQualityGrade::NeedsWork
    } else if debt_score >= 10 {
        CodebaseQualityGrade::Good
    } else {
        CodebaseQualityGrade::Excellent
    };

    CodebaseQualitySummary {
        grade,
        debt_score,
        security_score: security.score,
        patchable_findings,
        review_only_findings,
        public_api_pressure: impact.public_item_count,
        oversized_files: impact.oversized_files,
        oversized_functions: impact.oversized_functions,
        test_coverage_signal,
    }
}

fn summarize_evidence(
    workspace: &WorkspaceSummary,
    files: &[RefactorFileSummary],
    gates: &[CapabilityGate],
    has_behavior_spec: bool,
    measured: Option<&EvidenceRun>,
) -> EvidenceSummary {
    let has_tests = files.iter().any(|file| file.has_tests);
    let has_nextest = gates
        .iter()
        .any(|gate| gate.id == "nextest" && gate.available);
    let has_coverage_tool = gates
        .iter()
        .any(|gate| gate.id == "llvm-cov" && gate.available);
    let has_mutation_tool = gates
        .iter()
        .any(|gate| gate.id == "mutants" && gate.available);

    let inferred_grade = if !workspace.cargo_metadata_available {
        EvidenceGrade::None
    } else if has_tests || has_behavior_spec || has_nextest {
        EvidenceGrade::Tested
    } else {
        EvidenceGrade::Compiled
    };
    let grade = measured.map(|run| run.grade).unwrap_or(inferred_grade);
    let max_autonomous_tier = max_tier_for_evidence(grade);
    let analysis_depth = measured
        .map(|run| run.analysis_depth.clone())
        .unwrap_or_else(|| analysis_depth_for_evidence(grade));

    let mut signals = vec![
        EvidenceSignal {
            id: "cargo-metadata".to_string(),
            label: "Cargo metadata".to_string(),
            present: workspace.cargo_metadata_available,
            detail: if workspace.cargo_metadata_available {
                "workspace can be inspected and compile gates can run".to_string()
            } else {
                "no Cargo metadata was available for this target".to_string()
            },
        },
        EvidenceSignal {
            id: "tests-or-behavior-evals".to_string(),
            label: "Tests or behavior evals".to_string(),
            present: has_tests || has_behavior_spec,
            detail: if has_behavior_spec {
                "behavior eval spec was supplied".to_string()
            } else if has_tests {
                "at least one scanned file contains Rust test markers".to_string()
            } else {
                "no tests or behavior eval spec were detected for the scanned target".to_string()
            },
        },
        EvidenceSignal {
            id: "coverage-tool".to_string(),
            label: "Coverage tooling".to_string(),
            present: has_coverage_tool,
            detail: "cargo-llvm-cov availability is detected; run mdx-rust evidence --include-coverage to collect coverage evidence".to_string(),
        },
        EvidenceSignal {
            id: "mutation-tool".to_string(),
            label: "Mutation tooling".to_string(),
            present: has_mutation_tool,
            detail: "cargo-mutants availability is detected; run mdx-rust evidence --include-mutation to collect mutation evidence".to_string(),
        },
    ];
    if let Some(run) = measured {
        signals.push(EvidenceSignal {
            id: "measured-evidence".to_string(),
            label: "Measured evidence artifact".to_string(),
            present: true,
            detail: format!(
                "latest evidence run {} recorded {:?} evidence",
                run.run_id, run.grade
            ),
        });
    }

    let mut unlock_suggestions = Vec::new();
    if grade == EvidenceGrade::None {
        unlock_suggestions.push(
            "Run mdx-rust from a Cargo workspace before allowing autonomous changes.".to_string(),
        );
    }
    if measured.is_none() && grade < EvidenceGrade::Tested {
        unlock_suggestions.push(
            "Add Rust tests or pass --eval-spec to unlock tested evidence for future recipes."
                .to_string(),
        );
    }
    if measured.is_none() {
        unlock_suggestions.push(
            "Run mdx-rust evidence to replace inferred evidence with measured test results."
                .to_string(),
        );
    }
    if !has_coverage_tool {
        unlock_suggestions
            .push("Install cargo-llvm-cov to prepare for covered Tier 2 recipe gates.".to_string());
    }
    if !has_mutation_tool {
        unlock_suggestions.push(
            "Install cargo-mutants to prepare for hardened Tier 2 and Tier 3 recipe gates."
                .to_string(),
        );
    }

    EvidenceSummary {
        grade,
        max_autonomous_tier,
        analysis_depth,
        signals,
        profiled_files: measured.map(|run| run.file_profiles.len()).unwrap_or(0),
        unlocked_recipe_tiers: unlocked_recipe_tiers(grade),
        unlock_suggestions,
    }
}

fn security_posture_summary(findings: &[AuditFinding]) -> SecurityPostureSummary {
    let mut summary = SecurityPostureSummary::default();
    for finding in findings {
        match finding.severity {
            AuditSeverity::High => summary.high += 1,
            AuditSeverity::Medium => summary.medium += 1,
            AuditSeverity::Low => summary.low += 1,
            AuditSeverity::Info => summary.info += 1,
        }
    }
    let penalty = summary.high.saturating_mul(25)
        + summary.medium.saturating_mul(12)
        + summary.low.saturating_mul(5);
    summary.score = 100usize.saturating_sub(penalty).min(100) as u8;
    let mut top_findings = findings
        .iter()
        .filter(|finding| finding.severity != AuditSeverity::Info)
        .collect::<Vec<_>>();
    top_findings.sort_by(|left, right| {
        right
            .severity
            .cmp(&left.severity)
            .then_with(|| left.file.cmp(&right.file))
            .then_with(|| left.line.cmp(&right.line))
    });
    summary.top_findings = top_findings
        .into_iter()
        .take(5)
        .map(|finding| {
            let file = finding.file.as_deref().unwrap_or("<workspace>");
            let line = finding
                .line
                .map(|line| line.to_string())
                .unwrap_or_else(|| "?".to_string());
            format!(
                "{:?}: {} ({}:{})",
                finding.severity, finding.title, file, line
            )
        })
        .collect();
    summary
}

fn contract_posture_summary(run: &ContractRun) -> ContractPostureSummary {
    let missing = run.summary.public_functions_missing_contracts;
    let public = run.summary.public_function_count;
    let coverage_ratio = if public == 0 {
        1.0
    } else {
        (public.saturating_sub(missing)) as f32 / public as f32
    };
    let grade = if run.summary.function_count == 0 {
        ContractPostureGrade::Unknown
    } else if missing == 0 {
        ContractPostureGrade::Strong
    } else if coverage_ratio >= 0.5 {
        ContractPostureGrade::Partial
    } else {
        ContractPostureGrade::Weak
    };

    ContractPostureSummary {
        grade,
        function_count: run.summary.function_count,
        public_function_count: public,
        functions_with_contracts: run.summary.functions_with_contracts,
        public_functions_missing_contracts: missing,
        assertion_count: run.summary.assertion_count,
        top_recommendations: run
            .recommendations
            .iter()
            .take(5)
            .map(|recommendation| {
                format!(
                    "{}:{} {}",
                    recommendation.file.display(),
                    recommendation.line,
                    recommendation.message
                )
            })
            .collect(),
    }
}

fn performance_posture_summary(run: &PerformanceRun) -> PerformancePostureSummary {
    let penalty = run.summary.high.saturating_mul(25)
        + run.summary.medium.saturating_mul(10)
        + run.summary.low.saturating_mul(3);
    PerformancePostureSummary {
        score: 100usize.saturating_sub(penalty).min(100) as u8,
        finding_count: run.summary.finding_count,
        high: run.summary.high,
        medium: run.summary.medium,
        low: run.summary.low,
        top_findings: run
            .findings
            .iter()
            .take(5)
            .map(|finding| {
                format!(
                    "{}:{} [{}] {}",
                    finding.file.display(),
                    finding.line,
                    finding.severity,
                    finding.title
                )
            })
            .collect(),
        recommendations: run.recommendations.clone(),
    }
}

fn annotate_candidate_autonomy(
    candidates: &mut [RefactorCandidate],
    evidence: &EvidenceSummary,
    security: &SecurityPostureSummary,
) {
    for candidate in candidates {
        candidate.autonomy = candidate_autonomy_decision(candidate, evidence, security);
    }
}

fn candidate_autonomy_decision(
    candidate: &RefactorCandidate,
    evidence: &EvidenceSummary,
    security: &SecurityPostureSummary,
) -> CandidateAutonomyDecision {
    let mut reasons = Vec::new();
    if evidence.grade == EvidenceGrade::None {
        reasons.push("no usable evidence grade is available".to_string());
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::Blocked,
            reasons,
        };
    }
    if security.high > 0 {
        reasons.push(
            "high-severity security finding requires human review before autonomous apply"
                .to_string(),
        );
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::ReviewOnly,
            reasons,
        };
    }
    if !candidate.evidence_satisfied || candidate.required_evidence > evidence.grade {
        reasons.push(format!(
            "candidate requires {:?} evidence but target has {:?}",
            candidate.required_evidence, evidence.grade
        ));
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::Blocked,
            reasons,
        };
    }
    if candidate.public_api_impact {
        reasons.push("public API impact requires explicit human allowance".to_string());
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::ReviewOnly,
            reasons,
        };
    }
    if candidate.status != RefactorCandidateStatus::ApplyViaImprove {
        reasons.push("candidate is plan-only or needs human design".to_string());
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::ReviewOnly,
            reasons,
        };
    }
    if !is_supported_mechanical_recipe(&candidate.recipe) {
        reasons.push("candidate has no supported executable recipe".to_string());
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::ReviewOnly,
            reasons,
        };
    }
    if candidate.risk != RefactorRiskLevel::Low {
        reasons.push("only low-risk candidates are autonomous by default".to_string());
        return CandidateAutonomyDecision {
            decision: AutonomyDecision::ReviewOnly,
            reasons,
        };
    }

    reasons.push("low-risk executable recipe satisfies current evidence gates".to_string());
    CandidateAutonomyDecision {
        decision: AutonomyDecision::Allowed,
        reasons,
    }
}

fn autonomy_readiness(
    evidence: &EvidenceSummary,
    security: &SecurityPostureSummary,
    candidates: &[RefactorCandidate],
) -> AutonomyReadiness {
    let executable_candidates = candidates
        .iter()
        .filter(|candidate| candidate.autonomy.decision == AutonomyDecision::Allowed)
        .count();
    let review_only_candidates = candidates
        .iter()
        .filter(|candidate| candidate.autonomy.decision == AutonomyDecision::ReviewOnly)
        .count();
    let blocked_candidates = candidates
        .iter()
        .filter(|candidate| candidate.autonomy.decision == AutonomyDecision::Blocked)
        .count();
    let mut blockers = Vec::new();
    if evidence.grade == EvidenceGrade::None {
        blockers.push("no usable evidence grade is available".to_string());
    }
    if security.high > 0 {
        blockers.push("high-severity security findings require review first".to_string());
    }
    if executable_candidates == 0 {
        blockers.push("no low-risk executable candidates are currently allowed".to_string());
    }

    let has_tier2_allowed = candidates.iter().any(|candidate| {
        candidate.autonomy.decision == AutonomyDecision::Allowed
            && candidate.tier >= RecipeTier::Tier2
    });
    let has_tier3_plan = candidates.iter().any(|candidate| {
        candidate.tier >= RecipeTier::Tier3
            && candidate.autonomy.decision != AutonomyDecision::Blocked
    });
    let grade = if evidence.grade == EvidenceGrade::None {
        AutonomyReadinessGrade::Blocked
    } else if executable_candidates > 0 && has_tier2_allowed {
        AutonomyReadinessGrade::Tier2Ready
    } else if executable_candidates > 0 {
        AutonomyReadinessGrade::Tier1Ready
    } else if has_tier3_plan {
        AutonomyReadinessGrade::Tier3Planning
    } else {
        AutonomyReadinessGrade::ReviewOnly
    };
    let max_safe_tier = candidates
        .iter()
        .filter(|candidate| candidate.autonomy.decision == AutonomyDecision::Allowed)
        .map(|candidate| candidate.tier)
        .max()
        .unwrap_or(RecipeTier::Tier1);
    let recommended_command = match grade {
        AutonomyReadinessGrade::Tier2Ready => Some(
            "mdx-rust evolve <target> --budget 10m --tier 2 --min-evidence covered".to_string(),
        ),
        AutonomyReadinessGrade::Tier1Ready => {
            Some("mdx-rust evolve <target> --budget 10m --tier 1".to_string())
        }
        AutonomyReadinessGrade::Tier3Planning => Some("mdx-rust plan <target> --json".to_string()),
        AutonomyReadinessGrade::ReviewOnly | AutonomyReadinessGrade::Blocked => {
            Some("mdx-rust map <target> --json".to_string())
        }
    };

    AutonomyReadiness {
        grade,
        max_safe_tier,
        executable_candidates,
        review_only_candidates,
        blocked_candidates,
        blockers,
        recommended_command,
    }
}

fn analysis_depth_for_evidence(grade: EvidenceGrade) -> EvidenceAnalysisDepth {
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
        tiers.push("Tier 1 executable mechanical recipes".to_string());
    }
    if grade >= EvidenceGrade::Tested {
        tiers.push("Tier 2 boundary review candidates".to_string());
    }
    if grade >= EvidenceGrade::Covered {
        tiers.push("Tier 2 structural mechanical recipes".to_string());
    }
    if grade >= EvidenceGrade::Hardened {
        tiers.push("Tier 3 semantic candidates in review".to_string());
    }
    tiers
}

fn max_tier_for_evidence(grade: EvidenceGrade) -> u8 {
    match grade {
        EvidenceGrade::None => 0,
        EvidenceGrade::Compiled | EvidenceGrade::Tested => 1,
        EvidenceGrade::Covered => 2,
        EvidenceGrade::Hardened | EvidenceGrade::Proven => 3,
    }
}

fn measured_hardening_tier(measured: Option<&EvidenceRun>) -> u8 {
    match measured.map(|run| run.grade) {
        Some(EvidenceGrade::Hardened | EvidenceGrade::Proven) => 3,
        Some(EvidenceGrade::Covered) => 2,
        _ => 1,
    }
}

fn hardening_depth_for_evidence(measured: Option<&EvidenceRun>) -> HardeningEvidenceDepth {
    match measured.map(|run| run.grade) {
        Some(EvidenceGrade::Proven) => HardeningEvidenceDepth::Proven,
        Some(EvidenceGrade::Hardened) => HardeningEvidenceDepth::Hardened,
        Some(EvidenceGrade::Covered) => HardeningEvidenceDepth::Covered,
        Some(EvidenceGrade::Tested) => HardeningEvidenceDepth::Tested,
        _ => HardeningEvidenceDepth::Basic,
    }
}

fn hardening_depth_for_grade(grade: EvidenceGrade) -> HardeningEvidenceDepth {
    match grade {
        EvidenceGrade::Proven => HardeningEvidenceDepth::Proven,
        EvidenceGrade::Hardened => HardeningEvidenceDepth::Hardened,
        EvidenceGrade::Covered => HardeningEvidenceDepth::Covered,
        EvidenceGrade::Tested => HardeningEvidenceDepth::Tested,
        EvidenceGrade::None | EvidenceGrade::Compiled => HardeningEvidenceDepth::Basic,
    }
}

fn capability_gates() -> Vec<CapabilityGate> {
    vec![
        CapabilityGate {
            id: "nextest".to_string(),
            label: "cargo-nextest".to_string(),
            available: cargo_subcommand_exists("nextest"),
            command: "cargo nextest run".to_string(),
            purpose: "fast, isolated Rust test execution for behavior gates".to_string(),
        },
        CapabilityGate {
            id: "llvm-cov".to_string(),
            label: "cargo-llvm-cov".to_string(),
            available: cargo_subcommand_exists("llvm-cov"),
            command: "cargo llvm-cov".to_string(),
            purpose: "coverage evidence before broad autonomous refactoring".to_string(),
        },
        CapabilityGate {
            id: "mutants".to_string(),
            label: "cargo-mutants".to_string(),
            available: cargo_subcommand_exists("mutants"),
            command: "cargo mutants".to_string(),
            purpose: "mutation testing signal for high-value refactor targets".to_string(),
        },
        CapabilityGate {
            id: "semver-checks".to_string(),
            label: "cargo-semver-checks".to_string(),
            available: cargo_subcommand_exists("semver-checks"),
            command: "cargo semver-checks".to_string(),
            purpose: "public API compatibility gate for library refactors".to_string(),
        },
    ]
}

fn recommended_actions(
    quality: &CodebaseQualitySummary,
    impact: &RefactorImpactSummary,
    gates: &[CapabilityGate],
    evidence: &EvidenceSummary,
    security: &SecurityPostureSummary,
    contracts: &ContractPostureSummary,
    performance: &PerformancePostureSummary,
) -> Vec<String> {
    let mut actions = Vec::new();
    if security.high > 0 || security.medium > 0 {
        actions.push(
            "Run mdx-rust audit and inspect security posture before broad autonomous apply."
                .to_string(),
        );
    }
    if quality.patchable_findings > 0 && evidence.grade >= EvidenceGrade::Compiled {
        actions.push(
            "Run mdx-rust autopilot --apply to execute low-risk Tier 1 mechanical hardening passes."
                .to_string(),
        );
    } else if quality.patchable_findings > 0 {
        actions.push(
            "Autonomous execution is blocked until this target has at least compiled evidence."
                .to_string(),
        );
    }
    if quality.review_only_findings > 0 {
        actions.push(
            "Review security-sensitive findings before enabling broader recipes.".to_string(),
        );
    }
    if contracts.public_functions_missing_contracts > 0 {
        actions.push(
            "Run mdx-rust contracts and add explicit behavior contracts before semantic public-function changes."
                .to_string(),
        );
    }
    if performance.high > 0 || performance.medium > 0 {
        actions.push(
            "Run mdx-rust perf and add benchmark evidence before performance-oriented refactors."
                .to_string(),
        );
    }
    if impact.oversized_files > 0 || impact.oversized_functions > 0 {
        actions.push(
            "Use mdx-rust plan to stage larger module and function refactors behind behavior gates."
                .to_string(),
        );
    }
    if quality.public_api_pressure > 0
        && gates
            .iter()
            .any(|gate| gate.id == "semver-checks" && !gate.available)
    {
        actions.push(
            "Install cargo-semver-checks before allowing public API impacting refactors."
                .to_string(),
        );
    }
    if quality.test_coverage_signal == TestCoverageSignal::Sparse {
        actions.push(
            "Add a behavior eval spec or stronger Rust tests before broad autonomous apply."
                .to_string(),
        );
    }
    actions.extend(evidence.unlock_suggestions.iter().cloned());
    if actions.is_empty() {
        actions.push(
            "No immediate autonomous changes found. Keep policy and behavior gates current."
                .to_string(),
        );
    }
    actions
}

fn cargo_subcommand_exists(name: &str) -> bool {
    let command = format!("cargo-{name}");
    let Some(path_var) = std::env::var_os("PATH") else {
        return false;
    };
    std::env::split_paths(&path_var).any(|dir| dir.join(&command).is_file())
}

fn hardening_candidates(
    findings: &[HardeningFinding],
    config: &RefactorPlanConfig,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> Vec<RefactorCandidate> {
    findings
        .iter()
        .filter_map(|finding| {
            let file = finding.file.display().to_string();
            let required_evidence = required_evidence_for_hardening_strategy(&finding.strategy);
            let evidence_satisfied = evidence.grade >= required_evidence;
            let recipe = recipe_for_hardening_strategy(&finding.strategy);
            if !finding.patchable && !evidence_satisfied {
                return None;
            }

            Some(RefactorCandidate {
                id: format!(
                    "plan-hardening-{}-{}-{}",
                    sanitize_id(&file),
                    sanitize_id(&format!("{:?}", finding.strategy)),
                    finding.line
                ),
                candidate_hash: String::new(),
                recipe,
                title: finding.title.clone(),
                rationale: if finding.patchable {
                    if required_evidence >= EvidenceGrade::Covered {
                        "Patchable Tier 2 structural mechanical refactor can be applied only when measured coverage evidence unlocks it.".to_string()
                    } else {
                        "Patchable Tier 1 mechanical hardening can be applied through the existing isolated validation transaction.".to_string()
                    }
                } else {
                    "Higher-evidence review candidate surfaced from security or boundary analysis; it remains plan-only until a safe executable recipe exists.".to_string()
                },
                file: file.clone(),
                line: finding.line,
                risk: risk_for_hardening_strategy(&finding.strategy),
                status: if evidence_satisfied {
                    if finding.patchable {
                        RefactorCandidateStatus::ApplyViaImprove
                    } else {
                        RefactorCandidateStatus::PlanOnly
                    }
                } else {
                    RefactorCandidateStatus::PlanOnly
                },
                tier: if required_evidence >= EvidenceGrade::Hardened {
                    RecipeTier::Tier3
                } else if required_evidence >= EvidenceGrade::Covered {
                    RecipeTier::Tier2
                } else if finding.patchable {
                    RecipeTier::Tier1
                } else {
                    RecipeTier::Tier2
                },
                required_evidence,
                evidence_satisfied,
                evidence_context: candidate_evidence_context(&file, evidence, measured),
                autonomy: CandidateAutonomyDecision::default(),
                public_api_impact: false,
                apply_command: (finding.patchable && evidence_satisfied)
                    .then(|| apply_command(&file, config, required_evidence)),
                required_gates: if finding.patchable {
                    required_gates(config.behavior_spec_path.is_some())
                } else {
                    vec![
                        "human review of boundary contract".to_string(),
                        "behavior evals or tests must cover the boundary".to_string(),
                        "future executable recipe must route through hardening transactions"
                            .to_string(),
                    ]
                },
            })
        })
        .collect()
}

fn required_evidence_for_hardening_strategy(
    strategy: &mdx_rust_analysis::HardeningStrategy,
) -> EvidenceGrade {
    match strategy {
        mdx_rust_analysis::HardeningStrategy::LenCheckIsEmpty
        | mdx_rust_analysis::HardeningStrategy::OptionContextPropagation
        | mdx_rust_analysis::HardeningStrategy::RepeatedStringLiteralConst => {
            EvidenceGrade::Covered
        }
        mdx_rust_analysis::HardeningStrategy::OptionMatchContextPropagation => {
            EvidenceGrade::Hardened
        }
        mdx_rust_analysis::HardeningStrategy::ClonePressureReview
        | mdx_rust_analysis::HardeningStrategy::LongFunctionReview => EvidenceGrade::Hardened,
        mdx_rust_analysis::HardeningStrategy::EnvAccessReview
        | mdx_rust_analysis::HardeningStrategy::FileIoReview
        | mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview
        | mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview
        | mdx_rust_analysis::HardeningStrategy::UnsafeReview => EvidenceGrade::Tested,
        _ => EvidenceGrade::Compiled,
    }
}

fn recipe_for_hardening_strategy(
    strategy: &mdx_rust_analysis::HardeningStrategy,
) -> RefactorRecipe {
    match strategy {
        mdx_rust_analysis::HardeningStrategy::BorrowParameterTightening => {
            RefactorRecipe::BorrowParameterTightening
        }
        mdx_rust_analysis::HardeningStrategy::ErrorContextPropagation => {
            RefactorRecipe::ErrorContextPropagation
        }
        mdx_rust_analysis::HardeningStrategy::IteratorCloned => RefactorRecipe::IteratorCloned,
        mdx_rust_analysis::HardeningStrategy::LenCheckIsEmpty => RefactorRecipe::LenCheckIsEmpty,
        mdx_rust_analysis::HardeningStrategy::OptionContextPropagation => {
            RefactorRecipe::OptionContextPropagation
        }
        mdx_rust_analysis::HardeningStrategy::OptionMatchContextPropagation => {
            RefactorRecipe::OptionMatchContextPropagation
        }
        mdx_rust_analysis::HardeningStrategy::MustUsePublicReturn => {
            RefactorRecipe::MustUsePublicReturn
        }
        mdx_rust_analysis::HardeningStrategy::ClonePressureReview => {
            RefactorRecipe::ClonePressureReview
        }
        mdx_rust_analysis::HardeningStrategy::LongFunctionReview => {
            RefactorRecipe::LongFunctionReview
        }
        mdx_rust_analysis::HardeningStrategy::RepeatedStringLiteralConst => {
            RefactorRecipe::RepeatedStringLiteralConst
        }
        mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview => {
            RefactorRecipe::BoundaryValidationReview
        }
        mdx_rust_analysis::HardeningStrategy::EnvAccessReview
        | mdx_rust_analysis::HardeningStrategy::FileIoReview => {
            RefactorRecipe::BoundaryValidationReview
        }
        mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview
        | mdx_rust_analysis::HardeningStrategy::UnsafeReview => {
            RefactorRecipe::SecurityBoundaryReview
        }
        _ => RefactorRecipe::ContextualErrorHardening,
    }
}

fn risk_for_hardening_strategy(
    strategy: &mdx_rust_analysis::HardeningStrategy,
) -> RefactorRiskLevel {
    match strategy {
        mdx_rust_analysis::HardeningStrategy::ProcessExecutionReview
        | mdx_rust_analysis::HardeningStrategy::UnsafeReview => RefactorRiskLevel::High,
        mdx_rust_analysis::HardeningStrategy::EnvAccessReview
        | mdx_rust_analysis::HardeningStrategy::FileIoReview
        | mdx_rust_analysis::HardeningStrategy::ClonePressureReview
        | mdx_rust_analysis::HardeningStrategy::LongFunctionReview
        | mdx_rust_analysis::HardeningStrategy::HttpSurfaceReview => RefactorRiskLevel::Medium,
        _ => RefactorRiskLevel::Low,
    }
}

fn structural_candidates(
    files: &[RefactorFileSummary],
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> Vec<RefactorCandidate> {
    let mut candidates = Vec::new();
    let split_threshold = if evidence.grade >= EvidenceGrade::Hardened {
        220
    } else {
        300
    };
    let extract_threshold = if evidence.grade >= EvidenceGrade::Hardened {
        50
    } else {
        80
    };
    for file in files {
        let file_path = file.file.display().to_string();
        if file.line_count >= split_threshold {
            let required_evidence = EvidenceGrade::Covered;
            candidates.push(RefactorCandidate {
                id: format!("plan-split-module-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::SplitModuleCandidate,
                title: "Split oversized module".to_string(),
                rationale: format!(
                    "{} has {} lines. Current evidence threshold is {split_threshold} lines for split-module planning.",
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
                tier: RecipeTier::Tier2,
                required_evidence,
                evidence_satisfied: evidence.grade >= required_evidence,
                evidence_context: candidate_evidence_context(&file_path, evidence, measured),
                autonomy: CandidateAutonomyDecision::default(),
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

        if file.largest_function_lines >= extract_threshold {
            let required_evidence = EvidenceGrade::Covered;
            candidates.push(RefactorCandidate {
                id: format!("plan-extract-function-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::ExtractFunctionCandidate,
                title: "Extract long function".to_string(),
                rationale: format!(
                    "Largest function in {} is {} lines. Current evidence threshold is {extract_threshold} lines for extract-function planning.",
                    file_path, file.largest_function_lines
                ),
                file: file_path.clone(),
                line: 1,
                risk: RefactorRiskLevel::Medium,
                status: RefactorCandidateStatus::PlanOnly,
                tier: RecipeTier::Tier2,
                required_evidence,
                evidence_satisfied: evidence.grade >= required_evidence,
                evidence_context: candidate_evidence_context(&file_path, evidence, measured),
                autonomy: CandidateAutonomyDecision::default(),
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
            let required_evidence = EvidenceGrade::Tested;
            candidates.push(RefactorCandidate {
                id: format!("plan-public-api-{}", sanitize_id(&file_path)),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::PublicApiReview,
                title: "Protect public API before refactoring".to_string(),
                rationale: format!(
                    "{} exposes {} public item(s). Treat signature changes as semver-impacting.",
                    file_path, file.public_item_count
                ),
                file: file_path.clone(),
                line: 1,
                risk: RefactorRiskLevel::Medium,
                status: RefactorCandidateStatus::PlanOnly,
                tier: RecipeTier::Tier1,
                required_evidence,
                evidence_satisfied: evidence.grade >= required_evidence,
                evidence_context: candidate_evidence_context(&file_path, evidence, measured),
                autonomy: CandidateAutonomyDecision::default(),
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

fn security_candidates(
    findings: &[AuditFinding],
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> Vec<RefactorCandidate> {
    findings
        .iter()
        .filter(|finding| finding.severity != AuditSeverity::Info)
        .map(|finding| {
            let file = finding
                .file
                .clone()
                .unwrap_or_else(|| "<workspace>".to_string());
            let line = finding.line.unwrap_or(1);
            let required_evidence = match finding.severity {
                AuditSeverity::High => EvidenceGrade::Tested,
                AuditSeverity::Medium => EvidenceGrade::Tested,
                AuditSeverity::Low | AuditSeverity::Info => EvidenceGrade::Compiled,
            };
            let risk = match finding.severity {
                AuditSeverity::High => RefactorRiskLevel::High,
                AuditSeverity::Medium => RefactorRiskLevel::Medium,
                AuditSeverity::Low | AuditSeverity::Info => RefactorRiskLevel::Low,
            };
            RefactorCandidate {
                id: format!(
                    "plan-security-{}-{}-{}",
                    sanitize_id(&file),
                    sanitize_id(&finding.id),
                    line
                ),
                candidate_hash: String::new(),
                recipe: RefactorRecipe::SecurityBoundaryReview,
                title: finding.title.clone(),
                rationale: format!(
                    "Security audit flagged {:?}: {}",
                    finding.severity, finding.description
                ),
                file: file.clone(),
                line,
                risk,
                status: RefactorCandidateStatus::PlanOnly,
                tier: RecipeTier::Tier2,
                required_evidence,
                evidence_satisfied: evidence.grade >= required_evidence,
                evidence_context: candidate_evidence_context(&file, evidence, measured),
                autonomy: CandidateAutonomyDecision::default(),
                public_api_impact: false,
                apply_command: None,
                required_gates: vec![
                    "human security review".to_string(),
                    "policy update or explicit risk acceptance".to_string(),
                    "behavior evals or tests must cover the boundary".to_string(),
                ],
            }
        })
        .collect()
}

fn contract_candidates(
    run: &ContractRun,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> Vec<RefactorCandidate> {
    run.recommendations
        .iter()
        .take(25)
        .map(|recommendation| contract_candidate(recommendation, evidence, measured))
        .collect()
}

fn contract_candidate(
    recommendation: &ContractRecommendation,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> RefactorCandidate {
    let file = recommendation.file.display().to_string();
    let required_evidence = EvidenceGrade::Tested;
    RefactorCandidate {
        id: format!(
            "plan-contract-{}-{}",
            sanitize_id(&file),
            recommendation.line
        ),
        candidate_hash: String::new(),
        recipe: RefactorRecipe::ContractCoverageReview,
        title: "Add explicit behavior contract before semantic change".to_string(),
        rationale: recommendation.message.clone(),
        file: file.clone(),
        line: recommendation.line,
        risk: RefactorRiskLevel::Medium,
        status: RefactorCandidateStatus::PlanOnly,
        tier: RecipeTier::Tier2,
        required_evidence,
        evidence_satisfied: evidence.grade >= required_evidence,
        evidence_context: candidate_evidence_context(&file, evidence, measured),
        autonomy: CandidateAutonomyDecision::default(),
        public_api_impact: true,
        apply_command: None,
        required_gates: vec![
            "human review of intended behavior".to_string(),
            "contract docs or property tests before semantic refactor".to_string(),
            "existing validation and behavior gates must remain passing".to_string(),
        ],
    }
}

fn performance_candidates(
    run: &PerformanceRun,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> Vec<RefactorCandidate> {
    run.findings
        .iter()
        .take(25)
        .map(|finding| performance_candidate(finding, evidence, measured))
        .collect()
}

fn performance_candidate(
    finding: &PerformanceFinding,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> RefactorCandidate {
    let file = finding.file.display().to_string();
    let required_evidence = if finding.severity == "high" {
        EvidenceGrade::Covered
    } else {
        EvidenceGrade::Tested
    };
    let risk = match finding.severity.as_str() {
        "high" => RefactorRiskLevel::High,
        "medium" => RefactorRiskLevel::Medium,
        _ => RefactorRiskLevel::Low,
    };
    RefactorCandidate {
        id: format!(
            "plan-perf-{}-{}-{}",
            sanitize_id(&file),
            sanitize_id(&finding.category),
            finding.line
        ),
        candidate_hash: String::new(),
        recipe: RefactorRecipe::PerformanceHotspotReview,
        title: finding.title.clone(),
        rationale: format!(
            "{} Recommendation: {}",
            finding.evidence, finding.recommendation
        ),
        file: file.clone(),
        line: finding.line,
        risk,
        status: RefactorCandidateStatus::PlanOnly,
        tier: RecipeTier::Tier2,
        required_evidence,
        evidence_satisfied: evidence.grade >= required_evidence,
        evidence_context: candidate_evidence_context(&file, evidence, measured),
        autonomy: CandidateAutonomyDecision::default(),
        public_api_impact: false,
        apply_command: None,
        required_gates: vec![
            "benchmark or behavior eval before performance rewrite".to_string(),
            "human review of semantic equivalence".to_string(),
            "future executable recipe must route through hardening transactions".to_string(),
        ],
    }
}

fn candidate_evidence_context(
    file: &str,
    evidence: &EvidenceSummary,
    measured: Option<&EvidenceRun>,
) -> CandidateEvidenceContext {
    if let Some(profile) = measured
        .iter()
        .flat_map(|run| run.file_profiles.iter())
        .find(|profile| profile.file == file)
    {
        return CandidateEvidenceContext {
            grade: profile.grade,
            status: candidate_evidence_status(profile.grade),
            source: "measured file evidence profile".to_string(),
            profiled_file: Some(profile.file.clone()),
            signals: profile.signals.clone(),
        };
    }
    CandidateEvidenceContext {
        grade: evidence.grade,
        status: candidate_evidence_status(evidence.grade),
        source: if measured.is_some() {
            "measured run did not include this file; using run-level evidence".to_string()
        } else {
            "inferred evidence summary".to_string()
        },
        profiled_file: None,
        signals: evidence
            .signals
            .iter()
            .filter(|signal| signal.present)
            .map(|signal| signal.label.clone())
            .collect(),
    }
}

fn candidate_evidence_status(grade: EvidenceGrade) -> CandidateEvidenceStatus {
    match grade {
        EvidenceGrade::None => CandidateEvidenceStatus::Unmeasured,
        EvidenceGrade::Compiled => CandidateEvidenceStatus::Compiled,
        EvidenceGrade::Tested => CandidateEvidenceStatus::Tested,
        EvidenceGrade::Covered => CandidateEvidenceStatus::Covered,
        EvidenceGrade::Hardened => CandidateEvidenceStatus::MutationBacked,
        EvidenceGrade::Proven => CandidateEvidenceStatus::Proven,
    }
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

fn apply_command(file: &str, config: &RefactorPlanConfig, evidence: EvidenceGrade) -> String {
    let mut command = format!("mdx-rust improve {} --apply", shell_word_str(file));
    if evidence >= EvidenceGrade::Covered {
        command.push_str(" --tier 2");
    }
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

fn codebase_map_id(
    root: &Path,
    config: &CodebaseMapConfig,
    quality: &CodebaseQualitySummary,
    impact: &RefactorImpactSummary,
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.target).as_bytes());
    bytes.extend_from_slice(format!("{quality:?}").as_bytes());
    bytes.extend_from_slice(format!("{impact:?}").as_bytes());
    stable_hash_hex(&bytes)
}

fn codebase_map_hash(map: &CodebaseMap) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(map.schema_version.as_bytes());
    bytes.extend_from_slice(map.map_id.as_bytes());
    bytes.extend_from_slice(map.root.as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.target).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.quality).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.security).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.contracts).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.performance).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.evidence).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.measured_evidence).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.impact).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.files).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.module_edges).as_bytes());
    bytes.extend_from_slice(format!("{:?}", map.findings).as_bytes());
    stable_hash_hex(&bytes)
}

fn autopilot_run_id(root: &Path, config: &AutopilotConfig, map: &CodebaseMap) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.target).as_bytes());
    bytes.extend_from_slice(config.apply.to_string().as_bytes());
    bytes.extend_from_slice(config.max_passes.to_string().as_bytes());
    bytes.extend_from_slice(config.max_candidates.to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.max_tier).as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.min_evidence).as_bytes());
    bytes.extend_from_slice(map.map_hash.as_bytes());
    stable_hash_hex(&bytes)
}

fn evolution_scorecard_id(
    root: &Path,
    config: &EvolutionScorecardConfig,
    map: &CodebaseMap,
    plan: &RefactorPlan,
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.target).as_bytes());
    bytes.extend_from_slice(map.map_hash.as_bytes());
    bytes.extend_from_slice(plan.plan_hash.as_bytes());
    stable_hash_hex(&bytes)
}

fn evolution_brief_id(
    root: &Path,
    config: &EvolutionBriefConfig,
    scorecard: &EvolutionScorecard,
) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(root.display().to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", config.target).as_bytes());
    bytes.extend_from_slice(scorecard.scorecard_id.as_bytes());
    bytes.extend_from_slice(format!("{:?}", scorecard.map.contracts).as_bytes());
    bytes.extend_from_slice(format!("{:?}", scorecard.map.performance).as_bytes());
    stable_hash_hex(&bytes)
}

fn evolution_brief_sequence(target: Option<&Path>, scorecard: &EvolutionScorecard) -> Vec<String> {
    let target = target
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "<target>".to_string());
    let target_arg = if target == "<target>" {
        target
    } else {
        shell_quote_argument(&target)
    };
    let mut sequence = vec![
        "mdx-rust --json agent-contract".to_string(),
        format!("mdx-rust --json repo-map {target_arg}"),
        "mdx-rust --json noise-filter".to_string(),
        format!("mdx-rust --json contracts {target_arg}"),
        format!("mdx-rust --json perf {target_arg}"),
        "mdx-rust --json benchmark --spec .mdx-rust/benchmarks.json".to_string(),
        format!("mdx-rust --json evidence {target_arg}"),
        format!("mdx-rust --json scorecard {target_arg}"),
    ];
    sequence.extend(scorecard.next_commands.iter().cloned());
    sequence
}

fn scorecard_next_commands(readiness: &AutonomyReadiness, plan: &RefactorPlan) -> Vec<String> {
    let target = plan.target.as_deref().unwrap_or("<target>");
    let target_arg = if target == "<target>" {
        target.to_string()
    } else {
        shell_quote_argument(target)
    };
    let mut commands = vec![
        format!("mdx-rust --json contracts {target_arg}"),
        format!("mdx-rust --json perf {target_arg}"),
        "mdx-rust --json benchmark --spec .mdx-rust/benchmarks.json".to_string(),
        format!("mdx-rust --json evidence {target_arg}"),
        format!("mdx-rust --json map {target_arg}"),
        format!("mdx-rust --json plan {target_arg}"),
    ];
    match readiness.grade {
        AutonomyReadinessGrade::Tier2Ready => commands.push(format!(
            "mdx-rust --json evolve {target_arg} --budget 10m --tier 2 --min-evidence covered"
        )),
        AutonomyReadinessGrade::Tier1Ready => commands.push(format!(
            "mdx-rust --json evolve {target_arg} --budget 10m --tier 1"
        )),
        AutonomyReadinessGrade::Tier3Planning => {
            commands.push(format!("mdx-rust --json plan {target_arg} --max-files 250"))
        }
        AutonomyReadinessGrade::ReviewOnly | AutonomyReadinessGrade::Blocked => {
            commands.push("mdx-rust --json audit".to_string())
        }
    }
    if let Some(path) = &plan.artifact_path {
        commands.push(format!(
            "mdx-rust --json explain {}",
            shell_quote_argument(path)
        ));
    }
    commands
}

fn shell_quote_argument(value: &str) -> String {
    if value
        .bytes()
        .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'.' | b'-' | b'_'))
    {
        return value.to_string();
    }

    format!("'{}'", value.replace('\'', "'\\''"))
}

fn audit_scope_path(root: &Path, target: Option<&Path>) -> PathBuf {
    let Some(target) = target else {
        return root.to_path_buf();
    };
    if target.is_absolute() {
        target.to_path_buf()
    } else {
        root.join(target)
    }
}

fn refactor_plan_hash(plan: &RefactorPlan) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(plan.schema_version.as_bytes());
    bytes.extend_from_slice(plan.plan_id.as_bytes());
    bytes.extend_from_slice(plan.root.as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.target).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.evidence).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.measured_evidence).as_bytes());
    bytes.extend_from_slice(format!("{:?}", plan.security).as_bytes());
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
    bytes.extend_from_slice(format!("{:?}", candidate.tier).as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.required_evidence).as_bytes());
    bytes.extend_from_slice(candidate.evidence_satisfied.to_string().as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.evidence_context).as_bytes());
    bytes.extend_from_slice(format!("{:?}", candidate.autonomy).as_bytes());
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

fn stale_file_for_candidate(
    root: &Path,
    plan: &RefactorPlan,
    file: &str,
) -> anyhow::Result<Option<StaleSourceFile>> {
    let Some(snapshot) = plan
        .source_snapshots
        .iter()
        .find(|snapshot| snapshot.file == file)
    else {
        return Ok(Some(StaleSourceFile {
            file: file.to_string(),
            expected_hash: "<missing-snapshot>".to_string(),
            actual_hash: "<unknown>".to_string(),
        }));
    };
    let rel = safe_relative_path(&snapshot.file)?;
    let actual_hash = std::fs::read(root.join(&rel))
        .map(|content| stable_hash_hex(&content))
        .unwrap_or_else(|_| "<missing>".to_string());
    if actual_hash == snapshot.hash {
        Ok(None)
    } else {
        Ok(Some(StaleSourceFile {
            file: snapshot.file.clone(),
            expected_hash: snapshot.hash.clone(),
            actual_hash,
        }))
    }
}

fn executable_candidate_queue<'a>(
    plan: &'a RefactorPlan,
    config: &RefactorBatchApplyConfig,
) -> Vec<&'a RefactorCandidate> {
    let mut queue = Vec::new();
    let mut seen_files = std::collections::BTreeSet::new();
    for candidate in &plan.candidates {
        if queue.len() >= config.max_candidates {
            break;
        }
        if candidate.status != RefactorCandidateStatus::ApplyViaImprove
            || !is_supported_mechanical_recipe(&candidate.recipe)
        {
            continue;
        }
        if !candidate.evidence_satisfied
            || candidate.required_evidence > plan.evidence.grade
            || plan.evidence.grade < config.min_evidence
            || candidate.tier > config.max_tier
            || candidate.autonomy.decision != AutonomyDecision::Allowed
        {
            continue;
        }
        if candidate.public_api_impact && !config.allow_public_api_impact {
            continue;
        }
        if seen_files.insert(candidate.file.clone()) {
            queue.push(candidate);
        }
    }
    queue
}

fn is_supported_mechanical_recipe(recipe: &RefactorRecipe) -> bool {
    matches!(
        recipe,
        RefactorRecipe::BorrowParameterTightening
            | RefactorRecipe::ContextualErrorHardening
            | RefactorRecipe::ErrorContextPropagation
            | RefactorRecipe::IteratorCloned
            | RefactorRecipe::LenCheckIsEmpty
            | RefactorRecipe::MustUsePublicReturn
            | RefactorRecipe::OptionMatchContextPropagation
            | RefactorRecipe::OptionContextPropagation
            | RefactorRecipe::RepeatedStringLiteralConst
    )
}

fn count_executable_candidates(
    plan: &RefactorPlan,
    allow_public_api_impact: bool,
    max_candidates: usize,
    max_tier: RecipeTier,
    min_evidence: EvidenceGrade,
) -> usize {
    executable_candidate_queue(
        plan,
        &RefactorBatchApplyConfig {
            plan_path: PathBuf::new(),
            apply: false,
            allow_public_api_impact,
            validation_timeout: Duration::from_secs(1),
            max_candidates,
            max_tier,
            min_evidence,
        },
    )
    .len()
}

fn recipe_tier_number(tier: RecipeTier) -> u8 {
    match tier {
        RecipeTier::Tier1 => 1,
        RecipeTier::Tier2 => 2,
        RecipeTier::Tier3 => 3,
    }
}

fn autopilot_pass_status(status: &RefactorBatchApplyStatus) -> AutopilotPassStatus {
    match status {
        RefactorBatchApplyStatus::Reviewed => AutopilotPassStatus::Reviewed,
        RefactorBatchApplyStatus::Applied => AutopilotPassStatus::Applied,
        RefactorBatchApplyStatus::PartiallyApplied => AutopilotPassStatus::PartiallyApplied,
        RefactorBatchApplyStatus::NoExecutableCandidates => {
            AutopilotPassStatus::NoExecutableCandidates
        }
        RefactorBatchApplyStatus::Rejected | RefactorBatchApplyStatus::StalePlan => {
            AutopilotPassStatus::Rejected
        }
    }
}

fn autopilot_status(
    apply: bool,
    passes: &[AutopilotPass],
    executed_candidates: usize,
) -> AutopilotStatus {
    if executed_candidates == 0 {
        if passes
            .iter()
            .any(|pass| pass.status == AutopilotPassStatus::Rejected)
        {
            AutopilotStatus::Rejected
        } else {
            AutopilotStatus::NoExecutableCandidates
        }
    } else if !apply {
        AutopilotStatus::Reviewed
    } else if passes
        .iter()
        .any(|pass| pass.status == AutopilotPassStatus::Rejected)
    {
        AutopilotStatus::PartiallyApplied
    } else {
        AutopilotStatus::Applied
    }
}

fn autopilot_note(run: &AutopilotRun) -> String {
    match run.status {
        AutopilotStatus::Reviewed => format!(
            "reviewed {} candidate(s) across {} pass(es); rerun with --apply to land validated transactions",
            run.total_executed_candidates,
            run.passes.len()
        ),
        AutopilotStatus::Applied => format!(
            "applied {} candidate(s) across {} pass(es) with fresh plans before each pass",
            run.total_executed_candidates,
            run.passes.len()
        ),
        AutopilotStatus::PartiallyApplied => format!(
            "applied {} candidate(s) before an execution gate stopped the run",
            run.total_executed_candidates
        ),
        AutopilotStatus::NoExecutableCandidates => {
            if run.budget_exhausted {
                "budget exhausted before more executable work could run".to_string()
            } else {
                "no executable low-risk candidates were available".to_string()
            }
        }
        AutopilotStatus::Rejected => {
            "autopilot stopped because a planning or execution gate rejected the run".to_string()
        }
    }
}

fn autopilot_execution_summary(run: &AutopilotRun) -> AutopilotExecutionSummary {
    let plans_created = run.passes.len();
    let executable_candidates_seen = run
        .passes
        .iter()
        .map(|pass| pass.executable_candidates)
        .sum();
    let validated_transactions = run
        .passes
        .iter()
        .filter_map(|pass| pass.batch.as_ref())
        .flat_map(|batch| batch.steps.iter())
        .filter(|step| {
            step.hardening_run
                .as_ref()
                .is_some_and(|hardening| hardening.outcome.isolated_validation_passed)
        })
        .count();
    let applied_transactions = run
        .passes
        .iter()
        .filter_map(|pass| pass.batch.as_ref())
        .flat_map(|batch| batch.steps.iter())
        .filter(|step| {
            step.hardening_run
                .as_ref()
                .is_some_and(|hardening| hardening.outcome.applied)
        })
        .count();
    let blocked_or_plan_only_candidates = run
        .total_planned_candidates
        .saturating_sub(executable_candidates_seen);

    AutopilotExecutionSummary {
        plans_created,
        executable_candidates_seen,
        validated_transactions,
        applied_transactions,
        blocked_or_plan_only_candidates,
        evidence_grade: run.evidence.grade,
        analysis_depth: run.evidence.analysis_depth.clone(),
    }
}

fn batch_status(apply: bool, executed: usize, requested: usize) -> RefactorBatchApplyStatus {
    if requested == 0 {
        RefactorBatchApplyStatus::NoExecutableCandidates
    } else if executed == 0 {
        RefactorBatchApplyStatus::Rejected
    } else if !apply {
        RefactorBatchApplyStatus::Reviewed
    } else if executed == requested {
        RefactorBatchApplyStatus::Applied
    } else {
        RefactorBatchApplyStatus::PartiallyApplied
    }
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

fn persist_batch_apply_run(
    artifact_root: Option<&Path>,
    mut run: RefactorBatchApplyRun,
) -> anyhow::Result<RefactorBatchApplyRun> {
    if let Some(artifact_root) = artifact_root {
        let dir = artifact_root.join("plans");
        std::fs::create_dir_all(&dir)?;
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let path = dir.join(format!(
            "apply-plan-batch-{millis}-{}.json",
            sanitize_id(&run.plan_id)
        ));
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&run)?)?;
    }
    Ok(run)
}

fn persist_codebase_map(artifact_root: &Path, map: &CodebaseMap) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("maps");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!(
        "codebase-map-{millis}-{}.json",
        sanitize_id(&map.map_id)
    )))
}

fn persist_autopilot_run(
    artifact_root: Option<&Path>,
    mut run: AutopilotRun,
) -> anyhow::Result<AutopilotRun> {
    if let Some(artifact_root) = artifact_root {
        let dir = artifact_root.join("autopilot");
        std::fs::create_dir_all(&dir)?;
        let millis = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or(0);
        let path = dir.join(format!(
            "autopilot-{millis}-{}.json",
            sanitize_id(&run.run_id)
        ));
        run.artifact_path = Some(path.display().to_string());
        std::fs::write(&path, serde_json::to_string_pretty(&run)?)?;
    }
    Ok(run)
}

fn persist_evolution_scorecard(
    artifact_root: &Path,
    scorecard: &EvolutionScorecard,
) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("scorecards");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!(
        "evolution-scorecard-{millis}-{}.json",
        sanitize_id(&scorecard.scorecard_id)
    )))
}

fn persist_evolution_brief(
    artifact_root: &Path,
    brief: &EvolutionBrief,
) -> anyhow::Result<PathBuf> {
    let dir = artifact_root.join("briefs");
    std::fs::create_dir_all(&dir)?;
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    Ok(dir.join(format!(
        "evolution-brief-{millis}-{}.json",
        sanitize_id(&brief.brief_id)
    )))
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

        assert_eq!(plan.schema_version, "1.0");
        assert!(plan.candidates.iter().any(|candidate| candidate.status
            == RefactorCandidateStatus::ApplyViaImprove
            && candidate
                .apply_command
                .as_deref()
                .is_some_and(|command| command.contains("--eval-spec"))));
    }

    #[test]
    fn tested_evidence_surfaces_boundary_review_candidates() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "tested-plan-fixture"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"pub fn shell(cmd: &str) {
    std::process::Command::new(cmd);
}

#[cfg(test)]
mod tests {
    #[test]
    fn smoke() {
        assert_eq!(1, 1);
    }
}
"#,
        )
        .unwrap();

        let plan = build_refactor_plan(
            dir.path(),
            None,
            &RefactorPlanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..RefactorPlanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(plan.evidence.grade, EvidenceGrade::Tested);
        assert_eq!(
            plan.evidence.analysis_depth,
            EvidenceAnalysisDepth::BoundaryAware
        );
        assert!(plan.candidates.iter().any(|candidate| candidate.status
            == RefactorCandidateStatus::PlanOnly
            && candidate.required_evidence == EvidenceGrade::Tested
            && candidate.tier == RecipeTier::Tier2));
    }

    #[test]
    fn measured_covered_evidence_unlocks_tier2_executable_recipe() {
        let dir = tempdir().unwrap();
        std::fs::write(
            dir.path().join("Cargo.toml"),
            r#"[package]
name = "covered-plan-fixture"
version = "0.1.0"
edition = "2021"
"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("src")).unwrap();
        std::fs::write(
            dir.path().join("src/lib.rs"),
            r#"pub fn labels(items: &[String]) -> Vec<&'static str> {
    if items.len() == 0 {
        return vec!["shared boundary label"];
    }
    vec![
        "shared boundary label",
        "shared boundary label",
        "shared boundary label",
    ]
}
"#,
        )
        .unwrap();
        let artifact_root = dir.path().join(".mdx-rust");
        std::fs::create_dir_all(artifact_root.join("evidence")).unwrap();
        let evidence = crate::evidence::EvidenceRun {
            schema_version: "1.0".to_string(),
            run_id: "covered-fixture".to_string(),
            root: dir.path().canonicalize().unwrap().display().to_string(),
            target: Some("src/lib.rs".to_string()),
            grade: EvidenceGrade::Covered,
            analysis_depth: EvidenceAnalysisDepth::Structural,
            metrics: Vec::new(),
            file_profiles: Vec::new(),
            commands: Vec::new(),
            unlocked_recipe_tiers: vec!["Tier 2 structural mechanical recipes".to_string()],
            unlock_suggestions: Vec::new(),
            note: "fixture evidence".to_string(),
            artifact_path: Some(
                artifact_root
                    .join("evidence/evidence-fixture.json")
                    .display()
                    .to_string(),
            ),
        };
        std::fs::write(
            artifact_root.join("evidence/evidence-fixture.json"),
            serde_json::to_string_pretty(&evidence).unwrap(),
        )
        .unwrap();

        let plan = build_refactor_plan(
            dir.path(),
            Some(&artifact_root),
            &RefactorPlanConfig {
                target: Some(PathBuf::from("src/lib.rs")),
                ..RefactorPlanConfig::default()
            },
        )
        .unwrap();

        assert_eq!(plan.evidence.grade, EvidenceGrade::Covered);
        assert!(plan.measured_evidence.is_some());
        assert!(plan.candidates.iter().any(|candidate| candidate.recipe
            == RefactorRecipe::RepeatedStringLiteralConst
            && candidate.status == RefactorCandidateStatus::ApplyViaImprove
            && candidate.required_evidence == EvidenceGrade::Covered
            && candidate.tier == RecipeTier::Tier2
            && candidate
                .apply_command
                .as_deref()
                .is_some_and(|command| command.contains("--tier 2"))));
        assert!(plan.candidates.iter().any(|candidate| candidate.recipe
            == RefactorRecipe::LenCheckIsEmpty
            && candidate.status == RefactorCandidateStatus::ApplyViaImprove
            && candidate.required_evidence == EvidenceGrade::Covered
            && candidate.tier == RecipeTier::Tier2));
    }

    #[test]
    fn security_summary_prioritizes_high_severity_top_findings() {
        let findings = vec![
            AuditFinding {
                id: "low".to_string(),
                severity: AuditSeverity::Low,
                title: "Low first".to_string(),
                description: "low".to_string(),
                file: Some("src/a.rs".to_string()),
                line: Some(1),
            },
            AuditFinding {
                id: "high".to_string(),
                severity: AuditSeverity::High,
                title: "High second".to_string(),
                description: "high".to_string(),
                file: Some("src/b.rs".to_string()),
                line: Some(2),
            },
        ];

        let summary = security_posture_summary(&findings);

        assert!(summary
            .top_findings
            .first()
            .is_some_and(|finding| finding.starts_with("High: High second")));
    }
}
