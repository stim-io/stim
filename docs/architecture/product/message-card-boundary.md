# Message Card Boundary

This document defines the durable ownership split for message-card rendering across `stim` and `stim-packages`.

## Core rule

`stim` renderer is the product composition layer.

It should primarily do only these things for message cards:

- choose which business-facing card composition to render
- map controller/service data into explicit component props
- declare layout-related intent through bounded props
- assemble shared primitives into screen- and feature-level UI

It should not become the home of:

- atomic card visuals
- reusable layout primitives
- theme styling
- local CSS tuning for reusable message-card behavior

Those concerns belong in `stim-packages/`.

## `stim-packages/` owns

- atomic card/surface primitives used by message cards
- shared layout primitives that control spacing, stacking, framing, and vertical-space behavior
- theme-aware visual styling for those primitives
- reusable rich-content containers that remain below product/business composition

When message-card implementation pressure exposes missing visual or layout capability, the default response is to add or improve a shared primitive in `stim-packages/`, not to solve it with product-local renderer CSS.

## `stim` owns

- controller-to-view adaptation of message content
- business-facing card composition for chat/thread screens
- mapping from shared protocol `layout_hint` into bounded UI props
- product-level decisions about which card composition to show for a given message

`stim` may define app-local business components that assemble shared primitives, but those components should stay thin and prop-driven.

## Layout rule

Layout is part of the shared presentation foundation, not a product-local afterthought.

That means:

- reusable stack/frame/surface primitives belong in `stim-packages/`
- theme-aware spacing and framing belong in `stim-packages/`
- `stim` should express vertical-space intent through props such as layout family, pressure, or size hints rather than through ad hoc local CSS behavior

If a message-card feature requires repeated layout tuning in `stim`, treat that as a missing shared layout primitive.

## Theme rule

Theme styling authority for message cards stays in `stim-packages/`.

`stim` may choose theme at the application level, but it should not become the place where reusable message-card theme details are authored or patched.

## Slice 7 implication

For the first rich message-card path:

- keep protocol/content adaptation in controller and renderer composition code inside `stim`
- move reusable card, layout, and theme behavior into `stim-packages/`
- keep renderer implementation focused on composition and prop declaration rather than styling ownership

The first Slice 7 implementation should therefore land in two layers:

1. shared primitives in `stim-packages/`
2. thin message-card composition in `stim`

Do not invert that order by building product-local card/layout styling in `stim` first and planning to extract it later.
