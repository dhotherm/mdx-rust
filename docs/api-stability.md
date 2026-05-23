# API Stability

`mdx-rust` is a CLI-first project.

## v0.2 Stability Contract

For `0.2.x`, the supported product surface is:

- The `mdx-rust` binary.
- Command names, flags, exit behavior, and documented `--json` outputs.
- The documented safety invariants and audit packet schema.

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
- Versioned audit packet JSON.

## Strategy Interfaces

Strategy traits and planners are not stable in `0.2.x`. The current release
keeps accepted edits single-file and forces every candidate through the same
safety pipeline.
