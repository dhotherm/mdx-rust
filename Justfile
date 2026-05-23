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
    # Downstream crates depend on unpublished sibling 0.5 packages until the publish
    # order starts, so pre-publish checks can only inspect their package files.
    cargo package -p mdx-rust-core --list --allow-dirty >/dev/null
    cargo package -p mdx-rust --list --allow-dirty >/dev/null

first-run-smoke:
    # Run init/eval/doctor in a throwaway crate so the smoke does not depend on local .mdx-rust state.
    tmpdir="$(mktemp -d)"; mkdir -p "$tmpdir/src"; printf '[package]\nname = "mdx-first-run-smoke"\nversion = "0.1.0"\nedition = "2021"\n' > "$tmpdir/Cargo.toml"; printf 'pub fn ok() {}\n' > "$tmpdir/src/lib.rs"; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- init && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- eval --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- schema audit-packet --json
    cargo run -p mdx-rust -- schema hardening-run --json
    cargo run -p mdx-rust -- schema behavior-eval-report --json
    cargo run -p mdx-rust -- schema project-policy --json
    cargo run -p mdx-rust -- schema refactor-plan --json
    cargo run -p mdx-rust -- register example examples/rig-minimal-agent
    cargo run -p mdx-rust -- doctor example --json

example-smoke:
    cargo run -p mdx-rust -- optimize example --iterations 1 --budget light --json
    cargo run -p mdx-rust -- audit example --json

hardening-smoke:
    cargo run -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- audit --json
    cargo run -p mdx-rust -- eval --spec examples/evals/cargo-check.json --json
    cargo run -p mdx-rust -- improve crates/mdx-rust-analysis/src/hardening.rs --eval-spec examples/evals/cargo-check.json --json

plan-smoke:
    tmpdir="$(mktemp -d)"; mkdir -p "$tmpdir/src"; printf '[package]\nname = "mdx-plan-smoke"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nanyhow = "1"\n' > "$tmpdir/Cargo.toml"; printf 'pub fn load() -> anyhow::Result<String> {\n    let value = std::fs::read_to_string("missing.toml").unwrap();\n    Ok(value)\n}\n' > "$tmpdir/src/lib.rs"; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- plan src/lib.rs --json
    cargo run -p mdx-rust -- schema refactor-plan --json
