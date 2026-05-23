//! mdx-rust-core
//!
//! Core logic for the MDx Rust agent optimizer.

pub mod config;
pub mod llm;
pub mod optimizer;
pub mod registry;
pub mod runner;

pub use config::Config;
pub use optimizer::{mechanical_score, run_optimization, Candidate, OptimizeConfig, OptimizationRun};
pub use registry::{AgentContract, RegisteredAgent, Registry};
pub use runner::{run_agent, AgentRunResult, TraceEvent};