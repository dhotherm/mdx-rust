//! Rust source analysis and safe edit helpers for `mdx-rust`.
//!
//! This crate owns source discovery, Rust code finders, bundle construction,
//! conservative hardening analysis, isolated workspace creation, patch
//! application, validation command records, and rollback snapshots used by the
//! optimizer and hardening engine.
//!
//! ## Stability contract
//!
//! The supported product surface for `0.4.x` is the `mdx-rust` CLI. This crate
//! is published so the CLI can be installed from crates.io, but the library API
//! is intentionally unstable before `1.0`.

#[doc(hidden)]
pub mod bundler;
#[doc(hidden)]
pub mod editing;
#[doc(hidden)]
pub mod finders;
#[doc(hidden)]
pub mod hardening;

/// Analyze an agent crate and return the source scope mdx-rust may inspect.
pub use bundler::{analyze_agent, build_bundle_scope, AgentBundle, BundleScope};
/// Rust source finders used to identify prompts, tools, and entrypoints.
pub use finders::{
    find_preambles, find_run_agent_functions, find_tools, looks_like_rig_agent, AgentEntrypoint,
    ExtractedPrompt, ExtractedTool,
};
/// Conservative Rust hardening analysis for ordinary Rust modules.
pub use hardening::{
    analyze_hardening, HardeningAnalysis, HardeningAnalyzeConfig, HardeningFileChange,
    HardeningFinding, HardeningStrategy,
};
