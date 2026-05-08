# stim

Product client/application workspace for `stim.io`.

Published `@stim-io` package consumption should go through GitHub Packages via the repo-local `.npmrc` baseline rather than through machine-only registry state.

Current cold-start structure:

- `apps/renderer`: renderer delivery wrapper, with the Vue/Vite application under `apps/renderer/vite`
- `apps/tauri`: Tauri platform shell
- `apps/controller`: Rust controller placeholder
- `apps/packaged`: packaged launcher
- `crates/shared`: shared Rust primitives for non-UI layers

Sibling repos consumed via path deps from this workspace:

- [`stim-crates`](https://github.com/stim-io/stim-crates): shared `stim-platform` + `stim-sidecar` primitives.
- [`stim-proto`](https://github.com/stim-io/stim-proto): protocol contract types.
- [`stim-packages`](https://github.com/stim-io/stim-packages): `@stim-io/components`, `@stim-io/icons`, `@stim-io/client`, `@stim-io/agents-client`.
- [`stim-dev`](https://github.com/stim-io/stim-dev): the local dev-loop CLI; install via `cargo install --path ../stim-dev --force`.
- [`stim-agents`](https://github.com/stim-io/stim-agents): the agent-orchestration sidecar + standalone Tauri product.

- `AGENTS.md`: stable repo boundary and key file index
- `docs/operations/documentation.md`: documentation method
