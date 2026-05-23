# Release Readiness

This document is the release checklist for `v0.2.0`.

## Required Automated Gates

```bash
just release-candidate
```

GitHub CI also runs supply-chain and quality checks with `cargo-deny`,
`cargo-machete`, codespell, docs warnings, and OpenSSF Scorecard.

Before publishing, only `mdx-rust-analysis` can be fully package-verified because
it has no internal unpublished dependency. During the actual publish sequence,
run full dry-runs in dependency order after each dependency is indexed.

## Dependency Posture

`mdx-rust` intentionally uses a moderate dependency tree because it needs Rust
parsing, CLI ergonomics, async process execution, and optional model-provider
support. Before `v0.2.0` is published:

- No yanked crates should be present.
- Unknown registries and unknown git sources should be denied.
- Heavy transitive dependencies should be accepted only when they support the
  CLI product path or current analysis/runtime needs.
- License review is tracked separately from the automated advisory/source gate.

## Install Smoke

```bash
install_root="$(mktemp -d)"
cargo install --path crates/mdx-rust --root "$install_root" --locked --debug
"$install_root/bin/mdx-rust" --version
```

## First-Run Smoke

```bash
tmpdir="$(mktemp -d)"
cd "$tmpdir"
mdx-rust init
test -f .mdx-rust/config.toml
```

## Example Smoke

From a clean checkout:

```bash
cargo run -p mdx-rust -- init
cargo run -p mdx-rust -- register example examples/rig-minimal-agent
cargo run -p mdx-rust -- optimize example --iterations 1 --budget light
cargo run -p mdx-rust -- audit example
```

Confirm that the optimizer either accepts a net-positive change with an audit
packet or clearly reports that no safe improvement was accepted.

## Performance Sanity

Record rough timings before release:

- CLI startup with `mdx-rust --version`.
- Fresh `mdx-rust init`.
- One `optimize --iterations 1 --budget light` run on the example agent.

The exact numbers depend heavily on Cargo cache warmth. The release bar is not a
micro-benchmark; it is that the CLI starts promptly and the example optimization
does not hang or produce confusing output.

## Publish Order

Do not publish `v0.2.0` until the candidate commit has passed external pressure
testing.

When approved, publish in dependency order:

```bash
cargo publish -p mdx-rust-analysis --locked
cargo publish -p mdx-rust-core --locked
cargo publish -p mdx-rust --locked
```

Wait for crates.io indexing between dependent publishes.
