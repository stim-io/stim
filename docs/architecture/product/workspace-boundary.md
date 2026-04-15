# Stim Workspace Boundary

This file records the durable boundary between `stim` and the neighboring workspace repos.

## Repo roles

- `modules/stim/`: product client/application layer
- `modules/stim-packages/`: shared package workspace for atomic Vue components, layout primitives, theme definitions, and related support packages
- `modules/stim-server/`: server-side implementation of the `stim` product surface
- `modules/santi/`: paired agent/runtime service
- `modules/santi-link/`: upstream auth and forwarding gateway
- `modules/santi-cli/`: fast API-call and prototype-validation CLI for `santi`

## `stim` owns

- product-facing application flows and UI composition
- platform host integration needed to deliver the client application
- client-side adaptation of service responses into product views
- app-level theme selection and product interaction policy
- screen/business composition from primitives owned by `modules/stim-packages/`

## `stim` does not own

- atomic component/layout/theme definitions that belong in `modules/stim-packages/`
- server-side product communication models that belong in `modules/stim-server/`
- paired agent/runtime internals that belong in `modules/santi/`
- upstream account/auth gateway concerns that belong in `modules/santi-link/`

## Practical design rule

When ownership is unclear:

- put atomic reusable visual primitives and reusable layout primitives in `modules/stim-packages/`
- put client application composition in `modules/stim/`
- put server-side product communication and coordination in `modules/stim-server/`
- put agent/runtime semantics in `modules/santi/`

Prefer keeping those boundaries explicit over optimizing for convenience in one repo.

For message-card work specifically, read `message-card-boundary.md` for the stricter rule that card/layout/theme primitives stay in `stim-packages/` while `stim` remains a composition and prop-declaration layer.
