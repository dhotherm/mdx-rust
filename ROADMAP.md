# Roadmap

`mdx-rust` is an early Rust-native optimizer for LLM agents. The current public
release is useful for experimentation and early adopters, but still intentionally
conservative.

## Current Release

Version `0.1.0` is published on crates.io:

```bash
cargo install mdx-rust
```

Current strengths:

- Rust-aware source analysis using `syn` and `tree-sitter-rust`.
- Agent registration, invocation, diagnosis, optimization, evaluation stubs,
  doctor checks, and static audit checks.
- Single-file prompt and fallback-behavior edits.
- Isolated candidate validation with `cargo check` and `cargo clippy`.
- Net-positive acceptance gates, final validation, rollback, hook decisions,
  experiment ledgers, and provenance records.
- Machine-readable `--json` output for automation and coding agents.

## Current Scope

The first public release is a private-beta quality tool, not a general-purpose
autonomous refactoring system.

- Accepted edits are single-file only.
- Current strategies focus on prompts and common fallback behavior.
- Standalone scored `mdx-rust eval` is not complete yet.
- Native Rust contracts currently run through a process harness.
- External lifecycle hooks are not enabled yet.

See [SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md) for the acceptance contract.

## Near-Term Work

- Complete scored standalone `mdx-rust eval`.
- Expand edit strategies beyond prompt and simple fallback changes.
- Add richer native Rust harness support.
- Add example-level CI smoke tests for the README quickstart.
- Add docs.rs-oriented library documentation for the internal crates.
- Add optional security gates that can turn audit findings into hook denials.

## Release Checklist

Before publishing a new version:

```bash
cargo fmt --all -- --check
cargo check --workspace --locked
cargo test --workspace --locked
cargo clippy --workspace --locked -- -D warnings
cargo build --workspace --release --locked
cargo package -p mdx-rust-analysis --locked
cargo package -p mdx-rust-core --locked
cargo package -p mdx-rust --locked
```

Publish crates in dependency order:

```bash
cargo publish -p mdx-rust-analysis
cargo publish -p mdx-rust-core
cargo publish -p mdx-rust
```
