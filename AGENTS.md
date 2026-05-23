# AGENTS.md — Guidance for Coding Agents Working on mdx-rust

This document exists because `mdx-rust` is itself a tool designed to be used by coding agents (Cursor, Claude Code, Zed, Aider, etc.). The bar for code quality, safety, and clarity is therefore higher than average.

## Core Principles

1. **Safety is non-negotiable**
   - Every change that touches code editing, patching, or validation must preserve the invariant: "we never leave the user's repo in a broken state."
   - Prefer git worktrees or isolated temp copies for experiments.
   - `cargo check` + `clippy` are hard gates, not suggestions.

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
cargo build
cargo test
cargo clippy -- -D warnings
```

### Testing Changes to the Optimizer
When modifying the core loop, analysis, or editing logic:
- Always add or update a test that exercises the safety path (i.e., a bad patch must be rejected).
- Use the `examples/` directory or `tests/fixtures/` for small Rig agents you can optimize against.

### Running the CLI during development
```bash
cargo run -- init
cargo run -- register my-test-agent ../path/to/small-agent
cargo run -- doctor my-test-agent
```

### Commit Style
- Use conventional commits or clear imperative style.
- Large refactors should be broken into reviewable steps.

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