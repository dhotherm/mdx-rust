# mdx-rust

[![CI](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml/badge.svg)](https://github.com/dhotherm/mdx-rust/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/mdx-rust.svg)](https://crates.io/crates/mdx-rust)
[![Docs.rs](https://docs.rs/mdx-rust/badge.svg)](https://docs.rs/mdx-rust)
[![License: MIT](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)
[![Rust 1.80+](https://img.shields.io/badge/rust-1.80%2B-orange.svg)](https://www.rust-lang.org/)

**A Rust-native safe-change system for codebases.**

`mdx-rust` points at Rust code, finds scoped hardening opportunities, validates
changes in isolation, checks project policy and behavior evals when supplied,
and only lands edits that pass Rust gates. It still supports agent
optimization, but `v1.0 beta` is aimed at evidence-gated autonomous improvement
loops for ordinary Rust crates and service backends too.

The CLI is the supported product surface. The library crates are published for
installation and inspection, but their APIs remain unstable during the v1 beta.

## Current Scope

`mdx-rust` is an early public beta. It is useful for experimentation and
dogfooding on Rust agent crates, and it can now run guarded autonomous passes
against ordinary Rust modules. In plain terms: `v1.0 beta` can measure repo
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
- `runtime`, `mcp --stdio`, and bearer-token-capable `serve` local runtime
  surfaces so external coding agents can call mdx-rust without scraping human
  output.
- `agent-ready` compact readiness reports for coding agents that need a single
  go/no-go briefing before autonomous work.
- `agent-pack` generation for Codex, Claude, Cursor, Aider, Goose-style, and
  generic coding agent instruction files.
- `repo-map` and `noise-filter` orientation artifacts so agents can load the
  right context and avoid generated/build noise before planning.
- `contracts` read-only scans for documented preconditions, postconditions,
  invariants, safety notes, panic docs, and assertion hints.
- `perf` read-only scans for static performance pressure such as blocking work
  in async functions, clone pressure, and allocations in loops.
- `benchmark` command-based measurement runs that persist throughput, latency,
  wall-clock, stdout/stderr, timeout, and metric-summary evidence.
- `recipes` machine-readable recipe catalog with tier, evidence, execution, and
  mutation-path contracts.
- `explain` artifact summaries so coding agents can inspect saved JSON reports
  and choose safe next actions.
- `scorecard` agent briefings that combine map, plan, recipes, autonomy
  readiness, and next commands into one artifact.
- `brief` fused agent intake artifacts that combine repo context, noise
  filters, contracts, performance posture, scorecard, and recommended sequence.
- File/function evidence profiles that attach evidence context to plan
  candidates instead of relying only on repo-level grades.
- Per-candidate autonomy decisions that explain whether a candidate is allowed,
  blocked, or review-only.
- Security posture summaries in maps and plans, with high/medium/low finding
  counts and a security score that affects prioritization.
- Five executable Tier 1 mechanical recipes: contextual error hardening,
  boundary error context propagation, private borrow parameter tightening,
  iterator clone cleanup, and
  `#[must_use]` annotations for public value-returning functions.
- Three coverage-gated Tier 2 structural mechanical recipes: extracting
  repeated private string literals into file-local constants, replacing
  zero-length checks with `is_empty()`, and converting simple Option
  boundaries to `anyhow::Context`.
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

## What v1.0 beta Adds

`v1.0 beta` turns the v0.9 runtime into a firmer agent contract. The package is
versioned as `1.0.0`, but the project remains explicitly beta while the CLI and
JSON artifacts get real-world use.

Evidence is no longer only a repo-level grade: it profiles files and functions,
plans carry candidate evidence status, maps and plans include security posture,
and agents can call the CLI through JSON, stdio, or localhost HTTP runtime
surfaces.

- Run `mdx-rust evidence` to persist measured test evidence under
  `.mdx-rust/evidence/`.
- Add `--include-coverage`, `--include-mutation`, and `--include-semver` when
  you want heavier proof signals to unlock deeper autonomy.
- Run `mdx-rust agent-contract --json` before handing control to another
  coding agent. It tells the agent which commands are read-only, which require
  `--apply`, which schemas to expect, and which artifacts to inspect.
- Run `mdx-rust runtime --json` to inspect the local agent runtime manifest.
- Run `mdx-rust agent-ready <target> --json` when an agent needs a compact
  readiness report before deciding whether to continue.
- Run `mdx-rust mcp --stdio` when a coding agent wants a local stdio tool
  protocol.
- Run `mdx-rust serve --bind 127.0.0.1:3799 --token <token>` when a coding
  agent wants a localhost HTTP runtime with bearer-token protection.
- Run `mdx-rust agent-pack codex --write`, `mdx-rust agent-pack claude
  --write`, or `mdx-rust agent-pack cursor --write` to add repo-local
  instructions for an external agent.
- Run `mdx-rust recipes --json` to inspect every recipe, required evidence
  grade, tier, execution status, risk level, and mutation path.
- Run `mdx-rust scorecard <target> --json` to get one agent briefing with map,
  plan, recipe catalog, autonomy readiness, and suggested next commands.
- Run `mdx-rust explain <artifact> --json` to summarize evidence, plan, map,
  hardening, apply, or autopilot artifacts and get safe next actions.
- Run `mdx-rust map <target>` to get a repo quality profile, debt score,
  security score, measured evidence reference, capability gates, findings, and
  next actions.
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
- Convert simple `Option::ok_or("message")?` boundaries inside
  `anyhow::Result` functions to `anyhow::Context` under the same covered
  evidence and validation gates.

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

## v1.1 beta Direction

The next beta phase is about real-repo adoption and agent context engineering.
Before an external agent plans or evolves a target, it should be able to answer
"where am I, what should I ignore, and which instructions matter here?"

- Run `mdx-rust repo-map <target> --json` to get key files, instruction files,
  directory roles, crate boundaries, default noise filters, and safe intake
  steps for an agent.
- Run `mdx-rust noise-filter --json` to get default exclusions for generated
  and build artifacts such as `target/`, `.git/`, coverage output, and
  `.mdx-rust/` reports.
- Run `mdx-rust noise-filter --write` to create `.mdx-rust/agent-pack/`
  guidance files that agents can load before searching a large repo.
- Agent packs now describe a context cascade: root instructions, local package
  docs, target source, and only then generated mdx-rust artifacts referenced by
  `artifact_path`.

These surfaces are read-only orientation aids. They do not approve mutation,
weaken evidence gates, or replace scorecards, plans, validation, rollback, or
human approval for `--apply`.

## v1.2 beta Direction

The following beta slice brings lightweight spec-driven development into the
agent workflow. Instead of trusting an LLM to infer intended behavior from code
alone, `mdx-rust contracts` makes design intent visible and machine-readable.

- Run `mdx-rust contracts <target> --json` to scan Rust functions for
  `Requires:`, `Ensures:`, `Invariant:`, `Safety:`, and `Panics:` docs plus
  nearby assertion hints.
- Public functions with no visible contract docs or assertion hints are reported
  as recommendations, not automatic rewrites.
- Contract scans are design evidence. They can guide tests, reviews, policies,
  and future property-test work, but they do not replace validation or approve
  mutation.

## v1.3 beta Direction

The performance lane starts with static signals rather than automatic
micro-optimizing. `mdx-rust perf` gives agents a prioritized map of likely
performance pressure before they propose risky rewrites.

- Run `mdx-rust perf <target> --json` to find blocking operations inside async
  functions, clone pressure, allocations in loops, and synchronous lock hints.
- Findings are prioritization evidence only. They should guide benchmark work,
  evidence collection, and plan-first refactors.
- Performance findings do not make any candidate executable by themselves.
  Mutation still requires the normal plan, evidence, validation, behavior eval,
  and rollback gates.

## v1.4 beta Direction

The fusion phase makes the standalone intake surfaces work together.

- `mdx-rust map` and `mdx-rust plan` now carry contract and performance posture
  summaries alongside evidence, quality, security, and autonomy.
- Contract gaps and performance pressure become plan candidates, but remain
  review-only until a dedicated executable recipe and validation contract
  exists.
- `mdx-rust brief <target> --json` produces one agent-ready artifact with repo
  map, noise filters, contracts, performance, scorecard, and a recommended
  command sequence.
- `agent-ready` includes contract and performance readiness so external agents
  can explain why a repo is or is not ready for deeper autonomy.

## v1.5 beta Direction

The measurement phase gives performance work a real evidence artifact.

- `mdx-rust benchmark --spec .mdx-rust/benchmarks.json --json` runs a
  versioned command spec and persists `.mdx-rust/benchmarks/*.json`.
- Benchmark records include warmups, measured runs, command status, timeouts,
  truncated stdout/stderr, parsed throughput/latency metrics, and summary
  statistics.
- `agent-contract`, `runtime`, `schema`, `explain`, `scorecard`, and `brief`
  point agents toward benchmarks before performance-oriented refactors.
- Benchmarks are measured evidence only. They do not approve mutation, and
  performance changes still need the normal plan, validation, behavior eval,
  provenance, and rollback gates.

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
requires `--apply` before touching the real tree. In `v1.0 beta`, passing
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

Evidence is intentionally honest in the v1 beta. The strongest autonomy signal
comes from commands the project actually runs or parses: `cargo test`,
`cargo-llvm-cov`, `cargo-mutants`, `cargo-semver-checks`, and behavior eval
specs when supplied. If those tools are missing, mdx-rust does not pretend the
repo is safer than it is; it lowers the grade, records the missing capability,
and keeps deeper recipes blocked or review-only.

## Agent-First Usage

`mdx-rust` treats external coding agents as first-class callers. Agents should
start by reading the command contract:

```bash
mdx-rust --json agent-contract
mdx-rust --json runtime
mdx-rust --json schema agent-contract
```

A safe agent workflow for a normal Rust backend looks like this:

```bash
mdx-rust --json repo-map src/service
mdx-rust --json noise-filter
mdx-rust --json contracts src/service
mdx-rust --json perf src/service
mdx-rust --json evidence src/service --include-coverage
mdx-rust --json map src/service
mdx-rust --json plan src/service
mdx-rust --json evolve src/service --budget 10m --tier 2 --min-evidence covered
```

The final command is review mode by default. An agent should add `--apply` only
when the human asked for mutation. Every JSON response includes artifact paths
that the agent can inspect before recommending or continuing work.

Agents that prefer a tool runtime can use:

```bash
mdx-rust mcp --stdio
mdx-rust serve --bind 127.0.0.1:3799 --token <token>
```

Runtime mutation is not a shortcut. `evolve` calls with `apply=true` require
explicit mutation confirmation and still route through the same evidence,
freshness, validation, behavior eval, and rollback gates.

The HTTP runtime is for local developer machines and local agent processes. It
binds only to `127.0.0.1` or `localhost`, can require a bearer token, and is not
a remote multi-tenant service with rate limiting or internet-facing abuse
protection.

For runtime callers, the safe integration pattern is:

1. Discover: call `agent-contract`, `runtime`, `repo-map`, `noise-filter`,
   `contracts`, `perf`, and `recipes`.
2. Measure: call `evidence` for the target, adding coverage or mutation flags
   only when those tools are installed and the budget allows it.
3. Brief: call `scorecard` or `map` to understand quality, security, evidence,
   and capability gates.
4. Plan: call `plan` to inspect executable, review-only, and blocked
   candidates.
5. Review: explain the plan and artifact paths to the human.
6. Mutate only after approval: call CLI `evolve --apply`, or runtime `evolve`
   with both `apply=true` and `confirm_mutation=true`.

The concrete Tier 2 behavior in v1.0 beta is intentionally visible. On a target
with a measured `Covered` evidence artifact, this review-mode command shows the
supported queue without touching source files:

```bash
mdx-rust --json evidence src/service --include-coverage
mdx-rust --json evolve src/service --budget 10m --tier 2 --min-evidence covered
```

With `--apply` and human approval, Tier 2 can queue and validate repeated
private string literal extraction, `len() == 0` to `is_empty()`, and simple
`Option::ok_or("message")?` to `anyhow::Context`. On a `Compiled` or `Tested`
target, those same candidates remain blocked or review-only. Higher evidence
changes what the analyzer looks for, but it never bypasses validation.

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
mdx-rust runtime --json
mdx-rust mcp --stdio
mdx-rust serve --bind 127.0.0.1:3799 --token local-dev-token
mdx-rust agent-pack codex --write
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
mdx-rust schema agent-runtime-manifest --json
mdx-rust schema agent-pack --json
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

`v1.0 beta` evidence runs are written under `.mdx-rust/evidence/` with command
records, timeout flags, stdout/stderr captures, evidence grade, analysis depth,
file/function profiles, and unlocked recipe tiers.

`v1.0 beta` refactor plans produce versioned JSON reports under `.mdx-rust/plans/`
with impact summaries, source snapshot hashes, public API pressure, module
edges, security posture, required gates, policy/eval references, candidate
evidence context, per-candidate autonomy decisions, and candidate actions. Plan
artifacts are evidence for review and orchestration; they are not proof that a
change has been applied. `apply-plan` reports are also written under
`.mdx-rust/plans/` and record whether a candidate or execution queue was
reviewed, applied, rejected, stale, partially applied, or unsupported.

`v1.0 beta` codebase maps are written under `.mdx-rust/maps/` with quality grades,
debt scores, security posture, capability gates, findings, and recommended
actions. Autopilot runs are written under `.mdx-rust/autopilot/` with the
quality before/after, per-pass plan hashes, apply reports, skipped counts, and
final status.

Print the current JSON Schemas with:

```bash
mdx-rust schema audit-packet --json
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
mdx-rust schema evidence-run --json
mdx-rust schema recipe-catalog --json
mdx-rust schema artifact-explanation --json
mdx-rust schema evolution-scorecard --json
mdx-rust schema refactor-plan --json
mdx-rust schema refactor-apply-run --json
mdx-rust schema refactor-batch-apply-run --json
mdx-rust schema codebase-map --json
mdx-rust schema autopilot-run --json
```

## API Stability

`mdx-rust`, `mdx-rust-core`, and `mdx-rust-analysis` are all published so the
CLI can be installed from crates.io.

For `1.0.x`:

- The `mdx-rust` CLI is supported.
- The `mdx-rust-core` and `mdx-rust-analysis` APIs are unstable.
- Public library types may change during the v1 beta.
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

`v1.0 beta` is the current evidence-driven, agent-first evolution target. It adds
local runtime surfaces, agent-pack generation, candidate evidence status,
recipe catalog export, artifact explanations, scorecards, security posture in
maps/plans, and a stronger covered Tier 2 recipe set while keeping broad
semantic refactors behind explicit review and future verification work.

The next beta phase is focused on real-repo adoption: lower-noise readiness
signals, clearer next commands, better external-agent guidance, and scanner
precision before broader autonomous refactoring scope.

## License

MIT
