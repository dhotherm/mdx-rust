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
    just evidence-smoke
    just hardened-evidence-smoke
    cargo build --workspace --release --locked
    cargo package -p mdx-rust-analysis --locked --allow-dirty
    # Downstream crates depend on unpublished sibling 0.8 packages until the publish
    # order starts, so pre-publish checks can only inspect their package files.
    cargo package -p mdx-rust-core --list --allow-dirty >/dev/null
    cargo package -p mdx-rust --list --allow-dirty >/dev/null

first-run-smoke:
    # Run init/eval/doctor in a throwaway crate so the smoke does not depend on local .mdx-rust state.
    tmpdir="$(mktemp -d)"; mkdir -p "$tmpdir/src"; printf '[package]\nname = "mdx-first-run-smoke"\nversion = "0.1.0"\nedition = "2021"\n' > "$tmpdir/Cargo.toml"; printf 'pub fn ok() {}\n' > "$tmpdir/src/lib.rs"; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- init && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- eval --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- agent-contract --json
    cargo run -p mdx-rust -- recipes --json
    cargo run -p mdx-rust -- schema agent-contract --json
    cargo run -p mdx-rust -- schema recipe-catalog --json
    cargo run -p mdx-rust -- schema artifact-explanation --json
    cargo run -p mdx-rust -- schema audit-packet --json
    cargo run -p mdx-rust -- schema hardening-run --json
    cargo run -p mdx-rust -- schema behavior-eval-report --json
    cargo run -p mdx-rust -- schema project-policy --json
    cargo run -p mdx-rust -- schema evidence-run --json
    cargo run -p mdx-rust -- schema refactor-plan --json
    cargo run -p mdx-rust -- schema refactor-apply-run --json
    cargo run -p mdx-rust -- schema refactor-batch-apply-run --json
    cargo run -p mdx-rust -- schema codebase-map --json
    cargo run -p mdx-rust -- schema autopilot-run --json
    cargo run -p mdx-rust -- register example examples/rig-minimal-agent
    cargo run -p mdx-rust -- doctor example --json

example-smoke:
    cargo run -p mdx-rust -- optimize example --iterations 1 --budget light --json
    cargo run -p mdx-rust -- audit example --json

hardening-smoke:
    cargo run -p mdx-rust -- doctor --json
    cargo run -p mdx-rust -- audit --json
    cargo run -p mdx-rust -- eval --spec examples/evals/cargo-check.json --json
    cargo run -p mdx-rust -- evidence --json
    cargo run -p mdx-rust -- improve crates/mdx-rust-analysis/src/hardening.rs --eval-spec examples/evals/cargo-check.json --json

plan-smoke:
    tmpdir="$(mktemp -d)"; mkdir -p "$tmpdir/src"; printf '[package]\nname = "mdx-plan-smoke"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nanyhow = "1"\n' > "$tmpdir/Cargo.toml"; printf 'mod config;\n\npub fn load() -> anyhow::Result<String> {\n    let value = std::fs::read_to_string("missing.toml").unwrap();\n    Ok(format!("{}{}", value, config::load_config()?))\n}\n' > "$tmpdir/src/lib.rs"; printf 'pub fn load_config() -> anyhow::Result<String> {\n    let value = std::fs::read_to_string("config.toml").unwrap();\n    Ok(value)\n}\n' > "$tmpdir/src/config.rs"; cd "$tmpdir" && plan_json="$(cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- plan src --json)" && plan_path="$(printf '%s' "$plan_json" | python3 -c 'import json,sys; print(json.load(sys.stdin)["artifact_path"])')" && candidate="$(printf '%s' "$plan_json" | python3 -c 'import json,sys; data=json.load(sys.stdin); print(next(c["id"] for c in data["candidates"] if c["status"] == "ApplyViaImprove"))')" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- explain "$plan_path" --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- apply-plan "$plan_path" --candidate "$candidate" --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- apply-plan "$plan_path" --all --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- apply-plan "$plan_path" --all --apply --json
    cargo run -p mdx-rust -- schema refactor-plan --json
    cargo run -p mdx-rust -- schema refactor-apply-run --json
    cargo run -p mdx-rust -- schema refactor-batch-apply-run --json
    cargo run -p mdx-rust -- schema evidence-run --json
    cargo run -p mdx-rust -- schema recipe-catalog --json
    cargo run -p mdx-rust -- schema artifact-explanation --json

autopilot-smoke:
    tmpdir="$(mktemp -d)"; mkdir -p "$tmpdir/src"; printf '[package]\nname = "mdx-autopilot-smoke"\nversion = "0.1.0"\nedition = "2021"\n\n[dependencies]\nanyhow = "1"\n' > "$tmpdir/Cargo.toml"; printf 'mod config;\n\npub fn load() -> anyhow::Result<String> {\n    let value = std::fs::read_to_string("missing.toml").unwrap();\n    Ok(format!("{}{}", value, config::load_config()?))\n}\n' > "$tmpdir/src/lib.rs"; printf 'pub fn load_config() -> anyhow::Result<String> {\n    let value = std::fs::read_to_string("config.toml").unwrap();\n    Ok(value)\n}\n' > "$tmpdir/src/config.rs"; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- map src --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- autopilot src --max-passes 2 --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- evolve src --budget 60s --min-evidence tested --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- autopilot src --apply --max-passes 2 --timeout-seconds 90 --json
    cargo run -p mdx-rust -- schema codebase-map --json
    cargo run -p mdx-rust -- schema autopilot-run --json

evidence-smoke:
    tmpdir="$(mktemp -d)"; export MDX_SMOKE_DIR="$tmpdir"; python3 -c 'import os, pathlib; root=pathlib.Path(os.environ["MDX_SMOKE_DIR"]); (root/"src").mkdir(parents=True); (root/"Cargo.toml").write_text("[package]\nname = \"mdx-evidence-smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"); (root/"src/lib.rs").write_text("pub fn labels(items: &[String]) -> Vec<&'\''static str> {\n    if items.len() == 0 {\n        return vec![\"shared boundary label\"];\n    }\n    vec![\n        \"shared boundary label\",\n        \"shared boundary label\",\n        \"shared boundary label\",\n    ]\n}\n\n#[test]\nfn smoke() {\n    assert_eq!(labels(&[String::from(\"x\")]).len(), 3);\n}\n")'; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- init && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- agent-contract --json && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- evidence --json && python3 -c 'import json, pathlib; root=pathlib.Path.cwd(); p=root/".mdx-rust/evidence/evidence-covered-fixture.json"; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(json.dumps({"schema_version":"0.8","run_id":"covered-smoke","root":str(root),"target":"src/lib.rs","grade":"Covered","analysis_depth":"Structural","metrics":[{"id":"coverage-percent","label":"Line coverage","value":92.0,"unit":"percent","source_command":"coverage"}],"commands":[],"unlocked_recipe_tiers":["Tier 2 structural mechanical recipes"],"unlock_suggestions":[],"note":"smoke fixture","artifact_path":str(p)}, indent=2))' && plan_json="$(cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- plan src/lib.rs --json)" && printf '%s' "$plan_json" | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["evidence"]["grade"] == "Covered"; assert any(c["recipe"] == "RepeatedStringLiteralConst" and c["status"] == "ApplyViaImprove" for c in data["candidates"]); assert any(c["recipe"] == "LenCheckIsEmpty" and c["status"] == "ApplyViaImprove" for c in data["candidates"])' && run_json="$(cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- evolve src/lib.rs --budget 60s --tier 2 --min-evidence covered --json)" && printf '%s' "$run_json" | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["status"] == "Reviewed"; assert data["execution_summary"]["validated_transactions"] >= 1'

hardened-evidence-smoke:
    tmpdir="$(mktemp -d)"; export MDX_SMOKE_DIR="$tmpdir"; python3 -c 'import os, pathlib; root=pathlib.Path(os.environ["MDX_SMOKE_DIR"]); (root/"src").mkdir(parents=True); (root/"Cargo.toml").write_text("[package]\nname = \"mdx-hardened-smoke\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"); body="pub fn clone_pressure(values: &[String]) -> Vec<String> {\n    let a = values[0].clone();\n    let b = values[1].clone();\n    let c = values[2].clone();\n"; body += "".join(f"    let _v{i} = {i};\n" for i in range(55)); body += "    vec![a, b, c]\n}\n"; (root/"src/lib.rs").write_text(body)' ; cd "$tmpdir" && cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- init && python3 -c 'import json, pathlib; root=pathlib.Path.cwd(); p=root/".mdx-rust/evidence/evidence-hardened-fixture.json"; p.parent.mkdir(parents=True, exist_ok=True); p.write_text(json.dumps({"schema_version":"0.8","run_id":"hardened-smoke","root":str(root),"target":"src/lib.rs","grade":"Hardened","analysis_depth":"Structural","metrics":[{"id":"coverage-percent","label":"Line coverage","value":95.0,"unit":"percent","source_command":"coverage"},{"id":"mutation-score-percent","label":"Mutation score","value":88.0,"unit":"percent","source_command":"mutation"}],"commands":[],"unlocked_recipe_tiers":["Tier 1 mechanical recipes","Tier 2 structural mechanical recipes","Tier 3 semantic planning candidates"],"unlock_suggestions":[],"note":"smoke fixture","artifact_path":str(p)}, indent=2))' && plan_json="$(cargo run --manifest-path "{{justfile_directory()}}/Cargo.toml" -p mdx-rust -- plan src/lib.rs --json)" && printf '%s' "$plan_json" | python3 -c 'import json,sys; data=json.load(sys.stdin); assert data["evidence"]["grade"] == "Hardened"; recipes=[c["recipe"] for c in data["candidates"]]; assert "ClonePressureReview" in recipes; assert "LongFunctionReview" in recipes'
