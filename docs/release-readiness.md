# Release Readiness

This document is the release checklist for `v0.3.0`.

## Required Automated Gates

```bash
just release-candidate
```

GitHub CI also runs supply-chain and quality checks with `cargo-deny`,
`cargo-machete`, codespell, docs warnings, and OpenSSF Scorecard.

Before publishing, only `mdx-rust-analysis` can be fully package-verified because
it has no internal unpublished dependency. `just release-candidate` inspects the
downstream package file lists, but Cargo cannot fully prepare or verify packages
whose required sibling versions are not indexed on crates.io yet. During the
actual publish sequence, run full dry-runs in dependency order after each
dependency is indexed.

## Dependency Posture

`mdx-rust` intentionally uses a moderate dependency tree because it needs Rust
parsing, CLI ergonomics, async process execution, and optional model-provider
support. Before `v0.3.0` is published:

- No yanked crates should be present.
- Known RustSec advisories must be fixed or documented with a dated
  `deny.toml` exception before publishing.
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
mdx-rust schema audit-packet --json >/tmp/mdx-rust-audit-schema.json
mdx-rust schema hardening-run --json >/tmp/mdx-rust-hardening-schema.json
mdx-rust doctor --json >/tmp/mdx-rust-doctor.json
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

## Hardening Smoke

From a clean checkout:

```bash
cargo run -p mdx-rust -- doctor --json
cargo run -p mdx-rust -- audit --json
cargo run -p mdx-rust -- improve crates/mdx-rust-analysis/src/hardening.rs --json
```

Confirm that review mode does not mutate the working tree and that any proposed
change has isolated validation command records.

## Performance Sanity

Record rough timings before release:

- CLI startup with `mdx-rust --version`.
- Fresh `mdx-rust init`.
- One `optimize --iterations 1 --budget light` run on the example agent.
- One `improve <small-rust-file>` review run.

The exact numbers depend heavily on Cargo cache warmth. The release bar is not a
micro-benchmark; it is that the CLI starts promptly and the example optimization
does not hang or produce confusing output.

## Publish Order

Do not publish `v0.3.0` until the candidate commit has passed external pressure
testing.

When approved, publish in dependency order:

```bash
cargo publish -p mdx-rust-analysis --locked --dry-run
cargo publish -p mdx-rust-analysis --locked
cargo publish -p mdx-rust-core --locked --dry-run
cargo publish -p mdx-rust-core --locked
cargo publish -p mdx-rust --locked --dry-run
cargo publish -p mdx-rust --locked
```

Wait for crates.io indexing between dependent publishes.
