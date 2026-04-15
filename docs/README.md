# Docs Map

`docs/` is organized by question, not by implementation phase.

- `architecture/`: what the client system is and how responsibility is divided
- `contracts/`: stable client-facing and host-facing contracts that code must obey
- `operations/`: how to maintain the docs system and, later, run/verify the workspace locally

Read `docs/operations/documentation.md` before doing structural docs updates.

Published `@stim-io` package registry mapping belongs in the repo-local `.npmrc`; auth material should stay outside committed repo state.

Preferred sub-structure:

- `architecture/overview.md`: top-level client framework model and design principles
- `architecture/structure.md`: durable directory ownership and structure rules
- `architecture/desktop/`: desktop host and Tauri boundary notes
- `architecture/layers/`: durable layering and ownership rules inside `stim`
- `architecture/product/`: product-facing boundary notes when they add distinct value
- `architecture/product/message-card-boundary.md`: durable ownership split for message-card composition versus shared card/layout/theme primitives
- `contracts/host/`: host control-plane and desktop capability contracts when they become real
- `contracts/product/`: client-visible product and data-view contracts when they become real
- `contracts/host/inspection.md`: current local screenshot / inspect / renderer-probe contract
- `operations/documentation.md`: canonical docs update guide and anti-duplication rules

If two docs repeat the same fact, move that fact to one canonical document and link to it.
If a document is only useful as history, delete it and rely on git history instead.
