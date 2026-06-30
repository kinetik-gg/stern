# Catalogue Conformance Matrix

This matrix is the S14 review surface for restarted editor-toolkit S10-S13
coverage. It summarizes the live data-only registry in
`kinetik-ui-widgets::COMPONENT_CONFORMANCE_MATRIX` and points reviewers at the
existing showcase fixtures that make the coverage inspectable.

All rows are intentionally `Partial`. They prove public contracts, deterministic
model behavior, semantics, and existing showcase fixture reachability. They do
not claim complete rendered widgets, fake application/domain execution, or
committed raster baselines.

## S10 Outliner And Asset Browser

| Capability | Status | Conformance tests | Showcase fixture | Non-goals |
| --- | --- | --- | --- | --- |
| `s10-outliner-tree-selection-semantics` | Partial | `outliner_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Explorer frame/tree fixture | No real project graph mutation or domain scene execution. |
| `s10-asset-browser-grid-list-metadata` | Partial | `asset_browser_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Asset Browser frame/grid fixture | No filesystem asset indexing or import pipeline. |
| `s10-inline-edit-rename-lifecycle` | Partial | `inline_edit_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Explorer/Asset Browser metadata path | No committed rename side effects. |
| `s10-collection-drag-drop-context` | Partial | `collection_drag_context_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Explorer/Asset Browser collection fixtures | No live drag/drop domain mutation. |
| `s10-collection-filter-sort-selection-preservation` | Partial | `collection_projection_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Asset Browser filter/sort fixture | No persistent asset database. |

## S11 Timeline

| Capability | Status | Conformance tests | Showcase fixture | Non-goals |
| --- | --- | --- | --- | --- |
| `s11-timeline-layout-coordinate-selection` | Partial | `timeline_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Timeline frame fixture | No media playback or clip editing engine. |
| `s11-ruler-ticks-timecode` | Partial | `timeline_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Timeline ruler fixture | No renderer-specific time ruler implementation. |
| `s11-transport-action-controls` | Partial | `timeline_transport_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: transport controls/status fixture | No duplicated command logic in controls. |
| `s11-timeline-snapping` | Partial | `timeline_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Timeline snap metadata fixture | No app-owned edit operation execution. |
| `s11-timeline-preservation` | Partial | `timeline_conformance::*`, `component_taxonomy_conformance::s10_s11_*` | Editor page: Timeline state fixture | No project persistence format changes. |

## S12 Viewport Tools

| Capability | Status | Conformance tests | Showcase fixture | Non-goals |
| --- | --- | --- | --- | --- |
| `s12-viewport-surface-overlays` | Partial | `viewport_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: Viewport frame; Viewport page: Pan/Zoom Texture Surface | No GPU/domain texture production beyond existing fixtures. |
| `s12-viewport-tools-transform-handles` | Partial | `viewport_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: Viewport toolbar/tool fixture | No actual scene transform execution. |
| `s12-viewport-action-routing` | Partial | `viewport_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: viewport toolbar/status fixture | No application command execution beyond existing action recording. |
| `s12-viewport-guides-rulers-safe-areas-hud` | Partial | `viewport_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Viewport page: guides, rulers, safe area, HUD fixture | No renderer-owned overlay backend behavior. |

## S13 Jobs And Diagnostics

| Capability | Status | Conformance tests | Showcase fixture | Non-goals |
| --- | --- | --- | --- | --- |
| `s13-progress-indicator-metadata` | Partial | `status_bar_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: status bar progress fixture | No worker queue or async job runtime. |
| `s13-job-list-progress-cancel` | Partial | `status_bar_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: job list/status fixture | No real cancellation side effects. |
| `s13-diagnostic-strip-codes-fields-ordering` | Partial | `status_bar_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: diagnostic strip fixture | No compiler or domain diagnostic source execution. |
| `s13-feedback-stack-lifetime-repaint` | Partial | `status_bar_conformance::*`, `component_taxonomy_conformance::s12_s13_*` | Editor page: feedback stack fixture | No external feedback/report transport. |

## Manual Review Artifacts

Generate disposable CPU raster artifacts when a reviewer wants inspectable
showcase output for this matrix:

```text
cargo run -p kinetik-ui-showcase -- --dump-review-artifacts s14-s10-s13-matrix --width 1440 --height 900
```

The dump writes a `manifest.txt`, CPU raster BMP frames, and per-page
`*-pixel-smoke.txt` summaries under
`target/kinetik-ui-artifacts/kinetik-ui-showcase/review-dumps/`. These files are
manual review artifacts only. Do not commit them as raster baselines, and do not
add a bless/update workflow for this matrix.
