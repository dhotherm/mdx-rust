use clap::{Parser, Subcommand};
use mdx_rust_core::Config;

/// MDx Rust — A Rust-native safe-change system.
///
/// Point mdx-rust at Rust code, give it policies and validation gates,
/// and let it propose scoped, auditable hardening changes.
#[derive(Parser)]
#[command(name = "mdx-rust")]
#[command(version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    /// Use JSON output (for agent callers and automation)
    #[arg(long, global = true)]
    json: bool,

    /// Path to config file (defaults to .mdx-rust/config.toml or env)
    #[arg(long, global = true, env = "MDX_RUST_CONFIG")]
    config: Option<String>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize .mdx-rust/ directory and configuration in the current project
    Init,

    /// Register an agent (detect entrypoint, create harness if needed, smoke test)
    Register {
        /// Name for this agent (used in artifacts and reports)
        name: String,

        /// Path to the agent crate or entrypoint file (defaults to current dir)
        path: Option<String>,
    },

    /// Generate or update policy, eval spec, and dataset from the registered agent
    Spec {
        /// Agent name (from `register`)
        name: String,
    },

    /// Run the optimization loop against the registered agent
    Optimize {
        /// Agent name
        name: String,

        /// Maximum optimization iterations
        #[arg(long, default_value = "3")]
        iterations: u32,

        /// Optimization budget: light, medium, or heavy
        #[arg(long, default_value = "medium", value_parser = ["light", "medium", "heavy"])]
        budget: String,

        /// Pause and show proposed changes before applying (human review)
        #[arg(long)]
        review: bool,
    },

    /// Print the machine-readable command contract for coding agents
    AgentContract,

    /// Print the local agent runtime manifest
    Runtime,

    /// Generate agent instructions for Codex, Claude, Cursor, Aider, Goose, or generic coding agents
    AgentPack {
        /// Agent target: codex, claude, cursor, aider, goose, or generic
        #[arg(default_value = "generic", value_parser = ["codex", "claude", "cursor", "aider", "goose", "generic"])]
        target: String,

        /// Write the pack files into the current workspace
        #[arg(long)]
        write: bool,
    },

    /// Build an agent-oriented repository map and context guide
    RepoMap {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Maximum directory depth to include
        #[arg(long, default_value = "3")]
        max_depth: usize,

        /// Maximum directories to list
        #[arg(long, default_value = "80")]
        max_dirs: usize,
    },

    /// Print or write default noise filters for coding agents
    NoiseFilter {
        /// Write .mdx-rust/agent-pack/noise-filter artifacts
        #[arg(long)]
        write: bool,
    },

    /// Scan Rust functions for lightweight contract and invariant coverage
    Contracts {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// Scan Rust code for static performance pressure signals
    Perf {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// Run command-based benchmark specs and persist measured performance evidence
    Benchmark {
        /// Benchmark spec path, defaults to .mdx-rust/benchmarks.json
        #[arg(long, default_value = ".mdx-rust/benchmarks.json")]
        spec: String,
    },

    /// Run the local stdio agent tool protocol
    Mcp {
        /// Use stdin/stdout line-delimited JSON transport
        #[arg(long)]
        stdio: bool,

        /// Print the runtime manifest and exit
        #[arg(long)]
        describe: bool,
    },

    /// Serve the local agent runtime over localhost HTTP
    Serve {
        /// Bind address. Use localhost only.
        #[arg(long, default_value = "127.0.0.1:3799")]
        bind: String,

        /// Optional bearer token for HTTP runtime calls, also read from MDX_RUST_RUNTIME_TOKEN
        #[arg(long, env = "MDX_RUST_RUNTIME_TOKEN")]
        token: Option<String>,

        /// Handle one request and exit, useful for smoke tests
        #[arg(long)]
        once: bool,
    },

    /// Report whether this workspace is ready for safe coding-agent autonomy
    AgentReady {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and attach to the readiness report
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that future apply commands should pass
        #[arg(long)]
        eval_spec: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// List recipe tiers, evidence requirements, and executable mutation paths
    Recipes,

    /// Explain a saved mdx-rust artifact for human or agent follow-up
    Explain {
        /// Path to a JSON artifact produced by mdx-rust
        artifact: String,
    },

    /// Build one agent-first evolution scorecard with map, plan, recipes, and readiness
    Scorecard {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and attach to the scorecard
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that future apply commands should pass
        #[arg(long)]
        eval_spec: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// Build one fused agent brief with repo context, contracts, perf, scorecard, and next actions
    Brief {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and attach to the brief
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that future apply commands should pass
        #[arg(long)]
        eval_spec: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// Collect measured evidence that controls autonomous recipe depth
    Evidence {
        /// File or directory to associate with the evidence run
        target: Option<String>,

        /// Run cargo-llvm-cov when available
        #[arg(long)]
        include_coverage: bool,

        /// Run cargo-mutants when available
        #[arg(long)]
        include_mutation: bool,

        /// Run cargo-semver-checks when available
        #[arg(long)]
        include_semver: bool,

        /// Per-command timeout in seconds
        #[arg(long, default_value = "180")]
        timeout_seconds: u64,
    },

    /// Inspect what would be bundled, editable scope, and current state
    Doctor {
        /// Optional agent name. Omit to inspect the current Rust workspace.
        name: Option<String>,
    },

    /// Propose or apply scoped Rust hardening changes for ordinary modules
    Improve {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional goal text recorded in reports
        #[arg(long)]
        goal: Option<String>,

        /// Optional policy file to hash and attach to the hardening report
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec to run after compile validation
        #[arg(long)]
        eval_spec: Option<String>,

        /// Apply validated changes to the real tree. Without this, review mode is used.
        #[arg(long)]
        apply: bool,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "100")]
        max_files: usize,

        /// Maximum recipe tier to analyze and execute: 1, 2, or 3
        #[arg(long, default_value = "1", value_parser = ["1", "2", "3", "tier1", "tier2", "tier3"])]
        tier: String,

        /// Validation timeout in seconds
        #[arg(long, default_value = "180")]
        timeout_seconds: u64,
    },

    /// Build a plan-first refactor report without mutating the workspace
    Plan {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and attach to the plan
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that future apply commands must pass
        #[arg(long)]
        eval_spec: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "100")]
        max_files: usize,
    },

    /// Build a repo intelligence map without mutating the workspace
    Map {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and attach to the map
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that future apply commands should pass
        #[arg(long)]
        eval_spec: Option<String>,

        /// Maximum Rust files to scan
        #[arg(long, default_value = "250")]
        max_files: usize,
    },

    /// Plan, queue, and execute safe autonomous refactor passes
    Autopilot {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and enforce through hardening reports
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that every applied pass must satisfy
        #[arg(long)]
        eval_spec: Option<String>,

        /// Apply validated changes to the real tree. Without this, review mode is used.
        #[arg(long)]
        apply: bool,

        /// Allow execution when a candidate is marked as public API impacting
        #[arg(long)]
        allow_public_api_impact: bool,

        /// Maximum Rust files to scan per pass
        #[arg(long, default_value = "250")]
        max_files: usize,

        /// Maximum autonomous passes. Each apply pass replans before continuing.
        #[arg(long, default_value = "3")]
        max_passes: usize,

        /// Maximum executable candidates per pass
        #[arg(long, default_value = "25")]
        max_candidates: usize,

        /// Maximum recipe tier to execute: 1, 2, or 3
        #[arg(long, default_value = "1", value_parser = ["1", "2", "3", "tier1", "tier2", "tier3"])]
        tier: String,

        /// Minimum evidence grade required before executing autonomous work
        #[arg(long, default_value = "compiled", value_parser = ["none", "compiled", "tested", "covered", "hardened", "proven"])]
        min_evidence: String,

        /// Optional total run budget such as 10m or 300s
        #[arg(long)]
        budget: Option<String>,

        /// Validation timeout in seconds
        #[arg(long, default_value = "180")]
        timeout_seconds: u64,
    },

    /// Budget-bounded autonomous repo improvement for agent callers
    Evolve {
        /// File or directory to inspect (defaults to current workspace)
        target: Option<String>,

        /// Optional policy file to hash and enforce through hardening reports
        #[arg(long)]
        policy: Option<String>,

        /// Optional behavior eval spec that every applied pass must satisfy
        #[arg(long)]
        eval_spec: Option<String>,

        /// Apply validated changes to the real tree. Without this, review mode is used.
        #[arg(long)]
        apply: bool,

        /// Run budget such as 10m or 300s
        #[arg(long, default_value = "10m")]
        budget: String,

        /// Maximum recipe tier to execute: 1, 2, or 3
        #[arg(long, default_value = "1", value_parser = ["1", "2", "3", "tier1", "tier2", "tier3"])]
        tier: String,

        /// Minimum evidence grade required before executing autonomous work
        #[arg(long, default_value = "compiled", value_parser = ["none", "compiled", "tested", "covered", "hardened", "proven"])]
        min_evidence: String,

        /// Maximum Rust files to scan per pass
        #[arg(long, default_value = "250")]
        max_files: usize,

        /// Maximum executable candidates per pass
        #[arg(long, default_value = "25")]
        max_candidates: usize,

        /// Allow execution when a candidate is marked as public API impacting
        #[arg(long)]
        allow_public_api_impact: bool,
    },

    /// Execute an approved candidate from a saved refactor plan
    ApplyPlan {
        /// Path to a refactor plan JSON artifact
        plan: String,

        /// Candidate id from the plan artifact
        #[arg(long)]
        candidate: Option<String>,

        /// Review or apply every executable low-risk candidate in the plan
        #[arg(long, conflicts_with = "candidate")]
        all: bool,

        /// Apply the candidate to the real tree. Without this, review mode is used.
        #[arg(long)]
        apply: bool,

        /// Allow execution when the candidate is marked as public API impacting
        #[arg(long)]
        allow_public_api_impact: bool,

        /// Validation timeout in seconds
        #[arg(long, default_value = "180")]
        timeout_seconds: u64,

        /// Maximum executable candidates to process with --all
        #[arg(long, default_value = "25")]
        max_candidates: usize,
    },

    /// Evaluate a registered agent or run behavior evals for the current workspace
    Eval {
        /// Optional agent name. Omit to run workspace behavior evals.
        name: Option<String>,

        /// Path to dataset JSON file
        #[arg(long)]
        dataset: Option<String>,

        /// Path to workspace behavior eval spec JSON
        #[arg(long)]
        spec: Option<String>,
    },

    /// Run deterministic static security checks against an agent or current workspace
    Audit {
        /// Optional agent name. Omit to audit the current Rust workspace.
        name: Option<String>,

        /// Optional policy file to hash and include when auditing the current workspace
        #[arg(long)]
        policy: Option<String>,
    },

    /// Print JSON Schema for machine-readable mdx-rust artifacts
    Schema {
        /// Schema to print: agent-contract, agent-runtime-manifest, agent-pack, repo-map, noise-filter, contract-run, performance-run, evolution-brief, agent-ready-report, artifact-explanation, audit-packet, candidate, optimization-run, hook-decision, trace-event, hardening-run, hardening-finding, behavior-eval-report, project-policy, evidence-run, recipe-catalog, evolution-scorecard, refactor-plan, refactor-apply-run, refactor-batch-apply-run, codebase-map, autopilot-run
        #[arg(value_parser = ["agent-contract", "agent-runtime-manifest", "agent-pack", "repo-map", "noise-filter", "contract-run", "performance-run", "benchmark-spec", "benchmark-run", "evolution-brief", "agent-ready-report", "artifact-explanation", "audit-packet", "candidate", "optimization-run", "hook-decision", "trace-event", "hardening-run", "hardening-finding", "behavior-eval-report", "project-policy", "evidence-run", "recipe-catalog", "evolution-scorecard", "refactor-plan", "refactor-apply-run", "refactor-batch-apply-run", "codebase-map", "autopilot-run"])]
        kind: String,
    },

    /// (Dev) Invoke a registered agent with a JSON input (useful for testing)
    #[clap(hide = true)]
    Invoke {
        name: String,
        #[arg(long)]
        input: Option<String>,
    },
}

fn main() {
    let cli = Cli::parse();

    // Basic tracing setup (pretty for humans, json when --json)
    init_tracing(cli.json);

    match cli.command {
        Commands::Init => {
            if let Err(e) = cmd_init(cli.json) {
                emit_error(cli.json, "init", &e);
                std::process::exit(1);
            }
        }
        Commands::Register { name, path } => {
            if let Err(e) = cmd_register(&name, path.as_deref(), cli.json) {
                emit_error(cli.json, "register", &e);
                std::process::exit(1);
            }
        }
        Commands::Spec { name } => {
            if let Err(e) = cmd_spec(&name, cli.json) {
                emit_error(cli.json, "spec", &e);
                std::process::exit(1);
            }
        }
        Commands::Optimize {
            name,
            iterations,
            budget,
            review,
            ..
        } => {
            if let Err(e) = cmd_optimize(&name, iterations, &budget, review, cli.json) {
                emit_error(cli.json, "optimize", &e);
                std::process::exit(1);
            }
        }
        Commands::AgentContract => {
            if let Err(e) = cmd_agent_contract(cli.json) {
                emit_error(cli.json, "agent-contract", &e);
                std::process::exit(1);
            }
        }
        Commands::Runtime => {
            if let Err(e) = cmd_runtime(cli.json) {
                emit_error(cli.json, "runtime", &e);
                std::process::exit(1);
            }
        }
        Commands::AgentPack { target, write } => {
            if let Err(e) = cmd_agent_pack(&target, write, cli.json) {
                emit_error(cli.json, "agent-pack", &e);
                std::process::exit(1);
            }
        }
        Commands::RepoMap {
            target,
            max_depth,
            max_dirs,
        } => {
            if let Err(e) = cmd_repo_map(target.as_deref(), max_depth, max_dirs, cli.json) {
                emit_error(cli.json, "repo-map", &e);
                std::process::exit(1);
            }
        }
        Commands::NoiseFilter { write } => {
            if let Err(e) = cmd_noise_filter(write, cli.json) {
                emit_error(cli.json, "noise-filter", &e);
                std::process::exit(1);
            }
        }
        Commands::Contracts { target, max_files } => {
            if let Err(e) = cmd_contracts(target.as_deref(), max_files, cli.json) {
                emit_error(cli.json, "contracts", &e);
                std::process::exit(1);
            }
        }
        Commands::Perf { target, max_files } => {
            if let Err(e) = cmd_perf(target.as_deref(), max_files, cli.json) {
                emit_error(cli.json, "perf", &e);
                std::process::exit(1);
            }
        }
        Commands::Benchmark { spec } => {
            if let Err(e) = cmd_benchmark(&spec, cli.json) {
                emit_error(cli.json, "benchmark", &e);
                std::process::exit(1);
            }
        }
        Commands::Mcp { stdio, describe } => {
            if let Err(e) = cmd_mcp(stdio, describe, cli.json) {
                emit_error(cli.json, "mcp", &e);
                std::process::exit(1);
            }
        }
        Commands::Serve { bind, token, once } => {
            if let Err(e) = cmd_serve(&bind, token.as_deref(), once, cli.json) {
                emit_error(cli.json, "serve", &e);
                std::process::exit(1);
            }
        }
        Commands::AgentReady {
            target,
            policy,
            eval_spec,
            max_files,
        } => {
            if let Err(e) = cmd_agent_ready(
                target.as_deref(),
                policy.as_deref(),
                eval_spec.as_deref(),
                max_files,
                cli.json,
            ) {
                emit_error(cli.json, "agent-ready", &e);
                std::process::exit(1);
            }
        }
        Commands::Recipes => {
            if let Err(e) = cmd_recipes(cli.json) {
                emit_error(cli.json, "recipes", &e);
                std::process::exit(1);
            }
        }
        Commands::Explain { artifact } => {
            if let Err(e) = cmd_explain(&artifact, cli.json) {
                emit_error(cli.json, "explain", &e);
                std::process::exit(1);
            }
        }
        Commands::Scorecard {
            target,
            policy,
            eval_spec,
            max_files,
        } => {
            if let Err(e) = cmd_scorecard(
                target.as_deref(),
                policy.as_deref(),
                eval_spec.as_deref(),
                max_files,
                cli.json,
            ) {
                emit_error(cli.json, "scorecard", &e);
                std::process::exit(1);
            }
        }
        Commands::Brief {
            target,
            policy,
            eval_spec,
            max_files,
        } => {
            if let Err(e) = cmd_brief(
                target.as_deref(),
                policy.as_deref(),
                eval_spec.as_deref(),
                max_files,
                cli.json,
            ) {
                emit_error(cli.json, "brief", &e);
                std::process::exit(1);
            }
        }
        Commands::Evidence {
            target,
            include_coverage,
            include_mutation,
            include_semver,
            timeout_seconds,
        } => {
            if let Err(e) = cmd_evidence(EvidenceCommand {
                target: target.as_deref(),
                include_coverage,
                include_mutation,
                include_semver,
                timeout_seconds,
                json: cli.json,
            }) {
                emit_error(cli.json, "evidence", &e);
                std::process::exit(1);
            }
        }
        Commands::Doctor { name } => {
            if let Err(e) = cmd_doctor(name.as_deref(), cli.json) {
                emit_error(cli.json, "doctor", &e);
                std::process::exit(1);
            }
        }
        Commands::Improve {
            target,
            goal,
            policy,
            eval_spec,
            apply,
            max_files,
            tier,
            timeout_seconds,
        } => {
            if let Err(e) = cmd_improve(ImproveCommand {
                target: target.as_deref(),
                goal: goal.as_deref(),
                policy: policy.as_deref(),
                eval_spec: eval_spec.as_deref(),
                apply,
                max_files,
                tier: &tier,
                timeout_seconds,
                json: cli.json,
            }) {
                emit_error(cli.json, "improve", &e);
                std::process::exit(1);
            }
        }
        Commands::Plan {
            target,
            policy,
            eval_spec,
            max_files,
        } => {
            if let Err(e) = cmd_plan(
                target.as_deref(),
                policy.as_deref(),
                eval_spec.as_deref(),
                max_files,
                cli.json,
            ) {
                emit_error(cli.json, "plan", &e);
                std::process::exit(1);
            }
        }
        Commands::Map {
            target,
            policy,
            eval_spec,
            max_files,
        } => {
            if let Err(e) = cmd_map(
                target.as_deref(),
                policy.as_deref(),
                eval_spec.as_deref(),
                max_files,
                cli.json,
            ) {
                emit_error(cli.json, "map", &e);
                std::process::exit(1);
            }
        }
        Commands::Autopilot {
            target,
            policy,
            eval_spec,
            apply,
            allow_public_api_impact,
            max_files,
            max_passes,
            max_candidates,
            tier,
            min_evidence,
            budget,
            timeout_seconds,
        } => {
            if let Err(e) = cmd_autopilot(AutopilotCommand {
                target: target.as_deref(),
                policy: policy.as_deref(),
                eval_spec: eval_spec.as_deref(),
                apply,
                allow_public_api_impact,
                max_files,
                max_passes,
                max_candidates,
                tier: &tier,
                min_evidence: &min_evidence,
                budget: budget.as_deref(),
                timeout_seconds,
                json: cli.json,
            }) {
                emit_error(cli.json, "autopilot", &e);
                std::process::exit(1);
            }
        }
        Commands::Evolve {
            target,
            policy,
            eval_spec,
            apply,
            budget,
            tier,
            min_evidence,
            max_files,
            max_candidates,
            allow_public_api_impact,
        } => {
            let budget_duration = match parse_budget(&budget) {
                Ok(duration) => duration,
                Err(e) => {
                    emit_error(cli.json, "evolve", &e);
                    std::process::exit(1);
                }
            };
            let max_passes = max_passes_for_budget(budget_duration);
            if let Err(e) = cmd_autopilot(AutopilotCommand {
                target: target.as_deref(),
                policy: policy.as_deref(),
                eval_spec: eval_spec.as_deref(),
                apply,
                allow_public_api_impact,
                max_files,
                max_passes,
                max_candidates,
                tier: &tier,
                min_evidence: &min_evidence,
                budget: Some(&budget),
                timeout_seconds: budget_duration.as_secs().clamp(30, 180),
                json: cli.json,
            }) {
                emit_error(cli.json, "evolve", &e);
                std::process::exit(1);
            }
        }
        Commands::ApplyPlan {
            plan,
            candidate,
            all,
            apply,
            allow_public_api_impact,
            timeout_seconds,
            max_candidates,
        } => {
            if let Err(e) = cmd_apply_plan(ApplyPlanCommand {
                plan_path: &plan,
                candidate_id: candidate.as_deref(),
                all,
                apply,
                allow_public_api_impact,
                timeout_seconds,
                max_candidates,
                json: cli.json,
            }) {
                emit_error(cli.json, "apply-plan", &e);
                std::process::exit(1);
            }
        }
        Commands::Eval {
            name,
            dataset,
            spec,
        } => {
            if let Err(e) = cmd_eval(
                name.as_deref(),
                dataset.as_deref(),
                spec.as_deref(),
                cli.json,
            ) {
                emit_error(cli.json, "eval", &e);
                std::process::exit(1);
            }
        }
        Commands::Audit { name, policy } => {
            if let Err(e) = cmd_audit(name.as_deref(), policy.as_deref(), cli.json) {
                emit_error(cli.json, "audit", &e);
                std::process::exit(1);
            }
        }
        Commands::Schema { kind } => {
            if let Err(e) = cmd_schema(&kind, cli.json) {
                emit_error(cli.json, "schema", &e);
                std::process::exit(1);
            }
        }
        Commands::Invoke { name, input } => {
            if let Err(e) = cmd_invoke(&name, input.as_deref(), cli.json) {
                emit_error(cli.json, "invoke", &e);
                std::process::exit(1);
            }
        }
    }
}

fn emit_error(json: bool, command: &str, error: &anyhow::Error) {
    let suggestion = error_suggestion(command, &error.to_string());
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "command": command,
                "error": error.to_string(),
                "suggestion": suggestion
            })
        );
    } else {
        eprintln!("{} error: {}", command, error);
        if let Some(suggestion) = suggestion {
            eprintln!("next step: {}", suggestion);
        }
    }
}

fn error_suggestion(command: &str, error: &str) -> Option<&'static str> {
    if error.contains("not registered") {
        return Some("run `mdx-rust register <name> <path>` and then retry");
    }
    if error.contains("No Cargo.toml") || error.contains("Cannot find Cargo.toml") {
        return Some("point mdx-rust at a Rust crate root that contains Cargo.toml");
    }
    if error.contains("dataset") && command == "eval" {
        return Some("run `mdx-rust spec <name>` to generate a starter dataset, or pass --dataset");
    }
    if error.contains("unknown optimization budget") {
        return Some("use one of: light, medium, heavy");
    }
    None
}

/// Initialize tracing with nice human output by default, or JSON when requested.
/// Supports RUST_LOG for fine-grained control (e.g. RUST_LOG=mdx_rust_core::runner=debug)
fn init_tracing(json: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::from_default_env().add_directive(
        "mdx_rust=info"
            .parse()
            .unwrap_or_else(|_| "info".parse().unwrap()),
    );

    if json {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .json()
            .init();
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(filter)
            .with_target(false)
            .compact()
            .init();
    }
}

/// Run the `init` command
fn cmd_init(json: bool) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;

    let artifact_dir = &config.artifact_dir;

    if Path::new(artifact_dir).exists() {
        if json {
            println!(
                "{}",
                serde_json::json!({"status":"already_initialized","path":artifact_dir})
            );
        } else {
            println!(
                "mdx-rust is already initialized in this directory ({} exists).",
                artifact_dir
            );
        }
        return Ok(());
    }

    fs::create_dir(artifact_dir)?;

    // Write a good default config file (using the loaded defaults + comments)
    let config_content = r#"# mdx-rust configuration
# This file was generated by `mdx-rust init`. Edit freely.

# Artifact directory for mdx-rust state and reports.
artifact_dir = ".mdx-rust"

[models]
# analyzer = "claude-4-sonnet"     # Used for deep diagnosis and candidate generation
# judge    = "gpt-4o"              # Used for LLM-as-Judge scoring
# default  = "gpt-4o-mini"
"#;
    fs::write(format!("{}/config.toml", artifact_dir), config_content)?;

    // .mdx-rustignore
    let ignore_content = r#"# mdx-rust ignore file
# Patterns here (plus .gitignore) will be excluded when building code bundles for the LLM.

target/
**/*.rlib
**/*.rmeta
Cargo.lock
.idea/
.vscode/
.mdx-rust/
"#;
    fs::write(format!("{}/.mdx-rustignore", artifact_dir), ignore_content)?;

    // Starter policy
    let policies = r#"# Project Policy

## Purpose
[Describe the purpose of this Rust project in 1-2 sentences]

## Hardening Rules
1. Avoid panics in request, CLI, and service boundary paths.
2. Preserve contextual errors for filesystem, environment, network, and database failures.
3. Validate external inputs before using them in risky operations.

## Constraints
- ...

## Quality Expectations
- ...
"#;
    fs::write(format!("{}/policies.md", artifact_dir), policies)?;

    // Minimal eval spec
    let eval_spec = r#"{
  "version": 1,
  "description": "Evaluation specification for this agent",
  "scoring": { "fields": [] },
  "policy_path": "policies.md"
}"#;
    fs::write(format!("{}/eval_spec.json", artifact_dir), eval_spec)?;

    let behavior_evals = r#"{
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
}"#;
    fs::write(format!("{}/evals.json", artifact_dir), behavior_evals)?;

    if json {
        println!(
            "{}",
            serde_json::json!({"status":"initialized","artifact_dir":artifact_dir})
        );
    } else {
        println!("✅ mdx-rust initialized in {}", cwd.display());
        println!("   Artifact directory: {}", artifact_dir);
        println!();
        println!("Created files:");
        println!("  {}/config.toml", artifact_dir);
        println!("  {}/.mdx-rustignore", artifact_dir);
        println!("  {}/policies.md", artifact_dir);
        println!("  {}/eval_spec.json", artifact_dir);
        println!("  {}/evals.json", artifact_dir);
        println!();
        println!("Next: mdx-rust doctor");
        println!("Or:   mdx-rust register <name> [path]");
    }

    Ok(())
}

/// Evidence command arguments.
struct EvidenceCommand<'a> {
    target: Option<&'a str>,
    include_coverage: bool,
    include_mutation: bool,
    include_semver: bool,
    timeout_seconds: u64,
    json: bool,
}

fn cmd_agent_contract(json: bool) -> anyhow::Result<()> {
    let contract = mdx_rust_core::agent_contract();
    if json {
        println!("{}", serde_json::to_string_pretty(&contract)?);
        return Ok(());
    }

    println!("🤖 mdx-rust agent contract");
    println!("   Schema: {}", contract.schema_version);
    println!("   Version: {}", contract.product_version);
    println!("   JSON mode: {}", contract.json_mode_contract);
    println!("   Mutation: {}", contract.mutation_contract);
    println!("   Agent-safe commands:");
    for command in &contract.commands {
        let mutation = if command.mutates_source {
            "mutation-capable"
        } else {
            "read-only"
        };
        println!("   - {} ({mutation}): {}", command.name, command.purpose);
    }
    println!("   Start with: mdx-rust --json agent-contract");
    Ok(())
}

fn cmd_runtime(json: bool) -> anyhow::Result<()> {
    let manifest = mdx_rust_core::agent_runtime_manifest();
    if json {
        println!("{}", serde_json::to_string_pretty(&manifest)?);
        return Ok(());
    }

    println!("🔌 mdx-rust agent runtime");
    println!("   Schema: {}", manifest.schema_version);
    println!("   Version: {}", manifest.product_version);
    println!("   Transports:");
    for transport in &manifest.transports {
        println!("   - {}: {}", transport.id, transport.command);
    }
    println!("   Tools:");
    for tool in &manifest.tools {
        let mode = if tool.mutation_capable {
            "mutation-capable"
        } else {
            "read-only"
        };
        println!("   - {} ({mode}): {}", tool.name, tool.description);
    }
    Ok(())
}

fn cmd_agent_pack(target: &str, write: bool, json: bool) -> anyhow::Result<()> {
    let pack = mdx_rust_core::agent_pack(target);
    let mut written = Vec::new();
    if write {
        for file in &pack.files {
            let path = std::path::Path::new(&file.path);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, &file.content)?;
            written.push(path.display().to_string());
        }
    }

    if json {
        let mut value = serde_json::to_value(&pack)?;
        value["written_files"] = serde_json::json!(written);
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("📦 mdx-rust agent pack for {target}");
    println!("   {}", pack.install_note);
    for file in &pack.files {
        println!("   - {}: {}", file.path, file.purpose);
    }
    if write {
        println!("   Written files:");
        for path in written {
            println!("   - {path}");
        }
    } else {
        println!("   Review mode only. Re-run with --write to create files.");
    }
    Ok(())
}

fn cmd_repo_map(
    target: Option<&str>,
    max_depth: usize,
    max_dirs: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let map = mdx_rust_core::build_repo_map(
        &cwd,
        &mdx_rust_core::RepoMapConfig {
            target: target.map(std::path::PathBuf::from),
            max_depth,
            max_dirs,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&map)?);
        return Ok(());
    }

    println!("🧭 mdx-rust repo map");
    println!("   Target: {}", map.target.display());
    println!(
        "   Directories: {} | Rust files: {} | crates: {}",
        map.summary.total_directories_listed,
        map.summary.total_rust_files,
        map.summary.detected_crates
    );
    println!("   Agent intake:");
    for step in &map.agent_intake {
        println!("   - {step}");
    }
    println!("   Top directories:");
    for dir in map.directories.iter().take(12) {
        println!(
            "   - {} ({}) rust_files={}",
            dir.path.display(),
            dir.role,
            dir.rust_files
        );
    }
    Ok(())
}

fn cmd_noise_filter(write: bool, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let filter = mdx_rust_core::build_noise_filter(&cwd);
    let mut written = Vec::new();

    if write {
        let json_path = cwd.join(".mdx-rust/agent-pack/noise-filter.json");
        let md_path = cwd.join(".mdx-rust/agent-pack/noise-filter.md");
        if let Some(parent) = json_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&json_path, serde_json::to_string_pretty(&filter)?)?;
        std::fs::write(&md_path, mdx_rust_core::noise_filter_markdown(&filter))?;
        written.push(json_path.display().to_string());
        written.push(md_path.display().to_string());
    }

    if json {
        let mut value = serde_json::to_value(&filter)?;
        value["written_files"] = serde_json::json!(written);
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!("🧹 mdx-rust noise filter");
    for rule in &filter.rules {
        println!(
            "   - {}: {} ({})",
            rule.pattern, rule.reason, rule.default_action
        );
    }
    if write {
        println!("   Written files:");
        for path in written {
            println!("   - {path}");
        }
    } else {
        println!("   Review mode only. Re-run with --write to create agent-pack artifacts.");
    }
    Ok(())
}

fn cmd_contracts(target: Option<&str>, max_files: usize, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let run = mdx_rust_core::scan_contracts(
        &cwd,
        &mdx_rust_core::ContractScanConfig {
            target: target.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!("📐 mdx-rust contracts");
    println!("   Target: {}", run.target.display());
    println!(
        "   Functions: {} | public: {} | missing public contracts: {}",
        run.summary.function_count,
        run.summary.public_function_count,
        run.summary.public_functions_missing_contracts
    );
    println!("   Recommendations:");
    for recommendation in run.recommendations.iter().take(12) {
        println!(
            "   - {}:{} [{}] {}",
            recommendation.file.display(),
            recommendation.line,
            recommendation.severity,
            recommendation.message
        );
    }
    Ok(())
}

fn cmd_perf(target: Option<&str>, max_files: usize, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let run = mdx_rust_core::scan_performance(
        &cwd,
        &mdx_rust_core::PerformanceScanConfig {
            target: target.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!("⚡ mdx-rust performance signals");
    println!("   Target: {}", run.target.display());
    println!(
        "   Findings: {} | high: {} | medium: {} | low: {}",
        run.summary.finding_count, run.summary.high, run.summary.medium, run.summary.low
    );
    for finding in run.findings.iter().take(12) {
        println!(
            "   - {}:{} [{}] {} - {}",
            finding.file.display(),
            finding.line,
            finding.severity,
            finding.category,
            finding.title
        );
    }
    Ok(())
}

fn cmd_benchmark(spec: &str, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let run = mdx_rust_core::run_benchmarks(
        &cwd,
        &mdx_rust_core::BenchmarkRunConfig {
            spec_path: std::path::PathBuf::from(spec),
            artifact_root: Some(artifact_root),
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!("⏱️ mdx-rust benchmark");
    println!("   Spec: {}", run.spec_path);
    println!(
        "   Status: {:?} | measured runs: {} | metric summaries: {}",
        run.status,
        run.total_measured_runs,
        run.metrics.len()
    );
    for metric in run.metrics.iter().take(12) {
        println!(
            "   - {} {} {} mean={} min={} max={} samples={}",
            metric.command_id,
            metric.name,
            metric.unit,
            metric.mean,
            metric.min,
            metric.max,
            metric.samples
        );
    }
    if let Some(path) = &run.artifact_path {
        println!("   Artifact: {path}");
    }
    Ok(())
}

fn cmd_mcp(stdio: bool, describe: bool, json: bool) -> anyhow::Result<()> {
    if describe || !stdio {
        return cmd_runtime(json);
    }

    use std::io::{BufRead, Write};

    let stdin = std::io::stdin();
    let mut stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let request: serde_json::Value = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                let response = serde_json::json!({
                    "id": serde_json::Value::Null,
                    "error": {"message": format!("invalid JSON request: {error}")}
                });
                writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                stdout.flush()?;
                continue;
            }
        };
        let response = handle_runtime_request(&request);
        writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
        stdout.flush()?;
    }
    Ok(())
}

fn cmd_serve(bind: &str, token: Option<&str>, once: bool, json: bool) -> anyhow::Result<()> {
    if !(bind.starts_with("127.0.0.1:") || bind.starts_with("localhost:")) {
        anyhow::bail!("serve is localhost-only; bind to 127.0.0.1 or localhost");
    }
    let listener = std::net::TcpListener::bind(bind)?;
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "listening",
                "bind": bind,
                "schema_version": "1.0",
                "auth_required": token.is_some(),
                "once": once
            })
        );
    } else {
        println!("mdx-rust runtime listening on http://{bind}");
        if token.is_some() {
            println!("auth: bearer token required");
        }
    }

    for stream in listener.incoming() {
        let mut stream = stream?;
        let response = handle_http_runtime_request(&mut stream, token)?;
        use std::io::Write;
        stream.write_all(response.as_bytes())?;
        if once {
            break;
        }
    }
    Ok(())
}

fn handle_runtime_request(request: &serde_json::Value) -> serde_json::Value {
    let id = request
        .get("id")
        .cloned()
        .unwrap_or(serde_json::Value::Null);
    let method = request
        .get("method")
        .and_then(|value| value.as_str())
        .unwrap_or_default();
    let result = match method {
        "initialize" | "runtime/manifest" => {
            serde_json::to_value(mdx_rust_core::agent_runtime_manifest())
                .map_err(anyhow::Error::from)
        }
        "tools/list" => serde_json::to_value(mdx_rust_core::agent_runtime_manifest().tools)
            .map_err(anyhow::Error::from),
        "tools/call" => {
            runtime_tool_call(request.get("params").unwrap_or(&serde_json::Value::Null))
        }
        other => Err(anyhow::anyhow!("unknown runtime method: {other}")),
    };

    match result {
        Ok(value) => serde_json::json!({"id": id, "result": value}),
        Err(error) => serde_json::json!({
            "id": id,
            "error": {"message": error.to_string()}
        }),
    }
}

fn runtime_tool_call(params: &serde_json::Value) -> anyhow::Result<serde_json::Value> {
    let name = params
        .get("name")
        .and_then(|value| value.as_str())
        .ok_or_else(|| anyhow::anyhow!("tools/call requires params.name"))?;
    let args = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);

    match name {
        "agent-contract" => Ok(serde_json::to_value(mdx_rust_core::agent_contract())?),
        "runtime" => Ok(serde_json::to_value(
            mdx_rust_core::agent_runtime_manifest(),
        )?),
        "recipes" => Ok(serde_json::to_value(mdx_rust_core::recipe_catalog())?),
        "repo-map" => Ok(serde_json::to_value(mdx_rust_core::build_repo_map(
            &cwd,
            &mdx_rust_core::RepoMapConfig {
                target: json_path_arg(&args, "target"),
                max_depth: json_usize_arg(&args, "max_depth").unwrap_or(3),
                max_dirs: json_usize_arg(&args, "max_dirs").unwrap_or(80),
            },
        )?)?),
        "noise-filter" => Ok(serde_json::to_value(mdx_rust_core::build_noise_filter(
            &cwd,
        ))?),
        "contracts" => Ok(serde_json::to_value(mdx_rust_core::scan_contracts(
            &cwd,
            &mdx_rust_core::ContractScanConfig {
                target: json_path_arg(&args, "target"),
                max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
            },
        )?)?),
        "perf" => Ok(serde_json::to_value(mdx_rust_core::scan_performance(
            &cwd,
            &mdx_rust_core::PerformanceScanConfig {
                target: json_path_arg(&args, "target"),
                max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
            },
        )?)?),
        "benchmark" => Ok(serde_json::to_value(mdx_rust_core::run_benchmarks(
            &cwd,
            &mdx_rust_core::BenchmarkRunConfig {
                spec_path: json_path_arg(&args, "spec")
                    .unwrap_or_else(|| std::path::PathBuf::from(".mdx-rust/benchmarks.json")),
                artifact_root: Some(artifact_root.clone()),
            },
        )?)?),
        "brief" => Ok(serde_json::to_value(mdx_rust_core::build_evolution_brief(
            &cwd,
            Some(&artifact_root),
            &mdx_rust_core::EvolutionBriefConfig {
                target: json_path_arg(&args, "target"),
                policy_path: json_path_arg(&args, "policy"),
                behavior_spec_path: json_path_arg(&args, "eval_spec"),
                max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
            },
        )?)?),
        "scorecard" => Ok(serde_json::to_value(
            mdx_rust_core::build_evolution_scorecard(
                &cwd,
                Some(&artifact_root),
                &mdx_rust_core::EvolutionScorecardConfig {
                    target: json_path_arg(&args, "target"),
                    policy_path: json_path_arg(&args, "policy"),
                    behavior_spec_path: json_path_arg(&args, "eval_spec"),
                    max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
                },
            )?,
        )?),
        "agent-ready" => {
            let scorecard = mdx_rust_core::build_evolution_scorecard(
                &cwd,
                Some(&artifact_root),
                &mdx_rust_core::EvolutionScorecardConfig {
                    target: json_path_arg(&args, "target"),
                    policy_path: json_path_arg(&args, "policy"),
                    behavior_spec_path: json_path_arg(&args, "eval_spec"),
                    max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
                },
            )?;
            Ok(serde_json::to_value(agent_ready_report_from_scorecard(
                scorecard,
            ))?)
        }
        "evidence" => Ok(serde_json::to_value(mdx_rust_core::run_evidence(
            &cwd,
            Some(&artifact_root),
            &mdx_rust_core::EvidenceRunConfig {
                target: json_path_arg(&args, "target"),
                include_coverage: json_bool_arg(&args, "include_coverage"),
                include_mutation: json_bool_arg(&args, "include_mutation"),
                include_semver: json_bool_arg(&args, "include_semver"),
                command_timeout: std::time::Duration::from_secs(
                    json_u64_arg(&args, "timeout_seconds").unwrap_or(180),
                ),
            },
        )?)?),
        "map" => Ok(serde_json::to_value(mdx_rust_core::build_codebase_map(
            &cwd,
            Some(&artifact_root),
            &mdx_rust_core::CodebaseMapConfig {
                target: json_path_arg(&args, "target"),
                policy_path: json_path_arg(&args, "policy"),
                behavior_spec_path: json_path_arg(&args, "eval_spec"),
                max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
            },
        )?)?),
        "plan" => Ok(serde_json::to_value(mdx_rust_core::build_refactor_plan(
            &cwd,
            Some(&artifact_root),
            &mdx_rust_core::RefactorPlanConfig {
                target: json_path_arg(&args, "target"),
                policy_path: json_path_arg(&args, "policy"),
                behavior_spec_path: json_path_arg(&args, "eval_spec"),
                max_files: json_usize_arg(&args, "max_files").unwrap_or(100),
            },
        )?)?),
        "explain" => {
            let artifact = json_str_arg(&args, "artifact")
                .ok_or_else(|| anyhow::anyhow!("explain requires artifact"))?;
            Ok(serde_json::to_value(mdx_rust_core::explain_artifact(
                std::path::Path::new(&artifact),
            )?)?)
        }
        "evolve" => {
            let apply = json_bool_arg(&args, "apply");
            if apply && !json_bool_arg(&args, "confirm_mutation") {
                anyhow::bail!("runtime evolve with apply=true requires confirm_mutation=true");
            }
            let budget = json_str_arg(&args, "budget")
                .map(|value| parse_budget(&value))
                .transpose()?
                .unwrap_or_else(|| std::time::Duration::from_secs(600));
            let tier = json_str_arg(&args, "tier").unwrap_or_else(|| "1".to_string());
            let min_evidence =
                json_str_arg(&args, "min_evidence").unwrap_or_else(|| "compiled".to_string());
            Ok(serde_json::to_value(mdx_rust_core::run_autopilot(
                &cwd,
                Some(&artifact_root),
                &mdx_rust_core::AutopilotConfig {
                    target: json_path_arg(&args, "target"),
                    policy_path: json_path_arg(&args, "policy"),
                    behavior_spec_path: json_path_arg(&args, "eval_spec"),
                    apply,
                    max_files: json_usize_arg(&args, "max_files").unwrap_or(250),
                    max_passes: max_passes_for_budget(budget),
                    max_candidates: json_usize_arg(&args, "max_candidates").unwrap_or(25),
                    validation_timeout: std::time::Duration::from_secs(
                        json_u64_arg(&args, "timeout_seconds").unwrap_or(180),
                    ),
                    allow_public_api_impact: json_bool_arg(&args, "allow_public_api_impact"),
                    max_tier: parse_recipe_tier(&tier)?,
                    min_evidence: parse_evidence_grade(&min_evidence)?,
                    budget: Some(budget),
                },
            )?)?)
        }
        other => anyhow::bail!("unknown runtime tool: {other}"),
    }
}

fn handle_http_runtime_request(
    stream: &mut std::net::TcpStream,
    token: Option<&str>,
) -> anyhow::Result<String> {
    use std::io::Read;

    let mut buffer = [0u8; 64 * 1024];
    let bytes = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..bytes]);
    let (head, body) = request
        .split_once("\r\n\r\n")
        .unwrap_or((request.as_ref(), ""));
    let first_line = head.lines().next().unwrap_or_default();
    if let Some(token) = token {
        if !http_request_has_bearer_token(head, token) {
            return http_json_response(
                "401 Unauthorized",
                &serde_json::json!({
                    "status": "error",
                    "error": "missing or invalid bearer token"
                }),
            );
        }
    }
    let value = if first_line.starts_with("GET /runtime ") {
        serde_json::to_value(mdx_rust_core::agent_runtime_manifest())?
    } else if first_line.starts_with("POST /tools/call ") {
        let params: serde_json::Value =
            serde_json::from_str(body.trim()).unwrap_or_else(|_| serde_json::json!({}));
        runtime_tool_call(&params).unwrap_or_else(
            |error| serde_json::json!({"status": "error", "error": error.to_string()}),
        )
    } else {
        serde_json::json!({"status": "error", "error": "unknown route"})
    };
    http_json_response("200 OK", &value)
}

fn http_request_has_bearer_token(head: &str, expected: &str) -> bool {
    head.lines().skip(1).any(|line| {
        let Some((name, value)) = line.split_once(':') else {
            return false;
        };
        name.trim().eq_ignore_ascii_case("authorization")
            && value.trim() == format!("Bearer {expected}")
    })
}

fn http_json_response(status: &str, value: &serde_json::Value) -> anyhow::Result<String> {
    let body = serde_json::to_string_pretty(value)?;
    Ok(format!(
        "HTTP/1.1 {status}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
        body.len(),
        body
    ))
}

fn json_str_arg(args: &serde_json::Value, key: &str) -> Option<String> {
    args.get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn json_path_arg(args: &serde_json::Value, key: &str) -> Option<std::path::PathBuf> {
    json_str_arg(args, key).map(std::path::PathBuf::from)
}

fn json_bool_arg(args: &serde_json::Value, key: &str) -> bool {
    args.get(key)
        .and_then(|value| value.as_bool())
        .unwrap_or(false)
}

fn json_usize_arg(args: &serde_json::Value, key: &str) -> Option<usize> {
    args.get(key)
        .and_then(|value| value.as_u64())
        .and_then(|value| usize::try_from(value).ok())
}

fn json_u64_arg(args: &serde_json::Value, key: &str) -> Option<u64> {
    args.get(key).and_then(|value| value.as_u64())
}

fn cmd_recipes(json: bool) -> anyhow::Result<()> {
    let catalog = mdx_rust_core::recipe_catalog();
    if json {
        println!("{}", serde_json::to_string_pretty(&catalog)?);
        return Ok(());
    }

    println!("🧪 mdx-rust recipes");
    println!("   Schema: {}", catalog.schema_version);
    for recipe in &catalog.recipes {
        println!(
            "   - {} [{:?}] requires {:?} ({})",
            recipe.id,
            recipe.tier,
            recipe.required_evidence,
            if recipe.executable {
                "executable"
            } else {
                "plan-only"
            }
        );
        println!("     {}", recipe.description);
    }
    Ok(())
}

fn cmd_explain(artifact: &str, json: bool) -> anyhow::Result<()> {
    let explanation = mdx_rust_core::explain_artifact(std::path::Path::new(artifact))?;
    if json {
        println!("{}", serde_json::to_string_pretty(&explanation)?);
        return Ok(());
    }

    println!("🧾 mdx-rust explain");
    println!("   Artifact: {}", explanation.artifact_path);
    println!("   Kind: {:?}", explanation.artifact_kind);
    println!("   Summary: {}", explanation.summary);
    println!("   Next actions:");
    for action in &explanation.recommended_next_actions {
        println!("   - {}", action);
    }
    Ok(())
}

fn cmd_scorecard(
    target: Option<&str>,
    policy: Option<&str>,
    eval_spec: Option<&str>,
    max_files: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let scorecard = mdx_rust_core::build_evolution_scorecard(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::EvolutionScorecardConfig {
            target: target.map(std::path::PathBuf::from),
            policy_path: policy.map(std::path::PathBuf::from),
            behavior_spec_path: eval_spec.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&scorecard)?);
        return Ok(());
    }

    println!("📊 mdx-rust scorecard");
    println!("   Scorecard: {}", scorecard.scorecard_id);
    println!("   Root: {}", scorecard.root);
    if let Some(target) = &scorecard.target {
        println!("   Target: {}", target);
    }
    println!(
        "   Readiness: {:?} ({} executable, {} review-only, {} blocked)",
        scorecard.readiness.grade,
        scorecard.readiness.executable_candidates,
        scorecard.readiness.review_only_candidates,
        scorecard.readiness.blocked_candidates
    );
    println!(
        "   Quality: {:?} | debt={} | security={}",
        scorecard.map.quality.grade, scorecard.map.quality.debt_score, scorecard.map.security.score
    );
    println!("   Next commands:");
    for command in &scorecard.next_commands {
        println!("   - {}", command);
    }
    if let Some(path) = &scorecard.artifact_path {
        println!("   Artifact: {}", path);
    }
    Ok(())
}

fn cmd_brief(
    target: Option<&str>,
    policy: Option<&str>,
    eval_spec: Option<&str>,
    max_files: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let brief = mdx_rust_core::build_evolution_brief(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::EvolutionBriefConfig {
            target: target.map(std::path::PathBuf::from),
            policy_path: policy.map(std::path::PathBuf::from),
            behavior_spec_path: eval_spec.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&brief)?);
        return Ok(());
    }

    println!("🧠 mdx-rust evolution brief");
    println!("   Brief: {}", brief.brief_id);
    if let Some(target) = &brief.target {
        println!("   Target: {target}");
    }
    println!(
        "   Contract posture: {:?} | missing public contracts: {}",
        brief.contracts.grade, brief.contracts.public_functions_missing_contracts
    );
    println!(
        "   Performance score: {} | findings: {}",
        brief.performance.score, brief.performance.finding_count
    );
    println!("   Recommended sequence:");
    for command in brief.recommended_sequence.iter().take(10) {
        println!("   - {command}");
    }
    if let Some(path) = &brief.artifact_path {
        println!("   Artifact: {path}");
    }
    Ok(())
}

fn cmd_agent_ready(
    target: Option<&str>,
    policy: Option<&str>,
    eval_spec: Option<&str>,
    max_files: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let scorecard = mdx_rust_core::build_evolution_scorecard(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::EvolutionScorecardConfig {
            target: target.map(std::path::PathBuf::from),
            policy_path: policy.map(std::path::PathBuf::from),
            behavior_spec_path: eval_spec.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    let report = agent_ready_report_from_scorecard(scorecard);

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
        return Ok(());
    }

    println!("🤖 mdx-rust agent-ready");
    println!("   Status: {:?}", report.status);
    println!("   Readiness: {:?}", report.readiness.grade);
    println!("   Evidence: {:?}", report.evidence.grade);
    println!(
        "   Candidates: {} executable, {} review-only, {} blocked",
        report.readiness.executable_candidates,
        report.readiness.review_only_candidates,
        report.readiness.blocked_candidates
    );
    println!("   Next commands:");
    for command in &report.next_commands {
        println!("   - {command}");
    }
    Ok(())
}

fn agent_ready_report_from_scorecard(
    scorecard: mdx_rust_core::EvolutionScorecard,
) -> mdx_rust_core::AgentReadyReport {
    let ready_for_apply = scorecard.readiness.executable_candidates > 0
        && matches!(
            scorecard.readiness.grade,
            mdx_rust_core::AutonomyReadinessGrade::Tier1Ready
                | mdx_rust_core::AutonomyReadinessGrade::Tier2Ready
                | mdx_rust_core::AutonomyReadinessGrade::Tier3Planning
        );
    mdx_rust_core::AgentReadyReport {
        schema_version: "1.0".to_string(),
        product_version: env!("CARGO_PKG_VERSION").to_string(),
        status: if ready_for_apply {
            mdx_rust_core::AgentReadyStatus::Ready
        } else {
            mdx_rust_core::AgentReadyStatus::Review
        },
        target: scorecard.target.clone(),
        readiness: scorecard.readiness.clone(),
        evidence: scorecard.map.evidence.clone(),
        quality: scorecard.map.quality.clone(),
        security: scorecard.map.security.clone(),
        contracts: scorecard.map.contracts.clone(),
        performance: scorecard.map.performance.clone(),
        agent_contract: mdx_rust_core::AgentReadyContractRefs {
            discovery: "mdx-rust --json agent-contract".to_string(),
            runtime: "mdx-rust --json runtime".to_string(),
            scorecard_artifact: scorecard.artifact_path.clone(),
        },
        next_commands: scorecard.next_commands.clone(),
    }
}

fn cmd_evidence(args: EvidenceCommand<'_>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let run = mdx_rust_core::run_evidence(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::EvidenceRunConfig {
            target: args.target.map(std::path::PathBuf::from),
            include_coverage: args.include_coverage,
            include_mutation: args.include_mutation,
            include_semver: args.include_semver,
            command_timeout: std::time::Duration::from_secs(args.timeout_seconds),
        },
    )?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!("📏 mdx-rust evidence");
    println!("   Root: {}", run.root);
    if let Some(target) = &run.target {
        println!("   Target: {}", target);
    }
    println!("   Grade: {:?}", run.grade);
    println!("   Analysis depth: {:?}", run.analysis_depth);
    println!("   Profiled files: {}", run.file_profiles.len());
    println!("   Note: {}", run.note);
    for command in &run.commands {
        let status = if command.skipped {
            "skipped"
        } else if command.success {
            "passed"
        } else if command.timed_out {
            "timed out"
        } else {
            "failed"
        };
        println!("   - {}: {}", command.id, status);
        if let Some(reason) = &command.skip_reason {
            println!("     {}", reason);
        }
    }
    if let Some(path) = &run.artifact_path {
        println!("   Evidence artifact: {}", path);
    }

    Ok(())
}

/// `doctor` command shows project state using the loaded Config
fn cmd_doctor(name: Option<&str>, json: bool) -> anyhow::Result<()> {
    if let Some(name) = name {
        return cmd_agent_doctor(name, json);
    }

    cmd_workspace_doctor(json)
}

fn cmd_workspace_doctor(json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let run = mdx_rust_core::run_hardening(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::HardeningConfig {
            apply: false,
            max_files: 100,
            validation_timeout: std::time::Duration::from_secs(180),
            ..mdx_rust_core::HardeningConfig::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!("🔍 mdx-rust doctor — workspace");
    println!("   Root: {}", run.root);
    println!("   Artifact directory: {}", artifact_root.display());
    println!(
        "   Cargo metadata: {} package(s) ({})",
        run.workspace.package_count,
        if run.workspace.cargo_metadata_available {
            "available"
        } else {
            "unavailable"
        }
    );
    println!("   Scanned Rust files: {}", run.files_scanned);
    println!("   Findings: {}", run.findings.len());
    println!(
        "   Risk: high={}, medium={}, patchable={}",
        run.risk_summary.high, run.risk_summary.medium, run.risk_summary.patchable
    );
    println!("   Proposed hardening changes: {}", run.changes.len());
    println!("   Outcome: {:?}", run.outcome.status);
    for recommendation in &run.risk_summary.top_recommendations {
        println!("   → {}", recommendation);
    }
    if !run.changes.is_empty() {
        println!();
        println!("Suggested next step:");
        println!("  mdx-rust improve --target <file-or-dir>");
        println!("  mdx-rust improve --target <file-or-dir> --apply");
    }
    Ok(())
}

fn cmd_agent_doctor(name: &str, json: bool) -> anyhow::Result<()> {
    use mdx_rust_core::registry::Registry;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root).unwrap_or_default();

    let _agent_dir = artifact_root.join("agents").join(name);

    if json {
        if let Some(agent) = registry.get(name) {
            println!(
                "{}",
                serde_json::json!({
                    "agent": name,
                    "registered": true,
                    "path": agent.path.display().to_string()
                })
            );
        } else {
            println!("{}", serde_json::json!({"agent":name,"registered":false}));
        }
        return Ok(());
    }

    println!("🔍 mdx-rust doctor — agent '{}'", name);
    println!("   Artifact directory: {}", artifact_root.display());
    println!();

    if !artifact_root.exists() {
        println!(
            "  ❌ No {} directory. Run `mdx-rust init` first.",
            config.artifact_dir
        );
        return Ok(());
    }

    println!("  ✅ {} exists", config.artifact_dir);

    if let Some(agent) = registry.get(name) {
        println!("  ✅ Agent is registered");
        println!("     Path: {}", agent.path.display());

        // Real bundle scope using the analysis crate
        match mdx_rust_analysis::build_bundle_scope(&agent.path, None) {
            Ok(scope) => {
                println!(
                    "     Would send ~{} files for LLM analysis",
                    scope.optimizable_paths.len()
                );
            }
            Err(_) => {
                println!("     (Could not compute bundle scope yet)");
            }
        }
    } else {
        println!("  ℹ️  Agent is not registered yet");
        println!("     → mdx-rust register {}", name);
    }

    println!();

    // Show best version if present
    let best_dir = artifact_root.join("agents").join(name).join("best");
    if best_dir.exists() && best_dir.join("src").exists() {
        println!("  ✓ Best improved version available in best/ (from last accepted optimization)");
    }

    // Show recent experiments if they exist
    let exps = artifact_root.join("agents").join(name).join("experiments");
    if exps.exists() {
        if let Ok(entries) = std::fs::read_dir(&exps) {
            let mut reports: Vec<_> = entries
                .filter_map(|e| e.ok())
                .filter(|e| e.file_name().to_string_lossy().starts_with("report-"))
                .collect();

            reports.sort_by_key(|e| std::fs::metadata(e.path()).and_then(|m| m.modified()).ok());

            if !reports.is_empty() {
                println!("Recent optimization runs:");
                for r in reports.iter().rev().take(3) {
                    println!("  • {}", r.file_name().to_string_lossy());
                }
            }
        }
    }

    Ok(())
}

/// First real implementation of `register`
fn cmd_register(name: &str, path: Option<&str>, json: bool) -> anyhow::Result<()> {
    use mdx_rust_analysis::editing::run_command_with_timeout;
    use mdx_rust_core::registry::{RegisteredAgent, Registry};
    use std::path::Path;
    use std::process::Command;
    use std::time::Duration;

    let cwd = std::env::current_dir()?;
    let target_path = path
        .map(Path::new)
        .unwrap_or_else(|| Path::new("."))
        .to_path_buf();

    let cargo_toml = target_path.join("Cargo.toml");
    if !cargo_toml.exists() {
        anyhow::bail!(
            "No Cargo.toml found at {}. Is this a Rust crate?",
            target_path.display()
        );
    }

    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);

    // Load or create registry
    let mut registry = Registry::load_from(&artifact_root)?;

    // Basic contract detection (improve later with real analysis)
    let contract = detect_contract(&target_path);
    let contract_label = format!("{contract:?}");

    let absolute_path = target_path.canonicalize().unwrap_or(target_path.clone());

    let agent = RegisteredAgent {
        name: name.to_string(),
        path: absolute_path.clone(),
        contract,
        registered_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs().to_string())
            .unwrap_or_else(|_| "0".to_string()),
    };

    registry.register(agent);
    registry.save_to(&artifact_root)?;

    // Create per-agent directory for future artifacts
    let agent_dir = artifact_root.join("agents").join(name);
    std::fs::create_dir_all(&agent_dir)?;

    // Smoke test
    let mut check = Command::new("cargo");
    check.arg("check").current_dir(&target_path);
    let smoke = run_command_with_timeout(&mut check, Duration::from_secs(120));
    let smoke_passed = smoke.as_ref().is_some_and(|output| output.success());
    let smoke_timed_out = smoke.as_ref().is_some_and(|output| output.timed_out);

    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "registered",
                "name": name,
                "path": target_path.display().to_string(),
                "contract": contract_label,
                "smoke_test_passed": smoke_passed,
                "smoke_test_timed_out": smoke_timed_out
            })
        );
    } else {
        println!("✅ Registered agent '{}'", name);
        println!("   Path: {}", target_path.display());
        println!("   Contract detected: {contract_label}");
        println!(
            "   Smoke test (cargo check): {}",
            if smoke_passed {
                "PASSED"
            } else if smoke_timed_out {
                "TIMED OUT"
            } else {
                "FAILED or skipped"
            }
        );
        println!();
        println!("Next: mdx-rust doctor {}", name);
    }

    Ok(())
}

fn cmd_invoke(name: &str, input: Option<&str>, json: bool) -> anyhow::Result<()> {
    use mdx_rust_core::registry::Registry;
    use mdx_rust_core::runner::run_agent;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root)?;

    let agent = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not registered", name))?;

    let input_value: serde_json::Value = if let Some(s) = input {
        serde_json::from_str(s)?
    } else {
        serde_json::json!({"query": "hello from mdx-rust", "context": null})
    };

    let rt = tokio::runtime::Runtime::new()?;
    let result = rt.block_on(run_agent(agent, input_value))?;

    if json {
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{:#?}", result);
    }

    Ok(())
}

fn cmd_eval(
    name: Option<&str>,
    dataset: Option<&str>,
    spec: Option<&str>,
    json: bool,
) -> anyhow::Result<()> {
    let Some(name) = name else {
        return cmd_workspace_eval(spec, json);
    };
    use mdx_rust_core::registry::Registry;
    use mdx_rust_core::EvaluationDataset;
    use std::path::Path;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root)?;

    let agent = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not registered", name))?;

    let evaluation_dataset = if let Some(dataset_path) = dataset {
        EvaluationDataset::load_from_path(Path::new(dataset_path))?
    } else {
        EvaluationDataset::synthetic_v1()
    };
    let dataset_hash = evaluation_dataset.content_hash();
    let sample_count = evaluation_dataset.samples.len();

    if json {
        println!(
            "{}",
            serde_json::json!({
                "agent": name,
                "path": agent.path.display().to_string(),
                "dataset": dataset,
                "dataset_version": evaluation_dataset.version,
                "dataset_hash": dataset_hash,
                "sample_count": sample_count,
                "status": "loaded"
            })
        );
    } else {
        println!("Evaluating agent '{}' with dataset {:?}", name, dataset);
        println!(
            "Loaded {} sample(s), version {}, hash {}.",
            sample_count, evaluation_dataset.version, dataset_hash
        );
        println!("Scored evaluation execution is not implemented yet.");
    }

    Ok(())
}

fn cmd_workspace_eval(spec: Option<&str>, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let spec_path = spec
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::path::PathBuf::from(".mdx-rust/evals.json"));
    let report = mdx_rust_core::run_behavior_evals(&cwd, &spec_path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Behavior evals for workspace: {}", cwd.display());
        println!("  Passed: {}/{} command(s)", report.passed, report.total);
        for record in &report.command_records {
            println!(
                "  [{}] {} - {}",
                if record.success { "pass" } else { "fail" },
                record.id,
                record.command
            );
            if let Some(reason) = &record.failure_reason {
                println!("       {}", reason);
            }
        }
    }

    Ok(())
}

fn cmd_optimize(
    name: &str,
    iterations: u32,
    budget: &str,
    review: bool,
    json: bool,
) -> anyhow::Result<()> {
    use mdx_rust_core::optimizer::{run_optimization, OptimizeConfig};
    use mdx_rust_core::registry::Registry;
    use mdx_rust_core::{HookPolicy, OptimizationBudget};

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root)?;

    let agent = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not registered", name))?;
    let budget = OptimizationBudget::from_label(budget)?;

    let rt = tokio::runtime::Runtime::new()?;

    let runs = if json {
        // For pure machine-readable output, use a completely silent subscriber during the optimization run.
        // This guarantees zero human or INFO logs leak into the JSON stream — critical for automation / enterprise use.
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::ERROR)
            .finish();
        tracing::subscriber::with_default(subscriber, || {
            rt.block_on(run_optimization(
                agent,
                &OptimizeConfig {
                    max_iterations: iterations,
                    candidates_per_iteration: 2,
                    use_llm_judge: false,
                    budget,
                    hook_policy: HookPolicy::default(),
                    review_before_apply: review,
                    quiet: true,
                    candidate_timeout: std::time::Duration::from_secs(300),
                },
            ))
        })?
    } else {
        rt.block_on(run_optimization(
            agent,
            &OptimizeConfig {
                max_iterations: iterations,
                candidates_per_iteration: 2,
                use_llm_judge: false,
                budget,
                hook_policy: HookPolicy::default(),
                review_before_apply: review,
                quiet: false,
                candidate_timeout: std::time::Duration::from_secs(300),
            },
        ))?
    };

    // Landing happens inside the optimizer's safety pipeline.
    // We keep best/ persistence here for now, but only for runs that truly accepted.

    // Persist "best" version if any improvement was accepted (per original plan)
    if runs.iter().any(|r| r.accepted_changes > 0) {
        let best_dir = artifact_root.join("agents").join(name).join("best");
        let _ = std::fs::create_dir_all(&best_dir);

        // Copy the key source files from the registered agent (simple but effective)
        let src_dir = agent.path.join("src");
        if src_dir.exists() {
            let _ = copy_dir_recursive(&src_dir, &best_dir.join("src"));
        }
        // Also copy Cargo.toml for context
        if let Ok(cargo) = std::fs::read_to_string(agent.path.join("Cargo.toml")) {
            let _ = std::fs::write(best_dir.join("Cargo.toml"), cargo);
        }

        if !json {
            println!(
                "   ✓ Best improved version saved to .mdx-rust/agents/{}/best/",
                name
            );
        }
    }

    if json {
        println!("{}", serde_json::to_string_pretty(&runs)?);
    } else {
        println!("🚀 Optimization run for agent '{}'", name);
        println!("   ({} iterations)", runs.len());
        for run in &runs {
            let avg = if run.scores.is_empty() {
                0.0
            } else {
                run.scores.iter().sum::<f32>() / run.scores.len() as f32
            };
            println!(
                "   • Iteration {} | Avg: {:.2} | Validated: {} | Landed: {} | Accepted: {}",
                run.iteration, avg, run.validated_changes, run.landed_changes, run.accepted_changes
            );
            if !run.notes.is_empty() {
                println!("     → {}", run.notes);
            }
            if let Some(packet) = &run.audit_packet {
                println!(
                    "     Audit packet schema: {} | scope: {}",
                    packet.schema_version, packet.edit_scope_contract
                );
            }
            for (i, c) in run.candidates.iter().enumerate() {
                println!(
                    "       [Candidate {}] {} — {}",
                    i + 1,
                    c.focus,
                    c.description
                );
            }
        }
        println!(
            "\nArtifacts written under .mdx-rust/agents/{}/experiments/",
            name
        );
        println!("Use `mdx-rust doctor {}` to inspect scope and state.", name);
    }

    Ok(())
}

struct ImproveCommand<'a> {
    target: Option<&'a str>,
    goal: Option<&'a str>,
    policy: Option<&'a str>,
    eval_spec: Option<&'a str>,
    apply: bool,
    max_files: usize,
    tier: &'a str,
    timeout_seconds: u64,
    json: bool,
}

fn cmd_improve(args: ImproveCommand<'_>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let run = mdx_rust_core::run_hardening(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::HardeningConfig {
            target: args.target.map(std::path::PathBuf::from),
            policy_path: args.policy.map(std::path::PathBuf::from),
            behavior_spec_path: args.eval_spec.map(std::path::PathBuf::from),
            apply: args.apply,
            max_files: args.max_files,
            max_recipe_tier: parse_recipe_tier_number(args.tier)?,
            evidence_depth: evidence_depth_for_tier(parse_recipe_tier_number(args.tier)?),
            validation_timeout: std::time::Duration::from_secs(args.timeout_seconds),
        },
    )?;

    if args.json {
        let mut value = serde_json::to_value(&run)?;
        if let Some(goal) = args.goal {
            value["goal"] = serde_json::Value::String(goal.to_string());
        }
        println!("{}", serde_json::to_string_pretty(&value)?);
        return Ok(());
    }

    println!(
        "🛠️  mdx-rust improve — {}",
        if args.apply { "apply" } else { "review" }
    );
    if let Some(target) = args.target {
        println!("   Target: {}", target);
    }
    if let Some(goal) = args.goal {
        println!("   Goal: {}", goal);
    }
    if let Some(eval_spec) = args.eval_spec {
        println!("   Behavior eval spec: {}", eval_spec);
    }
    println!("   Findings: {}", run.findings.len());
    println!("   Proposed changes: {}", run.changes.len());
    println!("   Outcome: {:?}", run.outcome.status);
    println!("   Note: {}", run.outcome.note);
    if let Some(path) = &run.artifact_path {
        println!("   Report: {}", path);
    }
    for change in &run.changes {
        println!("   • {} — {}", change.file, change.description);
    }
    if !args.apply && !run.changes.is_empty() {
        println!();
        println!("Validated in isolation. Re-run with --apply to land the transaction.");
    }

    Ok(())
}

fn cmd_plan(
    target: Option<&str>,
    policy: Option<&str>,
    eval_spec: Option<&str>,
    max_files: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let plan = mdx_rust_core::build_refactor_plan(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::RefactorPlanConfig {
            target: target.map(std::path::PathBuf::from),
            policy_path: policy.map(std::path::PathBuf::from),
            behavior_spec_path: eval_spec.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&plan)?);
        return Ok(());
    }

    println!("🧭 mdx-rust plan — refactor review");
    println!("   Plan: {}", plan.plan_id);
    println!("   Root: {}", plan.root);
    if let Some(target) = &plan.target {
        println!("   Target: {}", target);
    }
    println!("   Files scanned: {}", plan.impact.files_scanned);
    println!(
        "   Impact: risk={:?}, public_items={}, public_files={}, patchable={}",
        plan.impact.risk_level,
        plan.impact.public_item_count,
        plan.impact.public_files,
        plan.impact.patchable_hardening_changes
    );
    println!(
        "   Security: score={}, high={}, medium={}",
        plan.security.score, plan.security.high, plan.security.medium
    );
    println!("   Candidates: {}", plan.candidates.len());
    if let Some(path) = &plan.artifact_path {
        println!("   Artifact: {}", path);
    }
    for candidate in plan.candidates.iter().take(8) {
        println!(
            "   • [{:?}] {} ({})",
            candidate.status, candidate.title, candidate.file
        );
        if let Some(command) = &candidate.apply_command {
            println!("     apply: {}", command);
        }
    }
    if plan.candidates.len() > 8 {
        println!(
            "   … {} more candidate(s) in the plan artifact",
            plan.candidates.len() - 8
        );
    }
    println!();
    println!("Required gates:");
    for gate in &plan.required_gates {
        println!("   - {}", gate);
    }

    Ok(())
}

fn cmd_map(
    target: Option<&str>,
    policy: Option<&str>,
    eval_spec: Option<&str>,
    max_files: usize,
    json: bool,
) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let map = mdx_rust_core::build_codebase_map(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::CodebaseMapConfig {
            target: target.map(std::path::PathBuf::from),
            policy_path: policy.map(std::path::PathBuf::from),
            behavior_spec_path: eval_spec.map(std::path::PathBuf::from),
            max_files,
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&map)?);
        return Ok(());
    }

    println!("🗺️  mdx-rust map");
    println!("   Map: {}", map.map_id);
    println!("   Root: {}", map.root);
    if let Some(target) = &map.target {
        println!("   Target: {}", target);
    }
    println!(
        "   Quality: {:?} (debt score {}, security score {})",
        map.quality.grade, map.quality.debt_score, map.quality.security_score
    );
    println!(
        "   Evidence: {:?} (max autonomous tier {})",
        map.evidence.grade, map.evidence.max_autonomous_tier
    );
    println!(
        "   Files: {}, patchable: {}, review-only: {}, public items: {}",
        map.impact.files_scanned,
        map.quality.patchable_findings,
        map.quality.review_only_findings,
        map.quality.public_api_pressure
    );
    println!("   Capability gates:");
    for gate in &map.capability_gates {
        println!(
            "   - {}: {}",
            gate.label,
            if gate.available {
                "available"
            } else {
                "missing"
            }
        );
    }
    println!("   Recommended actions:");
    for action in &map.recommended_actions {
        println!("   - {}", action);
    }
    if let Some(path) = &map.artifact_path {
        println!("   Artifact: {}", path);
    }

    Ok(())
}

struct AutopilotCommand<'a> {
    target: Option<&'a str>,
    policy: Option<&'a str>,
    eval_spec: Option<&'a str>,
    apply: bool,
    allow_public_api_impact: bool,
    max_files: usize,
    max_passes: usize,
    max_candidates: usize,
    tier: &'a str,
    min_evidence: &'a str,
    budget: Option<&'a str>,
    timeout_seconds: u64,
    json: bool,
}

fn cmd_autopilot(args: AutopilotCommand<'_>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let max_tier = parse_recipe_tier(args.tier)?;
    let min_evidence = parse_evidence_grade(args.min_evidence)?;
    let budget = args.budget.map(parse_budget).transpose()?;
    let run = mdx_rust_core::run_autopilot(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::AutopilotConfig {
            target: args.target.map(std::path::PathBuf::from),
            policy_path: args.policy.map(std::path::PathBuf::from),
            behavior_spec_path: args.eval_spec.map(std::path::PathBuf::from),
            apply: args.apply,
            max_files: args.max_files,
            max_passes: args.max_passes,
            max_candidates: args.max_candidates,
            validation_timeout: std::time::Duration::from_secs(args.timeout_seconds),
            allow_public_api_impact: args.allow_public_api_impact,
            max_tier,
            min_evidence,
            budget,
        },
    )?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!(
        "🚀 mdx-rust autopilot - {}",
        if args.apply { "apply" } else { "review" }
    );
    if let Some(target) = args.target {
        println!("   Target: {}", target);
    }
    println!("   Status: {:?}", run.status);
    println!(
        "   Quality before: {:?} (debt score {})",
        run.quality_before.grade, run.quality_before.debt_score
    );
    println!(
        "   Evidence: {:?} / {:?} (max tier {})",
        run.evidence.grade, run.evidence.analysis_depth, run.evidence.max_autonomous_tier
    );
    if let Some(after) = &run.quality_after {
        println!(
            "   Quality after: {:?} (debt score {})",
            after.grade, after.debt_score
        );
    }
    println!("   Passes: {}", run.passes.len());
    println!("   Budget seconds: {:?}", run.budget_seconds);
    println!("   Planned candidates: {}", run.total_planned_candidates);
    println!("   Executed candidates: {}", run.total_executed_candidates);
    println!("   Skipped candidates: {}", run.total_skipped_candidates);
    println!(
        "   Validated/applied transactions: {}/{}",
        run.execution_summary.validated_transactions, run.execution_summary.applied_transactions
    );
    println!("   Note: {}", run.note);
    for pass in &run.passes {
        println!(
            "   - pass {}: {:?}, executable {}",
            pass.pass_index, pass.status, pass.executable_candidates
        );
    }
    if let Some(path) = &run.artifact_path {
        println!("   Report: {}", path);
    }

    Ok(())
}

fn parse_recipe_tier(value: &str) -> anyhow::Result<mdx_rust_core::RecipeTier> {
    match value.to_ascii_lowercase().as_str() {
        "1" | "tier1" => Ok(mdx_rust_core::RecipeTier::Tier1),
        "2" | "tier2" => Ok(mdx_rust_core::RecipeTier::Tier2),
        "3" | "tier3" => Ok(mdx_rust_core::RecipeTier::Tier3),
        other => anyhow::bail!("unknown recipe tier: {other}"),
    }
}

fn parse_recipe_tier_number(value: &str) -> anyhow::Result<u8> {
    Ok(match parse_recipe_tier(value)? {
        mdx_rust_core::RecipeTier::Tier1 => 1,
        mdx_rust_core::RecipeTier::Tier2 => 2,
        mdx_rust_core::RecipeTier::Tier3 => 3,
    })
}

fn evidence_depth_for_tier(tier: u8) -> mdx_rust_core::HardeningEvidenceDepth {
    match tier {
        0 | 1 => mdx_rust_core::HardeningEvidenceDepth::Basic,
        2 => mdx_rust_core::HardeningEvidenceDepth::Covered,
        _ => mdx_rust_core::HardeningEvidenceDepth::Hardened,
    }
}

fn parse_evidence_grade(value: &str) -> anyhow::Result<mdx_rust_core::EvidenceGrade> {
    match value.to_ascii_lowercase().as_str() {
        "none" => Ok(mdx_rust_core::EvidenceGrade::None),
        "compiled" => Ok(mdx_rust_core::EvidenceGrade::Compiled),
        "tested" => Ok(mdx_rust_core::EvidenceGrade::Tested),
        "covered" => Ok(mdx_rust_core::EvidenceGrade::Covered),
        "hardened" => Ok(mdx_rust_core::EvidenceGrade::Hardened),
        "proven" => Ok(mdx_rust_core::EvidenceGrade::Proven),
        other => anyhow::bail!("unknown evidence grade: {other}"),
    }
}

fn parse_budget(value: &str) -> anyhow::Result<std::time::Duration> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        anyhow::bail!("budget cannot be empty");
    }
    let (number, multiplier) = if let Some(number) = trimmed.strip_suffix('m') {
        (number, 60)
    } else if let Some(number) = trimmed.strip_suffix("min") {
        (number, 60)
    } else if let Some(number) = trimmed.strip_suffix('s') {
        (number, 1)
    } else {
        (trimmed, 60)
    };
    let amount: u64 = number
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid budget value: {value}"))?;
    if amount == 0 {
        anyhow::bail!("budget must be greater than zero");
    }
    Ok(std::time::Duration::from_secs(
        amount.saturating_mul(multiplier),
    ))
}

fn max_passes_for_budget(budget: std::time::Duration) -> usize {
    let minutes = budget.as_secs().div_ceil(60);
    minutes.clamp(1, 6) as usize
}

struct ApplyPlanCommand<'a> {
    plan_path: &'a str,
    candidate_id: Option<&'a str>,
    all: bool,
    apply: bool,
    allow_public_api_impact: bool,
    timeout_seconds: u64,
    max_candidates: usize,
    json: bool,
}

fn cmd_apply_plan(args: ApplyPlanCommand<'_>) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    if args.all {
        let run = mdx_rust_core::apply_refactor_plan_batch(
            &cwd,
            Some(&artifact_root),
            &mdx_rust_core::RefactorBatchApplyConfig {
                plan_path: std::path::PathBuf::from(args.plan_path),
                apply: args.apply,
                allow_public_api_impact: args.allow_public_api_impact,
                validation_timeout: std::time::Duration::from_secs(args.timeout_seconds),
                max_candidates: args.max_candidates,
                max_tier: mdx_rust_core::RecipeTier::Tier1,
                min_evidence: mdx_rust_core::EvidenceGrade::Compiled,
            },
        )?;

        if args.json {
            println!("{}", serde_json::to_string_pretty(&run)?);
            return Ok(());
        }

        println!(
            "🧰 mdx-rust apply-plan --all — {}",
            if args.apply { "apply" } else { "review" }
        );
        println!("   Plan: {}", run.plan_id);
        println!("   Status: {:?}", run.status);
        println!("   Requested candidates: {}", run.requested_candidates);
        println!("   Executed candidates: {}", run.executed_candidates);
        println!("   Skipped candidates: {}", run.skipped_candidates);
        println!("   Note: {}", run.note);
        for step in &run.steps {
            println!("   - {}: {:?}", step.candidate_id, step.status);
            if !step.note.is_empty() {
                println!("     {}", step.note);
            }
        }
        if let Some(path) = &run.artifact_path {
            println!("   Report: {}", path);
        }
        return Ok(());
    }

    let Some(candidate_id) = args.candidate_id else {
        anyhow::bail!("pass --candidate <id> or --all");
    };

    let run = mdx_rust_core::apply_refactor_plan_candidate(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::RefactorApplyConfig {
            plan_path: std::path::PathBuf::from(args.plan_path),
            candidate_id: candidate_id.to_string(),
            apply: args.apply,
            allow_public_api_impact: args.allow_public_api_impact,
            validation_timeout: std::time::Duration::from_secs(args.timeout_seconds),
        },
    )?;

    if args.json {
        println!("{}", serde_json::to_string_pretty(&run)?);
        return Ok(());
    }

    println!(
        "🧰 mdx-rust apply-plan — {}",
        if args.apply { "apply" } else { "review" }
    );
    println!("   Plan: {}", run.plan_id);
    println!("   Candidate: {}", run.candidate_id);
    println!("   Status: {:?}", run.status);
    println!("   Note: {}", run.note);
    if !run.stale_files.is_empty() {
        println!("   Stale files:");
        for stale in &run.stale_files {
            println!(
                "   - {} expected {} but found {}",
                stale.file, stale.expected_hash, stale.actual_hash
            );
        }
    }
    if let Some(hardening) = &run.hardening_run {
        println!("   Hardening status: {:?}", hardening.outcome.status);
        println!("   Proposed changes: {}", hardening.changes.len());
        println!("   Applied: {}", hardening.outcome.applied);
    }
    if let Some(path) = &run.artifact_path {
        println!("   Report: {}", path);
    }

    Ok(())
}

fn cmd_audit(name: Option<&str>, policy: Option<&str>, json: bool) -> anyhow::Result<()> {
    let Some(name) = name else {
        return cmd_workspace_audit(policy, json);
    };

    use mdx_rust_core::registry::Registry;
    use mdx_rust_core::security::audit_agent;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root)?;

    let agent = registry
        .get(name)
        .ok_or_else(|| anyhow::anyhow!("Agent '{}' not registered", name))?;

    let report = audit_agent(&agent.path)?;

    if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        println!("Security audit for '{}': {}", name, report.summary());
        for finding in &report.findings {
            let location = finding
                .file
                .as_ref()
                .map(|file| {
                    finding
                        .line
                        .map(|line| format!("{file}:{line}"))
                        .unwrap_or_else(|| file.clone())
                })
                .unwrap_or_else(|| "workspace".to_string());
            println!(
                "  [{:?}] {} - {} ({})",
                finding.severity, finding.id, finding.title, location
            );
        }
    }

    Ok(())
}

fn cmd_workspace_audit(policy: Option<&str>, json: bool) -> anyhow::Result<()> {
    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = cwd.join(&config.artifact_dir);
    let run = mdx_rust_core::run_hardening(
        &cwd,
        Some(&artifact_root),
        &mdx_rust_core::HardeningConfig {
            policy_path: policy.map(std::path::PathBuf::from),
            apply: false,
            max_files: 100,
            validation_timeout: std::time::Duration::from_secs(180),
            ..mdx_rust_core::HardeningConfig::default()
        },
    )?;

    if json {
        println!("{}", serde_json::to_string_pretty(&run)?);
    } else {
        println!("Hardening audit for workspace: {}", run.root);
        println!("  Findings: {}", run.findings.len());
        println!("  Patchable changes: {}", run.changes.len());
        for finding in &run.findings {
            println!(
                "  [{}] {} - {} ({}:{})",
                if finding.patchable {
                    "patchable"
                } else {
                    "review"
                },
                finding.id,
                finding.title,
                finding.file.display(),
                finding.line
            );
        }
    }

    Ok(())
}

fn cmd_schema(kind: &str, json: bool) -> anyhow::Result<()> {
    let schema = match kind {
        "agent-contract" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::MdxAgentContract))?
        }
        "agent-runtime-manifest" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::AgentRuntimeManifest))?
        }
        "agent-pack" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::AgentPack))?,
        "repo-map" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::RepoMap))?,
        "noise-filter" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::NoiseFilter))?,
        "contract-run" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::ContractRun))?,
        "performance-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::PerformanceRun))?
        }
        "benchmark-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::BenchmarkRun))?
        }
        "benchmark-spec" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::BenchmarkSpec))?
        }
        "evolution-brief" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::EvolutionBrief))?
        }
        "agent-ready-report" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::AgentReadyReport))?
        }
        "artifact-explanation" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::ArtifactExplanation))?
        }
        "audit-packet" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::AuditPacket))?,
        "candidate" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::Candidate))?,
        "optimization-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::OptimizationRun))?
        }
        "hook-decision" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::HookDecision))?
        }
        "trace-event" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::TraceEvent))?,
        "hardening-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::HardeningRun))?
        }
        "hardening-finding" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_analysis::HardeningFinding))?
        }
        "behavior-eval-report" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::BehaviorEvalReport))?
        }
        "project-policy" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::ProjectPolicy))?
        }
        "evidence-run" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::EvidenceRun))?,
        "recipe-catalog" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::RecipeCatalog))?
        }
        "evolution-scorecard" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::EvolutionScorecard))?
        }
        "refactor-plan" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::RefactorPlan))?
        }
        "refactor-apply-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::RefactorApplyRun))?
        }
        "refactor-batch-apply-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::RefactorBatchApplyRun))?
        }
        "codebase-map" => serde_json::to_value(schemars::schema_for!(mdx_rust_core::CodebaseMap))?,
        "autopilot-run" => {
            serde_json::to_value(schemars::schema_for!(mdx_rust_core::AutopilotRun))?
        }
        other => anyhow::bail!("unknown schema kind: {other}"),
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&schema)?);
    } else {
        println!("JSON Schema for mdx-rust {kind}:");
        println!("{}", serde_json::to_string_pretty(&schema)?);
    }

    Ok(())
}

fn cmd_spec(name: &str, json: bool) -> anyhow::Result<()> {
    use mdx_rust_core::registry::Registry;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);
    let registry = Registry::load_from(&artifact_root)?;

    let agent = registry.get(name).ok_or_else(|| {
        anyhow::anyhow!(
            "Agent '{}' not registered. Run `mdx-rust register {}` first.",
            name,
            name
        )
    })?;

    let agent_dir = artifact_root.join("agents").join(name);
    std::fs::create_dir_all(&agent_dir)?;

    let bundle = mdx_rust_analysis::analyze_agent(&agent.path, None).ok();
    let analysis_summary = if let Some(b) = &bundle {
        format!(
            "{} files, Rig={}, preambles={:?}",
            b.scope.optimizable_paths.len(),
            b.is_rig_agent,
            b.preambles.iter().map(|p| &p.text).collect::<Vec<_>>()
        )
    } else {
        "limited analysis".into()
    };

    let policies = format!("# Policies for {}\n\n- Use explicit step-by-step reasoning in prompts.\n- Be concise but complete.\n- Structured output (answer + reasoning).\n\n(Generated from: {})", name, analysis_summary);
    std::fs::write(agent_dir.join("policies.md"), policies)?;

    let spec = serde_json::json!({"description": format!("Eval spec for {}", name), "dataset": "dataset.json"});
    std::fs::write(
        agent_dir.join("eval_spec.json"),
        serde_json::to_string_pretty(&spec)?,
    )?;

    let ds = serde_json::json!([{"query":"What is 2+2?"},{"query":"Explain the sky being blue."}]);
    std::fs::write(
        agent_dir.join("dataset.json"),
        serde_json::to_string_pretty(&ds)?,
    )?;

    if json {
        println!(
            "{}",
            serde_json::json!({"agent":name,"policies":"policies.md","eval_spec":"eval_spec.json","dataset":"dataset.json"})
        );
    } else {
        println!("✅ Spec generated for '{}'\n   • policies.md\n   • eval_spec.json\n   • dataset.json\n   Analysis: {}", name, analysis_summary);
    }
    Ok(())
}

fn detect_contract(path: &std::path::Path) -> mdx_rust_core::registry::AgentContract {
    use mdx_rust_analysis::finders::find_run_agent_functions;

    // Check for Rig first via Cargo.toml
    let cargo_content = std::fs::read_to_string(path.join("Cargo.toml")).unwrap_or_default();
    if cargo_content.contains("rig-core") || cargo_content.contains("rig") {
        return mdx_rust_core::registry::AgentContract::NativeRust;
    }

    // Try to find run_agent style functions using tree-sitter
    if let Ok(main_rs) = std::fs::read_to_string(path.join("src/main.rs")) {
        let found = find_run_agent_functions(&main_rs);
        if !found.is_empty() || mdx_rust_analysis::finders::looks_like_rig_agent(&main_rs) {
            return mdx_rust_core::registry::AgentContract::NativeRust;
        }
    }

    mdx_rust_core::registry::AgentContract::Process
}

/// Simple recursive copy for best/ persistence (ignores target/, .git, .worktrees)
fn copy_dir_recursive(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if ty.is_dir() {
            let name = entry.file_name();
            if name == "target" || name == ".git" || name == ".worktrees" {
                continue;
            }
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            std::fs::copy(&src_path, &dst_path)?;
        }
    }
    Ok(())
}
