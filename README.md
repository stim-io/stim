# stim

Product client/application workspace for `stim.io`.

Published `@stim-io` package consumption should go through GitHub Packages via the repo-local `.npmrc` baseline rather than through machine-only registry state.

Current cold-start structure:

- `apps/renderer`: renderer delivery wrapper, with the Vue/Vite application under `apps/renderer/vite`
- `apps/tauri`: Tauri platform shell
- `apps/controller`: Rust controller placeholder
- `crates/shared`: shared Rust primitives for non-UI layers
- `tools/stim-dev`: unified Rust dev entrypoint

- `AGENTS.md`: stable repo boundary and key file index
- `docs/operations/documentation.md`: documentation method
