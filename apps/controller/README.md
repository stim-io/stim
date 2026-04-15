# stim-controller

Rust controller scaffold for the Tauri-local sidecar/runtime surface.

Current scope includes:

- a controller-owned discovery cache/state seam
- a small server-facade trait representing the future controller-owned facade over server APIs
- the first controller-side consumer proof for the transport-neutral `DeliveryPort` seam using the loopback P2P carrier
- external-`stim-server` discovery support through `STIM_SERVER_BASE_URL` or the default local compose endpoint

This controller should be treated as part of `stim-tauri`'s local sidecar/runtime boundary, not as
an independently promoted runtime shape.

It is intentionally not part of the first `dev start` loop.
