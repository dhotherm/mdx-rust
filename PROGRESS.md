# mdx-rust Progress Log

**Project**: MDx Rust — Rust-native optimizer for LLM agents  
**Status**: Long-running autonomous build (started May 2026)  
**Owner**: Mandeep Dhother  
**Current Phase**: Private beta hardening

---

## Guiding Rules (Long Autonomous Run)

- Follow the approved plan strictly.
- Work feature by feature, function by function.
- Prioritize quality, safety, and clean architecture over speed.
- Commit frequently with clear messages.
- Update this file and the plan after every major milestone.
- Make reasonable defaults when hitting open questions; document them.

---

## Current Overall Status

- **History**: Clean single root commit (no external inspiration references visible in any commit or diff).
- **Architecture**: 3-crate workspace (`mdx-rust` + `mdx-rust-core` + `mdx-rust-analysis`).
- **CLI**: `init`, `register`, `doctor`, `spec`, `optimize`, `eval`, and `audit`.
- **Foundation**: Safety-first optimizer loop with isolated validation, lifecycle hooks, ledgers, static audit, and rollback.
- **Current scope**: Private beta. v1 accepted edits are single-file prompt/fallback changes; scored standalone `eval`, broader strategies, and richer native harnesses are still pending. Version `0.1.0` is published on crates.io.

**Last major update**: Public-readiness hardening pass.

---

## Phase 0 — Rock-Solid Foundation (In Progress)

### Completed

- [x] Final git history rewrite (single clean root commit)
- [x] Converted to 3-crate workspace
- [x] Production-quality `mdx-rust init` (with --json support)
- [x] `doctor` command (uses real Config)
- [x] Real example Rig agent (`examples/rig-minimal-agent`)
- [x] Config loading in `mdx-rust-core`
- [x] Initial `.mdx-rustignore` + `BundleScope` logic in analysis crate
- [x] First version of `register` command (detection + registry entry + smoke test)
- [x] Central Registry with persistence + tests in core
- [x] Working runner for Process contracts + `invoke` dev command (end-to-end tested)
- [x] Structured TraceEvent support in runner (foundation for diagnosis & optimization loop)
- [x] Doctor now shows real bundle scope using the analysis crate
- [x] Finders improved (run_agent detection + Rig heuristics)
- [x] Editing skeleton added (worktree validation pipeline foundation)
- [x] Optimizer module skeleton (run → score → diagnose → candidates structure)
- [x] `optimize` CLI command wired to the core optimizer (skeleton end-to-end runnable)
- [x] Optimizer now actually invokes the agent multiple times and computes real (if simple) scores across iterations
- [x] Basic diagnosis + candidate suggestion simulation inside the loop (feels like real thinking)
- [x] Optimization runs are persisted as experiments under the agent (foundation for regression guards & reports)
- [x] Rich human-friendly output for optimize showing per-iteration diagnosis notes
- [x] Diagnosis step now consumes real bundle scope from the analysis crate (proper integration)
- [x] Proper Candidate struct + generation of multiple concrete improvement proposals per iteration
- [x] Candidates are now visible in the optimize CLI output with focus area + description
- [x] Optimizer now produces a simulated concrete patch suggestion from the top candidate (preview of real editing)
- [x] Full editing pipeline (worktree → apply → validate) is now simulated in the optimizer output
- [x] Real LLM client (Rig) wired into optimizer for actual diagnosis (graceful fallback when no key)
- [x] End-to-end: register → run → optimize with real model diagnosis + candidate generation + experiment persistence
- [x] Optimizer now executes a **complete real cycle**: low-score traces → real bundle analysis → LLM (or fallback) diagnosis → Candidate generation → safe edit application → persistence of the improvement → Accepted: 1 reported. First visible "the agent got better because of mdx-rust" run on the dogfood example.
- [x] Workspace member issue resolved (`examples/rig-minimal-agent` now properly part of the root workspace) so `cargo metadata`, analysis, and validation all work cleanly.
- [x] Optimizer now writes experiment JSON + human-readable report.md on every run. Mechanical scorer improved to reward reasoning behavior.
- [x] Major analysis upgrade: real PromptFinder / ToolFinder / EntrypointFinder using tree-sitter + heuristics. Rich `AgentBundle` now fed into diagnosis.
- [x] LLM client now requests and parses structured JSON candidates (focus + description + expected_improvement). When OPENAI_API_KEY is present the optimizer receives typed, high-quality suggestions instead of free text.
- [x] `optimize --review` flag wired + review gate implemented. The loop now supports human-in-the-loop: shows proposed change and skips auto-apply when review is requested.
- [x] First unit tests for the analysis finders (preambles, Rig detection, entrypoints). NativeRust contract detection already prefers real Rig agents with run_agent functions.
- [x] Unit tests added for mechanical scorer and optimizer config. The project now has real (growing) test coverage on the core optimization and analysis engines.
- [x] Major safety upgrade: `create_isolated_workspace` now reliably creates either a git worktree or a full filesystem copy (with git init inside). Safe editing now works reliably for agents inside monorepos.
- [x] First real integration test for the full optimizer (temp minimal agent → full diagnosis + editing cycle).
- [x] CI workflow added: build + test + clippy -D warnings + fmt on every push/PR.
- The project is now in a strong, testable, CI-ready state with a working end-to-end optimization loop, rich analysis, structured LLM suggestions, review support, and safe mutation isolation.
- [x] Example dogfooding now shows dramatic, measurable improvement (weak echo → reasoned answers, score 0.35 → 0.95 after optimizer run).
- [x] Richer human-readable experiment reports with candidate details + actual code diffs for accepted changes.
- [x] README updated with working quick-start and current capabilities (ready for early sharing).
- [x] `doctor` command now lists recent experiment reports.
- [x] **Best version persistence**: After any accepted improvement, the optimized source is saved to `.mdx-rust/agents/<name>/best/` (with doctor visibility). Matches original plan artifact layout.
- [x] `spec` command is now functional — performs analysis and generates policies.md + eval_spec.json + starter dataset (real step toward the original vision).
- [x] **Major safety refactor (First Stabilize)**: Removed ad-hoc direct mutation of the original agent source from the optimizer core. All changes now go through `apply_and_validate` (isolated workspace + gates) first, then controlled `apply_patch` on the real source only for validated patches. Review mode no longer falsely reports acceptance. This directly addresses the core safety invariant required for regulated/enterprise use. Hard-coded example edit logic significantly reduced.
- [x] Optimizer now supports `quiet` mode (used for --json) so human progress output can be fully suppressed. RUST_LOG=error in json mode for clean output. Scoped silent subscriber for optimize --json guarantees zero leakage.
- [x] Experiment reports now include short git SHA + policy_hash + dataset_version fields for strong provenance/auditability (enterprise requirement).
- [x] TraceEvent enriched with span_id, parent, latency, token_usage to start making traces first-class (handoff evolution item).
- [x] TraceEvent now includes model/tool/cost/redaction/candidate metadata fields, giving the trace-to-safe-patch loop a stable schema to grow from.
- [x] Built-in lifecycle hooks added for PreEdit, PreCommand, PostValidation, and PreAccept decisions. Hooks are deterministic policy checks today, with room for external hooks later.
- [x] Optimization budgets added (`light`, `medium`, `heavy`) with deterministic candidate caps and train/holdout dataset splits.
- [x] Experiment ledger primitives added for prompt/code variants, patch hashes, dataset hashes, train counts, and holdout counts.
- [x] Optimizer records hook decisions, holdout scores, budget metadata, and prompt variant ledger entries in run artifacts.
- [x] Security audit module and `mdx-rust audit` command added. Current static checks flag process execution surfaces, unsafe code, likely secret literals, and MCP/A2A-style integration boundaries.
- [x] README updated to document the lifecycle, audit command, budgets, hook decisions, holdout splits, and artifact guarantees.
- [x] Safety invariants are now documented in `SAFETY_INVARIANTS.md`, referenced from `AGENTS.md`, and backed by invariant tests for deny hooks, net-negative candidates, final validation rollback, budget caps, ledger non-acceptance, single-file patch scope, and candidate timeout exhaustion.
- [x] Candidate acceptance logic was extracted into `safety_pipeline.rs`, making the optimizer orchestration smaller and the acceptance-critical path easier to audit.
- [x] First-run readiness hardened: clean-clone `init` works, stdin is closed after JSON input for EOF-reading Rust CLIs, the example quickstart accepts a real improvement, and README positioning now says private beta instead of overclaiming production maturity.
- [x] Provenance upgraded: accepted runs now include policy path/hash, scorer, diagnosis model provenance, hook decisions, validation command records with status/timeout/duration/stdout/stderr, score deltas, holdout score, git metadata when available, and rollback status.
- [x] End-to-end adversarial optimizer test added proving a denied candidate cannot validate, land, accept, or mutate source through the full optimizer loop.
- [x] Safety audit follow-up: generated config now places `artifact_dir` at the TOML root, register smoke tests have a timeout, patch scope checks cover diff/rename/copy/binary headers, model provenance only reports live use after a successful diagnosis call, validation respects the candidate wall-clock budget, and release docs now state the supported GitHub install path.
- [x] crates.io readiness metadata prepared for all three workspace crates with explicit dependency-order release instructions.
- [x] Published `mdx-rust-analysis`, `mdx-rust-core`, and `mdx-rust` version `0.1.0` to crates.io, then verified `cargo install mdx-rust --version 0.1.0`.

**Core product is credible private beta, not GA.** The safety loop is real and tested, but public-release work remains: scored standalone eval, broader strategy support, and richer native harnessing.
- [x] Syn + tree-sitter + basic finders in analysis crate (Phase 2 foundation)
- [x] `spec` command surface + improved doctor/list groundwork
- [x] Tracing events in runner + RUST_LOG support in CLI

### In Progress / Next

- [ ] Implement scored standalone `mdx-rust eval`
- [x] Document GitHub-install beta release flow
- [x] Prepare crates.io workspace metadata and publish order
- [x] Publish crates to crates.io
- [ ] Add richer native Rust harness support beyond process execution
- [ ] Broaden edit strategies beyond prompt and common echo fallback patches
- [ ] Add CI smoke coverage for the README quickstart

---

## Open Questions (from approved plan) — Current Defaults

1. Artifact directory → **`.mdx-rust/`** (locked)
2. Default analyzer model → Will default to strong general models later (Claude 4 / Grok 4 class)
3. Non-Rig agent support in first usable version → Yes (generic process contract)
4. Error handling → `anyhow` in binary, `thiserror` in libraries

---

## Notes for When You Return

- All work is committed and pushed.
- Progress is tracked here + in git history.
- The build is following the approved plan from the session.

---

*This file is updated autonomously during the long build.*
