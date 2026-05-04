# Host Status and Inspection Contract

This document defines the local desktop status and inspection surface exposed through `stim-dev`, the Tauri host, and the renderer inspection bridge.

Read `docs/architecture/desktop/tauri-boundary.md` first for the higher-level rule that desktop control, discovery, and inspection stay separate from product business APIs.

## Scope

The canonical local operator surface is:

- `stim-dev start [all|controller|renderer|tauri]`
- `stim-dev restart [all|controller|renderer|tauri]`
- `stim-dev status`
- `stim-dev [--namespace <value>] list`
- `stim-dev [--namespace <value>] stop`
- `stim-dev [--namespace <value>] reset`
- `stim-dev accept controller messaging [text]`
- `stim-dev smoke renderer messaging [text]`
- `stim-dev inspect <app> <subcommand>` where leaves are strictly enumerated:
  - `stim-dev inspect tauri host`
  - `stim-dev inspect tauri screenshot [label]`
  - `stim-dev inspect renderer landing`
  - `stim-dev inspect renderer messaging`

Lifecycle, status, and `inspect` commands are for local recovery, status, and UI evidence collection. Controller acceptance belongs under `accept`; renderer projection smoke belongs under `smoke`. None of these commands is a general-purpose product API or arbitrary renderer automation surface.

## Command rules

### `stim-dev start [all|controller|renderer|tauri]`

Starts the requested local dev-loop surface.

`start` must fail fast when an existing instance is detected for the selected namespace. It should not implicitly stop or reuse the existing instance. The operator must run `stim-dev stop` or `stim-dev restart` explicitly.

### `stim-dev restart [all|controller|renderer|tauri]`

Stops the matching stamped process surface and then starts the requested target.

Use `restart` for recovery instead of flags such as `start renderer --force` or `start tauri --reuse-renderer`.

### `stim-dev status`

Returns one IPC-backed runtime status payload for the current namespace.

The status payload combines:

- live host/window reachability when the Tauri inspection bridge is available
- live controller runtime snapshot and heartbeat when the controller bridge is available
- stamped process evidence for cleanup and diagnosis

Runtime truth comes from live IPC/inspection/probe surfaces. Stamp/process evidence is cleanup and leak-boundary evidence, not the source of runtime truth.

### `stim-dev [--namespace <value>] list`

Returns the namespace process view plus the same live IPC reachability evidence used by `status`.

Namespace selection is injected with `--namespace <value>`. If omitted, `default` is the fallback namespace. Do not pass the namespace as a positional command argument such as `stim-dev list dev-a`.

### `stim-dev [--namespace <value>] stop`

Stops stamped process trees for the namespace.

This is a fallback cleanup action for launcher-managed processes, not a graceful product workflow.

### `stim-dev [--namespace <value>] reset`

Runs `stop`, then removes namespace-scoped disposable logs, bridges, and locks.

`reset` must not remove persistent product data or persisted runtime truth files. Runtime truth is live IPC/inspection/probe state; reset only clears disposable local coordination and diagnostic residue.

## `inspect` leaves

All Tauri + renderer UI debugging and evidence collection belongs under `inspect <app> <subcommand>`. The tree is strictly enumerated: do not add default inspect targets, implied apps, or guessed subcommands.

### `stim-dev inspect tauri host`

Returns host-owned structured state:

- app/package identity
- expected renderer origin
- main window label/title/url
- size/position/visibility/focus/minimize/maximize/fullscreen state
- decoration/resizable/enabled state
- monitor snapshots

### `stim-dev inspect renderer landing`

Returns renderer-owned landing state without mutating the page:

- document ready state and title
- landing shell/card presence
- session drawer presence and collapsed state
- landing heading text
- primary action label
- active session id

### `stim-dev inspect renderer messaging`

Returns renderer-owned messaging state without mutating the page:

- active session id
- active conversation id
- total/user/assistant visible message counts
- last visible user and assistant text
- last response/final-sent debug text when visible
- last error text when visible
- assistant content kind and fragment presence
- primary action label

This command is an observation primitive. It must not click, type, send, reset, or wait for a chat turn.

### `stim-dev inspect tauri screenshot [label]`

Captures the host-visible main window and returns the artifact path.

Although it writes an artifact, it belongs under `inspect` because it is UI-debug evidence collection across Tauri and renderer state.

## Message-operation acceptance and renderer smoke

`stim-dev accept controller messaging [text]` is the machine-gated controller acceptance path for the local message operation loop.

It drives the controller service contract directly, not the renderer DOM:

1. start a clean controller runtime for the namespace
2. send a text operation through the controller message-operation WebSocket
3. assert the controller snapshot over the persisted transcript
4. restart the controller runtime
5. reload the same transcript through the controller WebSocket
6. fail with a non-zero exit code if any structured failure or content assertion fails

`stim-dev smoke renderer messaging [text]` is a renderer projection smoke. It may use the declared renderer action bridge to drive the visible composer, but it should validate UI projection only:

- active conversation is visible
- user and assistant entries are rendered
- no visible error is reported
- stable debug fields such as response source and final sent text remain readable

Do not use renderer smoke as the primary source of truth for whether the controller operation succeeded. Do not gate local verification on one exact open-ended model wording.

## Non-goals

The contract intentionally does not expose:

- arbitrary JavaScript evaluation
- arbitrary CSS selector queries from the CLI
- hidden CLI-driven chat automation outside declared `accept` / `smoke` leaves
- renderer-driven aggregate acceptance that replaces controller events and snapshots as the primary signal
- product/business workflow actions outside explicit controller service contracts

If a future web harness boundary becomes mature enough to expose declared app operations safely, add that as a new explicit contract. Do not grow hidden renderer automation inside `stim-dev`.

## Ownership split

- `crates/shared/` owns the shared status/inspection/probe contract shapes.
- `tools/stim-dev/` owns the local operator command surface.
- `apps/tauri/src-tauri/` owns the host bridge, request handling, and host-owned inspection snapshots.
- `apps/renderer/vite/` owns renderer-side implementation of declared read-only inspection snapshots.
