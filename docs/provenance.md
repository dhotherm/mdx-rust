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
`hook-decision`, `trace-event`, `hardening-run`, and `hardening-finding`.

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

`v0.4` hardening runs produce separate reports:

```text
.mdx-rust/hardening/hardening-<mode>-<timestamp>.json
```

The hardening report schema version is `"0.4"`.

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
