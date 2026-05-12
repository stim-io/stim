# Host Status and Inspection Contract

This document defines the local desktop status and inspection surface exposed through the external `sidecar` CLI and provider-owned SidecarRuntime IPC.

Read `docs/architecture/desktop/tauri-boundary.md` first for the higher-level rule that desktop control, discovery, and inspection stay separate from product business APIs.

## Scope

The canonical local operator surface is:

- `sidecar start --config sidecar.toml [target]`
- `sidecar restart --config sidecar.toml [target]`
- `sidecar status --config sidecar.toml [--format json]`
- `sidecar list --config sidecar.toml [--format json]`
- `sidecar stop --config sidecar.toml [target]`
- `sidecar reset --config sidecar.toml [--all]`
- `sidecar inspect <target> capabilities --config sidecar.toml`
- `sidecar inspect <target> <event> [payload] --config sidecar.toml`

`sidecar.toml` is the lifecycle contract. It must define command, cwd, args, env, stamp delivery, readiness, inspect socket, status identity, stop, and reset boundaries for every launcher-managed target.

## Event Index

Controller target:

- `capabilities`
- `runtime.snapshot`
- `runtime.heartbeat`
- `accept.messaging`
- `accept.participant-routing`
- `accept.tool-activity`

Tauri target:

- `capabilities`
- `host.snapshot`
- `host.inspect`
- `host.screenshot`
- `renderer.probe`
- `renderer.action`
- `renderer.smoke.messaging`
- `renderer.smoke.continuation`
- `agents.runtime`
- `agents.heartbeat`
- `controller.runtime`
- `controller.heartbeat`

Agents target:

- `capabilities`
- `runtime.snapshot`
- `instances.list`
- `profiles.list`
- `instances.select`
- `instances.launch`
- `instances.stop`
- `profiles.apply`
- `instances.probe`
- `providers.probe`

Renderer delivery is lifecycle-managed as a sidecar target, but visible UI projection and UI actions are exposed by the Tauri host because the webview host owns the live renderer inspection bridge.

## Rules

Lifecycle commands are for startup, recovery, process status, and cleanup. They are not product APIs.

Inspect events are explicit provider capabilities. Do not add a replacement runner command when a provider sidecar can own the event directly.

Runtime truth comes from live IPC/inspection/probe surfaces. Stamp/process evidence and target pid state are cleanup and leak-boundary evidence, not product truth.

Agent instance management belongs to the `agents` sidecar IPC/HTTP surface. It must not mutate Santi provider/runtime/session/tool/memory semantics directly.

Controller operation events may cover local runtime acceptance, debugging, and deterministic checks. They remain independent from `stim-server` product-ledger events and must not become the durable product IM ledger.
