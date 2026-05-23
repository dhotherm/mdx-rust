# AGENTS.md — Guidance for Coding Agents Working on mdx-rust

This document exists because `mdx-rust` is itself a tool designed to be used by coding agents (Cursor, Claude Code, Zed, Aider, etc.). The bar for code quality, safety, and clarity is therefore higher than average.

## Core Principles

1. **Safety is non-negotiable**
   - Every change that touches code editing, patching, or validation must preserve the invariant: "we never leave the user's repo in a broken state."
   - Prefer git worktrees or isolated temp copies for experiments.
   - `cargo check` + `clippy` are hard gates, not suggestions.
   - Read and preserve `SAFETY_INVARIANTS.md` before changing optimization, hooks, scoring, patch application, validation, ledgers, or acceptance counters.

2. **Clarity over cleverness**
   - Rust code in this project should be boring, well-named, and easy for both humans and LLMs to understand.
   - When in doubt, add a small comment explaining *why* a decision was made (especially around analysis or safety).

3. **Agent-first + Human-first**
   - CLI output must be beautiful for humans by default.
   - Every command must support `--json` for reliable parsing by other agents.
   - Structured data (serde) everywhere it makes sense.

4. **Policy as truth**
   - When generating or evaluating changes, the contents of `policies.md` (or equivalent) are the source of truth for what "good" looks like.

5. **Learn from the ecosystem**
   - We study existing agent optimization and self-improvement tools for UX patterns, experiment tracking, and safety mechanisms, then adapt the strongest ideas to Rust's strengths.
   - We do not copy implementations — we adapt proven patterns to take advantage of Rust's safety and performance.

## Development Workflow

### Setup
```bash
just ci
```

If `just` is not installed, run the commands from the `ci` recipe in
`Justfile` directly.

### Testing Changes to the Optimizer
When modifying the core loop, analysis, or editing logic:
- Always add or update a test that exercises the safety path (i.e., a bad patch must be rejected).
- Preserve the invariant tests in `crates/mdx-rust-core/src/safety_pipeline.rs`: deny hooks cannot accept, net-negative candidates cannot land, and final validation failures roll back.
- Add or update property/invariant coverage when the change touches patch scope parsing, hook decisions, rollback, timeouts, path handling, or acceptance counters.
- For autonomous changes, preserve `map` and `autopilot` review mode as
  non-mutating and route autopilot apply mode through plan/apply-plan/hardening
  transactions only.
- Preserve evidence-gated execution. `evolve`, `autopilot`, and `apply-plan
  --all` may only reduce the queue by tier, budget, or evidence; they must not
  weaken validation or rollback.
- Use the `examples/` directory or `tests/fixtures/` for small Rig agents you can optimize against.

### Running the CLI during development
```bash
cargo run -- init
cargo run -- register my-test-agent ../path/to/small-agent
cargo run -- doctor my-test-agent
cargo run -- map src --json
cargo run -- autopilot src --json
cargo run -- evolve src --budget 60s --json
```

### Commit Style
- Use conventional commits or clear imperative style.
- Large refactors should be broken into reviewable steps.

## Rust Idioms We Enforce

- Make invalid safety states unrepresentable where practical. Prefer small
  newtypes or stage-specific structs over passing raw patches through the
  acceptance path.
- Library crates should expose typed errors for stable domains. `anyhow` is
  acceptable at CLI edges and for unstable internal orchestration.
- Any type that crosses a CLI JSON, audit packet, hook, strategy, trace, or
  future LLM boundary should derive `Serialize`, `Deserialize`, and
  `schemars::JsonSchema`.
- Public types in the documented facade require rustdoc explaining their role
  and stability posture.
- Use structured records for validation, provenance, and rejections instead of
  relying only on human note strings.

## Required Rails

- Use `just ci` before claiming a branch is ready.
- Use `just release-candidate` before a release tag or crates.io publish.
- Run `just audit` after dependency changes.
- Run `just machete` after adding or removing dependencies.
- Add dependencies with `cargo add` when working interactively, then review the
  exact `Cargo.toml` and `Cargo.lock` diff.
- Keep generated artifacts out of commits unless a documented release process
  says otherwise.

## Code Organization (Current)

- `src/main.rs` — CLI entrypoint (clap)
- `src/cli/` — subcommand implementations (to be created)
- `src/core/` — the optimization loop, experiment tracking, scoring
- `src/analysis/` — Rust source understanding (syn + tree-sitter)
- `src/llm/` — LLM client, prompt templates, structured extraction
- `src/tracing/` — instrumentation helpers
- `src/safety/` — worktree management, validation gates, patch application

(We started lean. We will split into `mdx-rust-core` crate when the library surface stabilizes.)

## Important Constraints

- MSRV: Rust 1.80+
- We target Rig agents first, then expand.
- We must remain `cargo install`-able as a single (or very small number of) binaries.
- Never depend on rust-analyzer or full rustc internals for v1 — tree-sitter + syn is the foundation.

## When Adding New Features

Ask yourself:
- Does this make the agent *safer* or more *observable*?
- Will a coding agent calling this via `--json` have a good experience?
- Does the change respect an existing `policies.md` when one is present?

## Questions?

If something feels ambiguous while working on the optimizer, the analysis layer, or safety, open an issue or ask in chat. The whole point of this project is to make self-improving agents reliable — the same standard applies to how we build the tool itself.

---

Welcome. Let's make something excellent.
