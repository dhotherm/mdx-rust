# Release Readiness

This document is the release checklist for `v1.0.0 beta`.

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

The public crate line is expected to move from `0.9.0` to `1.0.0`. Release
notes should frame this as the v1 beta contract, not as a claim that the
library crates are a stable SDK.

The release notes should also explain the supported automation contract:
CLI commands, versioned JSON artifacts, schemas, local MCP/HTTP runtime
surfaces, and agent packs. They should not imply that `mdx-rust-core` or
`mdx-rust-analysis` are stable SDKs.

## Dependency Posture

`mdx-rust` intentionally uses a moderate dependency tree because it needs Rust
parsing, CLI ergonomics, async process execution, and optional model-provider
support. Before `v1.0.0 beta` is published:

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
mdx-rust schema behavior-eval-report --json >/tmp/mdx-rust-behavior-schema.json
mdx-rust schema project-policy --json >/tmp/mdx-rust-policy-schema.json
mdx-rust schema evidence-run --json >/tmp/mdx-rust-evidence-schema.json
mdx-rust schema agent-contract --json >/tmp/mdx-rust-agent-contract-schema.json
mdx-rust schema agent-runtime-manifest --json >/tmp/mdx-rust-runtime-schema.json
mdx-rust schema agent-pack --json >/tmp/mdx-rust-agent-pack-schema.json
mdx-rust schema agent-ready-report --json >/tmp/mdx-rust-agent-ready-schema.json
mdx-rust schema recipe-catalog --json >/tmp/mdx-rust-recipe-catalog-schema.json
mdx-rust schema artifact-explanation --json >/tmp/mdx-rust-artifact-explanation-schema.json
mdx-rust schema evolution-scorecard --json >/tmp/mdx-rust-evolution-scorecard-schema.json
mdx-rust schema refactor-plan --json >/tmp/mdx-rust-refactor-schema.json
mdx-rust schema refactor-apply-run --json >/tmp/mdx-rust-refactor-apply-schema.json
mdx-rust schema refactor-batch-apply-run --json >/tmp/mdx-rust-refactor-batch-apply-schema.json
mdx-rust schema codebase-map --json >/tmp/mdx-rust-codebase-map-schema.json
mdx-rust schema autopilot-run --json >/tmp/mdx-rust-autopilot-schema.json
mdx-rust eval --json >/tmp/mdx-rust-eval.json
mdx-rust agent-contract --json >/tmp/mdx-rust-agent-contract.json
mdx-rust runtime --json >/tmp/mdx-rust-runtime.json
mdx-rust agent-pack codex --json >/tmp/mdx-rust-agent-pack.json
mdx-rust agent-ready --json >/tmp/mdx-rust-agent-ready.json
mdx-rust recipes --json >/tmp/mdx-rust-recipes.json
mdx-rust scorecard --json >/tmp/mdx-rust-scorecard.json
mdx-rust evidence --json >/tmp/mdx-rust-evidence.json
mdx-rust doctor --json >/tmp/mdx-rust-doctor.json
mdx-rust map --json >/tmp/mdx-rust-map.json
mdx-rust autopilot --json >/tmp/mdx-rust-autopilot.json
mdx-rust evolve --budget 60s --json >/tmp/mdx-rust-evolve.json
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
If the optimizer accepts a change against the example fixture, restore
`examples/rig-minimal-agent/src/main.rs` before committing so future smoke runs
continue to exercise the improvement path.

## Hardening Smoke

From a clean checkout:

```bash
cargo run -p mdx-rust -- doctor --json
cargo run -p mdx-rust -- audit --json
cargo run -p mdx-rust -- eval --spec examples/evals/cargo-check.json --json
cargo run -p mdx-rust -- improve crates/mdx-rust-analysis/src/hardening.rs --eval-spec examples/evals/cargo-check.json --json
```

Confirm that review mode does not mutate the working tree and that any proposed
change has isolated validation command records and behavior eval evidence when
an eval spec is supplied.

## Refactor Plan Smoke

From a clean checkout:

```bash
cargo run -p mdx-rust -- plan crates/mdx-rust-core/src/refactor.rs --json
cargo run -p mdx-rust -- map crates/mdx-rust-core/src/refactor.rs --json
cargo run -p mdx-rust -- scorecard crates/mdx-rust-core/src/refactor.rs --json
cargo run -p mdx-rust -- autopilot crates/mdx-rust-core/src/refactor.rs --json
cargo run -p mdx-rust -- evolve crates/mdx-rust-core/src/refactor.rs --budget 60s --json
cargo run -p mdx-rust -- schema refactor-plan --json
cargo run -p mdx-rust -- schema refactor-apply-run --json
cargo run -p mdx-rust -- schema refactor-batch-apply-run --json
cargo run -p mdx-rust -- schema evidence-run --json
cargo run -p mdx-rust -- schema agent-contract --json
cargo run -p mdx-rust -- schema evolution-scorecard --json
cargo run -p mdx-rust -- schema codebase-map --json
cargo run -p mdx-rust -- schema autopilot-run --json
```

Confirm that the plan writes an artifact under `.mdx-rust/plans/`, reports
candidate risk, and does not mutate the working tree.

From a throwaway crate with a patchable candidate, also confirm:

```bash
mdx-rust plan src/lib.rs --json
mdx-rust apply-plan .mdx-rust/plans/<plan>.json --candidate <candidate-id> --json
mdx-rust apply-plan .mdx-rust/plans/<plan>.json --candidate <candidate-id> --apply --json
mdx-rust apply-plan .mdx-rust/plans/<plan>.json --all --json
mdx-rust apply-plan .mdx-rust/plans/<plan>.json --all --apply --json
mdx-rust map src --json
mdx-rust scorecard src --json
mdx-rust autopilot src --json
mdx-rust autopilot src --apply --json
mdx-rust evolve src --budget 60s --min-evidence tested --json
mdx-rust evolve src --budget 60s --tier 1 --apply --json
```

The review run must not mutate source files. The apply run must route through
the hardening transaction, reject stale source snapshots, and write an
apply-plan report. The `--all` run must process only executable low-risk
candidates, preserve review mode as non-mutating, and stop apply mode on the
first failed step.

The autopilot review run must not mutate source files. The autopilot apply run
must write a codebase map, fresh plan artifact, batch apply report, and
autopilot report. It must replan between apply passes and report quality
before/after when it changes the tree.

The evolve run with `--min-evidence tested` should refuse Tier 1 execution when
the fixture has only compiled evidence, and it must leave source files
unchanged. The evolve apply run should then execute Tier 1 candidates with the
compiled evidence default.

The v1.0 beta recipe smoke should include at least contextual error hardening,
boundary error context propagation, private borrow parameter tightening,
iterator clone cleanup, `#[must_use]` annotation, and the covered Tier 2
Option context propagation recipe in a throwaway crate. The apply run must show
each recipe only lands after isolated validation and final validation.

The evidence smoke should include a measured `Tested` fixture with a
review-only boundary finding and prove the plan surfaces a Tier 2 plan-only
candidate without making it executable.

The Tier 2 smoke should include a measured `Covered` fixture and prove that
repeated private string literal extraction and `len() == 0` to `is_empty()`
candidates, plus Option boundary context propagation, become executable only
when the caller requests Tier 2 and `--min-evidence covered`.

The runtime smoke should prove `mdx-rust mcp --stdio` can list tools, can run a
read-only tool, and rejects mutation-capable `evolve` calls with `apply=true`
unless mutation confirmation is present. `mdx-rust serve` must refuse non-local
bind addresses.

Runtime pressure testing must also verify that MCP and HTTP calls expose only
the wrapper contract: read-only tools do not mutate, mutation-capable tools
require explicit confirmation, and successful mutation calls produce the same
saved autopilot, apply-plan, hardening, validation, and rollback artifacts as
the equivalent CLI path.

The hardened evidence smoke should include a measured `Hardened` fixture and
prove that clone-pressure and long-function review findings appear where a
basic evidence scan would not surface them.

## Performance Sanity

Record rough timings before release:

- CLI startup with `mdx-rust --version`.
- Fresh `mdx-rust init`.
- One `optimize --iterations 1 --budget light` run on the example agent.
- One `improve <small-rust-file>` review run.
- One `plan <small-rust-directory>` run.
- One `map <small-rust-directory>` run.
- One `autopilot <small-rust-directory>` review run.
- One `evolve <small-rust-directory> --budget 60s` review run.

The exact numbers depend heavily on Cargo cache warmth. The release bar is not a
micro-benchmark; it is that the CLI starts promptly and the example optimization
does not hang or produce confusing output.

## Publish Order

Do not publish `v1.0.0 beta` until the candidate commit has passed external pressure
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
