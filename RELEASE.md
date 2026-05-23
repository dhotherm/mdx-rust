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
