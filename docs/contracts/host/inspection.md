# Host Inspection and Probe Contract

This document defines the current desktop inspection-plane surface exposed through `stim-dev-cli`, the Tauri host, and the renderer probe bridge.

Read `docs/architecture/desktop/tauri-boundary.md` first for the higher-level rule that inspection belongs on the desktop control/discovery/inspection plane rather than the product business API surface.

## Scope

This contract currently covers only local desktop verification helpers:

- `stim-dev-cli screenshot [label]`
- `stim-dev-cli inspect`
- `stim-dev-cli probe [landing]`

These commands are for local observability of the desktop shell and renderer landing.

They are not a general-purpose product API, and they are not a renderer automation surface.

## Boundary rules

- `screenshot` captures host-visible main-window truth and returns a file path.
- `inspect` returns a host-owned structured snapshot about the app, window, and monitor state.
- `probe` returns a renderer-owned structured snapshot for a **named** read-only probe.

The contract intentionally does **not** expose:

- arbitrary JavaScript evaluation
- arbitrary CSS selector queries from the CLI
- general renderer mutation/control commands
- product/business workflow actions

## Command shapes

### `stim-dev-cli screenshot [label]`

Returns the emitted screenshot file path.

The host captures the desktop main window and writes the artifact under `.tmp/dev/inspection/main-window-screenshots/`.

### `stim-dev-cli inspect`

Returns a JSON snapshot with host-owned facts such as:

- app/package identity
- expected renderer origin
- window label/title/url
- size/position/visibility/focus/minimize/maximize/fullscreen state
- enabled/decorated/resizable state
- current and primary monitor snapshots
- available monitor count

### `stim-dev-cli probe [landing]`

Returns a JSON snapshot for a named renderer probe.

Current supported probe:

- `landing` → `landing-basics`

`landing-basics` reports:

- `document_ready_state`
- `document_title`
- landing shell presence
- landing card presence
- landing heading text
- primary action label

## Ownership split

- `crates/shared/` owns the shared inspection/probe contract shapes.
- `crates/stim-dev-cli/` owns the local operator command surface.
- `apps/tauri/src-tauri/` owns the host bridge, request handling, and host-owned inspection snapshot.
- `apps/renderer/` owns the renderer-side implementation of named read-only probes.

The renderer must answer only predeclared probe names with predeclared snapshot schemas.

## Extension rule

When adding more verification surface:

1. prefer a new named read-only probe over a generalized query mechanism
2. keep host-owned facts in `inspect`
3. keep renderer-owned facts in named `probe` responses
4. do not add arbitrary eval unless a real need forces a tighter explicit design
