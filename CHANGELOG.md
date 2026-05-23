# Changelog

All notable public changes to `mdx-rust` are documented here.

## 0.2.0 - 2026-05-23

Release candidate scope for the first serious safety-first release.

### Added

- Versioned audit packets for accepted optimizer changes.
- Clear API stability documentation for the published crates.
- Stronger release-readiness documentation and CI documentation gates.
- A narrow parser-guarded fallback edit path that verifies Rust parses before
  and after rewriting echo-style fallback strings.

### Changed

- The CLI remains the supported product surface. Published library crates are
  explicitly unstable before 1.0 and expose a narrower documented facade.
- The v0.2 safety contract hard-enforces single-file accepted edits.

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
