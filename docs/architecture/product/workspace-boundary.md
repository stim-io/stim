# Stim Workspace Boundary

This file records the durable boundary between `stim` and the neighboring workspace repos.

## Repo roles

- `modules/stim/`: product client/application layer
- `modules/stim-components/`: atomic Vue components, layout primitives, and theme definitions
- `modules/stim-server/`: server-side implementation of the `stim` product surface
- `modules/santi/`: paired agent/runtime service
- `modules/santi-link/`: upstream auth and forwarding gateway
- `modules/santi-cli/`: fast API-call and prototype-validation CLI for `santi`

## `stim` owns

- product-facing application flows and UI composition
- platform host integration needed to deliver the client application
- client-side adaptation of service responses into product views
- app-level theme selection and product interaction policy
- screen/business composition from primitives owned by `modules/stim-components/`

## `stim` does not own

- atomic component/layout/theme definitions that belong in `modules/stim-components/`
- server-side product communication models that belong in `modules/stim-server/`
- paired agent/runtime internals that belong in `modules/santi/`
- upstream account/auth gateway concerns that belong in `modules/santi-link/`

## Practical design rule

When ownership is unclear:

- put atomic reusable visual primitives and reusable layout primitives in `modules/stim-components/`
- put client application composition in `modules/stim/`
- put server-side product communication and coordination in `modules/stim-server/`
- put agent/runtime semantics in `modules/santi/`

Prefer keeping those boundaries explicit over optimizing for convenience in one repo.
