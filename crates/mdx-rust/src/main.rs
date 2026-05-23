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

    // TODO: Initialize tracing (human pretty by default, JSON when --json)

    match cli.command {
        Commands::Init => {
            println!("Initializing mdx-rust in current directory...");
            // TODO: create .mdx-rust/, config, .mdx-rustignore, example policy
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
            println!("Running diagnostics for agent '{}'", name);
            // TODO: show bundle scope, editable paths, current best score, etc.
        }
        Commands::Eval { name, dataset } => {
            println!("Evaluating agent '{}' with dataset {:?}", name, dataset);
            // TODO: run evaluation harness
        }
    }
}
