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
- Model local launcher-managed surfaces as sidecar app instances where that improves startup, inspection, and cleanup symmetry; use `sidecar-mode` with only `dev` and `runtime` values, and do not use `runtime-mode` naming for that concept.
- Keep `crates/platform` limited to platform facts such as paths, processes, networking, environment, locks, and OS detection.
- Keep `crates/sidecar` limited to sidecar identity, namespace, layout, stamp, live inspect, and stamped-process cleanup concepts; it must not become a business API layer.
- Keep stamp identity intentionally small: `app + namespace + sidecar-mode + source`. Role, instance id, endpoint, health, and richer lifecycle facts belong in ready-line / inspect communication, not argv.
- Do not persist runtime truth in state files such as `state.json`, `runtime.json`, or `heartbeat.json`; live inspect/probe/health surfaces are the source of current runtime truth, while stamps define cleanup ownership and locks define startup exclusion only.
- Keep inspection focused on stable boundary truth (attachment target, visible state, error presence, message growth, visible content shape). Do not treat open-ended agent chat semantics as fully scriptable until those semantics have matured into a genuinely stable contract.
- Prefer `stim-dev` as the canonical local dev-loop, recovery, status, and inspection entrypoint. If local iteration needs new restart or recovery behavior, add it there instead of relying on ad hoc process choreography.
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

- Install or refresh the local dev-loop CLI: `cargo install --path tools/stim-dev --force`
- Normal `stim-dev` usage should call the installed CLI directly; use `cargo run -p stim-dev -- ...` only as a local fallback while iterating on `stim-dev` itself or debugging a narrow command implementation.
- Format workspace: `pnpm exec prettier --write .`
- Check formatting: `pnpm exec prettier --check .`
- Start full local app loop: `stim-dev start`
- Start controller-focused loop: `stim-dev start controller`
- Start renderer-focused loop: `stim-dev start renderer`
- Start Tauri-focused loop: `stim-dev start tauri`
- Restart full local app loop: `stim-dev restart`
- Restart renderer-focused loop: `stim-dev restart renderer`
- Inspect live runtime status: `stim-dev status`
- Inspect host/window state: `stim-dev inspect tauri host`
- Inspect renderer landing state: `stim-dev inspect renderer landing`
- Inspect renderer messaging state: `stim-dev inspect renderer messaging`
- Capture main-window evidence: `stim-dev inspect tauri screenshot [label]`
- Run renderer delivery wrapper directly: `cargo run -p stim-renderer -- serve --dev --stim-stamp-app=renderer --stim-stamp-namespace=default --stim-stamp-mode=dev --stim-stamp-source=tool:stim-dev`
- List stamped sidecar processes for the fallback namespace: `stim-dev list`
- Stop stamped sidecar processes for the fallback namespace: `stim-dev stop`
- Reset stamped sidecar namespace residue for the fallback namespace: `stim-dev reset`
- Inspect packaged runtime sidecar plan: `cargo run -p stim-packaged -- --plan --namespace default`
- Start packaged runtime composition: `cargo run -p stim-packaged -- launch all --namespace default`
- Start packaged controller sidecar foreground loop: `cargo run -p stim-packaged -- launch controller --namespace default`
- Start packaged renderer delivery sidecar: `cargo run -p stim-packaged -- launch renderer --namespace default`
- Start packaged Tauri host sidecar: `cargo run -p stim-packaged -- launch tauri --namespace default`
- Run renderer Vite app directly (wrapper preferred): `pnpm -C apps/renderer/vite dev`
- Build renderer Vite app directly: `pnpm -C apps/renderer/vite build`
- Typecheck renderer Vite app directly: `pnpm -C apps/renderer/vite typecheck`
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
- `docs/contracts/host/inspection.md`: host status and inspection boundary for `stim-dev`
- `apps/packaged/`: thin packaged/runtime launcher entry and packaged sidecar assembly plan
- `apps/renderer/`: renderer delivery sidecar wrapper plus Vite app under `apps/renderer/vite/`
- `crates/platform/`: platform primitive crate for path/process/network/env/lock/OS facts
- `crates/sidecar/`: sidecar namespace, layout, ready/inspect, and 4-field stamp primitive crate
- `../../AGENTS.md`: repo-root workspace boundary across all attached repos

## Update Rules

- Put ongoing design reasoning into `docs/`.
- Keep `AGENTS.md` short and durable.
- Only add indexes here for files that are likely to remain central.
- Before changing doc structure or adding new docs, read `docs/operations/documentation.md` and follow its canonical-source, split/merge, and no-history-baggage rules.
