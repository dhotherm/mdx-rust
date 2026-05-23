# API Stability

`mdx-rust` is a CLI-first project.

## v0.5 Stability Contract

For `0.5.x`, the supported product surface is:

- The `mdx-rust` binary.
- Command names, flags, exit behavior, and documented `--json` outputs.
- The documented safety invariants.
- Versioned optimizer audit packet schemas.
- Versioned hardening report schemas.
- Versioned behavior eval report and project policy schemas.
- Versioned refactor plan schemas.

The published library crates are intentionally unstable:

- `mdx-rust-core`
- `mdx-rust-analysis`

They are published because crates.io requires versioned dependencies for the
CLI package, and because advanced users may want to inspect the implementation.
They are not yet a stable SDK.

## What May Change Before 1.0

The following may change in minor releases before `1.0`:

- Module paths.
- Public structs and enum variants.
- Strategy names and planner internals.
- Hook internals.
- Registry internals.
- Ledger shape, except for explicitly versioned audit packets.

## What Should Be Treated As Stable Enough To Automate

Automation should prefer:

- CLI commands.
- `--json` output.
- Files written under `.mdx-rust/agents/<name>/experiments/`.
- Files written under `.mdx-rust/hardening/`.
- Files written under `.mdx-rust/plans/`.
- Versioned audit packet JSON.
- Versioned hardening report JSON.
- Versioned refactor plan JSON.

## Strategy Interfaces

Strategy traits and planners are not stable in `0.5.x`. The agent optimizer
still keeps accepted edits single-file. The hardening engine has a separate
bounded transaction path for scoped module hardening and requires validation,
optional behavior evals, and final validation before reporting applied success.

Refactor planning records are stable enough for CLI automation through
`mdx-rust plan --json` and `mdx-rust schema refactor-plan --json`. The Rust
types and module paths that produce those records remain unstable before `1.0`.
