# Changelog

All notable public changes to `mdx-rust` are documented here.

## 0.6.0 - 2026-05-23

Autonomous Rust evolution for scoped low-risk improvements.

This release turns the v0.5 plan-first workflow into a multi-pass autonomous
loop. `mdx-rust` can now map a Rust repo, identify safe executable work, run
review-mode autonomous passes, apply the low-risk queue, and replan before
continuing.

### Added

- `mdx-rust map [target]` for non-mutating codebase intelligence reports with
  quality grade, debt score, capability gates, findings, and next actions.
- `mdx-rust autopilot [target]` for autonomous review passes that map, plan,
  queue executable candidates, and preserve source files.
- `mdx-rust autopilot [target] --apply` for multi-pass autonomous apply. Each
  pass builds a fresh plan, executes only supported low-risk candidates through
  `apply-plan --all`, and stops on any failed gate.
- Versioned codebase map artifacts under `.mdx-rust/maps/`.
- Versioned autopilot artifacts under `.mdx-rust/autopilot/`.
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
- Release smoke coverage includes map and autopilot schema checks.
- Workspace package version is now `0.6.0`.

### Known Limitations

- Autopilot executes only supported low-risk recipes. It does not autonomously
  change public APIs or apply broad semantic rewrites.
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
