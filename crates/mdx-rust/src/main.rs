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
            println!("Registering agent '{}' at {:?}", name, path);
            // TODO: analyze crate, detect entrypoint, smoke test, write registry
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
            if let Err(e) = cmd_doctor(&name, cli.json) {
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
    }
}

/// Initialize tracing with nice human output by default, or JSON when requested.
fn init_tracing(json: bool) {
    use tracing_subscriber::EnvFilter;

    let filter = EnvFilter::from_default_env()
        .add_directive("mdx_rust=info".parse().unwrap());

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
