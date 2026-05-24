//! Machine-readable explanations for mdx-rust artifacts.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ArtifactExplanation {
    pub schema_version: String,
    pub artifact_path: String,
    pub artifact_kind: ArtifactKind,
    pub summary: String,
    pub mutates_source: bool,
    pub recommended_next_actions: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
pub enum ArtifactKind {
    AgentContract,
    AuditPacket,
    AutopilotRun,
    CodebaseMap,
    EvidenceRun,
    EvolutionScorecard,
    HardeningRun,
    RefactorApplyRun,
    RefactorBatchApplyRun,
    RefactorPlan,
    Unknown,
}

pub fn explain_artifact(path: &Path) -> anyhow::Result<ArtifactExplanation> {
    let content = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&content)?;
    let kind = artifact_kind(&value);
    let summary = artifact_summary(&kind, &value);
    let recommended_next_actions = artifact_next_actions(&kind, &value);

    Ok(ArtifactExplanation {
        schema_version: "0.8".to_string(),
        artifact_path: path.display().to_string(),
        artifact_kind: kind,
        summary,
        mutates_source: false,
        recommended_next_actions,
    })
}

fn artifact_kind(value: &serde_json::Value) -> ArtifactKind {
    if value.get("commands").is_some() && value.get("json_mode_contract").is_some() {
        ArtifactKind::AgentContract
    } else if value.get("autopilot").is_some() || value.get("edit_scope_contract").is_some() {
        ArtifactKind::AuditPacket
    } else if value.get("passes").is_some() && value.get("execution_summary").is_some() {
        ArtifactKind::AutopilotRun
    } else if value.get("scorecard_id").is_some() {
        ArtifactKind::EvolutionScorecard
    } else if value.get("map_id").is_some() {
        ArtifactKind::CodebaseMap
    } else if value.get("run_id").is_some() && value.get("unlocked_recipe_tiers").is_some() {
        ArtifactKind::EvidenceRun
    } else if value.get("outcome").is_some() && value.get("changes").is_some() {
        ArtifactKind::HardeningRun
    } else if value.get("plan_id").is_some() && value.get("candidate_id").is_some() {
        ArtifactKind::RefactorApplyRun
    } else if value.get("plan_id").is_some() && value.get("steps").is_some() {
        ArtifactKind::RefactorBatchApplyRun
    } else if value.get("plan_id").is_some() && value.get("candidates").is_some() {
        ArtifactKind::RefactorPlan
    } else {
        ArtifactKind::Unknown
    }
}

fn artifact_summary(kind: &ArtifactKind, value: &serde_json::Value) -> String {
    match kind {
        ArtifactKind::AutopilotRun => format!(
            "autopilot {:?}: {} executed, {} planned",
            value.get("status").and_then(|value| value.as_str()),
            value
                .get("total_executed_candidates")
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
            value
                .get("total_planned_candidates")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        ),
        ArtifactKind::CodebaseMap => format!(
            "codebase map with quality {:?} and security score {}",
            value
                .pointer("/quality/grade")
                .and_then(|value| value.as_str()),
            value
                .pointer("/security/score")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        ),
        ArtifactKind::EvidenceRun => format!(
            "evidence run graded {:?} with {} profiled file(s)",
            value.get("grade").and_then(|value| value.as_str()),
            value
                .get("file_profiles")
                .and_then(|value| value.as_array())
                .map(|profiles| profiles.len())
                .unwrap_or(0)
        ),
        ArtifactKind::EvolutionScorecard => format!(
            "evolution scorecard {:?}: {} executable candidate(s)",
            value
                .pointer("/readiness/grade")
                .and_then(|value| value.as_str()),
            value
                .pointer("/readiness/executable_candidates")
                .and_then(|value| value.as_u64())
                .unwrap_or(0)
        ),
        ArtifactKind::HardeningRun => format!(
            "hardening run {:?} with {} proposed change(s)",
            value
                .pointer("/outcome/status")
                .and_then(|value| value.as_str()),
            value
                .get("changes")
                .and_then(|value| value.as_array())
                .map(|changes| changes.len())
                .unwrap_or(0)
        ),
        ArtifactKind::RefactorPlan => format!(
            "refactor plan with {} candidate(s) and evidence {:?}",
            value
                .get("candidates")
                .and_then(|value| value.as_array())
                .map(|candidates| candidates.len())
                .unwrap_or(0),
            value
                .pointer("/evidence/grade")
                .and_then(|value| value.as_str())
        ),
        ArtifactKind::RefactorApplyRun | ArtifactKind::RefactorBatchApplyRun => format!(
            "plan application {:?}",
            value.get("status").and_then(|value| value.as_str())
        ),
        ArtifactKind::AgentContract => "agent command contract for mdx-rust".to_string(),
        ArtifactKind::AuditPacket => "optimization audit packet".to_string(),
        ArtifactKind::Unknown => "unrecognized mdx-rust JSON artifact".to_string(),
    }
}

fn artifact_next_actions(kind: &ArtifactKind, value: &serde_json::Value) -> Vec<String> {
    match kind {
        ArtifactKind::EvidenceRun => vec![
            "Run mdx-rust --json map <target> to see evidence-gated risk and recipe eligibility."
                .to_string(),
            "Run mdx-rust --json plan <target> before applying autonomous changes.".to_string(),
        ],
        ArtifactKind::CodebaseMap => vec![
            "Inspect recommended_actions and capability_gates before mutation.".to_string(),
            "Run mdx-rust --json plan <target> to create a stale-checked plan.".to_string(),
        ],
        ArtifactKind::EvolutionScorecard => vec![
            "Inspect readiness and next_commands before choosing mutation.".to_string(),
            "Only add --apply to suggested commands after explicit human approval.".to_string(),
        ],
        ArtifactKind::RefactorPlan => {
            let has_executable = value
                .get("candidates")
                .and_then(|value| value.as_array())
                .is_some_and(|candidates| {
                    candidates.iter().any(|candidate| {
                        candidate.get("status").and_then(|value| value.as_str())
                            == Some("ApplyViaImprove")
                    })
                });
            if has_executable {
                vec![
                    "Run mdx-rust --json apply-plan <artifact> --all for review mode.".to_string(),
                    "Only add --apply when the user explicitly approves source mutation."
                        .to_string(),
                ]
            } else {
                vec![
                    "No executable candidates are present; treat this as a design review plan."
                        .to_string(),
                ]
            }
        }
        ArtifactKind::AutopilotRun => vec![
            "Read execution_summary before reporting progress to a human.".to_string(),
            "Use artifact_path values to inspect nested plan and hardening reports.".to_string(),
        ],
        ArtifactKind::HardeningRun
        | ArtifactKind::RefactorApplyRun
        | ArtifactKind::RefactorBatchApplyRun => {
            vec![
                "Inspect validation and rollback records before claiming a change landed."
                    .to_string(),
            ]
        }
        ArtifactKind::AgentContract | ArtifactKind::AuditPacket | ArtifactKind::Unknown => {
            vec![
                "Use mdx-rust --json agent-contract before selecting the next command.".to_string(),
            ]
        }
    }
}
