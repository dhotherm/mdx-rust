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
but `v0.7` is aimed at evidence-gated autonomous improvement loops for ordinary
Rust crates and service backends too.

The CLI is the supported product surface. The library crates are published for
installation and inspection, but their APIs remain unstable before `1.0`.

## Current Scope

`mdx-rust` is an early public beta. It is useful for experimentation and
dogfooding on Rust agent crates, and it can now run guarded autonomous passes
against ordinary Rust modules. In plain terms: `v0.7` can measure repo
evidence, tell you where a Rust repo looks strong or weak, plan refactor work,
execute the safe subset in multiple passes, replan after each applied pass, and
preserve audit evidence. It is not a broad semantic rewrite engine yet, but it
is now an autonomous Rust evolution loop for scoped, evidence-backed
improvements.

Today it supports:

- Rust-aware source analysis with `syn` and `tree-sitter-rust`.
- Process-based agent invocation with lifecycle traces.
- Prompt and AST-guarded fallback-behavior improvement strategies.
- Review-first scoped Rust hardening for normal modules through `improve`.
- Structured markdown policy parsing and policy-to-finding matches.
- Workspace behavior evals through `.mdx-rust/evals.json`.
- Optional behavior eval gates for `improve --eval-spec`.
- Repo doctor risk summaries with prioritized next actions.
- Measured evidence artifacts through `mdx-rust evidence`.
- Plan-first refactor impact analysis with public API, module edge, long
  function, large file, and patchable hardening candidates.
- `apply-plan` execution for approved low-risk refactor candidates, with stale
  source snapshot rejection and all real edits routed through hardening
  transactions.
- `apply-plan --all` execution queues for reviewing or applying every
  executable low-risk candidate in a saved plan, with per-step validation.
- `map` repo intelligence reports with debt score, quality grade, evidence
  grade, available gate detection, hardening findings, and next actions.
- `autopilot` multi-pass orchestration that maps, plans, applies the safe
  queue, replans after mutation, and persists an audit report.
- `evolve` budget-bounded autonomous improvement for agent callers.
- `agent-contract` machine-readable command guidance so external coding agents
  can discover safe commands, mutation requirements, schemas, and artifact
  locations before acting.
- Five executable Tier 1 mechanical recipes: contextual error hardening,
  boundary error context propagation, private borrow parameter tightening,
  iterator clone cleanup, and
  `#[must_use]` annotations for public value-returning functions.
- Two coverage-gated Tier 2 structural mechanical recipes: extracting repeated
  private string literals into file-local constants, and replacing zero-length
  checks with `is_empty()`.
- Hardened evidence analysis that surfaces deeper clone-pressure and long
  function review candidates, with lower structural planning thresholds than
  low-evidence targets.
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

## What v0.7 Adds

`v0.7` is the release where mdx-rust starts using measured evidence to decide
how aggressive it may be.

- Run `mdx-rust evidence` to persist measured test evidence under
  `.mdx-rust/evidence/`.
- Add `--include-coverage`, `--include-mutation`, and `--include-semver` when
  you want heavier proof signals to unlock deeper autonomy.
- Run `mdx-rust agent-contract --json` before handing control to another
  coding agent. It tells the agent which commands are read-only, which require
  `--apply`, which schemas to expect, and which artifacts to inspect.
- Run `mdx-rust map <target>` to get a repo quality profile, debt score,
  measured evidence reference, capability gates, findings, and next actions.
- Run `mdx-rust plan <target>` to produce a non-mutating refactor plan.
- Review impact before editing: public items, module edges, file size, long
  functions, policy references, behavior eval references, source snapshots, and
  candidate risk.
- Execute approved low-risk candidates with `mdx-rust apply-plan --candidate
  <id>`.
- Execute the whole safe queue with `mdx-rust apply-plan --all`.
- Run `mdx-rust autopilot <target>` to review an autonomous pass without
  mutating source files.
- Run `mdx-rust autopilot <target> --apply` to execute low-risk queued
  candidates, replan after each applied pass, and stop on any failed gate.
- Run `mdx-rust evolve <target> --budget 10m --tier 1 --apply` when a coding
  agent wants a direct "do safe work within this budget" command.
- Run `mdx-rust evolve <target> --budget 10m --tier 2 --min-evidence covered
  --apply` after measured coverage evidence is available.
- Get JSON artifacts for maps, plans, apply runs, and autopilot runs so humans
  and agents can audit what happened.

The aggressive part is that `autopilot --apply` and `evolve --apply` can run
several safe passes on their own. The disciplined part is that each pass creates
a fresh plan, executes only supported low-risk recipes allowed by the current
evidence grade and requested tier, and routes every real edit through freshness
checks, isolated validation, final validation, and the hardening transaction
path.

The executable Tier 1 recipe set is deliberately broader than panic cleanup:

- Replace panic-prone `unwrap`/`expect` inside `anyhow::Result` functions with
  contextual errors and `?`.
- Add `anyhow::Context` to fallible filesystem and environment boundary calls
  that already use `?`.
- Tighten private parameters from `&String` to `&str` and from `&Vec<T>` to
  `&[T]` when compile and clippy gates prove the change.
- Replace clone-mapping collection with a simpler validated form such as
  `to_vec()`.
- Add `#[must_use]` to public value-returning functions when the return type is
  not already a common must-use type.

The first executable Tier 2 recipes are intentionally narrow but real:

- Extract a repeated private string literal into a file-local constant only
  when measured evidence reaches `Covered`, the caller allows Tier 2, and the
  normal validation and rollback gates pass.
- Replace `len() == 0` with `is_empty()` under the same measured `Covered`
  evidence, explicit Tier 2 request, and validation gates.

Evidence also changes analysis depth, not just execution permission. A
`Hardened` or `Proven` evidence artifact unlocks deeper clone-pressure findings
and lower thresholds for long-function and split-module planning. Those higher
risk items still remain plan-first unless a dedicated executable recipe exists.

Not yet supported:

- Arbitrary multi-file accepted edits outside the hardening transaction model.
- Autonomous public API changes or broad semantic rewrites.
- Direct application of stale plans or plan-only/high-risk candidates.
- Stable library APIs.
- Full semantic behavior proofs or MIR-backed refactors.
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
requires `--apply` before touching the real tree. In `v0.7`, passing
`--eval-spec` also requires the behavior commands in that spec to pass in the
isolated workspace and again after final application.

The refactor path is plan-first by design. `mdx-rust plan` never edits files.
It writes a versioned plan artifact, classifies candidate risk, snapshots source
hashes, surfaces public API impact, and identifies which candidates are
executable. `mdx-rust apply-plan` can review or execute approved low-risk
candidates, but it rejects stale source snapshots and still routes real edits
through the existing hardening transaction gates.

For higher-leverage cleanup, `mdx-rust apply-plan --all` builds an execution
queue from the saved plan, de-duplicates executable candidates by file, checks
freshness before each step, and validates each applied step before continuing.

The autonomous path is a coordinator over the same primitives. `mdx-rust
autopilot` first writes a codebase map, then creates a plan, executes the safe
queue in review or apply mode, and replans before any later apply pass. Review
mode must not mutate the real tree. Apply mode stops on stale plans, rejected
steps, unsupported candidates, or exhausted executable work.

Evidence controls proportional aggression. A target with no Cargo metadata gets
`None` evidence and cannot run autonomous changes. A normal Cargo target starts
at `Compiled`, which unlocks Tier 1 mechanical recipes that still must pass
`cargo check` and clippy before landing. Tests or a behavior eval spec raise the
visible grade to `Tested`, switch the analysis depth to boundary-aware, and
surface extra plan-only review candidates for process execution, unsafe code,
environment access, filesystem boundaries, and HTTP surfaces. `mdx-rust
evidence` can persist measured test, coverage, mutation, and semver command
outcomes. When the latest evidence artifact reaches `Covered`, Tier 2
structural mechanical recipes can enter the executable queue if the caller also
requests Tier 2. When evidence reaches `Hardened` or `Proven`, mdx-rust also
searches for deeper refactor pressure that low-evidence targets do not surface.

## Agent-First Usage

`mdx-rust` treats external coding agents as first-class callers. Agents should
start by reading the command contract:

```bash
mdx-rust --json agent-contract
mdx-rust --json schema agent-contract
```

A safe agent workflow for a normal Rust backend looks like this:

```bash
mdx-rust --json evidence src/service --include-coverage
mdx-rust --json map src/service
mdx-rust --json plan src/service
mdx-rust --json evolve src/service --budget 10m --tier 2 --min-evidence covered
```

The final command is review mode by default. An agent should add `--apply` only
when the human asked for mutation. Every JSON response includes artifact paths
that the agent can inspect before recommending or continuing work.

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
mdx-rust evidence
mdx-rust evidence --include-coverage
mdx-rust map src/api
mdx-rust autopilot src/api
mdx-rust autopilot src/api --apply --max-passes 3 --max-candidates 10
mdx-rust evolve src/api --budget 10m --tier 1 --apply
mdx-rust evolve src/api --budget 10m --tier 2 --min-evidence covered --apply
mdx-rust improve src/api/config.rs
mdx-rust plan src/api
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --candidate <id>
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --candidate <id> --apply
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --all
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --all --apply
mdx-rust improve src/api/config.rs --eval-spec .mdx-rust/evals.json --apply
```

Hardening artifacts are written under `.mdx-rust/hardening/`. Refactor plan
artifacts are written under `.mdx-rust/plans/`. Codebase maps are written under
`.mdx-rust/maps/`. Autopilot reports are written under `.mdx-rust/autopilot/`.

Behavior eval specs execute local commands from your repository. Treat them as
trusted project code, review changes to them like test scripts, and prefer
deterministic commands such as `cargo test`, golden CLI checks, or service
contract smoke tests.

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
mdx-rust evidence
mdx-rust evidence --include-coverage
mdx-rust map src/lib.rs
mdx-rust plan src/lib.rs
mdx-rust plan src/api --policy policies/backend-safety.md --eval-spec .mdx-rust/evals.json
mdx-rust autopilot src/api --policy policies/backend-safety.md --eval-spec .mdx-rust/evals.json
mdx-rust autopilot src/api --policy policies/backend-safety.md --eval-spec .mdx-rust/evals.json --apply
mdx-rust evolve src/api --budget 10m --tier 1 --min-evidence compiled --apply
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --candidate plan-hardening-src-lib-rs-2
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --candidate plan-hardening-src-lib-rs-2 --apply
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --all --max-candidates 10
mdx-rust apply-plan .mdx-rust/plans/refactor-plan-...json --all --apply --max-candidates 10
mdx-rust improve src/lib.rs --eval-spec .mdx-rust/evals.json --apply
mdx-rust eval --spec .mdx-rust/evals.json
mdx-rust eval my-agent --dataset .mdx-rust/agents/my-agent/dataset.json
mdx-rust optimize my-agent --iterations 3 --budget medium --review
mdx-rust schema audit-packet --json
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
mdx-rust schema project-policy --json
mdx-rust schema evidence-run --json
mdx-rust schema refactor-plan --json
mdx-rust schema refactor-apply-run --json
mdx-rust schema refactor-batch-apply-run --json
mdx-rust schema codebase-map --json
mdx-rust schema autopilot-run --json
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
`v0.4` and later hardening runs produce versioned JSON reports under
`.mdx-rust/hardening/` with findings, proposed changes, validation outcomes,
transaction status, rollback status, policy matches, behavior eval outcomes,
and workspace metadata.

`v0.7` evidence runs are written under `.mdx-rust/evidence/` with command
records, timeout flags, stdout/stderr captures, evidence grade, analysis depth,
and unlocked recipe tiers.

`v0.7` refactor plans produce versioned JSON reports under `.mdx-rust/plans/`
with impact summaries, source snapshot hashes, public API pressure, module
edges, required gates, policy/eval references, and candidate actions. Plan
artifacts are evidence for review and orchestration; they are not proof that a
change has been applied. `apply-plan` reports are also written under
`.mdx-rust/plans/` and record whether a candidate or execution queue was
reviewed, applied, rejected, stale, partially applied, or unsupported.

`v0.7` codebase maps are written under `.mdx-rust/maps/` with quality grades,
debt scores, capability gates, findings, and recommended actions. Autopilot
runs are written under `.mdx-rust/autopilot/` with the quality before/after,
per-pass plan hashes, apply reports, skipped counts, and final status.

Print the current JSON Schemas with:

```bash
mdx-rust schema audit-packet --json
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
mdx-rust schema evidence-run --json
mdx-rust schema refactor-plan --json
mdx-rust schema refactor-apply-run --json
mdx-rust schema refactor-batch-apply-run --json
mdx-rust schema codebase-map --json
mdx-rust schema autopilot-run --json
```

## API Stability

`mdx-rust`, `mdx-rust-core`, and `mdx-rust-analysis` are all published so the
CLI can be installed from crates.io.

For `0.7.x`:

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

`v0.7.0` is in development as the first measured-evidence autonomy release. It
adds evidence artifacts, evidence-fed maps and plans, coverage-gated Tier 2
execution, and keeps broad semantic refactors behind explicit review and future
verification work.

## License

MIT
