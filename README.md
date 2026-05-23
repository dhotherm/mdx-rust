# mdx-rust

[![CI](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdx-rust.svg)](https://crates.io/crates/mdx-rust)
[![Docs.rs](https://docs.rs/mdx-rust/badge.svg)](https://docs.rs/mdx-rust)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

**A Rust-native safe-change system for codebases.**

`mdx-rust` points at Rust code, finds scoped hardening opportunities, validates
changes in isolation, checks project policy and behavior evals when supplied,
and only lands edits that pass Rust gates. It still supports agent optimization,
but `v0.4` is aimed at ordinary Rust crates and service backends too.

The CLI is the supported product surface. The library crates are published for
installation and inspection, but their APIs remain unstable before `1.0`.

## Current Scope

`mdx-rust` is an early public beta. It is useful for experimentation and
dogfooding on Rust agent crates, but it is intentionally conservative. In
plain terms: `v0.4` is good at review-first, safety-gated hardening of scoped
Rust modules, simple policy mapping, deterministic behavior evals, and the
existing agent prompt/fallback optimizer. It is not a general autonomous
refactoring engine yet.

Today it supports:

- Rust-aware source analysis with `syn` and `tree-sitter-rust`.
- Process-based agent invocation with lifecycle traces.
- Prompt and AST-guarded fallback-behavior improvement strategies.
- Review-first scoped Rust hardening for normal modules through `improve`.
- Structured markdown policy parsing and policy-to-finding matches.
- Workspace behavior evals through `.mdx-rust/evals.json`.
- Optional behavior eval gates for `improve --eval-spec`.
- Repo doctor risk summaries with prioritized next actions.
- Bounded hardening transactions with all touched files snapshotted and rolled
  back on final validation failure.
- Isolated validation with `cargo check` and `cargo clippy -- -D warnings`.
- Net-positive scoring, final real-tree validation, and rollback on failure.
- Versioned audit packets for accepted optimizer changes and hardening reports
  for scoped module improvements.
- JSON Schema derivations for agent-facing records such as candidates, hooks,
  traces, eval datasets, audit packets, and validation command records.
- Human CLI output plus machine-parseable `--json` output.
- Deterministic static audit checks for risky agent surfaces.

Not yet supported:

- Arbitrary multi-file accepted edits outside the hardening transaction model.
- General autonomous refactoring.
- Stable library APIs.
- Coverage, mutation testing, or full semantic behavior proofs.
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

The hardening path for ordinary Rust modules is review-first by default:
`mdx-rust improve` validates candidate changes in an isolated workspace and
requires `--apply` before touching the real tree. In `v0.4`, passing
`--eval-spec` also requires the behavior commands in that spec to pass in the
isolated workspace and again after final application.

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

For an ordinary Rust crate or backend module:

```bash
cd your-rust-service
mdx-rust init
mdx-rust doctor
mdx-rust audit --policy policies/backend-safety.md
mdx-rust eval --spec .mdx-rust/evals.json
mdx-rust improve src/api/config.rs
mdx-rust improve src/api/config.rs --eval-spec .mdx-rust/evals.json --apply
```

Hardening artifacts are written under `.mdx-rust/hardening/`.

## Key Commands

```bash
mdx-rust init
mdx-rust register my-agent ./path/to/agent
mdx-rust doctor
mdx-rust spec my-agent
mdx-rust doctor my-agent
mdx-rust audit --policy policies/backend-safety.md
mdx-rust audit my-agent
mdx-rust improve src/lib.rs
mdx-rust improve src/lib.rs --eval-spec .mdx-rust/evals.json --apply
mdx-rust eval --spec .mdx-rust/evals.json
mdx-rust eval my-agent --dataset .mdx-rust/agents/my-agent/dataset.json
mdx-rust optimize my-agent --iterations 3 --budget medium --review
mdx-rust schema audit-packet --json
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
mdx-rust schema project-policy --json
```

Every command intended for automation supports `--json`.

## Audit Packets And Hardening Reports

Accepted changes produce versioned JSON audit packets in the experiment
directory. The optimizer `0.2` schema records:

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
`v0.4` hardening runs produce versioned JSON reports under
`.mdx-rust/hardening/` with findings, proposed changes, validation outcomes,
transaction status, rollback status, policy matches, behavior eval outcomes,
and workspace metadata.

Print the current JSON Schemas with:

```bash
mdx-rust schema audit-packet --json
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
```

## API Stability

`mdx-rust`, `mdx-rust-core`, and `mdx-rust-analysis` are all published so the
CLI can be installed from crates.io.

For `0.4.x`:

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

`v0.4.0` is in development as the first behavior and policy-driven hardening
release for ordinary Rust modules.

## License

MIT
