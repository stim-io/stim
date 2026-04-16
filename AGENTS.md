# AGENTS

## Purpose

This file manages two things only:

- the stable role of `stim/` as the product client and application boundary inside the workspace
- core constraints that should stay stable while the client framework evolves
- key file indexes for the most important design documents

Detailed framework and product thinking belongs in `docs/`, not here.

## Core Constraints

- `stim/` owns the product client/application layer, not the paired agent runtime or server-side business execution.
- Keep heavy communication, orchestration, and durable business logic behind server boundaries (`stim-server/` and `santi/`), not inside platform-specific client code.
- Treat Tauri as the desktop host/runtime boundary, not as the main product-logic home.
- Keep the web app, Tauri host, and local runtime-control surfaces separated by explicit boundaries rather than convenience-driven mixing.
- IPC/plugin commands are for local host control, discovery, diagnostics, and capability bridging; they must not become the primary business API surface.
- Keep inspection and scripted acceptance focused on stable boundary truth (attachment target, visible state, conversation reuse, error presence, message growth, visible content shape). Do not treat open-ended agent chat semantics as fully scriptable until those semantics have matured into a genuinely stable contract.
- Prefer `stim-dev` as the canonical local dev-loop, recovery, and acceptance entrypoint. If local iteration needs new restart or recovery behavior, add it there instead of relying on ad hoc process choreography.
- Real product/business communication should converge on explicit HTTP / SSE / WebSocket contracts exposed by owned services.
- Dev/prod differences belong in bootstrap, config, and provider/resource selection, not in the core identity of product features or control-plane contracts.
- Prefer a small number of stable client primitives over premature abstraction or framework layering.
- `stim-packages/` owns atomic Vue components, layout primitives, and theme definitions through package boundaries like `@stim-io/components`; `stim/` composes them into product-facing screens and business components but should not become the new home for visual styling logic.
- Keep shared product semantics explicit and durable; do not let platform-specific workarounds redefine product behavior.
- If a boundary is still unclear, prefer documenting the intended ownership split first, then let real implementation pressure refine it.

## Git / CI Baseline

- `main` should advance through PRs rather than direct pushes.
- Keep force-push protection and branch-deletion protection enabled for `main`.
- Keep squash merge as the default history strategy.
- Keep required green checks in front of merge once `.github/workflows/ci.yml` is active.

## Common Commands

- Format workspace: `pnpm exec prettier --write .`
- Check formatting: `pnpm exec prettier --check .`
- Start full local app loop: `cargo run -p stim-dev -- start`
- Start renderer-focused loop: `cargo run -p stim-dev -- start renderer`
- Force renderer re-optimization: `cargo run -p stim-dev -- start renderer --force`
- Start Tauri-focused loop: `cargo run -p stim-dev -- start tauri`
- Reattach Tauri to existing renderer: `cargo run -p stim-dev -- start tauri --reuse-renderer`
- Run renderer dev server directly: `pnpm -C apps/renderer dev`
- Build renderer directly: `pnpm -C apps/renderer build`
- Typecheck renderer directly: `pnpm -C apps/renderer typecheck`
- Run Tauri CLI directly: `pnpm -C apps/tauri tauri`

## Reference Project Index

### `santi`

- Role: core runtime/service reference beneath the product layer
- Repo path: `/Users/zqxy123/Projects/stim.io/modules/santi`

### `nexu-slim`

- Role: reference for desktop host vs sidecar vs IPC/HTTP boundary discipline
- Repo path: `/Users/zqxy123/Projects/giants.ai/nexu-slim`

## Key File Index

- `AGENTS.md`: stable constraints and file index
- `docs/operations/documentation.md`: must-read docs update guide, canonical-source rule, and anti-duplication process
- `docs/architecture/structure.md`: durable directory ownership and structure rules for app, service, and Tauri host code
- `docs/architecture/desktop/tauri-boundary.md`: boundary between the Tauri host, web app, and local runtime/service processes
- `docs/contracts/host/inspection.md`: host inspection, probe, and acceptance boundary for `stim-dev`
- `../../AGENTS.md`: repo-root workspace boundary across all attached repos

## Update Rules

- Put ongoing design reasoning into `docs/`.
- Keep `AGENTS.md` short and durable.
- Only add indexes here for files that are likely to remain central.
- Before changing doc structure or adding new docs, read `docs/operations/documentation.md` and follow its canonical-source, split/merge, and no-history-baggage rules.
