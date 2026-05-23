//! mdx-rust-analysis
//!
//! Responsible for:
//! - Discovering and parsing Rust source (syn + tree-sitter)
//! - Determining what code to bundle for the LLM
//! - Generating and applying safe patches
//!
//! This crate will be heavily developed in Phase 2.

pub mod bundler;

pub use bundler::{build_bundle_scope, BundleScope};