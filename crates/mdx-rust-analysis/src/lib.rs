//! mdx-rust-analysis
//!
//! Responsible for:
//! - Discovering and parsing Rust source (syn + tree-sitter)
//! - Determining what code to bundle for the LLM
//! - Generating and applying safe patches
//!
//! This crate will be heavily developed in Phase 2.

pub mod bundler;
pub mod editing;
pub mod finders;

pub use bundler::{analyze_agent, build_bundle_scope, AgentBundle, BundleScope};
pub use finders::{
    find_preambles, find_run_agent_functions, find_tools, looks_like_rig_agent,
    AgentEntrypoint, ExtractedPrompt, ExtractedTool,
};