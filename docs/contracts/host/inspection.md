# Host Inspection and Probe Contract

This document defines the current desktop inspection-plane surface exposed through `stim-dev-cli`, the Tauri host, and the renderer probe bridge.

Read `docs/architecture/desktop/tauri-boundary.md` first for the higher-level rule that inspection belongs on the desktop control/discovery/inspection plane rather than the product business API surface.

## Scope

This contract currently covers only local desktop verification helpers:

- `stim-dev-cli screenshot [label]`
- `stim-dev-cli inspect`
- `stim-dev-cli controller-runtime`
- `stim-dev-cli acceptance [first-message|multi-turn|context-chat]`
- `stim-dev-cli probe [landing|first-message|multi-turn|context-chat]`

These commands are for local observability of the desktop shell and renderer landing.

They are not a general-purpose product API, and they are not a renderer automation surface.

## Boundary rules

- `screenshot` captures host-visible main-window truth and returns a file path.
- `inspect` returns a host-owned structured snapshot about the app, window, and monitor state.
- `controller-runtime` returns host-owned controller snapshot/heartbeat truth from the Tauri-local sidecar bridge.
- `acceptance` returns one bounded operator verification payload that combines host controller-runtime truth with a named renderer acceptance probe.
- `probe` returns a renderer-owned structured snapshot for a **named** read-only probe.

## Script-versus-chat rule

Use inspection automation to constrain only the parts of the loop that are already stable enough to be treated as boundary truth, such as:

- whether `stim` is attached to the intended local runtime target
- whether the visible UI loop advances without errors
- whether later turns reuse the same conversation
- whether chat history visibly grows in the expected direction

Do **not** treat open-ended agent chat behavior as a fully scriptable contract yet.

- scripted checks may still drive real UI turns
- but they should avoid overfitting to one exact model wording or one brittle reply path
- the purpose is to verify stable boundaries, not to pretend current agent semantics are already deterministic

When a conversational behavior is still exploratory, validate it through real turn-by-turn interaction and judgment, then promote only the durable parts into scripted acceptance once they have proven stable over time.

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

### `stim-dev-cli controller-runtime`

Returns a JSON payload with:

- controller runtime snapshot
- controller heartbeat

This is the operator-facing command for checking the current Tauri-local controller attach target,
published HTTP base URL, ready/degraded state, and detail text such as compose-default versus env-override target selection.

### `stim-dev-cli acceptance [first-message|multi-turn|context-chat]`

Returns a JSON payload that combines:

- controller runtime snapshot + heartbeat
- renderer `first-message` probe result

Current supported acceptance target:

- `first-message`
- `multi-turn`
- `context-chat` (exploratory)

Use this when you want one operator command to verify both:

- the controller attached to the intended local runtime target
- the visible first-message UI loop still succeeds

`multi-turn` extends that same operator path to a bounded two-turn renderer proof. It resets the crude chat UI, sends two predefined turns through the existing UI controls, and returns whether the second turn reused the first turn's `conversation_id` while appending the expected chat history.

`context-chat` is an exploratory inspect-driven semantic probe. It drives a few real UI turns through the visible `stim` controls and reports useful context-retention evidence, but it is not a release-grade pass/fail contract for open-ended chat correctness.

Treat it as:

- a way to observe whether context appears to persist across turns
- a way to surface obvious regressions in visible multi-turn usability
- a bridge toward future stable acceptance criteria

Do not treat it as:

- proof that one exact wording path defines correctness
- proof that current agent chat semantics are deterministic
- a substitute for real turn-by-turn operator judgment while the behavior is still evolving

Current posture:

- keep this command as a raw JSON diagnostic surface for now
- do not collapse it into a stricter pass/fail assertion mode until the acceptance criteria are stable enough to stop changing

### `stim-dev-cli probe [landing|first-message|multi-turn|context-chat]`

Returns a JSON snapshot for a named renderer probe.

Current supported probe:

- `landing` → `landing-basics`
- `first-message` → `first-message-result`
- `multi-turn` → `multi-turn-result`
- `context-chat` → `context-chat-result` (exploratory)

`landing-basics` reports:

- `document_ready_state`
- `document_title`
- landing shell presence
- landing card presence
- landing heading text
- primary action label

`first-message-result` reports the current last visible response/debug block and, if needed, triggers one bounded send through the existing primary action.

`multi-turn-result` reports a bounded two-turn chat proof with:

- first-turn response/final-sent text
- second-turn response/final-sent text
- first/second `conversation_id`
- whether the same conversation was reused across both turns
- total/user/assistant chat-entry counts
- visible error text, if any

`context-chat-result` reports exploratory evidence from a bounded three-turn semantic chat run with:

- the remember / recall / count replies
- the shared `conversation_id`
- whether all three turns stayed on the same conversation
- whether the recall reply matched `blue cactus`
- whether the count reply matched `2`
- total/user/assistant chat-entry counts
- visible error text, if any

Use those fields as evidence for current behavior, not as a claim that agent chat is already stable enough for one exact scripted answer path to define correctness by itself.

If repeated real usage shows some subset of these semantics becoming durable and predictable, promote only that stable subset into stricter scripted acceptance.

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
4. constrain scripts around stable boundaries first; only script conversational semantics after they have become durable enough to count as a real contract
5. do not add arbitrary eval unless a real need forces a tighter explicit design
