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

## v0.3.0 Focus

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

## v0.4.0 Direction

Behavior and policy-driven improvement for Rust services.

- CLI, HTTP, and golden-file eval harnesses for non-agent Rust services.
- Policy language for backend safety rules such as no panics in request paths,
  contextual errors, validated external inputs, and handler/layer boundaries.
- `doctor` ranks findings by confidence and suggests regression tests.
- Optional `cargo nextest`, coverage, and mutation-probe integrations.
- Audit packets include before/after behavior evidence and generated tests.

## v0.5.0 Direction

Guardrailed Rust refactoring assistant with impact analysis.

- Plan-first workflow: `mdx-rust plan`, human review, then `mdx-rust apply`.
- Crate/module graph, touched-area model, and public API impact detection.
- Safe refactor recipes such as extract function, split oversized module,
  consolidate error handling, and isolate boundary validation.
- Public API protection through semver and public API checks where applicable.
- Multi-step transactions with plan hashes and rollback evidence.

## Current Non-Goals

- Arbitrary autonomous refactoring.
- External hook execution.
- MCP/A2A runtime integration.
- Multi-agent orchestration.
- Multi-language support.
- UI work.
