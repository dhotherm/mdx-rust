# Roadmap

`mdx-rust` is evolving from a Rust agent optimizer into a Rust-native
safe-change system for codebases.

The invariant stays the same: no change is trusted because an LLM or heuristic
suggested it. A change must be scoped, validated in isolation, measured against
the relevant policy or behavior signal, landed deliberately, and audited.

## v0.2.0 - Released

First serious safety-first release.

- CLI-first API stability contract.
- Explicitly unstable library APIs during the v1 beta.
- Versioned audit packets for accepted optimizer changes.
- Single-file edit scope hard-enforced for agent optimization.
- First narrow parser-validated Rust fallback edit strategy.
- Positive and negative end-to-end safety proof tests.
- Rustdoc and docs.rs gates in CI.
- Clear first-run and release-readiness documentation.

## v0.3.0 - Released

Safe scoped hardening for ordinary Rust modules.

Primary goals:

- `mdx-rust doctor` and `mdx-rust audit` work on normal Rust workspaces without
  agent registration.
- `mdx-rust improve [target]` proposes high-confidence hardening changes in
  review mode by default.
- `--apply` is required before hardening changes touch the real tree.
- Bounded hardening transactions snapshot every touched file and rollback on
  final validation failure.
- First hardening strategies focus on panic-prone `unwrap`/`expect` calls in
  `anyhow::Result` functions, risky process execution surfaces, unsafe code,
  and environment-derived config boundaries.
- Hardening reports use schema version `0.3` and include policy hash, workspace
  metadata, findings, proposed changes, validation records, transaction status,
  and rollback status.

## v0.4.0 - Released

Behavior and policy-driven improvement for Rust services.

- Workspace behavior eval specs in `.mdx-rust/evals.json`.
- `mdx-rust eval` runs deterministic command-based evals without requiring an
  agent registration.
- `mdx-rust improve --eval-spec` requires behavior evals to pass in isolation
  and after final application.
- Structured markdown policy parsing for backend safety rules such as no panics
  in request paths, contextual errors, validated external inputs, and boundary
  handling.
- `doctor` summarizes high, medium, and patchable findings and prints
  recommended next actions.
- Hardening reports include policy matches, behavior eval evidence, risk
  summaries, and generated schemas.

## v0.5.0 - Released

Guardrailed Rust refactoring assistant with impact analysis.

- Plan-first workflow: `mdx-rust plan`, human review, then
  `mdx-rust apply-plan` for supported low-risk candidates.
- Execution queue workflow: `mdx-rust apply-plan --all` can review or apply all
  executable low-risk candidates from a plan with per-step validation.
- Crate/module graph, touched-area model, and public API impact detection.
- Safe refactor recipe candidates such as extract function, split oversized
  module, consolidate error handling, isolate boundary validation, and apply
  patchable hardening.
- Stale-plan rejection through source snapshot hashes, plan hashes, and
  candidate hashes.
- Public API protection through semver and public API checks where applicable.
- Versioned refactor plan artifacts with plan hashes, required gates, risk
  summaries, and explicit non-goals.

## v0.6.0 - Released

Autonomous Rust evolution for safe executable work.

- `mdx-rust map` codebase intelligence with quality grade, debt score,
  evidence grade, capability gates, hardening findings, and recommended
  actions.
- `mdx-rust autopilot` review mode for non-mutating autonomous planning and
  execution simulation.
- `mdx-rust autopilot --apply` for multi-pass execution of low-risk candidates
  through the existing plan, apply-plan, hardening, validation, and rollback
  gates.
- `mdx-rust evolve --budget <time> --tier <n>` as the direct command coding
  agents can call for bounded autonomous Rust improvement.
- Candidate tiers and required evidence gates that control what can execute.
- Five compile-gated Tier 1 recipes: contextual error hardening, boundary error
  context propagation, private borrow parameter tightening, iterator clone
  cleanup, and `#[must_use]` annotation.
- Boundary-aware Tier 2 review candidates are surfaced only after `Tested`
  evidence so higher evidence changes what the engine discovers, not only what
  it may execute.
- Fresh planning before every autonomous apply pass.
- Versioned codebase map and autopilot artifacts for agents, CI, and future MCP
  or API surfaces.
- Optional gate detection for nextest, llvm-cov, mutants, and semver checks.

## v0.8.0 - Released

Agent-first evidence-driven evolution.

- Evidence artifacts include file/function profiles so plans can explain the
  evidence context behind each candidate.
- `mdx-rust recipes --json` exposes the typed recipe catalog, evidence
  thresholds, risk, executable status, and mutation path for external agents.
- `mdx-rust explain <artifact> --json` lets agents summarize evidence, map,
  plan, hardening, apply, and autopilot reports before choosing the next step.
- `mdx-rust scorecard <target> --json` gives agents one briefing artifact with
  map, plan, recipe catalog, autonomy readiness, and suggested next commands.
- Maps and plans include security posture scores and severity counts to
  prioritize risky modules without weakening safety gates.
- Plans include per-candidate autonomy decisions, and autonomous queues may
  execute only candidates marked `Allowed`.
- Security audit findings appear as review-oriented plan candidates so risky
  modules are visible inside the same planning surface.
- The agent contract advertises the new agent-facing commands and schema names.
- Mutation remains CLI/JSON-first. Runtime surfaces stay future work until the
  artifact contract has more field time.

## v1.0 beta Focus

Local agent runtime and candidate-evidence autonomy.

- `mdx-rust runtime --json` exposes a typed manifest for local agent
  transports, runtime tools, and mutation rules.
- `mdx-rust mcp --stdio` lets external coding agents call scorecard, evidence,
  map, plan, explain, recipes, and gated evolve over stdin/stdout.
- `mdx-rust serve --bind 127.0.0.1:3799` exposes the same local runtime over
  localhost HTTP for agents that prefer a socket API.
- `mdx-rust agent-pack codex|claude|generic` generates repo-local instruction
  files that teach agents to use mdx-rust without bypassing mutation gates.
- Candidate evidence context now includes an explicit status: unmeasured,
  compiled, tested, covered, mutation-backed, or proven.
- Evidence file profiles include coverage and mutation score fields when
  measured.
- Tier 2 gains Option boundary context propagation for simple
  `.ok_or("message")?` callsites inside `anyhow::Result` functions.
- Runtime mutation-capable calls require explicit confirmation and still route
  through evolve, autopilot, apply-plan, hardening transactions, validation,
  and rollback.

## v1.1 beta Focus - In Progress

Real-repo adoption and signal quality.

- Dogfood mdx-rust against mdx-rust itself and larger Rust backends before
  adding new broad refactoring promises.
- Reduce noisy security and readiness findings so `agent-ready`, `scorecard`,
  and `map` are useful on real workspaces with tests, fixtures, and detector
  code.
- Keep beta positioning explicit while improving first-run guidance and
  recommended next commands for both humans and external coding agents.
- Improve static scanner precision before increasing autonomous recipe scope.
- Make top risks and recommended actions prioritize true production callsites
  over examples, comments, string literals, and test harness code.
- Preserve the v1 contract: CLI JSON, schemas, local runtimes, and agent packs
  are the supported automation surface; Rust library APIs remain unstable
  during beta.

## Current Non-Goals

- Autonomous public API rewrites without explicit gates.
- External hook execution.
- Remote or multi-tenant MCP/A2A runtime integration.
- Multi-agent orchestration.
- Multi-language support.
- UI work.
