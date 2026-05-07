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
- Prefer `stim-dev` as the canonical local dev-loop, recovery, status, and inspection entrypoint. If local iteration needs new restart or recovery behavior, add it there instead of relying on ad hoc process choreography.
- `stim-dev` acceptance, controller operation events, and deterministic/local checks are development accelerators; they must support, not replace, the real end-to-end `stim -> santi` product loop.
- `apps/controller/` may own a local message-operation event layer for controller/runtime coverage, debugging, and acceptance through `stim-dev`. That layer is independent from `stim-server` product-ledger events and must not become the durable product IM ledger.
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

- Install or refresh the local dev-loop CLI: `cargo install --path ../stim-dev --force`
- Normal `stim-dev` usage calls the installed CLI directly with `STIM_WORKSPACE_ROOT="$(pwd)"` set inside this repo, or runs via `cargo run --manifest-path ../stim-dev/Cargo.toml -- ...` from a cwd inside this repo so workspace discovery resolves correctly. The CLI lives in the sibling `../stim-dev` repo; it is no longer a Cargo member of this workspace.
- Format workspace: `pnpm exec prettier --write .`
- Check formatting: `pnpm exec prettier --check .`
- Run repo guard: `pnpm run guard` (Rust fmt/controller-tool tests plus client and renderer typechecks)
- Run codestyle attribute guard: `cargo run --locked --manifest-path ../stim-guard/Cargo.toml -- check --root . --config stim-guard.json` or `pnpm run guard:style`
- Detect standalone prerequisites and next-step hints: `stim-dev detect`
- Run controller-owned machine acceptance for messaging: `stim-dev accept controller messaging [text]`
- Run controller-owned machine acceptance for tool activity visibility: `stim-dev accept controller tool-activity [text]`
- Run controller-owned machine acceptance for participant delivery routing: `stim-dev accept controller participant-routing [text]`
- Smoke renderer-visible messaging send path: `stim-dev smoke renderer messaging [text]`
- Smoke renderer-visible two-turn continuation path: `stim-dev smoke renderer continuation [text]`
- Start full local app loop: `stim-dev start`
- Start agents-focused loop: `stim-dev start agents`
- Start controller-focused loop: `stim-dev start controller`
- Start renderer-focused loop: `stim-dev start renderer`
- Start Tauri-focused loop: `stim-dev start tauri`
- Restart full local app loop: `stim-dev restart`
- Restart agents-focused loop: `stim-dev restart agents`
- Restart renderer-focused loop: `stim-dev restart renderer`
- Inspect live runtime status: `stim-dev status`
- Inspect agents runtime state: `stim-dev inspect agents runtime`
- Inspect agents HTTP instance list: `stim-dev inspect agents instances`
- Inspect agents profile catalog: `stim-dev inspect agents profiles`
- Select the active agents HTTP instance: `stim-dev agents select <instance_id>`
- Launch a managed Santi instance through agents HTTP orchestration: `stim-dev agents launch <instance_id>`
- Stop a managed Santi instance launched by the current agents sidecar: `stim-dev agents stop <instance_id>`
- Apply an agents-owned profile to Santi: `stim-dev agents apply-profile <instance_id> <profile_id>`
- Trigger a fresh agents HTTP probe: `stim-dev inspect agents probe <instance_id>`
- Trigger a focused agents provider probe: `stim-dev inspect agents provider-probe <instance_id>`
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
- `docs/contracts/controller/message-operation-events.md`: controller-owned message-operation event contract for local app-loop coverage and acceptance
- `apps/packaged/`: thin packaged/runtime launcher entry and packaged sidecar assembly plan
- `apps/renderer/`: renderer delivery sidecar wrapper plus Vite app under `apps/renderer/vite/`
- `../stim-agents/`: standalone agent orchestration sidecar (Rust HTTP service that manages local `santi` instances and publishes facts to `stim-server`); consumed by this repo's renderer over HTTP only
- `../stim-crates/`: standalone Rust platform / sidecar primitive crates consumed by this repo's apps and by the dev CLI
- `../stim-dev/`: standalone dev CLI binary that drives the local development loop across this repo
- `.github/workflows/guard.yml`: required guard workflow
- `../../AGENTS.md`: repo-root workspace boundary across all attached repos

## Update Rules

- Put ongoing design reasoning into `docs/`.
- Keep `AGENTS.md` short and durable.
- Only add indexes here for files that are likely to remain central.
- Before changing doc structure or adding new docs, read `docs/operations/documentation.md` and follow its canonical-source, split/merge, and no-history-baggage rules.
