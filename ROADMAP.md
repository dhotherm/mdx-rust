# Roadmap

`mdx-rust` is an early Rust-native optimizer for LLM agents. The project is
staying narrow on purpose: go deep on Rust safety, provenance, and evaluation
before expanding into broader agent orchestration.

## v0.2.0 Focus

`v0.2.0` is the first serious safety-first release candidate.

Primary goals:

- CLI-first API stability contract.
- Explicitly unstable library APIs before `1.0`.
- Versioned audit packets for accepted changes.
- Single-file edit scope hard-enforced and documented.
- First narrow parser-guarded Rust fallback edit strategy.
- Positive and negative end-to-end safety proof tests.
- Rustdoc and docs.rs gates in CI.
- Clear first-run and release-readiness documentation.

## Current Non-Goals

- Multi-file accepted edits.
- Stable SDK APIs.
- External hook execution.
- MCP/A2A runtime integration.
- Multi-agent orchestration.
- Multi-language support.
- TUI or UI work.

## Next After v0.2

- Complete scored standalone `mdx-rust eval`.
- Add a narrow structural Rust edit strategy using `syn`.
- Add transaction snapshots for future multi-file edits.
- Add richer policy/evaluation language support.
- Add signed or attestable provenance export.
- Add optional security gates that can turn audit findings into hook denials.
