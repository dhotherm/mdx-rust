# Architecture

`mdx-rust` is organized around a small number of deliberately boring stages.

## Crates

- `mdx-rust`: CLI entrypoint and human or JSON command output.
- `mdx-rust-core`: registry, runner, optimizer, hooks, ledgers, scoring, audit
  packets, and the candidate safety pipeline.
- `mdx-rust-analysis`: Rust source discovery, prompt/tool finders, isolated
  workspace creation, patch application, validation, and rollback snapshots.

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

## v0.2 Edit Scope

`v0.2` hard-enforces single-file accepted edits. A diff that touches a file other
than `ProposedEdit.file` is rejected before validation.

Multi-file accepted edits are a future feature and require transaction snapshots
for every touched file, tests proving rollback, and an update to
`SAFETY_INVARIANTS.md`.
