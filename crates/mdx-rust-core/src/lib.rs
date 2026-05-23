//! mdx-rust-core
//!
//! Core logic for the MDx Rust agent optimizer.

pub mod config;
pub mod eval;
pub mod llm;
pub mod optimizer;
pub mod registry;
pub mod runner;
pub mod trace;

pub use config::Config;
pub use eval::{EvaluationDataset, EvaluationSample, ScorerMetadata};
pub use optimizer::{
    mechanical_score, run_optimization, Candidate, EditStrategy, OptimizationRun, OptimizeConfig,
};
pub use registry::{AgentContract, RegisteredAgent, Registry};
pub use runner::{run_agent, AgentRunResult, TraceEvent};
pub use trace::{diagnose_run, FailureKind, FailureSignal, TraceDiagnosis};
