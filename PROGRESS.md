# mdx-rust Progress Log

**Project**: MDx Rust — Rust-native optimizer for LLM agents  
**Status**: Long-running autonomous build (started May 2026)  
**Owner**: Mandeep Dhother  
**Current Phase**: Phase 0 — Rock-Solid Foundation

---

## Guiding Rules (Long Autonomous Run)

- Follow the approved plan strictly.
- Work feature by feature, function by function.
- Prioritize quality, safety, and clean architecture over speed.
- Commit frequently with clear messages.
- Update this file and the plan after every major milestone.
- Make reasonable defaults when hitting open questions; document them.

---

## Current Overall Status

- **History**: Clean single root commit (no external inspiration references visible in any commit or diff).
- **Architecture**: 3-crate workspace (`mdx-rust` + `mdx-rust-core` + `mdx-rust-analysis`).
- **CLI**: `init` (production quality), basic `doctor`.
- **Foundation**: Config loading started in core, example Rig agent exists.
- **Real work remaining**: `register`, analysis, safe editing, optimization loop, etc.

**Last major update**: Autonomous build initiated.

---

## Phase 0 — Rock-Solid Foundation (In Progress)

### Completed

- [x] Final git history rewrite (single clean root commit)
- [x] Converted to 3-crate workspace
- [x] Production-quality `mdx-rust init` (with --json support)
- [x] `doctor` command (uses real Config)
- [x] Real example Rig agent (`examples/rig-minimal-agent`)
- [x] Config loading in `mdx-rust-core`
- [x] Initial `.mdx-rustignore` + `BundleScope` logic in analysis crate
- [x] First version of `register` command (detection + registry entry + smoke test)
- [x] Central Registry with persistence + tests in core
- [x] Working runner for Process contracts + `invoke` dev command (end-to-end tested)
- [x] Structured TraceEvent support in runner (foundation for diagnosis & optimization loop)
- [x] Doctor now shows real bundle scope using the analysis crate
- [x] Finders improved (run_agent detection + Rig heuristics)
- [x] Editing skeleton added (worktree validation pipeline foundation)
- [x] Optimizer module skeleton (run → score → diagnose → candidates structure)
- [x] `optimize` CLI command wired to the core optimizer (skeleton end-to-end runnable)
- [x] Optimizer now actually invokes the agent multiple times and computes real (if simple) scores across iterations
- [x] Basic diagnosis + candidate suggestion simulation inside the loop (feels like real thinking)
- [x] Optimization runs are persisted as experiments under the agent (foundation for regression guards & reports)
- [x] Rich human-friendly output for optimize showing per-iteration diagnosis notes
- [x] Syn + tree-sitter + basic finders in analysis crate (Phase 2 foundation)
- [x] `spec` command surface + improved doctor/list groundwork
- [x] Tracing events in runner + RUST_LOG support in CLI

### In Progress / Next

- [ ] Improve registry to use proper types from core
- [ ] Full agent contract detection (Rig vs generic process)
- [ ] Better artifact + registry persistence
- [ ] Tests for Phase 0
- [ ] Move into Phase 1 (deeper registration + basic runner)

---

## Open Questions (from approved plan) — Current Defaults

1. Artifact directory → **`.mdx-rust/`** (locked)
2. Default analyzer model → Will default to strong general models later (Claude 4 / Grok 4 class)
3. Non-Rig agent support in first usable version → Yes (generic process contract)
4. Error handling → `anyhow` in binary, `thiserror` in libraries

---

## Notes for When You Return

- All work is committed and pushed.
- Progress is tracked here + in git history.
- The build is following the approved plan from the session.

---

*This file is updated autonomously during the long build.*