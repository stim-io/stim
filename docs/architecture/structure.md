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
- host inspection/probe/operator contract → `contracts/host/inspection.md`

## Top-level stance

When `stim` grows into a real client repo, prefer a shape recognizable along these lines:

```txt
stim/
  apps/
    renderer/
    tauri/
    controller/
  crates/
    shared/
  tools/
    stim-dev/
  docs/
```

- `apps/renderer/` is the client application boundary
- `apps/tauri/` is the desktop host shell boundary
- `apps/controller/` is the local controller/runtime boundary
- `crates/` holds non-UI Rust support layers
- `tools/` holds repo-local Rust developer tools and operational entrypoints
- `docs/` is the durable architecture/contract/operations boundary

The internal Tauri `src-tauri/` directory is treated as an implementation detail of `apps/tauri/`, not as the repo's top-level architecture shape.

## `apps/renderer/` ownership

`apps/renderer/` should own the product client application.

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

## Suggested sub-split inside `apps/renderer/src/`

Prefer a split that keeps these concerns recognizably separate:

- `app/`: app bootstrap, routes, global providers, navigation composition
- `features/`: product feature flows and feature-local business composition
- `components/`: app-local business components and layout composition
- `services/`: network clients and transport-facing adapters for `stim-server` / `santi`
- `platform/`: thin platform adapters consumed by the web app
- `state/` or equivalent: app-level client state when needed

The exact names may change. The ownership split should not.

## `apps/renderer/src/services/` rule

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

## `apps/renderer/src/platform/` rule

If the web app needs access to host capabilities, keep that usage behind thin platform adapters.

That area may own:

- calls into narrow host-control APIs
- host capability detection
- thin wrappers over platform-specific bridges

It should not become:

- the real service API layer
- a dumping ground for mixed feature logic
- a hidden second business-backend surface

## `apps/renderer/src/components/` rule

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
- runtime/process concerns that do not belong in the UI or desktop shell

It should not become:

- renderer UI code
- Tauri host bootstrap code
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
