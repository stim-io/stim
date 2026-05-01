# Controller Message Operation Events

This contract defines the controller-owned message-operation event layer for `stim`.

## Purpose

The controller event layer exists to cover, debug, and accept the user's local operation boundary.

It should make `stim-dev` and the renderer able to observe a complete operation path:

- command accepted or rejected
- conversation selected or created
- user message operation applied locally
- delivery or runtime dependency invoked
- assistant response observed
- transcript/projection ready
- failure stage and diagnostic detail reported

This layer is for local app-loop coverage and acceptance. It is not the durable product IM message ledger.

## Ownership

`apps/controller/` owns this event layer.

`stim-dev` may drive it directly for machine-gated acceptance. The renderer may subscribe to it for product projection. The Tauri host may publish or discover the controller endpoint, but Tauri IPC must not become the business transport for these events.

`stim-server` product-ledger events remain a separate layer owned by `stim-server`. `santi` runtime/provider events remain a separate layer owned by `santi`. Controller events may correlate to both, but they do not replace either.

## Transport rule

Use a controller WebSocket for command and event flow.

The WebSocket is a controller service contract, not a host-control shortcut. Tauri may help the renderer discover the controller endpoint, then the renderer talks to the controller over the explicit service transport.

Do not mirror this business path through Tauri commands, plugin events, or filesystem bridges for convenience.

## Event envelope rule

Every controller operation event should carry enough identity to support deterministic acceptance and failure localization:

- `schema_version`
- `event_id`
- `operation_id`
- `correlation_id`
- `causation_id` when the event is caused by a prior event or external fact
- `conversation_id` when an operation is scoped to a conversation
- `message_id` when an operation is scoped to a message
- `stage`
- `status`
- `occurred_at`

External product-ledger ids and `santi` runtime ids should be explicit references. Do not assume the controller's local ids are durable `stim-server` product ids.

## Command rule

Controller commands should be operation-shaped, not UI-shaped.

Good command examples:

- start a new conversation
- continue a named conversation
- send a text message
- request a transcript/projection snapshot

Poor command examples:

- click the send button
- patch the renderer DOM
- replay a Tauri event

UI actions may produce controller commands, but the controller contract should describe the user's product operation, not the widget gesture that triggered it.

## Acceptance rule

`stim-dev` acceptance should use controller events and controller snapshots as the primary machine-gated path.

Renderer smoke should validate UI projection only:

- renderer sees the expected active conversation
- renderer displays expected user/assistant entries
- renderer reports no visible error

Renderer smoke should not be the primary source of truth for whether the operation succeeded.

## Separation from product ledger events

Controller events may include observations such as:

- controller accepted command
- WebSocket subscriber connected
- renderer projection observed
- local app restart completed
- smoke assertion failed at a named stage

Those observations are valuable for local coverage, but they are not product IM ledger facts.

Product ledger facts such as durable message operation, delivery state, participant state, and read state belong to `stim-server` once that ledger exists. Link the layers through explicit references, not by copying controller debug events into the product ledger.

## Anti-patterns

Do not introduce these shapes:

- controller events as the durable product transcript
- `stim-server` product-ledger events as renderer/debug observations
- `santi` provider deltas as product message rows
- localStorage as the source of active conversation truth for acceptance
- implicit correlation through equal `conversation_id` or `message_id` values across repos
- business message commands hidden behind Tauri IPC

## Success condition

The controller event layer is healthy when a local operation can be driven and diagnosed without confusing ledger ownership:

1. `stim-dev` sends an operation command over the controller WebSocket.
2. Controller events identify each stage and failure point with correlation ids.
3. Controller snapshots prove the expected local projection after restart/reload.
4. Renderer smoke proves only that the UI projects the same operation state.
5. Product-ledger ownership remains available for `stim-server` without inheriting controller debug semantics.
