# Architecture

`mdx-rust` is organized around a small number of deliberately boring stages.

## Crates

- `mdx-rust`: CLI entrypoint and human or JSON command output.
- `mdx-rust-core`: registry, runner, optimizer, hooks, ledgers, scoring, audit
  packets, hardening reports, codebase maps, refactor plans, autopilot runs,
  and the candidate safety pipeline.
- `mdx-rust-analysis`: Rust source discovery, prompt/tool finders, isolated
  workspace creation, patch application, hardening analysis, refactor impact
  analysis, validation, and rollback snapshots.

## Optimization Lifecycle

1. Load the registered agent.
2. Analyze Rust source for prompts, tools, entrypoints, and editable scope.
3. Run the agent on the train split.
4. Diagnose failures from traces and scores.
5. Build typed candidate strategies.
6. Convert a candidate into a single-file `ProposedEdit`.
7. Execute the safety pipeline.
8. Persist experiment JSON, reports, and audit packets.

## Safety Pipeline Ownership

`crates/mdx-rust-core/src/safety_pipeline.rs` owns the acceptance-critical path.
The optimizer may choose candidates, but it does not get to bypass:

- Pre-edit hooks.
- Pre-command hooks.
- Isolated validation.
- Patched scoring.
- Positive delta requirement.
- Pre-accept hooks.
- Real-tree landing.
- Final validation.
- Rollback on failure.

Inside the pipeline, stage-specific records separate proposed, scoped,
isolated-validated, and net-positive edits. This keeps the code honest: a
candidate cannot be handled as accept-ready until it has passed the earlier
stages.

The first native structural edit path is intentionally small: fallback response
rewrites can operate over parsed Rust syntax, mutate matching string literals
in the AST, unparse the file, and parse the result again before the safety
pipeline validates it in isolation. Macro-format fallback rewrites remain
parser-guarded. Larger structural edits remain future work.

Rejected candidates carry typed rejection reasons. The human `notes` field is
for readability only and should not be used as the source of truth by
automation.

## Schema Boundaries

Types that cross CLI JSON, audit packet, hook, trace, strategy, eval, or future
LLM boundaries derive `schemars::JsonSchema` alongside serde traits. This keeps
agent-facing contracts inspectable and gives future MCP or hook integrations a
stable validation target without making the Rust library API stable before
`1.0`.

## Hardening Lifecycle

The hardening engine is a separate path for ordinary Rust modules:

1. Discover the Rust workspace with `cargo metadata` when available.
2. Scan the requested target or workspace for high-confidence hardening
   findings.
3. Build bounded file-content changes for supported strategies.
4. Apply the changes in an isolated workspace.
5. Run `cargo check` and `cargo clippy -- -D warnings`.
6. In review mode, stop here and write a hardening report.
7. With `--apply`, snapshot every touched real file.
8. Write the already validated changes to the real tree.
9. Run final validation on the real tree.
10. Roll back every touched file if final validation fails.

The hardening path is intentionally review-first and scoped. It does not use
the agent optimizer scoring loop, and it does not relax the optimizer's
single-file acceptance contract.

## v0.2 Optimizer Edit Scope

`v0.2` hard-enforces single-file accepted edits. A diff that touches a file other
than `ProposedEdit.file` is rejected before validation.

## v0.4 Policy And Behavior Gates

`v0.4` keeps bounded hardening transactions outside the optimizer and adds two
evidence layers:

- Structured markdown policy rules are parsed into categorized records and
  matched back to findings for reviewer context.
- Optional behavior eval specs run deterministic commands after isolated
  validation and again after final validation when `--apply` is used.

Behavior eval commands are trusted local project commands. They are intentionally
simple and auditable: teams can wire in `cargo test`, golden CLI checks, service
contract smoke tests, or their own scripts, but should review eval specs with
the same care as CI scripts.

Hardening transactions snapshot every touched file, validate in isolation,
require `--apply` before landing, run final validation, run final behavior evals
when supplied, and roll back on failure. General multi-file refactoring remains
future work.

## v0.5 Refactor Planning

`v0.5` adds a plan-first refactoring path and a narrow execution pathway for
approved low-risk candidates. It does not introduce broad autonomous
refactoring.

1. Scan the requested file, directory, or workspace.
2. Summarize file size, function count, largest function size, test presence,
   public items, and module or use edges.
3. Reuse hardening analysis to identify patchable high-confidence candidates.
4. Build a risk summary and candidate list.
5. Snapshot source file hashes and persist a versioned plan under
   `.mdx-rust/plans/`.
6. Print human output or emit the same plan as JSON.

`mdx-rust apply-plan` reads a saved plan, verifies source snapshot hashes, finds
the requested candidate, and either reviews or applies it. Executable v0.5
candidates are deliberately narrow: contextual error hardening candidates are
routed through the existing hardening engine. That keeps real edits,
validation, optional behavior eval gates, final validation, and rollback in one
place.

`mdx-rust apply-plan --all` builds a bounded execution queue from the same saved
plan. The queue includes only executable low-risk candidates, de-duplicates by
file because the hardening transaction applies all patchable findings in a
target file, checks the target file snapshot before each step, and stops apply
mode if a step fails. This pulls the refactoring workflow closer to batch
execution without creating a second mutation engine.

Plan-only candidates such as extracting a function, splitting a module, or
reviewing public API pressure are still human-reviewed design work in `v0.5`.
Public API-impacting candidates require explicit allowance before execution.

## v0.6 Autonomous Evolution

`v0.6` adds two higher-level surfaces without creating a second mutation
engine.

`mdx-rust map` scans the requested workspace, file, or directory and writes a
codebase map under `.mdx-rust/maps/`. The map includes workspace metadata,
quality grade, debt score, evidence grade, hardening findings, public API
pressure, module edges, available optional gates such as `cargo-nextest`,
`cargo-llvm-cov`, `cargo-mutants`, and `cargo-semver-checks`, and recommended
next actions.

`mdx-rust autopilot` coordinates the existing map, plan, apply-plan, and
hardening paths:

1. Build and persist a codebase map.
2. Build and persist a fresh refactor plan.
3. Select only executable low-risk candidates from that plan.
4. Run the same `apply-plan --all` machinery in review or apply mode.
5. In apply mode, replan before any later pass.
6. Stop on stale snapshots, rejected steps, unsupported recipes, behavior eval
   failures, validation failures, or exhausted executable candidates.
7. Persist an autopilot report under `.mdx-rust/autopilot/`.

The autonomous loop is allowed to run multiple passes, but every source edit
still routes through hardening transactions with isolated validation, optional
behavior evals, final validation, and rollback.

`mdx-rust evolve` is the agent-friendly wrapper over the same machinery. It
adds a time budget, recipe tier, and minimum evidence grade. Those options only
reduce the execution queue; they never weaken validation or rollback.

Evidence grades control proportional aggression:

- `None`: no autonomous source changes.
- `Compiled`: Tier 1 mechanical recipes may attempt compile/clippy-gated
  transactions.
- `Tested`: Tier 1 remains executable, analysis depth becomes
  boundary-aware, and plan generation surfaces extra Tier 2 review candidates
  for boundary and security-sensitive findings.
- `Covered`, `Hardened`, and `Proven`: reserved for future Tier 2 and Tier 3
  recipes once coverage, mutation, and property evidence are actually run.

The v0.6 executable Tier 1 recipe set is intentionally mechanical:

- contextual error hardening in `anyhow::Result` functions
- boundary error context propagation for filesystem and environment calls that
  already use `?`
- private borrow parameter tightening from owned container references to
  borrowed views
- iterator clone cleanup from clone-mapping collection to a simpler validated
  form such as `to_vec()`
- `#[must_use]` annotations for public value-returning functions
