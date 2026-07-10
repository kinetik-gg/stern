# Showcase App Plan

The showcase app is the proof surface for Kinetik UI. It should feel like a
small editor workbench built from the toolkit, not a marketing page and not a
bag of decorative primitives.

The S10-S13 catalogue evidence review surface lives in
[`catalogue-conformance-matrix.md`](catalogue-conformance-matrix.md). It maps
restarted editor-toolkit capabilities to honest `Experimental` status,
Model-evidenced deterministic contracts, existing showcase fixtures, and
explicit non-goals. Fixture reachability alone remains metadata-only evidence.

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

For review packages that need inspectable CPU raster artifacts without invoking a
GPU renderer, generate an explicit review dump:

```text
cargo run -p kinetik-ui-showcase -- --dump-review-artifacts review-label --page components --width 1440 --height 900
```

Review dumps are written under
`target/kinetik-ui-artifacts/kinetik-ui-showcase/review-dumps/` and include a
`manifest.txt`, CPU raster BMP frames, and `<page>-pixel-smoke.txt` summaries for
the selected page, or all showcase pages when `--page` is omitted. Pixel-smoke
summaries record dimensions, total pixels, visible variation, non-first-pixel
count, bounded unique color count, and a deterministic checksum for manual
comparison. They are manual review outputs only; they are not committed baselines
and there is no bless/update workflow.
