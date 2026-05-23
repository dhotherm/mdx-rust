# mdx-rust

**A Rust-native optimizer for LLM agents.**

Point `mdx-rust` at an existing Rust agent (or crate), give it a behavioral **policy**, and let it safely improve system prompts, tool definitions, decision logic, and model choices through structured experimentation — with compile-time safety gates at every step.

A production-grade, policy-driven optimizer for Rust LLM agents, with compile-time safety and deep code understanding as its core differentiators.

## Why mdx-rust?

- **Native Rust understanding** — Uses `syn` + `tree-sitter-rust` to actually read and reason about your agent’s code instead of treating it as text.
- **Safety first** — Every proposed change is validated with `cargo check`, `clippy`, and smoke tests inside isolated git worktrees or copies. No broken states.
- **Policy-driven** — Your domain rules, constraints, and quality expectations (`policies.md`) guide diagnosis, candidate generation, and scoring — not just “maximize the metric.”
- **Agent-friendly CLI** — Excellent human output by default + first-class `--json` mode so other coding agents can drive it.
- **Observable** — Full tracing of LLM calls, tool invocations, and decisions. Export to Jaeger, local files, or JSONL.
- **Single binary** — `cargo install mdx-rust` and you’re done.

## Quick Start (Planned)

```bash
# Install (once published)
cargo install mdx-rust

# In your Rust agent project
mdx-rust init

# Register your agent
mdx-rust register my-agent

# Generate aligned policy + eval spec + dataset
mdx-rust spec my-agent

# Run the optimization loop
mdx-rust optimize my-agent --iterations 5
```

Artifacts live in `.mdx-rust/agents/<name>/` (experiments, traces, reports, best version, etc.).

## How It Works

1. **Register** — Detects entrypoint (Rig agent, async fn, or generic JSON contract), creates a thin harness if needed, and smoke-tests invocation.
2. **Spec** — LLM analyzes your agent + existing policy → produces `policies.md`, `eval_spec.json`, and a starter dataset (synthetic or from your tests).
3. **Optimize** — Runs the agent on the dataset with deep tracing → scores outputs → strong model diagnoses failures grounded in your policy → generates targeted candidate patches → validates safely → keeps only net-positive changes with regression guards.
4. **Repeat** — Multiple iterations, holdout sets, experiment ledger.

## Status

This is an active build in progress (May 2026).

Current focus:
- Solid CLI foundation (`init`, `register`, `spec`, `optimize`, `doctor`)
- Safe editing + validation pipeline for Rust code
- Rig-first agent support + generic JSON fallback
- Excellent tracing and reporting

## Contributing

We welcome contributions, especially around:

- Better Rust code analysis (tree-sitter queries, syn visitors)
- New candidate generation strategies
- Support for additional agent frameworks
- Evaluation harnesses and scoring functions

See `AGENTS.md` for guidance on working in this codebase (especially important since this tool is designed to be used by agents).

## License

MIT

---

**The machine that improves the machines — in Rust.**