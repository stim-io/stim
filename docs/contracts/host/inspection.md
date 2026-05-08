# Host Status and Inspection Contract

This document defines the local desktop status and inspection surface exposed through `stim-dev`, the Tauri host, and the renderer inspection bridge.

Read `docs/architecture/desktop/tauri-boundary.md` first for the higher-level rule that desktop control, discovery, and inspection stay separate from product business APIs.

## Scope

The canonical local operator surface is:

- `stim-dev start [all|agents|controller|renderer|tauri]`
- `stim-dev restart [all|agents|controller|renderer|tauri]`
- `stim-dev status`
- `stim-dev [--namespace <value>] list`
- `stim-dev [--namespace <value>] stop`
- `stim-dev [--namespace <value>] reset`
- `stim-dev agents select <instance_id>`
- `stim-dev agents launch <instance_id>`
- `stim-dev agents stop <instance_id>`
- `stim-dev accept controller messaging [text]`
- `stim-dev smoke renderer messaging [text]`
- `stim-dev smoke renderer continuation [text]`
- `stim-dev inspect <app> <subcommand>` where leaves are strictly enumerated:
  - `stim-dev inspect tauri host`
  - `stim-dev inspect tauri screenshot [label]`
  - `stim-dev inspect agents runtime`
  - `stim-dev inspect agents instances`
  - `stim-dev inspect agents probe <instance_id>`
  - `stim-dev inspect agents provider-probe <instance_id>`
  - `stim-dev inspect renderer landing`
  - `stim-dev inspect renderer messaging`

Lifecycle, status, and `inspect` commands are for local recovery, status, and UI evidence collection. Agents management belongs under `agents`; controller acceptance belongs under `accept`; renderer projection smoke belongs under `smoke`. None of these commands is a general-purpose product API or arbitrary renderer automation surface.

## Command rules

### `stim-dev start [all|agents|controller|renderer|tauri]`

Starts the requested local dev-loop surface.

`start` must fail fast when an existing instance is detected for the selected namespace. It should not implicitly stop or reuse the existing instance. The operator must run `stim-dev stop` or `stim-dev restart` explicitly.

### `stim-dev restart [all|agents|controller|renderer|tauri]`

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

### `stim-dev agents select <instance_id>`

Calls the local `agents` HTTP service selection endpoint and returns the updated active agent instance id.

This is an operator client for agents-sidecar orchestration state. It must not edit local config files, maintain a second active-instance registry in `stim-dev`, or mutate Santi provider/runtime/session/tool/memory semantics directly.

This selection is local management focus, not chat routing truth. Chat surfaces should choose product-visible participants from `stim-server` through `participant_id`.

### `stim-dev agents launch <instance_id>`

Calls the local `agents` HTTP service launch endpoint for a configured managed Santi instance and returns the action result.

This command only forwards an explicit orchestration request. It must not infer launch commands, rewrite config, or mutate Santi provider/runtime/session/tool/memory semantics directly. The instance must be configured as managed by the agents service.

### `stim-dev agents stop <instance_id>`

Calls the local `agents` HTTP service stop endpoint for a managed Santi instance launched by the current agents sidecar and returns the action result.

This command is scoped to the local process tree the agents sidecar launched. It is not a general process killer and must not become a fallback for arbitrary Santi or provider cleanup.

### `stim-dev agents apply-profile <instance_id> <profile_id>`

Calls the local `agents` HTTP service profile-apply endpoint and returns the action result.

This command only sends a profile id to the agents sidecar. Profile catalogs and secret handoff belong to `stim-agents`; `stim-dev`, renderer code, and `stim-controller` must not send raw API keys or provider configs. Santi still owns the atomic config apply semantics.

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

### `stim-dev inspect agents runtime`

Returns host-owned structured state for the local `agents` sidecar relationship:

- namespace
- agents sidecar instance id
- lifecycle state
- current HTTP base URL
- host-visible detail

This command observes sidecar attachment/runtime truth. It is not the API for managing `santi` instances; that belongs to the `agents` HTTP service.

### `stim-dev inspect agents instances`

Calls the local `agents` HTTP service and returns its current agent-instance list response.

This command is an operator client for the `agents` service contract. It must not infer, cache, or manage `santi` instance state locally in `stim-dev`.

The returned list may contain one or more configured `santi` endpoints plus the active instance id selected by the `agents` service. Instance registration, active selection, and probing policy belong to the `agents` HTTP service; `stim-dev` only resolves the local agents endpoint and forwards the read.

### `stim-dev inspect agents profiles`

Calls the local `agents` HTTP service profile catalog endpoint and returns safe profile summaries.

The response may include profile ids, labels, launch profiles, non-secret provider facts, and whether the needed secret is available. It must not expose raw API keys.

### `stim-dev inspect agents probe <instance_id>`

Calls the local `agents` HTTP service probe endpoint for one registered agent instance and returns the fresh snapshot.

The probe action is a live observation through the `agents` HTTP API. It is not a lifecycle start/stop command and must not bypass Santi-owned atomic runtime facts. If provider/gateway reachability or effective config facts are included, those facts must come from Santi HTTP APIs such as `POST /api/v1/admin/provider/probe` and `GET /api/v1/admin/config`.

### `stim-dev inspect agents provider-probe <instance_id>`

Calls the local `agents` HTTP service provider-probe endpoint for one registered agent instance and returns the Santi-owned provider/gateway probe result.

This is a focused operator view of provider reachability. It must not become a profile switch command, model-completion test, or arbitrary URL fetcher.

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
2. send a first text operation through the controller message-operation WebSocket
3. assert the controller snapshot over the persisted transcript
4. restart the controller runtime
5. reload the same transcript through the controller WebSocket
6. send a second text operation into the same conversation asking the assistant to quote the previous user message
7. restart and reload again
8. assert the transcript contains both user turns, assistant replies, and the final assistant reply includes the prior user text
9. fail with a non-zero exit code if any structured failure or content assertion fails

`stim-dev smoke renderer messaging [text]` is a one-turn renderer projection smoke. It may use the declared renderer action bridge to drive the visible composer, but it should validate UI projection only:

- active conversation is visible
- user and assistant entries are rendered
- no visible error is reported
- stable debug fields such as response source and final sent text remain readable

`stim-dev smoke renderer continuation [text]` is the human-visible projection smoke for two-turn continuation. It should drive the visible New Conversation button and composer twice, then assert the final rendered assistant bubble includes the first user text.

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
- `../stim-dev/` owns the local operator command surface.
- `apps/tauri/src-tauri/` owns the host bridge, request handling, and host-owned inspection snapshots.
- `../stim-agents/` owns the local agent-instance management HTTP service surface.
- `apps/renderer/vite/` owns renderer-side implementation of declared read-only inspection snapshots.
