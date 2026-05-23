# Release Readiness

`mdx-rust` is currently ready for private beta use from the GitHub repository.
The manifests are prepared for crates.io publication once the owner is ready to
publish the three workspace crates in dependency order.

## Supported Install Paths Today

Install the CLI from GitHub:

```bash
cargo install --git https://github.com/dhotherm/mdx-rust --package mdx-rust
```

Install from a local checkout:

```bash
cargo install --path crates/mdx-rust
```

Both paths keep the current three-crate workspace intact and are the release
paths we should validate before sharing the repository with early users.

## Crates.io Publish Order

The workspace currently has internal path dependencies:

- `mdx-rust`
- `mdx-rust-core`
- `mdx-rust-analysis`

The binary crate depends on internal crates. Publish them in this exact order:

1. `mdx-rust-analysis`
2. `mdx-rust-core`
3. `mdx-rust`

Each crate has `publish = true` and versioned path dependencies where needed.
Local development uses the path dependency; the published package resolves the
same version from crates.io.

Dry-run each step immediately before publishing that crate. `mdx-rust-core`
dry-run requires `mdx-rust-analysis` to already exist on crates.io, and
`mdx-rust` dry-run requires both library crates to already exist.

```bash
cargo publish -p mdx-rust-analysis --dry-run
cargo publish -p mdx-rust-core --dry-run
cargo publish -p mdx-rust --dry-run
```

Publish in the same order:

```bash
cargo publish -p mdx-rust-analysis
cargo publish -p mdx-rust-core
cargo publish -p mdx-rust
```

Wait for the crates.io index to update between steps if a dependent crate cannot
see the crate that was just published.

## Beta Release Gate

Before tagging a private beta, run:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo build --workspace --release
cargo install --path crates/mdx-rust --root /tmp/mdx-rust-install-check --debug
cargo publish -p mdx-rust-analysis --dry-run
cargo publish -p mdx-rust-core --dry-run
cargo publish -p mdx-rust --dry-run
```

For a clean first-run check:

```bash
tmpdir="$(mktemp -d)"
cd "$tmpdir"
mdx-rust init --json
test -f .mdx-rust/config.toml
```

## Public Claims To Avoid For Now

- Do not call the tool generally production-ready.
- Do not claim arbitrary multi-file rollback.
- Do not claim crates.io installation until all three crates are published.
- Do not imply scored standalone `mdx-rust eval` is complete.

The honest public phrase is: private beta, safety-first, single-file
prompt/fallback optimization for Rust agents, installable from GitHub.
