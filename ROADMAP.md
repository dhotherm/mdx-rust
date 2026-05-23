# Roadmap

`mdx-rust` is evolving from a Rust agent optimizer into a Rust-native
safe-change system for codebases.

The invariant stays the same: no change is trusted because an LLM or heuristic
suggested it. A change must be scoped, validated in isolation, measured against
the relevant policy or behavior signal, landed deliberately, and audited.

## v0.2.0 - Released

First serious safety-first release.

- CLI-first API stability contract.
- Explicitly unstable library APIs before `1.0`.
- Versioned audit packets for accepted optimizer changes.
- Single-file edit scope hard-enforced for agent optimization.
- First narrow parser-validated Rust fallback edit strategy.
- Positive and negative end-to-end safety proof tests.
- Rustdoc and docs.rs gates in CI.
- Clear first-run and release-readiness documentation.

## v0.3.0 - Released

Safe scoped hardening for ordinary Rust modules.

Primary goals:

- `mdx-rust doctor` and `mdx-rust audit` work on normal Rust workspaces without
  agent registration.
- `mdx-rust improve [target]` proposes high-confidence hardening changes in
  review mode by default.
- `--apply` is required before hardening changes touch the real tree.
- Bounded hardening transactions snapshot every touched file and rollback on
  final validation failure.
- First hardening strategies focus on panic-prone `unwrap`/`expect` calls in
  `anyhow::Result` functions, risky process execution surfaces, unsafe code,
  and environment-derived config boundaries.
- Hardening reports use schema version `0.3` and include policy hash, workspace
  metadata, findings, proposed changes, validation records, transaction status,
  and rollback status.

## v0.4.0 - Released

Behavior and policy-driven improvement for Rust services.

- Workspace behavior eval specs in `.mdx-rust/evals.json`.
- `mdx-rust eval` runs deterministic command-based evals without requiring an
  agent registration.
- `mdx-rust improve --eval-spec` requires behavior evals to pass in isolation
  and after final application.
- Structured markdown policy parsing for backend safety rules such as no panics
  in request paths, contextual errors, validated external inputs, and boundary
  handling.
- `doctor` summarizes high, medium, and patchable findings and prints
  recommended next actions.
- Hardening reports include policy matches, behavior eval evidence, risk
  summaries, and generated schemas.

## v0.5.0 - Released

Guardrailed Rust refactoring assistant with impact analysis.

- Plan-first workflow: `mdx-rust plan`, human review, then
  `mdx-rust apply-plan` for supported low-risk candidates.
- Execution queue workflow: `mdx-rust apply-plan --all` can review or apply all
  executable low-risk candidates from a plan with per-step validation.
- Crate/module graph, touched-area model, and public API impact detection.
- Safe refactor recipe candidates such as extract function, split oversized
  module, consolidate error handling, isolate boundary validation, and apply
  patchable hardening.
- Stale-plan rejection through source snapshot hashes, plan hashes, and
  candidate hashes.
- Public API protection through semver and public API checks where applicable.
- Versioned refactor plan artifacts with plan hashes, required gates, risk
  summaries, and explicit non-goals.

## v0.6.0 Focus

Autonomous Rust evolution for safe executable work.

- `mdx-rust map` codebase intelligence with quality grade, debt score,
  capability gates, hardening findings, and recommended actions.
- `mdx-rust autopilot` review mode for non-mutating autonomous planning and
  execution simulation.
- `mdx-rust autopilot --apply` for multi-pass execution of low-risk candidates
  through the existing plan, apply-plan, hardening, validation, and rollback
  gates.
- Fresh planning before every autonomous apply pass.
- Versioned codebase map and autopilot artifacts for agents, CI, and future MCP
  or API surfaces.
- Optional gate detection for nextest, llvm-cov, mutants, and semver checks.

## v0.7.0 Direction

Broader recipe execution with stronger semantic analysis.

- Narrow multi-file recipe support with full transaction snapshots and rollback
  evidence.
- Public API compatibility checks for crates that opt in.
- Deeper Rust-aware impact analysis before any broader refactor is allowed.
- First autonomous extraction or module-splitting recipe behind behavior eval,
  semver, and transaction gates.
- Agent distribution surfaces: documented skill packs, MCP schema boundaries,
  and an API-friendly runner over the CLI JSON contract.

## Current Non-Goals

- Autonomous public API rewrites without explicit gates.
- External hook execution.
- MCP/A2A runtime integration.
- Multi-agent orchestration.
- Multi-language support.
- UI work.
