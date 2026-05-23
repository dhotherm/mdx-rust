# Safety Invariants

`mdx-rust` is allowed to propose changes, but it must never treat a proposed
change as accepted until the safety pipeline proves it is safe and useful.

This document is the contract for every optimizer, hook, ledger, and edit
planner change.

## Acceptance Loop

Every accepted change must pass this sequence:

1. Build a targeted `ProposedEdit` for one file.
2. Run `PreEdit` hooks. Any deny decision stops the candidate.
3. Run `PreCommand` hooks before validation commands. Any deny decision stops validation.
4. Apply the edit in an isolated workspace.
5. Run validation in the isolated workspace.
   - Today this means `cargo check` and `cargo clippy -- -D warnings`.
   - Validation must use timeouts and process cleanup.
6. Score the patched isolated workspace against the train split.
7. Run `PreAccept` hooks with the score delta.
8. Require a strictly positive score delta.
9. Land the already validated edit on the real agent tree.
10. Run final validation on the real agent tree.
11. If final validation fails, restore the pre-land snapshot and do not count the change as landed or accepted.
12. Only after final validation succeeds may the run increment `accepted_changes`.

## V1 Edit Scope

The current public safety contract is intentionally single-file.

- A candidate patch must match `ProposedEdit.file`.
- A patch that advertises another file in its diff headers is rejected before validation.
- Multi-file or structural edits require transaction snapshots for every touched file before they can be allowed.
- Future multi-file strategies must update this document and add rollback tests before landing.

## Non-Bypass Rules

- Hooks can only add gates. They must never skip isolated validation, net-positive scoring, landing validation, or rollback.
- Budgets can only reduce candidate count or split evaluation data. They must never reduce the validation requirements for a candidate that is executed.
- Ledgers are records only. A `PromptVariantRecord` means "considered", not "validated", "landed", or "accepted".
- Security audits are advisory unless explicitly wired as a hook. Audit findings must not imply acceptance or rejection by themselves.
- JSON output must remain machine-parseable. Human progress output belongs outside `--json` mode.
- Any code path that lands a change must have a rollback path.
- Candidate execution has an outer wall-clock budget in addition to command-level timeouts. Synchronous cargo/git work is bounded by its own process timeout; once the candidate budget is exhausted, the pipeline must not continue to later stages.

## Counters

- `validated_changes`: candidate passed isolated validation.
- `landed_changes`: candidate was applied to the real agent tree and final validation passed.
- `accepted_changes`: candidate landed, final validation passed, and the score delta was strictly positive.

`accepted_changes` must never exceed `landed_changes`, and `landed_changes` must never exceed `validated_changes`.

## Provenance

Accepted runs must record enough evidence for another engineer or agent to inspect what happened without guessing:

- dataset version and content hash
- policy path and content hash when a policy file is available
- scorer id/version
- diagnosis model provider/name and whether a live model was used
- git SHA before/after when the agent root is a git repository
- diff hash for the accepted patch
- hook decisions
- isolated and final validation command records, including status, timeout flag, duration, stdout, and stderr
- train score, accepted patched score, score delta, and holdout score when available
- rollback status and error when rollback is attempted

## Required Tests

Changes touching optimization, hooks, validation, scoring, patch application, or ledgers must include or preserve tests proving:

- A deny hook cannot validate, land, or accept a candidate.
- A net-negative candidate is rejected before landing.
- A final validation failure rolls back the real tree and does not accept.
- Budget limits cap candidate attempts but do not remove validation requirements.
- Ledger records do not imply acceptance.
- A patch whose diff touches a different file than `ProposedEdit.file` is rejected before validation.
- Candidate timeout exhaustion prevents validation, landing, or acceptance.
- At least one end-to-end optimizer test proves a denied candidate cannot land or accept.

The current invariant tests live primarily in:

- `crates/mdx-rust-core/src/safety_pipeline.rs`
- `crates/mdx-rust-core/src/ledger.rs`
- `crates/mdx-rust-analysis/src/editing.rs`

## Change Discipline

Before expanding platform features such as external hooks, native tool rewrites,
model config edits, MCP/A2A support, or richer security gates, first make sure
the acceptance loop above remains mechanically obvious in code and green under:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
```
