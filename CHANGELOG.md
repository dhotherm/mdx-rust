# Changelog

All notable public changes to `mdx-rust` are documented here.

## 1.0.0 - 2026-05-24

v1 beta trust-contract release.

This release keeps the project in beta while establishing the v1 agent-facing
contract: stable CLI-first workflows, versioned 1.0 schemas for agent-facing
artifacts, stronger local runtime security, and a compact readiness command for
external coding agents.

### Added

- `mdx-rust agent-ready` for a compact non-mutating readiness report that
  external agents can call before deciding whether to run `evolve`.
- JSON Schema export for `agent-ready-report`.
- `mdx-rust serve --token <token>` and `MDX_RUST_RUNTIME_TOKEN` support for
  localhost HTTP bearer-token protection.
- Runtime manifest `protocol_version` and `http_auth` fields.
- Agent pack targets for Cursor, Aider, and Goose-style workflows in addition
  to Codex, Claude, and generic agents.
- Regression coverage for HTTP runtime auth and MCP malformed-input recovery.

### Changed

- Workspace package version is now `1.0.0`.
- Agent-facing map, plan, scorecard, hardening, evidence, autopilot, recipe,
  artifact explanation, agent contract, runtime, agent-pack, and agent-ready
  artifacts now use schema version `1.0`.
- The API stability docs now describe the v1 beta contract: the CLI and
  versioned JSON artifacts are the supported automation surface; library APIs
  remain unstable during beta.

### Known Limitations

- v1.0.0 is intentionally still labeled beta in the docs and positioning.
- Broad semantic refactors, public API rewrites, and MIR-backed analysis remain
  future work.
- Runtime HTTP auth is local bearer-token protection, not a remote
  multi-tenant service security model.

## 0.9.0 - 2026-05-24

Agent runtime and candidate-evidence release.

This release turns the v0.8 agent-first contract into callable local
infrastructure. External coding agents can now discover runtime transports,
call mdx-rust tools over a stdio protocol or localhost HTTP, generate local
agent instruction packs, and inspect candidate-level evidence status before
choosing whether to run autonomous evolution.

This is the next public minor release after `0.8.0` on crates.io.

### Added

- `mdx-rust runtime` for the typed local agent runtime manifest.
- `mdx-rust mcp --stdio` for line-delimited JSON tool calls over stdin/stdout.
- `mdx-rust serve --bind 127.0.0.1:3799` for a localhost-only HTTP runtime
  surface.
- `mdx-rust agent-pack codex|claude|generic` for generating agent instruction
  packs that teach external coding agents how to use mdx-rust safely.
- JSON Schema export for `agent-runtime-manifest` and `agent-pack`.
- Runtime manifest entries in `mdx-rust agent-contract`.
- Candidate evidence status on refactor candidates: unmeasured, compiled,
  tested, covered, mutation-backed, or proven.
- Coverage and mutation score fields on evidence file profiles, with coverage
  surfaced on function profiles when measured.
- A new executable Tier 2 recipe, `option-context-propagation`, which converts
  simple `Option::ok_or("message")?` boundaries inside `anyhow::Result`
  functions into `anyhow::Context` calls under covered evidence gates.
- Runtime tests proving MCP tool listing is machine-parseable and mutation is
  rejected unless explicitly confirmed.

### Changed

- Agent-facing map, plan, scorecard, hardening, evidence, autopilot, recipe,
  artifact explanation, agent contract, and runtime artifacts now use schema
  version `0.9`.
- The agent contract now advertises runtime, MCP, serve, and agent-pack
  workflows.
- Tier 2 recipes now include zero-length cleanup, repeated literal extraction,
  and Option boundary context propagation.
- Workspace package version is now `0.9.0`.

### Known Limitations

- The MCP and HTTP runtime surfaces are intentionally local and minimal. They
  are designed for local coding agents, not remote multi-tenant services.
- Runtime mutation still routes through the existing autopilot and hardening
  gates. There is no separate runtime mutation path.
- Tier 2 autonomy is real but narrow: the executable set is repeated private
  string literal extraction, zero-length cleanup, and simple Option boundary
  context propagation under measured `Covered` evidence.
- Candidate-level coverage is currently propagated from measured workspace
  evidence when per-function coverage mapping is not available.
- Broad semantic refactors, public API rewrites, and MIR-backed analysis remain
  future work.

## 0.8.0 - 2026-05-23

Agent-first evidence-driven Rust evolution.

This release makes mdx-rust easier for coding agents to drive safely and gives
evidence more teeth. Evidence runs now profile files and functions, plans carry
candidate evidence context, maps and plans include security posture, and the CLI
can explain saved artifacts, expose a typed recipe catalog, and produce one
agent-first evolution scorecard.

### Added

- File/function evidence profiles in `mdx-rust evidence` artifacts.
- Candidate evidence context on refactor plan candidates.
- Security posture summaries on maps and plans, including score and severity
  counts.
- `mdx-rust recipes` for a machine-readable recipe catalog with tier, required
  evidence, risk, execution status, and mutation path.
- `mdx-rust explain <artifact>` for agent-readable artifact summaries and safe
  next actions.
- `mdx-rust scorecard [target]` for a single read-only briefing artifact that
  combines map, plan, recipe catalog, autonomy readiness, and next commands.
- Per-candidate autonomy decisions: allowed, review-only, or blocked.
- Security audit findings as review-oriented plan candidates.
- JSON Schema export for `recipe-catalog`, `artifact-explanation`, and
  `evolution-scorecard`.
- Agent contract entries for `recipes`, `explain`, and `scorecard`.
- Tests proving recipe catalog, artifact explanation, evidence profiles, and
  scorecards remain machine-parseable.

### Changed

- Refactor plan, apply-plan, batch apply, codebase map, autopilot, hardening,
  evidence, recipe catalog, artifact explanation, and agent contract artifacts
  now use schema version `0.8`.
- Codebase quality scoring now includes security posture as an advisory
  prioritization signal.
- `apply-plan --all`, `autopilot`, and `evolve` now require candidate autonomy
  decisions to be `Allowed` before queueing a candidate.
- README, safety invariants, API stability docs, architecture docs, provenance
  docs, release readiness, and the local mdx-rust hardening skill were refreshed
  for the v0.8 agent-first contract.
- Workspace package version is now `0.8.0`.

### Known Limitations

- Security posture influences prioritization but does not weaken validation,
  evidence, stale-plan, rollback, or behavior eval gates.
- File/function evidence profiles are intentionally conservative and mostly
  syntactic until deeper coverage mapping lands.
- Broad semantic refactors, public API rewrites, and MIR-backed analysis remain
  future work.
- The Rust library APIs remain unstable before `1.0`; automate through CLI JSON
  and versioned artifacts.

## 0.7.0 - 2026-05-23

Measured evidence-gated autonomy for deeper Rust evolution.

This release makes evidence a real artifact instead of an inferred hint.
`mdx-rust` can now collect bounded command evidence, feed that evidence into
maps and plans, change analysis depth, and unlock narrow Tier 2 executable
refactors when a target has measured coverage evidence.

### Added

- `mdx-rust evidence [target]` for persisted evidence runs under
  `.mdx-rust/evidence/`.
- Evidence command records with status code, timeout flag, duration, stdout,
  stderr, skipped state, and skip reason.
- Evidence metrics for parsed coverage and mutation percentages when tool
  output exposes them.
- Optional evidence collection flags for coverage, mutation testing, and semver
  checks when `cargo-llvm-cov`, `cargo-mutants`, and `cargo-semver-checks` are
  installed.
- Measured evidence references on codebase maps, refactor plans, and autopilot
  runs.
- JSON Schema export for `evidence-run`.
- `mdx-rust agent-contract` and JSON Schema export for `agent-contract`, giving
  external coding agents a machine-readable command, mutation, schema, and
  artifact contract.
- Coverage-gated Tier 2 execution for supported structural mechanical recipes.
- First executable Tier 2 recipe: repeated private string literal extraction
  into a file-local constant.
- Second executable Tier 2 recipe: `len() == 0` to `is_empty()` cleanup under
  the same measured evidence and validation gates.
- Hardened evidence analysis that surfaces clone-pressure review and
  long-function review candidates that lower-evidence targets do not see.
- `mdx-rust improve --tier 2` for directly reviewing or applying supported Tier
  2 hardening recipes.
- Tests proving Tier 2 recipes stay disabled at Tier 1 and become executable
  only when measured `Covered` evidence is present.
- `just evidence-smoke` for the measured evidence and Tier 2 queue path.

### Changed

- Refactor plan, apply-plan, batch apply, codebase map, autopilot, and hardening
  artifacts now use schema version `0.7`.
- `map`, `plan`, `autopilot`, and `evolve` consume the latest evidence artifact
  when available.
- Hardened or Proven evidence lowers structural planning thresholds for
  long-function and split-module candidates while keeping them plan-first.
- Autonomy language in README, safety invariants, provenance docs, architecture
  docs, release readiness, and roadmap now describes measured evidence as the
  proportional aggression gate.
- Workspace package version is now `0.7.0`.

### Known Limitations

- Tier 2 execution is intentionally narrow and still routes through the
  hardening transaction path.
- Mutation and semver checks are collected only when explicitly requested.
- Broad semantic refactors, public API rewrites, and MIR-backed analysis remain
  future work.
- The Rust library APIs remain unstable before `1.0`; automate through CLI JSON
  and versioned artifacts.

## 0.6.0 - 2026-05-23

Autonomous Rust evolution for scoped low-risk improvements.

This release turns the v0.5 plan-first workflow into a multi-pass autonomous
loop. `mdx-rust` can now map a Rust repo, identify safe executable work, run
review-mode autonomous passes, apply the low-risk queue, and replan before
continuing.

### Added

- `mdx-rust map [target]` for non-mutating codebase intelligence reports with
  quality grade, debt score, evidence grade, capability gates, findings, and
  next actions.
- `mdx-rust autopilot [target]` for autonomous review passes that map, plan,
  queue executable candidates, and preserve source files.
- `mdx-rust autopilot [target] --apply` for multi-pass autonomous apply. Each
  pass builds a fresh plan, executes only supported low-risk candidates through
  `apply-plan --all`, and stops on any failed gate.
- `mdx-rust evolve [target] --budget <time> --tier <n>` as the agent-friendly
  autonomous improvement entrypoint.
- Versioned codebase map artifacts under `.mdx-rust/maps/`.
- Versioned autopilot artifacts under `.mdx-rust/autopilot/`.
- Evidence summaries on codebase maps, refactor plans, and autopilot runs,
  including grade, analysis depth, signals, unlocked recipe tiers, unlock
  suggestions, and max autonomous tier.
- Candidate tier and required-evidence fields, enforced before queue execution.
- Tier 1 mechanical recipes for boundary error context propagation, private
  borrow parameter tightening, iterator clone collection cleanup, and
  `#[must_use]` annotations for public value-returning functions.
- Boundary-aware plan-only review candidates for process execution, unsafe
  code, environment access, filesystem boundaries, and HTTP surfaces when a
  target reaches `Tested` evidence.
- Autopilot execution summaries with plans created, executable candidates seen,
  validated transactions, applied transactions, blocked candidates, evidence
  grade, and analysis depth.
- JSON Schema export for `codebase-map` and `autopilot-run`.
- Capability gate detection for `cargo-nextest`, `cargo-llvm-cov`,
  `cargo-mutants`, and `cargo-semver-checks`.
- CLI integration tests proving map review is non-mutating and autopilot review
  and apply modes remain machine-parseable and safety-gated.

### Changed

- Refactor plan, apply-plan, and batch apply artifacts now use schema version
  `0.6`.
- README, safety invariants, provenance docs, API stability docs, architecture
  docs, and release readiness docs now describe autonomous orchestration.
- Execution queues now filter by evidence grade, requested tier, public API
  allowance, and candidate support status.
- Release smoke coverage includes map and autopilot schema checks.
- Workspace package version is now `0.6.0`.

### Known Limitations

- Autopilot executes only supported low-risk Tier 1 recipes. It does not
  autonomously change public APIs or apply broad semantic rewrites.
- Larger candidates such as extracting functions and splitting modules remain
  plan-only until stronger semantic analysis and transaction design land.
- Optional coverage, mutation testing, and semver tools are detected as
  capability gates but are not executed automatically.
- The Rust library APIs remain unstable before `1.0`; automate through CLI JSON
  and versioned artifacts.

## 0.5.0 - 2026-05-23

Plan-first guardrailed refactoring for Rust crates and service modules.

This release moves mdx-rust from scoped hardening into an executable
plan-first refactoring workflow. The new plan path is intentionally more
ambitious than previous releases, but executable candidates remain limited to
low-risk work that can pass through existing hardening transactions.

### Added

- `mdx-rust plan [target]` for refactor impact analysis without mutating the
  workspace.
- Versioned refactor plan artifacts under `.mdx-rust/plans/`.
- `mdx-rust apply-plan <plan> --candidate <id>` for reviewing or applying
  executable low-risk plan candidates.
- `mdx-rust apply-plan <plan> --all` for reviewing or applying an execution
  queue of every executable low-risk candidate in the plan.
- Refactor impact summaries for scanned files, module edges, public API
  pressure, long functions, large files, and patchable hardening candidates.
- Source snapshot hashes, plan hashes, and candidate hashes for stale-plan
  rejection.
- Plan candidates that route safe patchable work back through
  `mdx-rust improve --apply` instead of creating a second mutation path.
- JSON Schema export for `refactor-plan`, `refactor-apply-run`, and
  `refactor-batch-apply-run`.
- Refactor analysis in `mdx-rust-analysis` for public items, module edges, file
  size, function count, test presence, and largest function size.
- CLI integration tests proving `plan --json` is machine parseable, does not
  mutate source files, can execute one approved plan candidate or a full
  executable queue, and rejects stale plans.

### Changed

- `README.md`, `SAFETY_INVARIANTS.md`, and architecture docs now describe the
  v0.5 plan-first refactor contract.
- Release smoke coverage includes refactor plan schema and plan generation.
- Crate-level stability language now targets `0.5.x`.

### Known Limitations

- Refactor plans are review and orchestration artifacts only. They do not apply
  broad refactors.
- Plan candidates are intentionally conservative and heuristic.
- Patchable candidates still use the existing hardening engine and transaction
  safety gates.
- `apply-plan` only executes supported low-risk candidates in v0.5; plan-only
  structural candidates remain review artifacts.
- `apply-plan --all` de-duplicates executable candidates by file because the
  current hardening transaction applies all patchable findings in that file.
- Public API impact detection is source-level analysis, not a semver proof.

## 0.4.0 - 2026-05-23

Behavior and policy-driven hardening for Rust services and ordinary crates.

### Added

- Structured project policy parsing for markdown rules in `.mdx-rust/policies.md`
  or a supplied `--policy` file.
- Policy-to-finding matches in hardening reports so reviewers can see which
  rule explains a finding.
- Workspace behavior eval specs through `.mdx-rust/evals.json` and
  `mdx-rust eval --json`.
- Optional `mdx-rust improve --eval-spec <path>` gating. Proposed hardening
  changes must pass behavior evals in isolation before review/apply can
  succeed, and applied changes must pass final behavior evals before success.
- JSON Schema export for `behavior-eval-report` and `project-policy`.
- Repo doctor risk summaries with high, medium, patchable, and recommended
  next-action counts.
- Additional advisory findings for filesystem and HTTP/route boundaries.

### Changed

- Hardening report schema version is now `0.4`.
- `mdx-rust init` writes a starter `.mdx-rust/evals.json` behavior eval spec.
- `mdx-rust eval` can now run workspace behavior evals when no agent name is
  supplied.

### Known Limitations

- Behavior eval specs run deterministic commands; they are not a coverage or
  mutation-testing system yet.
- Policy parsing is intentionally simple markdown extraction, not a full policy
  language.
- General autonomous refactoring remains out of scope.

## 0.3.0 - 2026-05-23

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
