# mdx-rust

[![CI](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdx-rust.svg)](https://crates.io/crates/mdx-rust)
[![Docs.rs](https://docs.rs/mdx-rust/badge.svg)](https://docs.rs/mdx-rust)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

**A Rust-native safety loop for improving LLM agents.**

`mdx-rust` points at an existing Rust agent, runs it on a small evaluation set,
diagnoses weak behavior, proposes narrow edits, validates those edits in
isolation, and only lands changes that pass Rust gates and improve the score.

The CLI is the supported product surface. The library crates are published for
installation and inspection, but their APIs remain unstable before `1.0`.

## Current Scope

`mdx-rust` is an early public beta. It is useful for experimentation and
dogfooding on Rust agent crates, but it is intentionally conservative.

Today it supports:

- Rust-aware source analysis with `syn` and `tree-sitter-rust`.
- Process-based agent invocation with lifecycle traces.
- Prompt and parser-guarded fallback-behavior improvement strategies.
- Single-file accepted edits only.
- Isolated validation with `cargo check` and `cargo clippy -- -D warnings`.
- Net-positive scoring, final real-tree validation, and rollback on failure.
- Versioned audit packets for accepted changes.
- JSON Schema derivations for agent-facing records such as candidates, hooks,
  traces, eval datasets, audit packets, and validation command records.
- Human CLI output plus machine-parseable `--json` output.
- Deterministic static audit checks for risky agent surfaces.

Not yet supported:

- Arbitrary multi-file accepted edits.
- General autonomous refactoring.
- Stable library APIs.
- Complete standalone scored `mdx-rust eval`.
- External hook runners.
- Multi-language optimization.

## Safety Model

The acceptance contract is the center of the project:

1. Build a targeted `ProposedEdit` for one file.
2. Run pre-edit and pre-command hooks.
3. Apply the edit in an isolated workspace.
4. Run `cargo check` and `cargo clippy -- -D warnings` with timeouts.
5. Score the patched isolated workspace.
6. Require a strictly positive score delta.
7. Run pre-accept hooks.
8. Land the already validated edit on the real tree.
9. Run final validation on the real tree.
10. Roll back if final validation fails or times out.
11. Count the change as accepted only after landing and final validation pass.

The full non-bypass contract lives in
[SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md).

The implementation also uses typed rejection records and internal stage
wrappers so accepted changes cannot be represented the same way as proposed or
rejected candidates.

## Quick Start

Install the CLI:

```bash
cargo install mdx-rust
```

Try the built-in example from a checkout:

```bash
git clone https://github.com/dhotherm/mdx-rust
cd mdx-rust

cargo run -p mdx-rust -- init
cargo run -p mdx-rust -- register example examples/rig-minimal-agent
cargo run -p mdx-rust -- optimize example --iterations 2
cargo run -p mdx-rust -- audit example
cargo run -p mdx-rust -- invoke example --input '{"query":"What is 9 + 10?"}'
```

For your own Rust agent:

```bash
cd your-rust-agent
mdx-rust init
mdx-rust register my-agent .
mdx-rust optimize my-agent --iterations 3 --budget medium --review
```

Artifacts are written under `.mdx-rust/agents/<name>/`.

## Key Commands

```bash
mdx-rust init
mdx-rust register my-agent ./path/to/agent
mdx-rust spec my-agent
mdx-rust doctor my-agent
mdx-rust audit my-agent
mdx-rust eval my-agent --dataset .mdx-rust/agents/my-agent/dataset.json
mdx-rust optimize my-agent --iterations 3 --budget medium --review
```

Every command intended for automation supports `--json`.

## Audit Packets

Accepted changes produce versioned JSON audit packets in the experiment
directory. The `0.2` schema records:

- Agent name and iteration.
- Single-file edit scope contract.
- Accepted diff and diff hash.
- Dataset version and hash.
- Policy path and hash when available.
- Scorer id and version.
- Diagnosis model metadata and whether a live model was used.
- Hook decisions.
- Isolated and final validation command outcomes.
- Baseline, patched, delta, and holdout scores.
- Rollback status if rollback was attempted.

See [docs/provenance.md](./docs/provenance.md) for the schema contract.

## API Stability

`mdx-rust`, `mdx-rust-core`, and `mdx-rust-analysis` are all published so the
CLI can be installed from crates.io.

For `0.2.x`:

- The `mdx-rust` CLI is supported.
- The `mdx-rust-core` and `mdx-rust-analysis` APIs are unstable.
- Public library types may change before `1.0`.
- The intended facade is documented on docs.rs, but direct module usage is not
  a stability promise.

See [docs/api-stability.md](./docs/api-stability.md).

## Project Docs

- [SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md) - acceptance loop and non-bypass rules.
- [docs/architecture.md](./docs/architecture.md) - module and lifecycle overview.
- [docs/provenance.md](./docs/provenance.md) - audit packet schema.
- [docs/release-readiness.md](./docs/release-readiness.md) - release gates and manual checks.
- [ROADMAP.md](./ROADMAP.md) - current scope and next work.
- [CONTRIBUTING.md](./CONTRIBUTING.md) - development and safety expectations.

## Contributor Rails

This repo uses a `Justfile` as the canonical local command surface:

```bash
just ci
just audit
just machete
just release-candidate
```

These commands mirror the public CI expectations and keep coding agents from
guessing which checks matter.

## Status

`v0.2.0` is being prepared as the first serious safety-first release. Before it
is published, the candidate commit should pass local gates, GitHub CI, install
smoke, docs.rs-style rustdoc, package checks, and external pressure testing.

## License

MIT
