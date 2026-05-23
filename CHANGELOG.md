# Changelog

All notable public changes to `mdx-rust` are documented here.

## 0.3.0 - Unreleased

Safe scoped hardening for ordinary Rust modules.

### Added

- `mdx-rust improve [target]` for review-first hardening of Rust files or
  directories without requiring agent registration.
- Workspace `mdx-rust doctor` and `mdx-rust audit` modes when no agent name is
  provided.
- Hardening reports under `.mdx-rust/hardening/` with schema version `0.3`.
- JSON Schema export for `hardening-run` and `hardening-finding`.
- Bounded hardening transactions that validate in isolation, snapshot touched
  files, apply only with `--apply`, and rollback on final validation failure.
- A first conservative hardening strategy that replaces panic-prone
  `unwrap`/`expect` calls in `anyhow::Result` functions with contextual errors.

### Changed

- Product framing expands from Rust agent optimization to safe, auditable Rust
  codebase improvement.
- `doctor` and `audit` remain compatible with registered agents while also
  supporting normal Rust workspaces.

### Known Limitations

- Hardening strategies are intentionally narrow and high-confidence.
- General autonomous refactoring remains out of scope.
- Broader multi-file refactors require future impact analysis and plan-first
  review.

## 0.2.0 - 2026-05-23

First serious safety-first release candidate.

### Added

- Versioned audit packets for accepted optimizer changes.
- `mdx-rust schema <kind> --json` for machine-readable JSON Schema export of
  audit packets and agent-facing records.
- Clear API stability documentation for the published crates.
- Stronger release-readiness documentation and CI documentation gates.
- A narrow AST-backed fallback edit path that verifies Rust parses before and
  after rewriting echo-style fallback strings.

### Changed

- The CLI remains the supported product surface. Published library crates are
  explicitly unstable before 1.0 and expose a narrower documented facade.
- The v0.2 safety contract hard-enforces single-file accepted edits.
- Release-candidate checks now include docs warnings, release build, install
  smoke coverage, dependency audit, unused dependency checks, package inspection,
  and explicit publish-order documentation.
- `tree-sitter`, `tree-sitter-rust`, and `ring` were updated to clear the
  active RustSec advisory path.

### Known Limitations

- Accepted edits are still limited to one file.
- Current strategies focus on prompt and fallback behavior improvements.
- The AST-backed edit path is intentionally narrow; broad native Rust
  refactoring remains future work.
- Library APIs are unstable before 1.0.
- Standalone scored `mdx-rust eval` is still incomplete.

## 0.1.1 - 2026-05-23

### Fixed

- Added a small library facade to the CLI crate so docs.rs can build
  documentation for the published package.

## 0.1.0 - 2026-05-23

Initial public release.

### Added

- `mdx-rust` CLI with `init`, `register`, `doctor`, `spec`, `invoke`,
  `eval`, `audit`, and `optimize` commands.
- Rust-aware source analysis with prompt, tool, and entrypoint detection.
- Safe candidate pipeline with isolated validation, net-positive scoring,
  final validation, rollback, lifecycle hooks, and timeout handling.
- Experiment ledgers, provenance records, trace diagnosis, and JSON output.
- Static security audit checks for common risky agent surfaces.
- crates.io publication for `mdx-rust`, `mdx-rust-core`, and
  `mdx-rust-analysis`.

### Known Limitations

- Accepted edits are limited to one file.
- Current strategies focus on prompts and simple fallback behavior.
- Standalone scored `mdx-rust eval` is still incomplete.
- Native Rust execution currently uses a process harness.
