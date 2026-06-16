# Showcase App Plan

The showcase app is the proof surface for Kinetik UI. It should feel like a
small editor workbench built from the toolkit, not a marketing page and not a
bag of decorative primitives.

## Goals

- Demonstrate the application-facing facade crate first.
- Exercise real widget responses, layout models, semantic output, renderer
  primitives, platform-shaped input, and deterministic raster verification.
- Keep each page useful as a focused regression surface.
- Make every visible interaction mutate app state through toolkit APIs.
- Keep the app fast enough for repeated local smoke runs.

## Page Roles

| Page | Purpose |
| --- | --- |
| Components | Buttons, controls, text fields, list/grid states, tabs, and primitive output. |
| Layout | Measurement-aware layout, interactive docking, splitter output, and virtualized tables. |
| Viewport | Texture surfaces, pan/zoom mapping, guides, crosshair overlays, and dynamic surface placeholders. |
| Systems | Actions, menus, command palette, overlays, runtime diagnostics, and primitive stress. |

## Implementation Rules

- Use `kinetik-ui` as the app dependency and import toolkit layers through the
  facade.
- Keep custom drawing helpers limited to shell chrome, labels, and diagnostic
  visuals that do not exist as widgets yet.
- Use widget APIs for actual controls.
- Use deterministic models for layout, docking, collections, viewport
  transforms, actions, overlays, and diagnostics.
- Preserve render-once output for visual inspection and raster tests.
- Avoid showcase-only behavior shortcuts that bypass toolkit state transitions.

## Verification

Required local checks before showcase changes are review-ready:

```text
cargo fmt --all -- --check
cargo test -p kinetik-ui-showcase --all-features
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
```

For visual changes, also render at least one full-size frame and one smaller
frame through `--render-once` and inspect the resulting bitmaps.
