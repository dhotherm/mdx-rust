# mdx-rust

**A Rust-native optimizer for LLM agents.**

Point `mdx-rust` at an existing Rust agent (or crate), give it a behavioral **policy**, and let it safely improve system prompts, tool definitions, decision logic, and model choices through structured experimentation — with compile-time safety gates at every step.

A production-grade, policy-driven optimizer for Rust LLM agents, with compile-time safety and deep code understanding as its core differentiators.

## Why mdx-rust?

- **Native Rust understanding** — Uses `syn` + `tree-sitter-rust` to actually read and reason about your agent’s code instead of treating it as text.
- **Safety first** — Every proposed change is validated with `cargo check`, `clippy`, and smoke tests inside isolated git worktrees or copies. No broken states.
- **Policy-driven** — Your domain rules, constraints, and quality expectations (`policies.md`) guide diagnosis, candidate generation, and scoring — not just “maximize the metric.”
- **Agent-friendly CLI** — Excellent human output by default + first-class `--json` mode so other coding agents can drive it.
- **Observable** — First-class trace records with span ids, parent ids, latency, token/cost fields, redaction flags, and links to candidate edits.
- **Disciplined lifecycle** — Built-in hook stages for pre-edit, pre-command, post-validation, and pre-accept decisions.
- **Audit aware** — Deterministic static audit checks surface risky agent surfaces like process execution, secret literals, unsafe code, and MCP/A2A-style integration boundaries.
- **Single binary** — `cargo install mdx-rust` and you’re done.

## Quick Start

Clone the repo and try the built-in example (the fastest way to see mdx-rust in action):

```bash
git clone https://github.com/dhotherm/mdx-rust
cd mdx-rust

# The example rig-minimal-agent starts in a deliberately weak state
cargo run -p mdx-rust -- register example
cargo run -p mdx-rust -- optimize example --iterations 2
cargo run -p mdx-rust -- audit example

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
4. **Gate** — Every candidate moves through explicit lifecycle stages: pre-edit, isolated validation, patched evaluation, pre-accept hook, final validation, and rollback on failure.
5. **Repeat** — Multiple iterations, budgeted candidate pools, holdout splits, and experiment ledgers.

## Key Commands

```bash
mdx-rust init
mdx-rust register my-agent ./path/to/agent
mdx-rust spec my-agent
mdx-rust doctor my-agent
mdx-rust audit my-agent
mdx-rust eval my-agent --dataset .mdx-rust/agents/my-agent/dataset.json
mdx-rust optimize my-agent --iterations 3 --budget medium --review
```

Every command that matters supports `--json` for coding agents and automation.

## Safety Model

The optimizer is built around a conservative lifecycle:

1. Analyze source scope with Rust-aware finders.
2. Run the agent on a train split and record trace diagnoses.
3. Generate typed candidate strategies.
4. Build a targeted edit only when a safe planner exists.
5. Run built-in lifecycle hooks.
6. Apply and validate in an isolated workspace.
7. Score the patched workspace.
8. Land only net-positive changes.
9. Run final validation on the real tree.
10. Roll back if final validation fails.

Experiment records include dataset version/hash, scorer version, git SHAs, validation commands, score deltas, hook decisions, holdout score, and prompt variant ledger entries.

The full acceptance contract is documented in [SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md).

## Status

**Active and usable (May 2026).**

mdx-rust can already:
- Register Rig and generic agents
- Run them with tracing
- Perform deep Rust analysis (prompts, tools, entrypoints)
- Run LLM-driven diagnosis with structured candidates
- Safely propose, validate (cargo check + clippy in isolation), and accept improvements
- Execute typed strategies for prompts, fallback behavior, output schemas, and tool guidance
- Split evaluation data into train/holdout sets under `light`, `medium`, or `heavy` budgets
- Record prompt variant ledgers and lifecycle hook decisions
- Run deterministic static security audits
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
