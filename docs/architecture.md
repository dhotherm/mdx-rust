# Architecture

`mdx-rust` is organized around a small number of deliberately boring stages.

## Crates

- `mdx-rust`: CLI entrypoint and human or JSON command output.
- `mdx-rust-core`: registry, runner, optimizer, hooks, ledgers, scoring, audit
  packets, hardening reports, and the candidate safety pipeline.
- `mdx-rust-analysis`: Rust source discovery, prompt/tool finders, isolated
  workspace creation, patch application, hardening analysis, validation, and
  rollback snapshots.

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

Hardening transactions snapshot every touched file, validate in isolation,
require `--apply` before landing, run final validation, run final behavior evals
when supplied, and roll back on failure. General multi-file refactoring remains
future work.
