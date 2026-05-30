//! Agent-facing command contract for `mdx-rust`.
//!
//! This artifact is intentionally small and stable enough for coding agents to
//! inspect before deciding which CLI command to call.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct MdxAgentContract {
    pub schema_version: String,
    pub product_version: String,
    pub json_mode_contract: String,
    pub mutation_contract: String,
    pub commands: Vec<AgentCommandSpec>,
    pub workflows: Vec<AgentWorkflow>,
    pub artifact_globs: Vec<String>,
    pub safety_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentCommandSpec {
    pub name: String,
    pub purpose: String,
    pub mutates_source: bool,
    pub required_flags_for_mutation: Vec<String>,
    pub primary_schema: String,
    pub example: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentWorkflow {
    pub id: String,
    pub goal: String,
    pub steps: Vec<String>,
}

pub fn agent_contract() -> MdxAgentContract {
    MdxAgentContract {
        schema_version: "1.0".to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        json_mode_contract:
            "Pass --json for machine-pure stdout. Errors are emitted as structured JSON when --json is set."
                .to_string(),
        mutation_contract:
            "Only improve --apply, apply-plan --apply, autopilot --apply, evolve --apply, or a runtime evolve call with apply=true and confirm_mutation=true may mutate source files; every mutation routes through isolated validation and rollback gates."
                .to_string(),
        commands: vec![
            AgentCommandSpec {
                name: "runtime".to_string(),
                purpose:
                    "Describe local agent runtime transports, tools, and mutation rules."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "agent-runtime-manifest".to_string(),
                example: "mdx-rust --json runtime".to_string(),
            },
            AgentCommandSpec {
                name: "mcp".to_string(),
                purpose:
                    "Run the local stdio agent tool protocol for scorecard, evidence, map, plan, explain, recipes, and gated evolve."
                        .to_string(),
                mutates_source: true,
                required_flags_for_mutation: vec![
                    "apply=true".to_string(),
                    "confirm_mutation=true".to_string(),
                ],
                primary_schema: "agent-runtime-manifest".to_string(),
                example: "mdx-rust mcp --stdio".to_string(),
            },
            AgentCommandSpec {
                name: "serve".to_string(),
                purpose:
                    "Expose the same local agent tools over localhost HTTP for agents that prefer a socket API."
                        .to_string(),
                mutates_source: true,
                required_flags_for_mutation: vec![
                    "apply=true".to_string(),
                    "confirm_mutation=true".to_string(),
                ],
                primary_schema: "agent-runtime-manifest".to_string(),
                example: "mdx-rust serve --bind 127.0.0.1:3799 --token <token>".to_string(),
            },
            AgentCommandSpec {
                name: "agent-pack".to_string(),
                purpose:
                    "Generate agent instruction files that teach Codex, Claude, Cursor, Aider, Goose-style, or generic tools to use mdx-rust safely."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "agent-pack".to_string(),
                example: "mdx-rust --json agent-pack codex".to_string(),
            },
            AgentCommandSpec {
                name: "repo-map".to_string(),
                purpose:
                    "Build an agent-oriented repository map with instruction files, key directories, and noise filters."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "repo-map".to_string(),
                example: "mdx-rust --json repo-map src".to_string(),
            },
            AgentCommandSpec {
                name: "noise-filter".to_string(),
                purpose:
                    "Print or write default search exclusions for coding agents working in this repository."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "noise-filter".to_string(),
                example: "mdx-rust --json noise-filter".to_string(),
            },
            AgentCommandSpec {
                name: "contracts".to_string(),
                purpose:
                    "Scan Rust functions for documented preconditions, postconditions, invariants, safety notes, panic docs, and assertion hints."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "contract-run".to_string(),
                example: "mdx-rust --json contracts src/service".to_string(),
            },
            AgentCommandSpec {
                name: "perf".to_string(),
                purpose:
                    "Scan Rust code for static performance pressure such as clone pressure, allocation in loops, and blocking operations in async functions."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "performance-run".to_string(),
                example: "mdx-rust --json perf src/service".to_string(),
            },
            AgentCommandSpec {
                name: "benchmark".to_string(),
                purpose:
                    "Run command-based benchmark specs and persist measured performance evidence."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "benchmark-run".to_string(),
                example: "mdx-rust --json benchmark --spec .mdx-rust/benchmarks.json"
                    .to_string(),
            },
            AgentCommandSpec {
                name: "brief".to_string(),
                purpose:
                    "Build one fused agent briefing with repo context, contracts, performance, scorecard, and recommended next commands."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "evolution-brief".to_string(),
                example: "mdx-rust --json brief src/service".to_string(),
            },
            AgentCommandSpec {
                name: "recipes".to_string(),
                purpose:
                    "List recipe tiers, required evidence, and executable mutation paths."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "recipe-catalog".to_string(),
                example: "mdx-rust --json recipes".to_string(),
            },
            AgentCommandSpec {
                name: "scorecard".to_string(),
                purpose:
                    "Build one agent briefing with map, plan, recipe catalog, readiness, and next commands."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "evolution-scorecard".to_string(),
                example: "mdx-rust --json scorecard src/service".to_string(),
            },
            AgentCommandSpec {
                name: "agent-ready".to_string(),
                purpose:
                    "Return a compact readiness report for safe external-agent autonomy."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "agent-ready-report".to_string(),
                example: "mdx-rust --json agent-ready src/service".to_string(),
            },
            AgentCommandSpec {
                name: "evidence".to_string(),
                purpose:
                    "Collect measured evidence profiles that control autonomous recipe depth."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "evidence-run".to_string(),
                example: "mdx-rust --json evidence src/service --include-coverage".to_string(),
            },
            AgentCommandSpec {
                name: "map".to_string(),
                purpose:
                    "Build a non-mutating repo intelligence map with evidence gates and risks.".to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "codebase-map".to_string(),
                example: "mdx-rust --json map src/service".to_string(),
            },
            AgentCommandSpec {
                name: "plan".to_string(),
                purpose:
                    "Build a non-mutating refactor plan with executable and plan-only candidates."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "refactor-plan".to_string(),
                example: "mdx-rust --json plan src/service".to_string(),
            },
            AgentCommandSpec {
                name: "evolve".to_string(),
                purpose:
                    "Run budget-bounded autonomous improvement over evidence-allowed candidates."
                        .to_string(),
                mutates_source: true,
                required_flags_for_mutation: vec!["--apply".to_string()],
                primary_schema: "autopilot-run".to_string(),
                example:
                    "mdx-rust --json evolve src/service --budget 10m --tier 2 --min-evidence covered"
                        .to_string(),
            },
            AgentCommandSpec {
                name: "apply-plan".to_string(),
                purpose:
                    "Execute explicitly executable candidates from a saved plan after hash and staleness checks."
                        .to_string(),
                mutates_source: true,
                required_flags_for_mutation: vec!["--apply".to_string()],
                primary_schema: "refactor-batch-apply-run".to_string(),
                example: "mdx-rust --json apply-plan .mdx-rust/plans/plan.json --all".to_string(),
            },
            AgentCommandSpec {
                name: "explain".to_string(),
                purpose:
                    "Summarize an mdx-rust artifact and recommend safe next actions."
                        .to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "artifact-explanation".to_string(),
                example: "mdx-rust --json explain .mdx-rust/plans/refactor-plan.json"
                    .to_string(),
            },
            AgentCommandSpec {
                name: "schema".to_string(),
                purpose: "Emit JSON Schema for agent-facing artifacts.".to_string(),
                mutates_source: false,
                required_flags_for_mutation: Vec::new(),
                primary_schema: "json-schema".to_string(),
                example: "mdx-rust --json schema agent-contract".to_string(),
            },
        ],
        workflows: vec![
            AgentWorkflow {
                id: "safe-intake".to_string(),
                goal: "Understand a Rust target before proposing code changes.".to_string(),
                steps: vec![
                    "mdx-rust --json agent-contract".to_string(),
                    "mdx-rust --json runtime".to_string(),
                    "mdx-rust --json repo-map <target>".to_string(),
                    "mdx-rust --json noise-filter".to_string(),
                    "mdx-rust --json contracts <target>".to_string(),
                    "mdx-rust --json perf <target>".to_string(),
                    "mdx-rust --json benchmark --spec .mdx-rust/benchmarks.json".to_string(),
                    "mdx-rust --json brief <target>".to_string(),
                    "mdx-rust --json recipes".to_string(),
                    "mdx-rust --json agent-ready <target>".to_string(),
                    "mdx-rust --json scorecard <target>".to_string(),
                    "mdx-rust --json evidence <target>".to_string(),
                    "mdx-rust --json map <target>".to_string(),
                    "mdx-rust --json plan <target>".to_string(),
                ],
            },
            AgentWorkflow {
                id: "agent-runtime".to_string(),
                goal: "Let an external coding agent call mdx-rust through a local runtime surface.".to_string(),
                steps: vec![
                    "mdx-rust --json runtime".to_string(),
                    "mdx-rust mcp --stdio".to_string(),
                    "Call tools/list before tools/call.".to_string(),
                    "Only call mutation-capable tools with apply=true and confirm_mutation=true after human approval.".to_string(),
                ],
            },
            AgentWorkflow {
                id: "covered-autonomy".to_string(),
                goal: "Apply Tier 2 mechanical improvements only when measured coverage allows it."
                    .to_string(),
                steps: vec![
                    "mdx-rust --json evidence <target> --include-coverage".to_string(),
                    "mdx-rust --json evolve <target> --budget 10m --tier 2 --min-evidence covered"
                        .to_string(),
                    "Review the autopilot artifact before rerunning with --apply.".to_string(),
                ],
            },
        ],
        artifact_globs: vec![
            ".mdx-rust/evidence/*.json".to_string(),
            ".mdx-rust/maps/*.json".to_string(),
            ".mdx-rust/scorecards/*.json".to_string(),
            ".mdx-rust/briefs/*.json".to_string(),
            ".mdx-rust/benchmarks/*.json".to_string(),
            ".mdx-rust/plans/*.json".to_string(),
            ".mdx-rust/autopilot/*.json".to_string(),
            ".mdx-rust/hardening/*.json".to_string(),
            ".mdx-rust/agent-pack/*".to_string(),
        ],
        safety_rules: vec![
            "Treat repo-map and noise-filter as orientation surfaces, not proof that mutation is safe."
                .to_string(),
            "Treat contract scans as design evidence, not validation proof.".to_string(),
            "Treat performance scans as prioritization evidence, not proof that a refactor is safe."
                .to_string(),
            "Treat benchmark runs as measured evidence, not approval to mutate by themselves."
                .to_string(),
            "Treat plan and map commands as read-only.".to_string(),
            "Respect noise-filter exclusions before default search or context loading.".to_string(),
            "Never add --apply unless the user explicitly asked for mutation.".to_string(),
            "Runtime evolve calls with apply=true must also include confirm_mutation=true.".to_string(),
            "Do not bypass min-evidence or tier restrictions.".to_string(),
            "Re-run plan after any source file changes because stale plans are rejected.".to_string(),
            "Treat localhost HTTP as a local developer surface, not a remote multi-tenant service.".to_string(),
            "Use artifact_path fields as the source of truth for follow-up inspection.".to_string(),
        ],
    }
}
