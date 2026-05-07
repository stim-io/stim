# Stim Structure and Ownership Method

This file defines the durable structure and ownership method for `stim`.

It does not freeze exact framework choices or file names forever.
It does define what kinds of code belong together and what kinds of code must stay separate.

## Goal

Keep the product app, service-consumption code, and Tauri host/control code structurally distinct from the start.

The main purpose of this structure is boundary protection, not aesthetic neatness.

`stim` is the product client/application repo.

Use this file to answer questions like:

- where a new concern belongs inside `stim`
- when a concern actually belongs in `stim-packages/` or an external service repo instead
- when a local product leftover should stay local instead of forcing a new shared abstraction

## Quick reading guide

Use this file first when you are deciding:

- which top-level area should own new code
- whether a concern belongs in `stim`, `stim-packages`, or a service repo
- whether a local UI/layout concern should stay local or become shared

Then read more specific docs only if the question narrows to:

- desktop host/control-plane ownership → `architecture/desktop/tauri-boundary.md`
- controller message-operation event contract → `contracts/controller/message-operation-events.md`
- host inspection/probe/operator contract → `contracts/host/inspection.md`

## Top-level stance

When `stim` grows into a real client repo, prefer a shape recognizable along these lines:

```txt
stim/
  apps/
    agents/
    renderer/
      vite/
    tauri/
    controller/
    packaged/
  crates/
    platform/
    sidecar/
    shared/
  tools/
    stim-dev/
  docs/
```

- `apps/renderer/` is the renderer delivery boundary: the Rust wrapper lives at the boundary root, while the product Vite app lives under `apps/renderer/vite/`
- `apps/agents/` is the local agent-instance management sidecar boundary; it publishes agent/instance observability facts and participant projection inputs to `stim-server`, while product chat selection is keyed by server-owned `participant_id`
- `apps/tauri/` is the desktop host shell boundary
- `apps/controller/` is the local controller/runtime boundary
- `apps/packaged/` is the thin packaged/runtime launcher boundary
- `crates/platform/` owns platform facts and primitives
- `crates/sidecar/` owns sidecar namespace, layout, ready/inspect, and stamp primitives
- `crates/` holds non-UI Rust support layers
- `tools/` holds repo-local Rust developer tools and operational entrypoints
- `docs/` is the durable architecture/contract/operations boundary

The internal Tauri `src-tauri/` directory is treated as an implementation detail of `apps/tauri/`, not as the repo's top-level architecture shape.

## Sidecar and platform stance

Use `sidecar-mode` for the launcher-owned mode of a sidecar app instance.

The only valid sidecar-mode values are:

- `dev`
- `runtime`

Do not use `runtime-mode` for this concept; that name conflicts with the `runtime` enum value and makes launcher mode ambiguous.

`crates/platform/` owns low-level platform facts only:

- path derivation
- process spawning and process table access
- network binding helpers
- environment normalization
- file locks
- OS and architecture detection

`crates/platform/` must not own sidecar identity, app lifecycle policy, controller attach targets, inspection schemas, or business protocol behavior.

`crates/sidecar/` owns local sidecar control-plane primitives:

- namespace defaults
- sidecar app identity
- sidecar-mode parsing
- stamp argument construction and parsing
- namespace-scoped layout for logs, locks, and bridges
- stamped-process matching and cleanup concepts

The argv stamp is deliberately low-dimensional:

- `app`
- `namespace`
- `sidecar-mode`
- `source`

Do not put role, instance id, endpoint, health, or richer lifecycle facts into stamp args. Those facts are live runtime facts and should be carried by ready-line / inspect / health communication.

`crates/sidecar/` must not own product/business APIs.

Do not introduce persisted runtime truth files such as `state.json`, `runtime.json`, or `heartbeat.json`. Runtime truth should be produced by live inspect/probe/health surfaces. Stamps define cleanup ownership and worst-case process leak boundaries; locks define startup exclusion only.

## `apps/packaged/` ownership

`apps/packaged/` should own the packaged/runtime launcher once that entry exists.

It may:

- choose namespace and sidecar-mode for packaged assembly
- stamp direct child sidecar processes
- launch packaged renderer, controller, and Tauri sidecar instances
- wait for startup readiness through a live ready handshake or inspect surface
- apply packaged resource and path selection

It should not:

- become a product host
- own business protocol behavior
- proxy controller HTTP APIs
- maintain a persistent runtime registry
- duplicate `stim-dev` operator-only commands

The executable surface is intentionally thin:

- `stim-packaged --plan --namespace <value>` prints the runtime sidecar assembly.
- `stim-packaged launch controller --namespace <value>` starts the packaged controller sidecar in the foreground.
- `stim-packaged launch renderer --namespace <value>` delegates renderer delivery to `stim-renderer --runtime`, emits a renderer-delivery ready line, and holds a stamped runner process so fallback cleanup has a process boundary.
- `stim-packaged launch tauri --namespace <value>` starts the Tauri host as a stamped runner process and passes namespace and sidecar-mode into the host.
- `stim-packaged launch all --namespace <value>` starts renderer delivery, controller runtime, and Tauri host as top-level runtime sidecars. The packaged launcher injects the controller endpoint into Tauri so the host attaches instead of starting its own controller child.

Each launch path waits for a live ready line, validates the 4-field stamp plus live role, prints readiness, and then waits on the child or runner process. The hidden runners are implementation details used to keep third-party tool argv clean while preserving stamped process-tree cleanup.

Tauri should load the renderer through a URL supplied by launcher-owned launch configuration. Packaged and dev composition write the renderer-delivery launch bridge under `.tmp/sidecars/<sidecar-mode>/<namespace>/bridges/renderer-delivery/launch.json`; the Tauri host reads it without treating it as persisted runtime truth.

## `apps/renderer/` ownership

`apps/renderer/` should own renderer delivery as a whole. The Rust `stim-renderer` wrapper belongs at the boundary root, and the product Vite application belongs in `apps/renderer/vite/`.

That includes:

- routes, screens, and app bootstrap
- feature-level UI composition
- local presentation state and view models
- app-level business components composed from `stim-packages`
- service-consumption adapters used by the app

`src/` should not become the long-term home for:

- Tauri command handler implementations
- controller/runtime process lifecycle code
- desktop-native capability implementations
- broad host-control plumbing

The Rust `stim-renderer` binary owns renderer delivery. In dev mode it starts the Vite server from `apps/renderer/vite/` and emits the renderer-delivery ready line. In runtime mode it serves built static assets and emits the same ready-line role with the bound endpoint.

## Suggested sub-split inside `apps/renderer/vite/src/`

Prefer a split that keeps these concerns recognizably separate:

- `app/`: app bootstrap, routes, global providers, navigation composition
- `features/`: product feature flows and feature-local business composition
- `components/`: app-local business components and layout composition
- `services/`: network clients and transport-facing adapters for `stim-server` / `santi`
- `platform/`: thin platform adapters consumed by the web app
- `state/` or equivalent: app-level client state when needed

The exact names may change. The ownership split should not.

## `apps/renderer/vite/src/services/` rule

If the client talks to `stim-server` or `santi`, that communication should converge in a service-facing area such as `src/services/`.

That area may own:

- HTTP clients
- SSE / WebSocket clients
- request/retry/auth adaptation on the client side
- translation from wire contracts into app-facing objects

It should not own:

- Tauri command wrappers for host control
- desktop runtime lifecycle logic
- broad UI state management

## `apps/renderer/vite/src/platform/` rule

If the web app needs access to host capabilities, keep that usage behind thin platform adapters.

That area may own:

- calls into narrow host-control APIs
- host capability detection
- thin wrappers over platform-specific bridges

It should not become:

- the real service API layer
- a dumping ground for mixed feature logic
- a hidden second business-backend surface

## `apps/renderer/vite/src/components/` rule

`stim` may define app-local components for:

- page composition
- feature composition
- business-oriented widgets
- layout glue around shared atoms

It should not absorb ownership that belongs in `stim-packages/`, such as:

- atomic reusable component primitives
- canonical theme definitions
- base visual primitives intended for reuse across the product surface

Small product-local composition leftovers may stay in `stim` when they are clearly screen-specific, such as bounded page width or copy-measure constraints.

Do not promote those leftovers into shared primitives automatically.

Promote them only when repeated pressure shows that the same concern is reappearing across screens or message-card compositions.

## `apps/agents/` ownership

`apps/agents/` owns the local agent-instance management sidecar.

It is a first-class HTTP service sidecar, following the same basic service-surface method as `stim-server`: `/api/v1` routes, JSON responses, explicit error envelopes, and OpenAPI documentation.

It may:

- publish local `santi` instance snapshots for web and operator clients
- probe `santi` health, non-secret service/profile/runtime/provider facts, current effective config facts, and Santi-owned provider/gateway reachability through `santi` HTTP APIs
- orchestrate local `santi` lifecycle and active endpoint selection
- own carrier-agnostic Santi profile catalogs and secret handoff for provider/profile switching orchestration, while applying those profiles through Santi-owned HTTP config atoms
- publish Santi protocol discovery records to `stim-server`
- publish registered-agent projections to `stim-server` through registration and heartbeat requests
- expose management actions to the web app and `stim-dev` through HTTP service contracts

Configured `santi` instances enter through the `apps/agents/` service boundary. The fallback single-instance path uses `STIM_AGENTS_SANTI_BASE_URL` / `SANTI_BASE_URL` plus optional label/profile/participant/delivery-endpoint environment, while multi-instance local loops can provide `STIM_AGENTS_SANTI_INSTANCES_JSON` as an array of `{ id, endpoint, label?, profile?, managed?, agent_id?, participant_id?, delivery_endpoint_id?, launch? }`.

Managed launch is explicit orchestration. `launch` may provide `{ command, args?, cwd?, env? }`; the agents sidecar may spawn and stop that local process tree, but the resulting runtime/provider/session/tool/memory semantics still belong to the launched `santi` HTTP service.

Provider profile catalogs and secret handoff belong to the agents sidecar rather than the Stim IM controller. A single `santi` soul can serve sessions carried by Stim, Feishu, Slack, local conversations, and other message carriers, so provider/profile orchestration must stay carrier-agnostic. Renderer code may choose a `profile_id` through an agents HTTP action, but must never send raw API keys or provider configs.

That configuration and the current active instance selection are agents-sidecar concerns. Renderer code and `stim-dev` may observe and request explicit actions through the agents HTTP API, but they must not maintain their own instance registry or active-selection state.

Chat routing should not read the local active instance as durable product truth. `stim-agents` publishes available agent instances and their delivery endpoints to `stim-server`; chat surfaces should choose product-visible participants from `stim-server` state, and controller delivery should resolve the selected `participant_id` through `stim-server`.

It should not:

- own `santi` provider/runtime/session/tool/memory atomic semantics
- make the renderer or Stim IM controller own Santi provider profiles, API keys, or carrier-agnostic agent configuration
- become the message-operation controller
- store durable product IM ledger facts
- become the product registered-agent source of truth
- become a hidden data-management layer for the web app
- move root workspace attachment assumptions into the child repo

The renderer may use Tauri only to discover the current `agents` sidecar endpoint. It should then call the `agents` HTTP service directly for agent-instance management views and actions.

The web app remains a renderer and service client: it renders state returned by `agents` and submits explicit actions, but it must not become the owner of agent-instance data management.

## `apps/tauri/` ownership

`apps/tauri/` should own the desktop host/control plane.

That includes:

- Tauri bootstrap
- command/event registration
- local capability bridging
- runtime status/probe/restart surfaces
- local sidecar/process management if needed
- desktop diagnostics and observability

`src-tauri/` should not become the main home for:

- product feature semantics
- core chat/session workflow logic
- duplicated HTTP business endpoints encoded as Tauri commands

The internal `apps/tauri/src-tauri/` folder exists because of Tauri CLI/tooling expectations, but it should not become the top-level ownership model for the repo.

## Suggested sub-split inside `apps/tauri/src-tauri/src/`

Prefer a split along these lines when the desktop host becomes real:

- `app/`: app bootstrap and lifecycle wiring
- `control/`: host control-plane commands/events/contracts
- `runtime/`: local runtime/sidecar management and status
- `diagnostics/`: logs, probes, inspection, local observability
- `platform/`: desktop-native integrations that do not fit the web layer

Do not collapse those concerns into one large `lib.rs` or one mixed command module.

## `apps/controller/` ownership

`apps/controller/` should own the Rust controller/runtime surface when that surface becomes real.

That includes:

- local runtime/control entrypoints
- controller-facing orchestration and health/probe surfaces
- controller-owned message-operation events for local coverage, debugging, and acceptance
- runtime/process concerns that do not belong in the UI or desktop shell

It should not become:

- renderer UI code
- Tauri host bootstrap code
- the durable product IM message ledger owned by `stim-server`
- a dumping ground for unrelated dev-only glue

## `crates/` and `tools/` rule

Prefer `crates/` for non-UI Rust support layers that are shared with the product/runtime code.

Prefer `tools/` for repo-local Rust developer tooling and operational entrypoints that are not part of the main runtime/support-layer architecture.

Current intended examples:

- `crates/shared/`: non-UI shared Rust primitives
- `tools/stim-dev/`: unified Rust development orchestration entrypoint

## Shared component rule

`stim` composes shared atoms, layout primitives, and theme-backed primitives from `stim-packages/`.

It should not become the long-term home for:

- reusable card visuals
- reusable layout behavior
- shared typography treatment
- theme-owned styling logic

When UI pressure appears, ask in this order:

1. is this clearly product composition that should stay in `stim`?
2. is this a repeated visual/layout concern that belongs in `stim-packages`?
3. is this really a service or runtime boundary problem rather than a component problem?

Do not make `stim` look thinner by moving obviously local product leftovers into shared packages too early.

### Quick placement examples

Keep in `stim`:

- one screen's copy width constraint
- feature-local page composition
- product-specific arrangement of shared message-card primitives

Move to `stim-packages` only when repeated pressure appears:

- the same text treatment starts repeating across screens
- multiple screens need the same reusable card frame or layout behavior
- `stim` is duplicating the same visual prop-shaping or CSS for shared presentation concerns

## Shared contract rule

When a shape must be shared across web app and Tauri host, define it once in a clear contract boundary.

Typical examples:

- runtime snapshot/status
- host capability availability
- diagnostics payloads
- host control-plane event envelopes

Do not let nearly identical types drift across app and host code in parallel.

## `shared` vs `contracts` stance

If `stim` later introduces internal shared modules/packages, keep this split:

- `shared`: service-agnostic primitives and helpers with no concrete runtime/entity protocol meaning
- `contracts`: boundary shapes that host/app or app/service participants must agree on

Do not use `shared` as a disguised home for concrete control-plane or product protocol shapes.

## Dev structure rule

Development support may add:

- local bootstrap helpers
- dev-only entrypoints
- mock or alternate providers
- diagnostics conveniences

Those additions should not erase the core structure.

In particular:

- dev-specific host wiring still belongs under host/control ownership
- dev-specific service clients still belong under service-consumption ownership
- dev-only shortcuts should not become the hidden canonical architecture

## Anti-patterns

Do not introduce or extend these patterns:

- feature UI code importing deep `src-tauri` runtime logic directly
- service HTTP clients mixed into Tauri command modules
- app-local components reimplementing atomic component ownership from `stim-packages`
- one generic `utils/` bucket quietly becoming the home of contracts, policy, and business logic
- `platform/` or `services/` becoming catch-all dumping grounds with no ownership discipline

## Structure check

Before adding a new directory or module area, ask:

1. Is this product app composition, service consumption, or host/control logic?
2. Does the chosen location preserve that ownership clearly?
3. Would a new reader know whether this code is web-app code or host code?
4. If the host changed, would product code remain mostly in the same place?
5. If a shared shape is being introduced, is its canonical home explicit?

If the answer is unclear, tighten the structure before expanding it.
