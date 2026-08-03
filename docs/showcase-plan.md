# Showcase App Plan

The showcase app is the proof surface for Stern. It should feel like a
small editor workbench built from the toolkit, not a marketing page and not a
bag of decorative primitives.

Current component conformance status is tracked honestly in
[`catalogue-conformance-matrix.md`](catalogue-conformance-matrix.md): no
component has verified paint/platform/accessibility evidence yet, and the
intended verification ledger is the design system's parity index, not this
repository.

## Goals

- Demonstrate the application-facing facade crate first.
- Exercise real widget responses, layout models, semantic output, renderer
  primitives, platform-shaped input, and deterministic raster verification.
- Keep each page useful as a focused regression surface.
- Make every visible interaction mutate app state through toolkit APIs.
- Keep the app fast enough for repeated local smoke runs.

## Page Roles

The demo currently ships two navigable workspaces (`DemoWorkspace` in
`apps/stern-demo/src/app_model.rs`): `Edit` and `Graph`. The other planned
pages below were never built as separate pages; their intended content lives,
if at all, inside the `Edit` workspace's dock (see
`apps/stern-demo/src/edit_workspace.rs` and `timeline_workspace.rs`).

| Page | Purpose | Status |
| --- | --- | --- |
| Editor | Integrated DCC/game-engine workbench proving the toolkit layers compose into a reachable application surface. | Built as the `Edit` workspace. |
| Graph | Node graph editing surface: node/edge selection, connection, and layout. | Built as the `Graph` workspace (`apps/stern-demo/src/graph_workspace.rs`); not part of the original plan. |
| Components | Buttons, controls, text fields, list/grid states, tabs, and primitive output as a standalone page. | Not built. |
| Layout | Measurement-aware layout, interactive docking, splitter output, and virtualized tables as a standalone page. | Not built. |
| Viewport | Texture surfaces, pan/zoom mapping, guides, crosshair overlays, and dynamic surface placeholders as a standalone page. | Not built. |
| Systems | Actions, menus, command palette, overlays, runtime diagnostics, and primitive stress as a standalone page. | Not built. |

## Implementation Rules

- Use `stern` as the app dependency and import toolkit layers through the
  facade.
- Keep custom drawing helpers limited to shell chrome, labels, and diagnostic
  visuals that do not exist as widgets yet.
- Use widget APIs for actual controls.
- Use deterministic models for layout, docking, collections, viewport
  transforms, actions, overlays, and diagnostics.
- Preserve render-once output for visual inspection and raster tests.
- Avoid showcase-only behavior shortcuts that bypass toolkit state transitions.

## Action Truth Contract

- An enabled action must produce its labeled, deterministic state transition or
  output. Updating only a generic status string does not make an action
  implemented.
- Unfinished actions remain visible for catalogue context only when their label
  ends in `(Experimental)`. They are disabled, have no shortcut, and cannot
  enter an invocation queue or handler.
- The editor currently demonstrates play, stop, grid visibility, tool selection,
  panel focus, dock rearrangement, and one fixed online-documentation request.
  Help-menu, About-modal, and F1 surfaces dispatch that same application-owned
  action; only the Winit shell opens the URL. Pause, persistence, project/file
  operations, build/export/package, preferences, and command palette lifecycles
  remain Experimental.
- The Components counter and Systems dispatch actions mutate dedicated demo
  counters. Systems `Save Workspace` captures a deterministic in-memory snapshot;
  it does not imply file persistence.
- These showcase corrections do not promote any capability to Stable.

## Verification

Required local checks before showcase changes are review-ready:

```text
cargo fmt --all -- --check
cargo test -p stern-demo --all-features
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
```

For visual changes, also render at least one full-size frame and one smaller
frame through `--render-once` and inspect the resulting bitmaps.

There is currently no CLI-driven CPU raster review-dump workflow; the demo
binary only supports `--dump-identity-evidence`.
