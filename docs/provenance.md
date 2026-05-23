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
`hook-decision`, and `trace-event`.

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
