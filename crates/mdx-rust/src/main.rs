use clap::{Parser, Subcommand};

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

    let artifact_dir = ".mdx-rust";
    let cwd = std::env::current_dir()?;

    if Path::new(artifact_dir).exists() {
        if json {
            println!(r#"{{"status":"already_initialized","path":"{}"}}"#, artifact_dir);
        } else {
            println!("mdx-rust is already initialized in this directory ({} exists).", artifact_dir);
        }
        return Ok(());
    }

    fs::create_dir(artifact_dir)?;

    // Create config.toml
    let config_content = r#"# mdx-rust configuration
# See docs for more options

[models]
# analyzer = "claude-4-sonnet"   # Strong model for diagnosis & improvements
# judge = "gpt-4o"               # Model used for LLM-as-Judge scoring
# default = "gpt-4o-mini"

artifact_dir = ".mdx-rust"
"#;
    fs::write(format!("{}/config.toml", artifact_dir), config_content)?;

    // Create .mdx-rustignore with sensible defaults
    let ignore_content = r#"# Files and directories that mdx-rust should never include when bundling
# for the LLM (in addition to .gitignore and common VCS/lock files)

# Build artifacts
target/
**/*.rlib
**/*.rmeta
**/*.so
**/*.dylib
**/*.dll

# Lockfiles and generated
Cargo.lock
**/*.lock

# IDE / editor
.idea/
.vscode/
*.swp
*.swo

# Test fixtures that are large or noisy
**/test-data/
**/fixtures/large/

# mdx-rust's own artifacts (never bundle ourselves)
.mdx-rust/
"#;
    fs::write(format!("{}/.mdx-rustignore", artifact_dir), ignore_content)?;

    // Create a starter policies.md
    let policies_content = r#"# Agent Policy

## Purpose
Describe what this agent is supposed to do in 1-2 sentences.

## Decision Rules
1. ...
2. ...

## Constraints
- Never ...
- Always ...

## Quality Expectations
- ...
- ...

## Edge Cases
| Scenario | Expected Behaviour |
|----------|--------------------|
| ...      | ...                |
"#;
    fs::write(format!("{}/policies.md", artifact_dir), policies_content)?;

    // Create a basic eval_spec.json template
    let eval_spec = r#"{
  "version": 1,
  "description": "Evaluation spec for this agent",
  "scoring": {
    "fields": []
  },
  "policy_path": "policies.md"
}
"#;
    fs::write(format!("{}/eval_spec.json", artifact_dir), eval_spec)?;

    if json {
        println!(r#"{{"status":"initialized","path":"{}","files_created":["config.toml",".mdx-rustignore","policies.md","eval_spec.json"]}}"#, artifact_dir);
    } else {
        println!("✅ mdx-rust initialized successfully in {}", cwd.display());
        println!();
        println!("Created:");
        println!("  {}/config.toml", artifact_dir);
        println!("  {}/.mdx-rustignore", artifact_dir);
        println!("  {}/policies.md      (edit this with your agent's rules)", artifact_dir);
        println!("  {}/eval_spec.json", artifact_dir);
        println!();
        println!("Next steps:");
        println!("  mdx-rust register <name> [path]");
        println!("  mdx-rust doctor <name>");
    }

    Ok(())
}

/// First version of `doctor` — shows the state of the .mdx-rust/ directory
/// and basic information about a registered agent (if any).
fn cmd_doctor(name: &str, json: bool) -> anyhow::Result<()> {
    use std::fs;
    use std::path::Path;

    let artifact_root = ".mdx-rust";
    let agent_dir = format!("{}/agents/{}", artifact_root, name);

    if json {
        println!(r#"{{"command":"doctor","agent":"{}","artifact_root":"{}","exists":{}}}"#, 
                 name, artifact_root, Path::new(&agent_dir).exists());
        return Ok(());
    }

    println!("🔍 mdx-rust doctor for agent '{}'", name);
    println!();

    if !Path::new(artifact_root).exists() {
        println!("  ❌ No {} directory found in this project.", artifact_root);
        println!("     Run `mdx-rust init` first.");
        return Ok(());
    }

    println!("  ✅ {} directory exists", artifact_root);

    // Check for key files
    let config_path = format!("{}/config.toml", artifact_root);
    if Path::new(&config_path).exists() {
        println!("  ✅ config.toml found");
    } else {
        println!("  ⚠️  config.toml missing");
    }

    if Path::new(&agent_dir).exists() {
        println!("  ✅ Agent '{}' is registered", name);
        // In later phases we'll show registry.json, best version, etc.
    } else {
        println!("  ℹ️  Agent '{}' is not yet registered", name);
        println!("     Run: mdx-rust register {}", name);
    }

    println!();
    println!("  (More detailed bundle scope, ignore analysis, and experiment");
    println!("   status will appear here in later phases.)");

    Ok(())
}
