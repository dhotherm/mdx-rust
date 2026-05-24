# API Stability

`mdx-rust` is a CLI-first project.

## v1.0 beta Stability Contract

For `1.0.x`, the supported product surface is:

- The `mdx-rust` binary.
- Command names, flags, exit behavior, and documented `--json` outputs.
- The documented safety invariants.
- Versioned optimizer audit packet schemas.
- Versioned hardening report schemas.
- Versioned behavior eval report and project policy schemas.
- Versioned evidence run schemas.
- Versioned agent contract schemas.
- Versioned agent runtime manifest schemas.
- Versioned agent-pack schemas.
- Versioned recipe catalog schemas.
- Versioned artifact explanation schemas.
- Versioned evolution scorecard schemas.
- Versioned refactor plan schemas.
- Versioned refactor apply-run schemas.
- Versioned codebase map schemas.
- Versioned autopilot run schemas.

The published library crates are intentionally unstable:

- `mdx-rust-core`
- `mdx-rust-analysis`

They are published because crates.io requires versioned dependencies for the
CLI package, and because advanced users may want to inspect the implementation.
They are not yet a stable SDK.

## What May Change During The v1 Beta

The following may change in minor releases during the v1 beta:

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
- Files written under `.mdx-rust/evidence/`.
- Files written under `.mdx-rust/plans/`.
- `mdx-rust agent-contract --json`.
- `mdx-rust runtime --json`.
- `mdx-rust mcp --stdio`.
- `mdx-rust serve --bind 127.0.0.1:3799`.
- `mdx-rust agent-pack <agent> --json`.
- `mdx-rust recipes --json`.
- `mdx-rust explain <artifact> --json`.
- `mdx-rust scorecard <target> --json`.
- Versioned audit packet JSON.
- Versioned hardening report JSON.
- Versioned evidence run JSON.
- Versioned agent runtime manifest JSON.
- Versioned agent pack JSON.
- Versioned agent-ready report JSON.
- Versioned evolution scorecard JSON.
- Versioned refactor plan JSON.
- Versioned refactor apply-run JSON.

## Strategy Interfaces

Strategy traits and planners are not stable in `1.0.x`. The agent optimizer
still keeps accepted edits single-file. The hardening engine has a separate
bounded transaction path for scoped module hardening and requires validation,
optional behavior evals, and final validation before reporting applied success.

Refactor planning and autonomous orchestration records are stable enough for
CLI automation through `mdx-rust plan --json`, `mdx-rust apply-plan --json`,
`mdx-rust evidence --json`, `mdx-rust map --json`,
`mdx-rust autopilot --json`, `mdx-rust evolve --json`,
`mdx-rust agent-contract --json`, `mdx-rust runtime --json`,
`mdx-rust recipes --json`, `mdx-rust explain <artifact> --json`,
`mdx-rust scorecard --json`, `mdx-rust agent-pack <agent> --json`, and their
schema commands:

- `mdx-rust schema refactor-plan --json`
- `mdx-rust schema refactor-apply-run --json`
- `mdx-rust schema refactor-batch-apply-run --json`
- `mdx-rust schema evidence-run --json`
- `mdx-rust schema agent-contract --json`
- `mdx-rust schema agent-runtime-manifest --json`
- `mdx-rust schema agent-pack --json`
- `mdx-rust schema agent-ready-report --json`
- `mdx-rust schema recipe-catalog --json`
- `mdx-rust schema artifact-explanation --json`
- `mdx-rust schema evolution-scorecard --json`
- `mdx-rust schema codebase-map --json`
- `mdx-rust schema autopilot-run --json`

The Rust types and module paths that produce those records remain unstable
during the v1 beta.
