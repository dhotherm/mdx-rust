set dotenv-load := false

fmt:
    cargo fmt --all

fmt-check:
    cargo fmt --all -- --check

check:
    cargo check --workspace --locked

clippy:
    cargo clippy --workspace --locked -- -D warnings

test:
    cargo test --workspace --locked

docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

audit:
    cargo deny check advisories bans sources

machete:
    cargo machete

ci:
    cargo fmt --all -- --check
    cargo check --workspace --locked
    cargo test --workspace --locked
    cargo clippy --workspace --locked -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --locked

release-candidate:
    just ci
    cargo build --workspace --release --locked
    cargo package -p mdx-rust-analysis --locked --allow-dirty
    # Downstream crates depend on unpublished sibling 0.3 packages until the publish
    # order starts, so pre-publish checks can only inspect their package files.
    cargo package -p mdx-rust-core --list --allow-dirty >/dev/null
    cargo package -p mdx-rust --list --allow-dirty >/dev/null

first-run-smoke:
    cargo run -p mdx-rust -- init
    cargo run -p mdx-rust -- schema audit-packet --json
    cargo run -p mdx-rust -- schema hardening-run --json
    cargo run -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- register example examples/rig-minimal-agent
    cargo run -p mdx-rust -- doctor example --json

example-smoke:
    cargo run -p mdx-rust -- optimize example --iterations 1 --budget light --json
    cargo run -p mdx-rust -- audit example --json

hardening-smoke:
    cargo run -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- audit --json
    cargo run -p mdx-rust -- improve crates/mdx-rust-analysis/src/hardening.rs --json
