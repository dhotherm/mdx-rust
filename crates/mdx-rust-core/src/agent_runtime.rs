//! Agent runtime manifest and pack artifacts.
//!
//! These records describe how external coding agents can call `mdx-rust`
//! without scraping human output.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRuntimeManifest {
    pub schema_version: String,
    pub product_version: String,
    pub protocol_version: String,
    pub http_auth: AgentRuntimeAuth,
    pub transports: Vec<AgentRuntimeTransport>,
    pub tools: Vec<AgentRuntimeTool>,
    pub mutation_rules: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRuntimeAuth {
    pub mode: String,
    pub header: String,
    pub env_var: String,
    pub required_when_token_configured: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRuntimeTransport {
    pub id: String,
    pub command: String,
    pub description: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentRuntimeTool {
    pub name: String,
    pub description: String,
    pub read_only: bool,
    pub mutation_capable: bool,
    pub required_flags_for_mutation: Vec<String>,
    pub request_schema: String,
    pub response_schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentPack {
    pub schema_version: String,
    pub target_agent: String,
    pub files: Vec<AgentPackFile>,
    pub install_note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentPackFile {
    pub path: String,
    pub purpose: String,
    pub content: String,
}

pub fn agent_runtime_manifest() -> AgentRuntimeManifest {
    AgentRuntimeManifest {
        schema_version: "1.0".to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        protocol_version: "mdx-runtime/1.0".to_string(),
        http_auth: AgentRuntimeAuth {
            mode: "optional-local-bearer-token".to_string(),
            header: "authorization: Bearer <token>".to_string(),
            env_var: "MDX_RUST_RUNTIME_TOKEN".to_string(),
            required_when_token_configured: true,
        },
        transports: vec![
            AgentRuntimeTransport {
                id: "cli-json".to_string(),
                command: "mdx-rust --json <command>".to_string(),
                description: "Stable command-line JSON contract for humans, scripts, and coding agents.".to_string(),
            },
            AgentRuntimeTransport {
                id: "mcp-stdio".to_string(),
                command: "mdx-rust mcp --stdio".to_string(),
                description: "Line-delimited JSON tool protocol over stdin/stdout for local coding agents.".to_string(),
            },
            AgentRuntimeTransport {
                id: "local-http".to_string(),
                command: "mdx-rust serve --bind 127.0.0.1:3799 --token <token>".to_string(),
                description: "Localhost-only HTTP surface for read-only tool calls and explicit mutation-gated evolution calls. A bearer token is required when configured.".to_string(),
            },
        ],
        tools: runtime_tools(),
        mutation_rules: vec![
            "Read-only tools must never mutate source files.".to_string(),
            "Mutation-capable tools require explicit apply=true and the same CLI safety gates.".to_string(),
            "Runtime callers cannot bypass evidence, stale-plan, validation, behavior eval, or rollback gates.".to_string(),
            "HTTP runtime callers must pass the configured bearer token when MDX_RUST_RUNTIME_TOKEN or --token is set.".to_string(),
            "Runtime callers should inspect artifact_path fields instead of scraping human output.".to_string(),
        ],
    }
}

pub fn agent_pack(target_agent: &str) -> AgentPack {
    let file_path = match target_agent {
        "codex" => ".codex/skills/mdx-rust-evolution/SKILL.md",
        "claude" => ".claude/skills/mdx-rust-evolution/SKILL.md",
        "cursor" => ".cursor/rules/mdx-rust-evolution.mdc",
        "aider" => ".mdx-rust/agent-pack/aider-conventions.md",
        "goose" => ".mdx-rust/agent-pack/goosehints.md",
        _ => ".mdx-rust/agent-pack/mdx-rust-evolution.md",
    };
    AgentPack {
        schema_version: "1.0".to_string(),
        target_agent: target_agent.to_string(),
        files: vec![AgentPackFile {
            path: file_path.to_string(),
            purpose: "Teach a coding agent how to use mdx-rust as a safe Rust evolution engine.".to_string(),
            content: agent_pack_content(target_agent),
        }],
        install_note:
            "Review the generated file before committing it. The pack is instructions only and never grants mutation permission by itself."
                .to_string(),
    }
}

fn runtime_tools() -> Vec<AgentRuntimeTool> {
    vec![
        runtime_tool(
            "agent-contract",
            "Discover commands, schemas, artifacts, and mutation rules.",
            true,
            false,
            "agent-contract-request",
            "agent-contract",
        ),
        runtime_tool(
            "recipes",
            "List evidence-gated recipe capabilities.",
            true,
            false,
            "recipes-request",
            "recipe-catalog",
        ),
        runtime_tool(
            "scorecard",
            "Build a single target briefing for external agents.",
            true,
            false,
            "scorecard-request",
            "evolution-scorecard",
        ),
        runtime_tool(
            "agent-ready",
            "Return a compact readiness report for safe external-agent autonomy.",
            true,
            false,
            "agent-ready-request",
            "agent-ready-report",
        ),
        runtime_tool(
            "evidence",
            "Collect measured test, coverage, mutation, and semver evidence.",
            true,
            false,
            "evidence-request",
            "evidence-run",
        ),
        runtime_tool(
            "map",
            "Build a non-mutating codebase map.",
            true,
            false,
            "map-request",
            "codebase-map",
        ),
        runtime_tool(
            "plan",
            "Build a non-mutating refactor plan.",
            true,
            false,
            "plan-request",
            "refactor-plan",
        ),
        runtime_tool(
            "explain",
            "Explain a saved mdx-rust artifact.",
            true,
            false,
            "explain-request",
            "artifact-explanation",
        ),
        runtime_tool(
            "evolve",
            "Run budget-bounded evolution through autopilot gates.",
            false,
            true,
            "evolve-request",
            "autopilot-run",
        ),
    ]
}

fn runtime_tool(
    name: &str,
    description: &str,
    read_only: bool,
    mutation_capable: bool,
    request_schema: &str,
    response_schema: &str,
) -> AgentRuntimeTool {
    AgentRuntimeTool {
        name: name.to_string(),
        description: description.to_string(),
        read_only,
        mutation_capable,
        required_flags_for_mutation: if mutation_capable {
            vec![
                "apply=true".to_string(),
                "confirm_mutation=true".to_string(),
            ]
        } else {
            Vec::new()
        },
        request_schema: request_schema.to_string(),
        response_schema: response_schema.to_string(),
    }
}

fn agent_pack_content(target_agent: &str) -> String {
    format!(
        r#"---
name: mdx-rust-evolution
description: Use mdx-rust to inspect, plan, and safely evolve Rust codebases with evidence-gated autonomy.
---

# mdx-rust Evolution

Use this when working on Rust repositories, especially when asked to harden,
refactor, improve quality, or let an agent make autonomous changes.

## Required Intake

1. Run `mdx-rust --json agent-contract`.
2. Run `mdx-rust --json scorecard <target>`.
3. Inspect `readiness`, `next_commands`, `security`, and candidate autonomy decisions.

## Mutation Rule

Never add `--apply` unless the human explicitly asked for mutation. Plans,
maps, recipes, explanations, evidence runs, and scorecards are read-only.

## Safe Workflows

- Review only: `mdx-rust --json evolve <target> --budget 10m --tier 2 --min-evidence covered`
- Apply Tier 1: `mdx-rust --json evolve <target> --budget 10m --tier 1 --apply`
- Apply Tier 2: `mdx-rust --json evidence <target> --include-coverage`, then `mdx-rust --json evolve <target> --budget 10m --tier 2 --min-evidence covered --apply`

## Reporting

Report artifact paths, evidence grade, executed candidates, validation status,
rollback status, and remaining blocked or review-only work.

Generated for: {target_agent}
"#
    )
}
