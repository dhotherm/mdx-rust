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
        #[arg(long, default_value = "5")]
        iterations: u32,

        /// Number of candidate fixes to generate per iteration
        #[arg(long, default_value = "3")]
        candidates: u32,

        /// Dry run (do not apply any changes)
        #[arg(long)]
        dry_run: bool,

        /// Review mode: ask for confirmation before applying each candidate
        #[arg(long)]
        review: bool,
    },

    /// Inspect what would be bundled, editable scope, and current state
    Doctor {
        /// Agent name (optional; if omitted, lists all registered agents)
        name: Option<String>,
    },

    /// Evaluate the current (or a specific) version of the agent on a dataset
    Eval {
        /// Agent name
        name: String,

        /// Path to dataset JSON file
        #[arg(long)]
        dataset: Option<String>,
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
                if cli.json {
                    println!(r#"{{"status":"error","error":"{}"}}"#, e);
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Register { name, path } => {
            if let Err(e) = cmd_register(&name, path.as_deref(), cli.json) {
                if cli.json {
                    println!(r#"{{"status":"error","error":"{}"}}"#, e);
                } else {
                    eprintln!("Error during registration: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Spec { name } => {
            println!("Generating policy + eval spec for '{}'", name);
            // TODO: LLM analysis → policies.md + eval_spec.json + dataset preview
        }
        Commands::Optimize {
            name,
            iterations,
            candidates,
            dry_run,
            review,
        } => {
            println!(
                "Optimizing '{}' (iterations={}, candidates={}, dry_run={}, review={})",
                name, iterations, candidates, dry_run, review
            );
            // TODO: full loop with tracing, diagnosis, candidate generation, validation
        }
        Commands::Doctor { name } => {
            if let Err(e) = cmd_doctor(name.as_deref(), cli.json) {
                if cli.json {
                    println!(r#"{{"status":"error","error":"{}"}}"#, e);
                } else {
                    eprintln!("Error: {}", e);
                }
                std::process::exit(1);
            }
        }
        Commands::Eval { name, dataset } => {
            println!("Evaluating agent '{}' with dataset {:?}", name, dataset);
            // TODO: run evaluation harness
        }
        Commands::Invoke { name, input } => {
            if let Err(e) = cmd_invoke(&name, input.as_deref(), cli.json) {
                eprintln!("Invoke error: {}", e);
                std::process::exit(1);
            }
        }
    }
}

/// Initialize tracing with nice human output by default, or JSON when requested.
/// Supports RUST_LOG for fine-grained control (e.g. RUST_LOG=mdx_rust_core::runner=debug)
fn init_tracing(json: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::from_default_env()
        .add_directive("mdx_rust=info".parse().unwrap_or_else(|_| "info".parse().unwrap()));

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
            println!(r#"{{"status":"already_initialized","path":"{}"}}"#, artifact_dir);
        } else {
            println!("mdx-rust is already initialized in this directory ({} exists).", artifact_dir);
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
        println!(r#"{{"status":"initialized","artifact_dir":"{}"}}"#, artifact_dir);
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
    use std::path::Path;

    let cwd = std::env::current_dir()?;
    let config = Config::load_from_project(&cwd).unwrap_or_default();
    let artifact_root = &config.artifact_dir;
    let agent_dir = format!("{}/agents/{}", artifact_root, name);

    if json {
        println!(r#"{{"agent":"{}","artifact_dir":"{}","registered":{}}}"#, 
                 name, artifact_root, Path::new(&agent_dir).exists());
        return Ok(());
    }

    println!("🔍 mdx-rust doctor — agent '{}'", name);
    println!("   Artifact directory: {}", artifact_root);
    println!();

    if !Path::new(artifact_root).exists() {
        println!("  ❌ {} does not exist. Run `mdx-rust init` first.", artifact_root);
        return Ok(());
    }

    println!("  ✅ {} exists", artifact_root);

    if Path::new(&agent_dir).exists() {
        println!("  ✅ Agent is registered");
    } else {
        println!("  ℹ️  Agent is not registered yet");
        println!("     → mdx-rust register {}", name);
    }

    println!();
    println!("(Deeper analysis, bundle scope, and experiment status coming in later phases)");

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
        anyhow::bail!("No Cargo.toml found at {}. Is this a Rust crate?", target_path.display());
    }

    let config = Config::load_from_project(&cwd)?;
    let artifact_root = cwd.join(&config.artifact_dir);

    // Load or create registry
    let mut registry = Registry::load_from(&artifact_root)?;

    // Basic contract detection (improve later with real analysis)
    let contract = detect_contract(&target_path);

    let agent = RegisteredAgent {
        name: name.to_string(),
        path: target_path.clone(),
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
        println!(r#"{{"status":"registered","name":"{}","path":"{}","contract":"{:?}","smoke_test_passed":{}}}"#,
                 name, target_path.display(), registry.get(name).unwrap().contract, smoke_passed);
    } else {
        println!("✅ Registered agent '{}'", name);
        println!("   Path: {}", target_path.display());
        println!("   Contract detected: {:?}", registry.get(name).unwrap().contract);
        println!("   Smoke test (cargo check): {}", if smoke_passed { "PASSED" } else { "FAILED or skipped" });
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
        if !found.is_empty() {
            return mdx_rust_core::registry::AgentContract::NativeRust;
        }
    }

    mdx_rust_core::registry::AgentContract::Process
}
