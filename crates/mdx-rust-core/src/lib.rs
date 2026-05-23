//! Core primitives for the `mdx-rust` CLI.
//!
//! `mdx-rust-core` contains the optimizer, hardening engine, safety pipeline,
//! registry, evaluation, ledger, and audit primitives used by the `mdx-rust`
//! binary.
//!
//! ## Stability contract
//!
//! The supported product surface for `0.3.x` is the `mdx-rust` CLI. This crate
//! is published so the CLI can be installed from crates.io and so advanced
//! users can inspect the internal data structures, but the library API is not
//! yet stable. Public items may change before `1.0`.
//!
//! The intentionally documented facade is the set of `pub use` exports below.
//! Module paths are left public for the CLI and tests, but most modules are
//! hidden from rustdoc because they are implementation detail for now.

#[doc(hidden)]
pub mod config;
#[doc(hidden)]
pub mod eval;
#[doc(hidden)]
pub mod hardening;
#[doc(hidden)]
pub mod hooks;
#[doc(hidden)]
pub mod ledger;
#[doc(hidden)]
pub mod llm;
#[doc(hidden)]
pub mod optimizer;
#[doc(hidden)]
pub mod registry;
#[doc(hidden)]
pub mod runner;
#[doc(hidden)]
pub mod safety_pipeline;
#[doc(hidden)]
pub mod security;
#[doc(hidden)]
pub mod trace;

/// Configuration loading and defaults used by the CLI.
pub use config::Config;
/// Dataset and scorer metadata used by optimizer reports.
pub use eval::{EvaluationDataset, EvaluationSample, ScorerMetadata};
/// Scoped Rust hardening engine for ordinary Rust modules. Unstable before `1.0`.
pub use hardening::{
    run_hardening, HardeningChangeSummary, HardeningConfig, HardeningMode, HardeningOutcome,
    HardeningPolicyRecord, HardeningRun, HardeningStatus, WorkspaceSummary,
};
/// Built-in lifecycle hook primitives. These are unstable before `1.0`.
pub use hooks::{
    evaluate_builtin_hook, HookAction, HookContext, HookDecision, HookPolicy, HookStage,
};
/// Experiment budget and ledger records. These are unstable before `1.0`.
pub use ledger::{
    split_dataset, DatasetSplit, ExperimentLedger, OptimizationBudget, PromptVariantRecord,
};
/// Optimizer entrypoint and run records. These are unstable before `1.0`.
pub use optimizer::{
    mechanical_score, run_optimization, AcceptedEditSummary, AuditPacket, AuditProvenance,
    Candidate, EditStrategy, ModelProvenance, OptimizationRun, OptimizeConfig, ScoreProvenance,
};
/// Agent registry types used by CLI commands.
pub use registry::{AgentContract, RegisteredAgent, Registry};
/// Agent runner result and trace events. These are unstable before `1.0`.
pub use runner::{run_agent, AgentRunResult, TraceEvent};
/// Candidate safety pipeline. Direct use is unstable before `1.0`.
pub use safety_pipeline::{
    execute_candidate_edit, CandidateExecutionConfig, CandidateExecutionContext,
    CandidateExecutionOutcome, SafetyRejection, SafetyRejectionKind,
};
/// Deterministic static audit reports.
pub use security::{audit_agent, AuditFinding, AuditSeverity, SecurityAuditReport};
/// Trace diagnosis records.
pub use trace::{diagnose_run, FailureKind, FailureSignal, TraceDiagnosis};
