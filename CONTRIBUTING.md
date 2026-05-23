# Contributing

Thanks for helping improve `mdx-rust`.

This project optimizes code written by other agents, so changes that touch
editing, validation, scoring, rollback, hooks, or provenance have a higher than
usual safety bar.

## Development Setup

```bash
cargo build --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo fmt --all -- --check
```

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

## Release Flow

For release details, see [RELEASE.md](./RELEASE.md).
