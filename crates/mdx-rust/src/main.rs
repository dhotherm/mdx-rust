use clap::{Parser, Subcommand};
use mdx_rust_core::Config;

/// MDx Rust — A Rust-native optimizer for LLM agents.
///
/// Point mdx-rust at your existing Rust agent, give it a policy,
/// and let it safely improve prompts, tools, and logic through
/// structured experimentation with compile-time validation gates.
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

    /// Inspect what would be bundled, editable scope, and current state
    Doctor {
        /// Agent name
        name: String,
    },

    /// Evaluate the current (or a specific) version of the agent on a dataset
    Eval {
        /// Agent name
        name: String,

        /// Path to dataset JSON file
        #[arg(long)]
        dataset: Option<String>,
    },

    /// Run deterministic static security checks against a registered agent
    Audit {
        /// Agent name
        name: String,
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
        Commands::Doctor { name } => {
            if let Err(e) = cmd_doctor(&name, cli.json) {
                emit_error(cli.json, "doctor", &e);
                std::process::exit(1);
            }
        }
        Commands::Eval { name, dataset } => {
            if let Err(e) = cmd_eval(&name, dataset.as_deref(), cli.json) {
                emit_error(cli.json, "eval", &e);
                std::process::exit(1);
            }
        }
        Commands::Audit { name } => {
            if let Err(e) = cmd_audit(&name, cli.json) {
                emit_error(cli.json, "audit", &e);
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
    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "error",
                "command": command,
                "error": error.to_string()
            })
        );
    } else {
        eprintln!("{} error: {}", command, error);
    }
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

[models]
# analyzer = "claude-4-sonnet"     # Used for deep diagnosis and candidate generation
# judge    = "gpt-4o"              # Used for LLM-as-Judge scoring
# default  = "gpt-4o-mini"

artifact_dir = ".mdx-rust"
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
    let policies = r#"# Agent Policy

## Purpose
[Describe the purpose of your agent in 1-2 sentences]

## Decision Rules
1. ...
2. ...

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
        println!();
        println!("Next: mdx-rust register <name> [path]");
    }

    Ok(())
}

/// `doctor` command — shows project state using the loaded Config
fn cmd_doctor(name: &str, json: bool) -> anyhow::Result<()> {
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
    use mdx_rust_core::registry::{RegisteredAgent, Registry};
    use std::path::Path;
    use std::process::Command;

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
    let check = Command::new("cargo")
        .arg("check")
        .current_dir(&target_path)
        .output();

    let smoke_passed = match check {
        Ok(output) => output.status.success(),
        Err(_) => false,
    };

    if json {
        println!(
            "{}",
            serde_json::json!({
                "status": "registered",
                "name": name,
                "path": target_path.display().to_string(),
                "contract": format!("{:?}", registry.get(name).unwrap().contract),
                "smoke_test_passed": smoke_passed
            })
        );
    } else {
        println!("✅ Registered agent '{}'", name);
        println!("   Path: {}", target_path.display());
        println!(
            "   Contract detected: {:?}",
            registry.get(name).unwrap().contract
        );
        println!(
            "   Smoke test (cargo check): {}",
            if smoke_passed {
                "PASSED"
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

fn cmd_eval(name: &str, dataset: Option<&str>, json: bool) -> anyhow::Result<()> {
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
            },
        ))?
    };

    // Landing now happens inside the optimizer's safety pipeline (per Codex stabilization handoff).
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

fn cmd_audit(name: &str, json: bool) -> anyhow::Result<()> {
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
