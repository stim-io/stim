# Tauri Boundary

This file defines the durable boundary between the Tauri host, the client web app, and any local runtime/service processes used by `stim`.

## Purpose

Tauri gives `stim` a desktop host shell.

That shell exists to:

- bootstrap the desktop app
- expose narrow host capabilities to the client
- manage local runtime control/discovery/diagnostics where needed
- package and integrate the app on desktop platforms

It does not exist to become the primary business protocol layer.

## Three-plane model

Prefer this split whenever desktop architecture questions arise:

### 1. Product app plane

The web app owns:

- product screens and routes
- user interactions
- client-side state and view composition
- adaptation of service responses into product-facing views

This is the primary home of product behavior on the client side.

### 2. Host control plane

The Tauri host owns:

- app/window lifecycle
- local capability bridging
- local runtime discovery and status publication
- diagnostics, logs, and explicit operator actions
- platform integration that cannot live in the web app

This is a host-local control plane, not the business service plane.

### 3. Service plane

Owned services such as `stim-server` and `santi` own:

- business requests/responses
- streaming business events
- durable synchronization semantics
- agent/runtime behavior

The service plane should converge on explicit network contracts such as HTTP, SSE, and WebSocket.

## Tauri host ownership

Tauri code may own:

- startup and shutdown
- single-instance policy
- deep-link handling
- tray/menu/window integration
- filesystem/native capability bridging when justified
- runtime probe/restart/status surfaces
- endpoint discovery for local services
- desktop-specific diagnostics and observability

Tauri code should not own:

- the main chat/session/message API surface
- durable product workflow orchestration
- duplicated business transport contracts that already exist over HTTP/SSE/WS
- app-specific view semantics better expressed in the web layer

## Command/event rule

Tauri commands, events, and plugin bridges are the desktop **control/discovery/inspection plane**.

They are appropriate for:

- retrieving runtime snapshot/status
- reporting current local endpoint and capability availability
- exposing diagnostics, logs, and probe results
- explicit host actions such as restart/reconnect/open-path
- narrow native capabilities that do not belong on the network service plane

They are not appropriate for:

- mirroring the main business API
- becoming a sidecar-to-sidecar workflow bus
- carrying the canonical session/chat protocol
- replacing explicit service transport contracts for convenience

Rule of thumb:

> if the payload is about what the product/service does, it belongs on the service plane; if it is about how the desktop host finds, controls, or observes a local capability, it may belong on the Tauri control plane.

For sidecar-backed local services, apply this split strictly:

- IPC/control-plane surfaces publish local sidecar truth such as discovery, current endpoint, readiness, and heartbeat
- HTTP / SSE / WebSocket carry business requests, responses, and streaming semantics

Do not use IPC as a shortcut business API between the web app and a local sidecar service.

## Sidecar/runtime rule

If local sidecars or helper processes are used, keep their boundary explicit.

For `stim`, the controller should be treated as a Tauri-local sidecar/runtime component rather than as a separate long-term service form.
Do not spend design effort on promoting controller into an independently managed runtime shape inside this project.

The Tauri host may:

- decide whether a local runtime is enabled
- start and stop it
- observe its health
- publish its current endpoint
- expose narrow restart/probe/diagnostic affordances

The Tauri host should not:

- become the business-protocol adapter for that runtime
- redefine service semantics in command handlers
- mix runtime lifecycle logic with product UI composition logic

For multi-sidecar development or runtime composition, IPC should stay namespaced and small:

- publish which sidecar instance is current
- publish which HTTP endpoint is current
- publish `starting` / `ready` / `heartbeat` / `degraded` style lifecycle truth
- give the host enough authority to attach, recover, or report failure clearly

That IPC truth should make HTTP attach targets trustworthy; it should not replace HTTP as the real business surface.

## Authority rule

For host-local runtime integration, there should be one clear authority for local runtime truth.

That authority may include:

- current runtime instance identity
- lifecycle state
- current valid endpoint
- last probe/status snapshot
- last host-visible error

HTTP reachability alone should not become the entire authority model when a richer host-owned runtime snapshot exists.

## Dev/prod rule

Development and production may differ in:

- which frontend source is loaded
- whether a local service is externally provided or host-started
- provider/resource selection
- debug-only diagnostics surfaces

They should not differ in:

- the ownership split between web app, Tauri host, and service plane
- the meaning of product/business contracts
- the meaning of host control-plane contracts

## Isolation rule

If desktop-local events/channels need instance isolation, keep that policy in one shared control-plane strategy.

Do not let each feature invent its own isolation or naming rules.

Namespace and isolation policy should be explicit infrastructure, not accidental side effects of whichever feature was built first.

## Suggested code split

Exact file paths may evolve, but the split should stay recognizable:

- web app code under `apps/renderer/`
- Tauri host/bootstrap/control code under `apps/tauri/`, with the internal `src-tauri/` directory treated as a tooling detail rather than the repo's top-level architecture model
- service clients separate from host-control clients
- shared host/app contract shapes defined once when truly needed

Avoid collapsing all desktop concerns into one large mixed host module.

## Anti-patterns

Do not introduce or extend these patterns:

- Tauri commands that duplicate the main business API surface
- business workflow orchestration encoded in desktop command handlers
- runtime lifecycle and product screen state tightly coupled in one module
- feature-by-feature host event naming with no shared isolation strategy
- host-local bridges becoming the only path to core product behavior

## Success condition

The Tauri boundary is healthy when all of the following stay true:

- the web app remains the product client surface
- Tauri remains a host/control layer
- real business communication remains on explicit service contracts
- local runtime integration stays observable and controllable without redefining business semantics
- desktop-specific code can evolve without dragging core product behavior into the host layer
