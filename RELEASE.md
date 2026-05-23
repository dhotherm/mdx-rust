# Release Notes And Publishing

`mdx-rust v0.5.0` is the current release target. Publish only from an exact
commit that has passed local release validation, GitHub CI, and external
pressure testing.

## Supported Install Paths

From crates.io:

```bash
cargo install mdx-rust
```

From GitHub:

```bash
cargo install --git https://github.com/dhotherm/mdx-rust --package mdx-rust
```

From a local checkout:

```bash
cargo install --path crates/mdx-rust
```

## Publish Order

The workspace has internal versioned path dependencies. Publish in this order:

1. `mdx-rust-analysis`
2. `mdx-rust-core`
3. `mdx-rust`

Dry-run immediately before each publish:

```bash
cargo publish -p mdx-rust-analysis --locked --dry-run
cargo publish -p mdx-rust-core --locked --dry-run
cargo publish -p mdx-rust --locked --dry-run
```

Then publish:

```bash
cargo publish -p mdx-rust-analysis --locked
cargo publish -p mdx-rust-core --locked
cargo publish -p mdx-rust --locked
```

Wait for the crates.io index to update between dependent publishes.

## Release Gate

Use [docs/release-readiness.md](./docs/release-readiness.md) as the source of
truth for `v0.5.0` release validation.

## What Is New In v0.5.0

`v0.5.0` turns mdx-rust into a plan-first guardrailed refactoring workflow:

- `mdx-rust plan [target]` writes a non-mutating refactor plan with source
  snapshots, public API pressure, module edges, candidate risk, policy
  references, behavior eval references, and required gates.
- `mdx-rust apply-plan <plan> --candidate <id>` reviews or applies one
  executable low-risk candidate from a saved plan.
- `mdx-rust apply-plan <plan> --all` reviews or applies a bounded queue of all
  executable low-risk candidates in a saved plan.
- `refactor-plan`, `refactor-apply-run`, and `refactor-batch-apply-run` schemas
  are available for agents and automation.
- Every executable refactor candidate still routes through the existing
  hardening transaction and validation path.

## Known Limitations

- v0.5 does not perform arbitrary autonomous refactors.
- v0.5 executable candidates are intentionally narrow. Today they cover
  contextual error hardening routed through the hardening engine.
- Split-module, extract-function, public API, and boundary validation
  candidates are still plan or design artifacts.
- Static analysis is syntactic and file/module oriented. It is not yet a type
  graph, ownership analysis, semantic call graph, or semver proof.
- `apply-plan --all` de-duplicates by file because the current hardening
  transaction applies all patchable findings in a target file.
- Behavior evals are deterministic command gates, not coverage or mutation
  testing.

## Public Claims To Avoid

- Do not call the tool generally production-ready.
- Do not claim arbitrary autonomous refactoring.
- Do not imply the library API is stable.
- Do not imply behavior evals are coverage, mutation testing, or full semantic
  proofs.
- Do not imply `improve` applies changes unless `--apply` is passed.
- Do not imply `plan` applies refactors or proves semver compatibility.
- Do not imply `apply-plan` executes every refactor candidate. In v0.5 it only
  executes supported low-risk candidates, including queued execution through
  `--all`, and rejects stale plans.

The honest public phrase for `v0.5.0` is:

> A safety-first public beta for guardrailed Rust refactoring, with executable
> low-risk plan candidates, queued apply-plan execution, stale-plan rejection,
> policy and behavior-gated hardening, validation gates, transaction rollback,
> and versioned provenance.
