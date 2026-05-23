# rig-minimal-agent

A tiny Rig-based agent used to dogfood and test `mdx-rust`.

## Purpose

This agent exists so we have a real, small, controllable target for:

- `mdx-rust register`
- `mdx-rust doctor`
- Future optimization loop testing

It supports both real LLM calls (when `OPENAI_API_KEY` is set) and a deterministic fallback.

## Running

```bash
cargo run -p rig-minimal-agent
```

Pipe JSON like:

```json
{"query": "What is the capital of France?", "context": null}
```

## Contract

The main entrypoint is:

```rust
pub async fn run_agent(input: AgentInput) -> anyhow::Result<AgentOutput>
```

This is the kind of clean contract `mdx-rust` is designed to detect and work with.