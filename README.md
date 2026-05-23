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

## Quick Start

Clone the repo and try the built-in example (the fastest way to see mdx-rust in action):

```bash
git clone https://github.com/dhotherm/mdx-rust
cd mdx-rust

# The example rig-minimal-agent starts in a deliberately weak state
cargo run -p mdx-rust -- register example
cargo run -p mdx-rust -- optimize example --iterations 2

# See the improvement
cargo run -p mdx-rust -- invoke example --input '{"query":"What is 9 + 10?"}'
```

You should see the optimizer detect the weak echo behavior, strengthen the system prompt with explicit reasoning instructions, validate the change safely, and produce a measurable lift in score.

Full flow for your own agent:

```bash
cd your-rust-agent
/path/to/mdx-rust init
/path/to/mdx-rust register my-agent
/path/to/mdx-rust optimize my-agent --iterations 5 --review
```

Artifacts (traces, diagnoses, candidates, reports, diffs) live under `.mdx-rust/agents/<name>/`.

## How It Works

1. **Register** — Detects entrypoint (Rig agent, async fn, or generic JSON contract), creates a thin harness if needed, and smoke-tests invocation.
2. **Spec** — LLM analyzes your agent + existing policy → produces `policies.md`, `eval_spec.json`, and a starter dataset (synthetic or from your tests).
3. **Optimize** — Runs the agent on the dataset with deep tracing → scores outputs → strong model diagnoses failures grounded in your policy → generates targeted candidate patches → validates safely → keeps only net-positive changes with regression guards.
4. **Repeat** — Multiple iterations, holdout sets, experiment ledger.

## Status

**Active and usable (May 2026).**

mdx-rust can already:
- Register Rig and generic agents
- Run them with tracing
- Perform deep Rust analysis (prompts, tools, entrypoints)
- Run LLM-driven diagnosis with structured candidates
- Safely propose, validate (cargo check + clippy in isolation), and accept improvements
- Support human review (`--review`)
- Produce experiment reports and artifacts

The built-in example demonstrates a real before/after optimization win.

See [PROGRESS.md](./PROGRESS.md) for the detailed build log.

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