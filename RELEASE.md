# Release Readiness

`mdx-rust` is currently ready for private beta use from the GitHub repository.
It is not yet ready for crates.io publication.

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

## Crates.io Status

The workspace currently has internal path dependencies:

- `mdx-rust`
- `mdx-rust-core`
- `mdx-rust-analysis`

`cargo package -p mdx-rust` is not expected to be the release gate yet because
the binary crate depends on unpublished internal crates. Before crates.io
publication, choose one of these paths:

1. Publish `mdx-rust-analysis`, then `mdx-rust-core`, then `mdx-rust`.
2. Collapse the public install surface into a single publishable crate.
3. Keep crates.io disabled and make GitHub install the official beta channel.

Until that decision is made, README language should say GitHub install is the
supported path and crates.io packaging is pending.

## Beta Release Gate

Before tagging a private beta, run:

```bash
cargo fmt --all -- --check
cargo check --workspace
cargo test --workspace
cargo clippy --workspace -- -D warnings
cargo build --workspace --release
cargo install --path crates/mdx-rust --root /tmp/mdx-rust-install-check --debug
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
- Do not claim crates.io installation until the packaging path is resolved.
- Do not imply scored standalone `mdx-rust eval` is complete.

The honest public phrase is: private beta, safety-first, single-file
prompt/fallback optimization for Rust agents, installable from GitHub.
