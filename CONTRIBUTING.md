# Contributing

Thanks for helping improve `mdx-rust`.

This project optimizes code written by other agents, so changes that touch
editing, validation, scoring, rollback, hooks, or provenance have a higher than
usual safety bar.

## Development Setup

```bash
just ci
```

If you do not have `just`, read the `ci` recipe in `Justfile` and run those
commands directly.

## Pull Request Expectations

- Keep changes focused and easy to review.
- Add tests for optimizer, safety, scoring, validation, or CLI behavior changes.
- Preserve machine-parseable `--json` output.
- Do not weaken the acceptance loop documented in
  [SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md).
- Update README, ROADMAP, or CHANGELOG when user-visible behavior changes.

## Safety-Sensitive Changes

For changes touching candidate edits or acceptance:

- Prove bad patches are rejected.
- Prove final validation failure rolls back real source.
- Prove counters cannot report accepted changes that did not land.
- Prove timeouts stop execution instead of hanging.
- Preserve the `v0.2` single-file accepted edit contract unless the change also
  adds transaction rollback support and updates `SAFETY_INVARIANTS.md`.
- Preserve complete audit packets for every accepted change.
- Preserve typed rejection reasons for every rejected candidate.
- Preserve JSON Schema derivations for structs that cross CLI, hook, trace,
  strategy, audit, or future LLM boundaries.

## API Changes

The CLI is the supported surface before `1.0`. Library APIs in
`mdx-rust-core` and `mdx-rust-analysis` are unstable. If a PR adds a new public
type, explain why it needs to be public and whether automation should consume it
through the CLI instead.

## Dependency Hygiene

- Run `just audit` after dependency updates.
- Run `just machete` after adding, removing, or moving dependencies.
- Advisory ignores in `deny.toml` must include a review date, affected path,
  reason, and removal condition.

## Release Flow

For release details, see [RELEASE.md](./RELEASE.md).
