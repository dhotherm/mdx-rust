//! mdx-rust-core
//!
//! Core logic for the MDx Rust agent optimizer.

pub mod config;
pub mod registry;
pub mod runner;

pub use config::Config;
pub use registry::{AgentContract, RegisteredAgent, Registry};
pub use runner::{run_agent, AgentRunResult};