# Provenance

Accepted changes produce a versioned audit packet:

```text
.mdx-rust/agents/<name>/experiments/audit-packet-<timestamp>-iteration-<n>.json
```

The schema version for `v0.2` is `"0.2"`.

Print the current machine-readable schema with:

```bash
mdx-rust schema audit-packet --json
```

Other exported schemas include `candidate`, `optimization-run`,
`hook-decision`, `trace-event`, `hardening-run`, `hardening-finding`,
`evidence-run`, `agent-contract`, `refactor-plan`, `refactor-apply-run`,
`refactor-batch-apply-run`, `codebase-map`, `evolution-scorecard`, and
`autopilot-run`.

## Required Fields

Each packet records:

- `schema_version`
- `agent_name`
- `iteration`
- `edit_scope_contract`
- `accepted_edit`
- `provenance`
- `scores`
- `hook_decisions`
- `validation_command_records`
- `final_validation_command_records`
- `rollback_succeeded`
- `rollback_error`
- `candidate_timed_out`

## Accepted Edit

`accepted_edit` contains:

- `description`
- `changed_file`
- `diff_hash`
- `diff`

`v0.2` uses the `single-file-v0.2` edit scope contract.

## Provenance

`provenance` contains:

- `git_sha_before`
- `git_sha_after`
- `working_tree_dirty_after`
- `policy_path`
- `policy_hash`
- `dataset_version`
- `dataset_hash`
- `scorer_id`
- `scorer_version`
- `model`

When no live model is used, model metadata still records the configured
diagnosis model and `used = false`.

## Validation Records

Each validation command record includes:

- Command label.
- Success flag.
- Timeout flag.
- Exit status.
- Duration in milliseconds.
- Captured stdout.
- Captured stderr.

The records are intentionally bounded by command timeouts and candidate timeout.

## Non-Goals

The `0.2` audit packet is unsigned JSON. Signed attestations, SLSA-style output,
and external compliance integrations are future work.

## Hardening Reports

`v1.0 beta` hardening runs produce separate reports:

```text
.mdx-rust/hardening/hardening-<mode>-<timestamp>.json
```

The hardening report schema version is `"1.0"`.

Hardening reports record:

- workspace root and target
- review or apply mode
- `cargo metadata` workspace summary when available
- optional policy path and content hash
- structured policy rules and policy-to-finding matches for reviewer context
- risk summary and recommended next actions
- files scanned
- findings and whether each finding is patchable
- proposed change summaries and old/new content hashes
- isolated validation command records
- isolated behavior eval command records when `--eval-spec` is supplied
- final validation command records when `--apply` is used
- final behavior eval command records when `--eval-spec --apply` is used
- transaction status
- rollback status and rollback error when rollback is attempted

Behavior eval command records include malformed command and bad working
directory failures as structured records, rather than treating them as accepted
behavior evidence.

Print the hardening schema with:

```bash
mdx-rust schema hardening-run --json
mdx-rust schema behavior-eval-report --json
mdx-rust schema project-policy --json
```

## Evidence Runs

`v1.0 beta` evidence runs produce separate reports:

```text
.mdx-rust/evidence/evidence-<timestamp>-<run-id>.json
```

The evidence run schema version is `"1.0"`.

Evidence runs record:

- workspace root and optional target
- measured grade and analysis depth
- parsed metrics such as coverage percentage and mutation score when present in
  tool output
- command records for cargo metadata, cargo test, coverage, mutation, and
  semver checks
- file/function evidence profiles used by maps and plans to explain candidate
  eligibility
- skipped command reasons when heavier evidence was not requested or the tool
  was unavailable
- timeout flags, status code, duration, stdout, and stderr for executed
  commands
- unlocked recipe tiers and unlock suggestions

Evidence artifacts are gates for proportional autonomy. They are not proof that
a candidate is safe. Every executed candidate still needs plan freshness,
isolated validation, final validation, and rollback evidence.

Print the evidence schema with:

```bash
mdx-rust schema evidence-run --json
```

## Agent Contract

`v1.0 beta` also exposes an agent-facing command contract and local runtime
manifest:

```bash
mdx-rust agent-contract --json
mdx-rust runtime --json
mdx-rust schema agent-contract --json
mdx-rust schema agent-runtime-manifest --json
mdx-rust schema agent-pack --json
mdx-rust schema repo-map --json
mdx-rust schema noise-filter --json
mdx-rust schema contract-run --json
mdx-rust schema performance-run --json
mdx-rust schema evolution-brief --json
mdx-rust schema agent-ready-report --json
```

The contract records:

- CLI product version and contract schema version
- JSON mode expectations
- mutation contract for `--apply`
- read-only and mutation-capable commands
- required flags for source mutation
- primary schema names for each command
- recommended agent workflows
- artifact globs agents should inspect
- safety rules for external automation

The agent contract is guidance for safe automation. It is not itself validation
or permission to mutate source files.

The runtime manifest records local transports, runtime tools, request schema
names, response schema names, and mutation rules. The MCP and HTTP runtime
surfaces produce the same underlying artifacts as CLI commands. They do not
create a separate provenance path.

The HTTP runtime can require a bearer token through `--token` or
`MDX_RUST_RUNTIME_TOKEN`. This is local bearer-token protection for a developer
machine or local agent process. It is not a remote service security model and
does not claim tenant isolation or rate limiting.

Runtime provenance follows the underlying tool. A read-only runtime call
returns the same artifact references as the matching CLI command. A
mutation-capable runtime call must produce the same autopilot, apply-plan,
hardening, validation, rollback, and behavior-eval records that a CLI
`evolve --apply` call would produce. The runtime response is only an envelope;
the saved artifacts remain the source of truth.

Agent packs are instruction artifacts. They can be written into a repo for
Codex, Claude, Cursor, Aider, Goose-style, or generic agents, but they are not
approval artifacts and they do not validate or apply source changes.

Repo maps and noise filters are orientation artifacts. They tell agents which
instructions, directories, crate boundaries, and generated paths to inspect or
ignore before planning. `repo-map` and `noise-filter` do not validate, approve,
or apply source changes. `noise-filter --write` may only write instruction
artifacts under `.mdx-rust/agent-pack/`.

Contract runs are read-only spec artifacts. They record documented function
intent and assertion hints, then recommend where public functions have no
visible contract. They are useful planning evidence, but they are not behavior
proofs and they do not approve source mutation.

Performance runs are read-only prioritization artifacts. They record static
performance pressure such as blocking work in async functions, clone pressure,
allocations in loops, and synchronous lock hints. They are not benchmark
evidence, behavior proofs, or approval to mutate source.

Evolution briefs are read-only fusion artifacts:

```text
.mdx-rust/briefs/evolution-brief-<timestamp>-<brief-id>.json
```

They bundle repo context, noise filters, contract posture, performance posture,
and the saved scorecard. The brief is an intake artifact for agents. It is not
validation evidence and it does not approve mutation.

`mdx-rust agent-ready --json` is a compact readiness envelope derived from the
scorecard. It is not mutation evidence. It points agents at the command
contract, runtime manifest, readiness grade, evidence grade, and next commands
without replacing the saved scorecard, plan, hardening, or autopilot artifacts.

## Evolution Scorecards

`v1.0 beta` scorecards produce separate reports:

```text
.mdx-rust/scorecards/evolution-scorecard-<timestamp>-<scorecard-id>.json
```

The scorecard schema version is `"1.0"`.

Scorecards record:

- workspace root and optional target
- autonomy readiness grade, max safe recipe tier, candidate counts, blockers,
  and recommended command
- embedded codebase map
- embedded refactor plan
- recipe catalog snapshot
- next commands for agents to run safely
- artifact path

Scorecards are read-only briefing artifacts. They do not validate or apply any
candidate. Their purpose is to let a coding agent make an informed next move
from one schema-backed artifact instead of combining human-oriented output.

Print the scorecard schema with:

```bash
mdx-rust schema evolution-scorecard --json
```

## Recipe Catalog And Artifact Explanation

`v1.0 beta` exposes read-only agent surfaces:

```bash
mdx-rust recipes --json
mdx-rust explain <artifact> --json
mdx-rust agent-pack codex --json
mdx-rust schema recipe-catalog --json
mdx-rust schema artifact-explanation --json
```

The recipe catalog records recipe id, tier, required evidence, executable
status, risk, mutation path, and description. Artifact explanations summarize a
saved mdx-rust JSON artifact and provide recommended next actions. Neither
surface mutates source files or grants permission to mutate source files.

## Refactor Plans

`v1.0 beta` refactor plans produce separate reports:

```text
.mdx-rust/plans/refactor-plan-<timestamp>-<plan-id>.json
```

The refactor plan schema version is `"1.0"`.

Refactor plans record:

- workspace root and target
- optional policy path and content hash
- optional behavior eval spec path
- measured evidence artifact reference when available
- security posture summary
- autonomy readiness summary
- plan hash and source snapshot hashes
- file and module scan counts
- public API pressure
- patchable hardening candidate counts
- required gates for any future application
- explicit non-goals
- candidate recipe, risk, status, rationale, touched files, and optional apply
  command
- candidate evidence context explaining whether eligibility came from a
  measured file profile or a broader evidence summary
- candidate autonomy decision explaining whether it is allowed, review-only, or
  blocked for autonomous execution

A refactor plan is not acceptance evidence. It is a review artifact.
`mdx-rust apply-plan` must verify source snapshot hashes before it can execute
an approved candidate. Executable candidates still go through
`mdx-rust improve`, and future broader plan application commands must re-run
safety gates rather than trusting stale plan output.

Apply-plan reports record:

- plan id and plan hash
- candidate id and candidate hash
- review or apply mode
- stale file evidence when hashes do not match
- public API impact allowance
- embedded hardening run when an executable candidate is reviewed or applied
- final status: reviewed, applied, rejected, stale, or unsupported

Batch apply-plan reports additionally record the execution queue shape:

- max candidate budget
- requested, executed, and skipped candidate counts
- one step record per executable candidate selected from the plan
- per-step candidate hash, file, stale-file evidence, hardening report, and
  final status

Print the refactor plan schema with:

```bash
mdx-rust schema refactor-plan --json
mdx-rust schema refactor-apply-run --json
mdx-rust schema refactor-batch-apply-run --json
```

## Codebase Maps And Autopilot Runs

`v1.0 beta` codebase maps produce separate reports:

```text
.mdx-rust/maps/codebase-map-<timestamp>-<map-id>.json
```

The codebase map schema version is `"1.0"`.

Codebase maps record:

- workspace root and target
- optional policy path and content hash
- optional behavior eval spec path
- measured evidence artifact reference when available
- evidence grade, analysis depth, evidence signals, unlocked recipe tiers, max
  autonomous tier, and unlock suggestions
- autonomy readiness grade, max safe tier, and allowed/review-only/blocked
  candidate counts
- quality grade, debt score, and security score
- security posture severity counts and top findings
- patchable and review-only finding counts
- public API pressure
- oversized file and function counts
- test coverage signal
- optional capability gate availability for nextest, llvm-cov, mutants, and
  semver checks
- findings, module edges, file summaries, and recommended next actions

Autopilot runs produce separate reports:

```text
.mdx-rust/autopilot/autopilot-<timestamp>-<run-id>.json
```

The autopilot run schema version is `"1.0"`.

Autopilot runs record:

- review or apply mode
- optional budget seconds and whether the budget was exhausted
- evidence grade, analysis depth, and max autonomous tier used for the run
- execution summary with plans created, executable candidates seen, validated
  transactions, applied transactions, and blocked or plan-only candidates
- quality before and after when apply mode changes the tree
- max pass and candidate budgets
- one pass record per fresh plan
- plan id, plan hash, and plan artifact path per pass
- embedded batch apply reports with hardening evidence for executed steps
- total planned, executed, and skipped candidate counts
- final status and note

Refactor plan candidates also record recipe tier, required evidence, and
whether the plan evidence satisfied that requirement. Queue execution filters
on those fields before any candidate reaches the hardening transaction.

Print the schemas with:

```bash
mdx-rust schema codebase-map --json
mdx-rust schema autopilot-run --json
```
