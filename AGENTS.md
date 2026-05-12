# AGENTS

## Purpose

This file manages two things only:

- the stable role of `stim/` as the product client and application boundary inside the workspace
- core constraints that should stay stable while the client framework evolves
- key file indexes for the most important design documents

Detailed framework and product thinking belongs in `docs/`, not here.

## Core Constraints

- `stim/` owns the product client/application layer, not the paired agent runtime or server-side business execution.
- `stim/` work should start from real Agent-Native IM workflow slices that exercise the path through the app, controller/service boundaries, and `santi`; avoid isolated client wins that do not improve that loop.
- Keep UI usable enough for daily workflow validation: reliable input, message display, reload/continuation, error visibility, and inspection. Defer visual polish unless it unlocks the core workflow.
- Keep heavy communication, orchestration, and durable business logic behind server boundaries (`stim-server/` and `santi/`), not inside platform-specific client code.
- Agent orchestration (the carrier-agnostic local agent-instance management sidecar that orchestrates `santi` instances and publishes discovery / registration / heartbeat facts to `stim-server`) lives in the sibling [`stim-agents`](https://github.com/stim-io/stim-agents) repo, not here. The renderer in this repo consumes that sidecar via its HTTP surface; do not re-introduce agent orchestration concerns into this workspace.
- Chat surfaces should consume product-visible participants and selection events from `stim-server`, not renderer/controller-local active selection. Controller delivery may resolve the chosen `participant_id` through `stim-server` into a protocol endpoint id.
- Treat Tauri as the desktop host/runtime boundary, not as the main product-logic home.
- Keep the web app, Tauri host, and local runtime-control surfaces separated by explicit boundaries rather than convenience-driven mixing.
- IPC/plugin commands are for local host control, discovery, diagnostics, and capability bridging; they must not become the primary business API surface.
- Model local launcher-managed surfaces as sidecar app instances where that improves startup, inspection, and cleanup symmetry; use `sidecar-mode` with only `dev` and `runtime` values, and do not use `runtime-mode` naming for that concept.
- Consume platform and sidecar primitives from sibling `../stim-crates`; do not reintroduce local ownership for path/process/network/env/lock/OS facts or sidecar identity/layout/stamp/cleanup primitives inside `stim`.
- Keep stamp identity intentionally small: `app + namespace + sidecar-mode + source`. Role, instance id, endpoint, health, and richer lifecycle facts belong in ready-line / inspect communication, not argv.
- Do not persist runtime truth in state files such as `state.json`, `runtime.json`, or `heartbeat.json`; live inspect/probe/health surfaces are the source of current runtime truth, while stamps define cleanup ownership and locks define startup exclusion only.
- Keep inspection focused on stable boundary truth (attachment target, visible state, error presence, message growth, visible content shape). Do not treat open-ended agent chat semantics as fully scriptable until those semantics have matured into a genuinely stable contract.
- Use the external `sidecar` CLI as the canonical local dev-loop, recovery, status, and inspection entrypoint. Lifecycle must close in `sidecar.toml`; provider-owned behavior must be exposed as explicit `sidecar inspect <target> <event> [payload]` events.
- Sidecar inspect events, controller operation events, and deterministic/local checks are development accelerators; they must support, not replace, the real end-to-end `stim -> santi` product loop.
- `apps/controller/` may own a local message-operation event layer for controller/runtime coverage, debugging, and acceptance through provider-owned inspect events. That layer is independent from `stim-server` product-ledger events and must not become the durable product IM ledger.
- Controller operation events may correlate to product-ledger facts and `santi` runtime facts, but they should not erase those layers or pretend to be their source of truth.
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
- Keep required green checks in front of merge once `.github/workflows/guard.yml` is active.

## Common Commands

- Format workspace: `pnpm exec prettier --write .`
- Check formatting: `pnpm exec prettier --check .`
- Run repo guard: `pnpm run guard` (Rust fmt/controller-tool tests plus client and renderer typechecks)
- Run code flavor guard: `flavor check --root . --config flavor.json` or `pnpm run guard:style`
- Inspect sidecar plan: `sidecar plan --config sidecar.toml --format json`
- Start full local app loop: `sidecar start --config sidecar.toml`
- Start controller-focused loop: `sidecar start controller --config sidecar.toml`
- Restart full local app loop: `sidecar restart --config sidecar.toml`
- Inspect live runtime status: `sidecar status --config sidecar.toml --format json`
- Inspect controller runtime: `sidecar inspect controller runtime.snapshot --config sidecar.toml --format json`
- Run controller messaging acceptance: `sidecar inspect controller accept.messaging --config sidecar.toml --inspect-timeout 60 --format json`
- Inspect host/window state: `sidecar inspect tauri host.snapshot --config sidecar.toml --format json`
- Inspect renderer projection via the Tauri host: `sidecar inspect tauri renderer.probe '{"request_id":"manual-probe","requested_at":"manual","probe":{"probe":"messaging-state"}}' --config sidecar.toml --format json`
- Capture main-window evidence via the Tauri host: `sidecar inspect tauri host.screenshot '{"request_id":"manual-screenshot","requested_at":"manual","label":"manual"}' --config sidecar.toml --format json`
- Run renderer delivery wrapper directly: `cargo run -p stim-renderer -- serve --dev --sidecar-stamp-app=renderer --sidecar-stamp-namespace=default --sidecar-stamp-mode=dev --sidecar-stamp-source=tool:sidecar`
- List sidecar processes for the fallback namespace: `sidecar list --config sidecar.toml`
- Stop sidecar processes for the fallback namespace: `sidecar stop --config sidecar.toml`
- Reset sidecar namespace residue for the fallback namespace: `sidecar reset --config sidecar.toml`
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
- `docs/contracts/host/inspection.md`: host status and provider-owned `sidecar inspect` boundary
- `docs/contracts/controller/message-operation-events.md`: controller-owned message-operation event contract for local app-loop coverage and acceptance
- `apps/packaged/`: thin packaged/runtime launcher entry and packaged sidecar assembly plan
- `apps/renderer/`: renderer delivery sidecar wrapper plus Vite app under `apps/renderer/vite/`
- `../stim-agents/`: standalone agent orchestration sidecar (Rust HTTP service that manages local `santi` instances and publishes facts to `stim-server`); consumed by this repo's renderer over HTTP only
- `../stim-crates/`: standalone Rust platform / sidecar primitive crates consumed by this repo's apps
- `.github/workflows/guard.yml`: required guard workflow
- `../../AGENTS.md`: repo-root workspace boundary across all attached repos

## Update Rules

- Put ongoing design reasoning into `docs/`.
- Keep `AGENTS.md` short and durable.
- Only add indexes here for files that are likely to remain central.
- Before changing doc structure or adding new docs, read `docs/operations/documentation.md` and follow its canonical-source, split/merge, and no-history-baggage rules.
