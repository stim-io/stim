# Stim Layering Principles

This file defines the durable ownership split inside `stim`.

## Layer model

Prefer the following high-level split:

- **app/product layer**: product routes, screens, business-facing components, local presentation state
- **service-consumption layer**: API clients, streaming clients, request/retry policy, server/runtime contract adaptation
- **host/control layer**: Tauri host integration, local runtime control, diagnostics, and platform capability bridging
- **presentation foundation**: composition of atoms and layout primitives from `stim-packages/` plus app-level screen assembly

Exact directory names may evolve. The ownership split should not.

## Ownership rules

### 1. Product screens stay above transport details

Screens and feature flows should consume app-local services or view models rather than directly scattering raw host or transport calls through UI code.

### 1.1 Product composition stays above visual styling authority

`stim` should primarily assemble screens from `stim-packages` atoms and layout primitives.

Do not treat product-local CSS or ad hoc visual patches as the normal place to solve spacing, color, typography, or layout-expression friction.

When visual composition pressure reveals a missing primitive, missing token, or missing layout capability, the default response should be to improve `stim-packages/` rather than re-own styling logic inside `stim/`.

### 2. Service clients stay separate from host-control clients

Keep these two concerns distinct:

- service communication with `stim-server` / `santi`
- host-local control/discovery/diagnostics for desktop runtime integration

Do not turn Tauri commands into a mirror of server business APIs.

### 3. Host control is narrow

Host integration may expose:

- runtime snapshot/status
- capability discovery
- diagnostics and logs
- explicit restart/reconnect/probe actions
- platform-specific affordances that cannot live in the web layer

Host integration should not expose broad business workflows that really belong to network services.

### 4. Platform differences stay local

If desktop/web/mobile need different behavior, keep that difference at the platform edge.

Do not let platform-specific compatibility work leak upward and redefine product semantics.

### 5. Contracts are shared intentionally

If a shape must be shared across host and app code, define it once in a clear contract boundary.

Do not let the same runtime snapshot, event envelope, or capability shape drift into multiple near-duplicate local definitions.

### 6. Dev-mode support does not redefine architecture

Dev-mode tooling may change:

- where the frontend is served from
- how local runtimes are discovered or started
- which providers/resources are selected

It should not create a separate conceptual architecture with different ownership rules.

### 7. Namespace and isolation belong to control-plane infrastructure

If host-local IPC/event channels are introduced, isolation strategy should live in the control-plane infrastructure rather than being improvised per feature.

Feature code should extend one shared namespacing strategy instead of inventing parallel ones.

## Anti-patterns

Do not introduce or extend these patterns:

- product screens directly orchestrating Tauri host runtime logic
- Tauri commands becoming the de facto business API surface
- platform-specific hacks mixed into shared product semantics
- `stim` re-owning atomic component/theme responsibilities already assigned to `stim-packages/`
- `stim` accumulating product-local CSS or visual patch layers that should be solved in `stim-packages/`
- client code importing server/runtime implementation assumptions instead of consuming stable contracts
- feature-by-feature local IPC naming schemes with no unified isolation strategy

## Boundary check

Before adding a new client feature, ask:

1. Is this product UI composition, host capability wiring, or real service communication?
2. Does it preserve the distinction between host control plane and business service plane?
3. Does it keep atomic component/layout/theme ownership in `stim-packages/`?
4. Would the same feature still make sense if the platform host changed?
5. If shared contracts are needed, is there one canonical definition?

If those answers are unclear, tighten the boundary before implementing.
