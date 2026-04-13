# Stim Client Framework Overview

`stim` is the product client/application layer for the `stim.io` workspace.

It owns the user-facing application experience, client composition, and platform-host integration needed to deliver that experience across supported surfaces.

It does not own the paired agent runtime, upstream gateway, or server-side product coordination.

## Core model

The client framework is organized around three durable concerns:

1. **product application**
   - screens, flows, interaction models, and client-side view composition
2. **platform host integration**
   - desktop/mobile/web bootstrap, shell integration, window/app lifecycle, and local host capability wiring
3. **service consumption**
   - explicit communication with owned backend/runtime services over stable network contracts

Those concerns may collaborate closely, but they should not collapse into one mixed layer.

The canonical directory-level ownership split for those concerns lives in `architecture/structure.md`.

## Boundary stance

### Product layer

`stim` is the product-facing application surface.

It owns:

- product screens and navigation
- client-side state needed to present and interact with product features
- composition of atomic UI pieces from `stim-components/` into business components and screens
- platform-safe client behavior and interaction policy

It should not become the new home for:

- agent runtime internals
- server-side workflow orchestration
- durable cross-service business semantics that belong behind HTTP services

### Host layer

When `stim` uses Tauri or another host shell, that host layer owns:

- app bootstrap and lifecycle
- host capability exposure
- local runtime control/discovery/diagnostics surfaces
- packaging- and platform-specific integration

The host layer is not the primary home of product/business protocol semantics.

The canonical desktop split between Tauri host, web app, and local runtime/service processes lives in `architecture/desktop/tauri-boundary.md`.

### Service layer

Real business communication should converge on explicit service contracts such as:

- HTTP
- SSE
- WebSocket

The client should treat those services as the home of durable business behavior rather than rebuilding that behavior inside host-local bridges.

## Control-plane rule

Host-local control surfaces such as Tauri commands/events exist to help the client:

- discover local runtime state
- control host-owned capabilities
- observe diagnostics
- bridge narrow platform features into the app

They must not quietly become the main product API.

If the payload is primarily about product/business behavior, it should land on a real service contract instead.

## Dev/prod rule

Development and production may differ in:

- bootstrap path
- resource/provider selection
- local endpoint discovery
- build and packaging mechanics

They should not differ in the core identity of product features or in the meaning of the client-to-service contracts.

## UI composition rule

`stim-components/` owns atomic Vue components, layout primitives, and theme definitions.

`stim/` may:

- compose those atoms into business-facing components
- coordinate feature-level UI behavior
- choose and switch themes at the application level

`stim/` should not absorb the atomic component, layout, or theme-definition boundary back into the product repo.

`stim/` should treat visual styling friction as a `stim-components/` problem to solve rather than a reason to add product-local CSS or visual patch layers. The product repo is the composition surface, not the visual styling authority.

## Success condition

The framework boundary is healthy when all of the following stay true:

- `stim` remains the product application layer
- host-specific code remains a host layer rather than the product brain
- real business communication stays on explicit service contracts
- product screens compose `stim-components` rather than replacing its ownership boundary
- server-side and agent-side semantics stay behind `stim-server/` and `santi/`
