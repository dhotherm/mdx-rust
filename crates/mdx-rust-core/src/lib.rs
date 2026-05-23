//! mdx-rust-core
//!
//! Core logic for the MDx Rust agent optimizer:
//! - Agent registration and discovery
//! - Experiment tracking and safety
//! - The main optimization loop (implemented in later phases)
//! - Config and artifact management

pub mod config;
pub mod registry;

// Re-export common types
pub use anyhow::Result;