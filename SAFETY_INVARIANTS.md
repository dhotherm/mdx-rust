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

## v0.2 Agent Optimizer Edit Scope

The agent optimizer safety contract remains intentionally single-file.

- A candidate patch must match `ProposedEdit.file`.
- A patch that advertises another file in its diff headers is rejected before validation.
- Multi-file or structural edits require transaction snapshots for every touched file before they can be allowed.
- Future multi-file strategies must update this document and add rollback tests before landing.
- The current optimizer edit scope label in audit packets is `single-file-v0.2`.

## Hardening Transaction Scope

The hardening path is separate from the agent optimizer.

- `mdx-rust improve` runs in review mode by default.
- Hardening changes must validate in an isolated workspace before they can be
  shown as validated or applied.
- `--apply` is required before hardening changes touch the real tree.
- Every touched file must be snapshotted before applying to the real tree.
- If final validation fails or times out, every touched file must be restored
  from the transaction snapshot.
- A hardening transaction must reject absolute paths, parent-directory escapes,
  and any path outside the workspace root.
- The `0.8` hardening report schema records findings, change summaries,
  validation command records, final validation command records, policy hash,
  policy-to-finding matches, behavior eval records when supplied, workspace
  metadata, transaction status, and rollback status.
- When `--eval-spec` is supplied, behavior evals must pass in the isolated
  workspace before a hardening review/apply can succeed.
- When `--eval-spec` and `--apply` are supplied, behavior evals must also pass
  after final validation on the real tree. A final behavior failure must roll
  back the transaction and report no applied success.

## Refactor Plan Scope

`mdx-rust plan` is a planning and impact-analysis command. It must never mutate
the user's source tree.

- Plan generation may scan Rust files, parse public items, inspect module edges,
  summarize policy and behavior eval references, and write a plan artifact
  under `.mdx-rust/plans/`.
- Plan generation may recommend `mdx-rust improve --apply` for patchable
  hardening candidates, but that command remains the mutation boundary.
- A refactor plan is not validation evidence and does not increment any
  accepted, landed, applied, or validated counters.
- `mdx-rust apply-plan` must reject stale source snapshots before executing a
  candidate.
- `mdx-rust apply-plan` may execute only candidates marked as executable.
  Today that means supported Tier 1 and coverage-gated Tier 2 hardening
  candidates routed through `mdx-rust improve`.
- `mdx-rust apply-plan` must re-run the appropriate safety pipeline or
  hardening transaction. It must not trust stale plan evidence.
- `mdx-rust apply-plan --all` may execute a queue only for candidates already
  marked executable in the plan. It must verify the plan hash once, reject an
  initially stale plan, de-duplicate executable candidates by file, check the
  target file snapshot before each step, and stop applying if a step fails.
- Broad multi-file refactors require explicit transaction design, plan hashes,
  rollback evidence, and dedicated invariant tests before they can apply.

## Autonomous Evolution Scope

`mdx-rust autopilot` is an orchestrator over existing safe primitives. It is
allowed to move quickly, but it must not create a second mutation path.

- `mdx-rust map` must never mutate the user's source tree.
- `mdx-rust autopilot` without `--apply` must never mutate the user's source
  tree. It may write `.mdx-rust/maps/`, `.mdx-rust/plans/`, and review-mode
  reports only.
- `mdx-rust autopilot --apply` must build a fresh plan before each apply pass.
- Each apply pass may execute only candidates already marked executable in the
  fresh plan.
- Each executed candidate must satisfy the plan evidence grade, candidate
  required evidence, requested recipe tier, public API allowance, and support
  status before entering the execution queue.
- Each executed candidate must route through `apply-plan --all` and the
  hardening transaction path. Autopilot must not write Rust source files
  directly.
- v0.8 executable Tier 1 recipes are contextual error hardening, boundary error
  context propagation, private borrow parameter tightening, iterator clone
  cleanup, and `#[must_use]` annotation. They are allowed to execute only
  through the same hardening transaction path.
- v0.8 executable Tier 2 recipes require measured `Covered` evidence, an
  explicit Tier 2 request, and the same hardening transaction path. The
  supported Tier 2 recipes are repeated private string literal extraction into a
  file-local constant and `len() == 0` to `is_empty()` cleanup.
- v0.8 `Hardened` and `Proven` evidence may unlock deeper analysis findings,
  such as clone-pressure review and long-function review. These findings are
  planning evidence only unless a dedicated executable recipe marks them
  executable and routes them through the same hardening transaction path.
- v0.8 evidence artifacts may include file/function profiles. Candidate
  evidence context is explanatory and may only narrow or justify a queue; it
  must not override the plan evidence grade or required recipe evidence.
- v0.8 security posture in maps and plans is advisory prioritization evidence.
  It can raise risk, add recommendations, or keep candidates plan-only, but it
  must not bypass validation, evidence, stale snapshot, behavior eval, or
  rollback gates.
- `mdx-rust recipes` and `mdx-rust explain` are read-only agent surfaces. They
  must never mutate source files or approve mutation by themselves.
- `Tested` evidence may surface additional boundary-aware Tier 2 review
  candidates, but those candidates remain plan-only until a dedicated
  executable recipe and validation contract exists.
- Autopilot must stop on stale plans, rejected steps, unsupported candidates,
  final validation failures, behavior eval failures, or exhausted executable
  work.
- Autopilot budgets (`--max-passes`, `--max-candidates`, and validation
  timeout) can only reduce work. They must never reduce validation, rollback,
  or provenance requirements.
- `mdx-rust evolve` is an agent-facing wrapper around the same autonomous
  execution contract. Its `--budget`, `--tier`, and `--min-evidence` options can
  only reduce work.
- Public API impacting candidates remain blocked unless the caller passes an
  explicit public API allowance and the underlying executable recipe supports
  that scope.

## Non-Bypass Rules

- Hooks can only add gates. They must never skip isolated validation, net-positive scoring, landing validation, or rollback.
- Budgets can only reduce candidate count or split evaluation data. They must never reduce the validation requirements for a candidate that is executed.
- Ledgers are records only. A `PromptVariantRecord` means "considered", not "validated", "landed", or "accepted".
- Refactor plans are records only. A `RefactorPlan` means "reviewed candidate
  areas", not "validated", "applied", "landed", or "accepted".
- Codebase maps are records only. A `CodebaseMap` means "scanned and
  summarized", not "validated", "applied", "landed", or "accepted".
- Evidence grades are execution gates, not proof by themselves. A `Compiled`
  grade means Tier 1 candidates may attempt the compile/clippy-gated hardening
  path; it does not mean a candidate has already passed validation.
- Measured evidence artifacts can raise the visible grade, but they never
  replace per-candidate isolated validation, final validation, rollback, or
  behavior eval gates.
- Measured evidence may change what the analyzer looks for. That extra analysis
  must only add candidates or gates; it must not make a lower-evidence
  candidate executable.
- Autopilot reports are orchestration evidence only. They must point back to
  the concrete plans, apply-plan reports, hardening reports, validation
  records, and rollback evidence that justified each step.
- Refactor apply reports are records of an attempted plan execution. They must
  include stale-plan status when source snapshots do not match.
- The safety pipeline must keep stage-specific internal records for scoped,
  isolated-validated, and net-positive edits. A raw `ProposedEdit` is never
  enough to land or accept a change.
- Rejected candidates must carry a typed rejection reason in addition to any
  human note string.
- Security audits are advisory unless explicitly wired as a hook. Audit findings must not imply acceptance or rejection by themselves.
- JSON output must remain machine-parseable. Human progress output belongs outside `--json` mode.
- Any code path that lands a change must have a rollback path.
- Candidate execution has an outer wall-clock budget in addition to command-level timeouts. Synchronous cargo/git work is bounded by its own process timeout; once the candidate budget is exhausted, the pipeline must not continue to later stages.
- Hardening review mode may validate proposed changes in isolation, but it must
  not mutate the real tree.

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

Agent-facing provenance, hook, trace, candidate, eval, and audit records should
derive JSON Schema so external agents can validate the contract before
depending on it.

`mdx-rust agent-contract --json` is the machine-readable entrypoint for coding
agents. It must describe read-only commands, mutation-capable commands,
required mutation flags, schemas, artifact globs, and safety rules. It is
guidance for agents, not permission to bypass any invariant in this document.

For `v0.2`, every accepted change must also emit a versioned JSON audit packet
under `.mdx-rust/agents/<name>/experiments/`. See
`docs/provenance.md` for the `0.2` schema.

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
- At least one end-to-end optimizer test proves a real improvement can be accepted, improves the final on-disk score, and includes a complete audit packet.
- At least one hardening test proves review mode validates without touching the
  real tree.
- At least one hardening test proves `--apply` uses transaction snapshots and
  final validation before reporting success.
- At least one hardening test proves unscoped transaction paths are rejected.
- At least one hardening test proves behavior eval failure blocks apply.
- At least one CLI integration test proves workspace behavior eval JSON output
  is machine parseable.
- At least one CLI integration test proves refactor plan JSON output is machine
  parseable and does not mutate the source tree.
- At least one CLI integration test proves `apply-plan` can review and apply an
  executable candidate through hardening gates.
- At least one CLI integration test proves `apply-plan --all` can review and
  apply an executable queue while preserving review mode as non-mutating.
- At least one CLI integration test proves `apply-plan` rejects stale source
  snapshots before mutation.
- At least one CLI integration test proves `map --json` is machine parseable
  and does not mutate source files.
- At least one CLI integration test proves `autopilot --json` review mode is
  machine parseable and does not mutate source files.
- At least one CLI integration test proves `autopilot --apply --json` applies
  only executable low-risk candidates through the hardening transaction path
  and records quality before/after.
- At least one CLI integration test proves `evolve --json` respects budget and
  evidence gating, including a higher-than-available evidence request that
  blocks execution without mutating source files.

The current invariant tests live primarily in:

- `crates/mdx-rust-core/src/safety_pipeline.rs`
- `crates/mdx-rust-core/src/ledger.rs`
- `crates/mdx-rust-analysis/src/editing.rs`
- `crates/mdx-rust-core/src/hardening.rs`
- `crates/mdx-rust-analysis/src/hardening.rs`
- `crates/mdx-rust-core/src/refactor.rs`
- `crates/mdx-rust-analysis/src/refactor.rs`

## Change Discipline

Before expanding platform features such as external hooks, native tool rewrites,
model config edits, MCP/A2A support, or richer security gates, first make sure
the acceptance loop above remains mechanically obvious in code and green under:

```bash
just ci
```
