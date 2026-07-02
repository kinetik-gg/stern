# Kinetik UI Specification: Testing, CI, Showcase, Performance, And Workflow

This file is part of the Kinetik UI architecture specification. The canonical entrypoint is [../specs.md](../specs.md).

Contained sections: 29-34.

## 29. Testing Strategy

Most UI behavior should be testable without opening a window.

Unit-testable areas:

```text
geometry math
DPI conversions
layout calculations
measurement
widget ID stability
hit testing
hover/press/click transitions
drag behavior
pointer capture
focus movement
shortcut routing
action dispatch
scroll clamping
slider value mapping
table virtualization
tree visible node calculation
theme token resolution
primitive emission
text edit state
text undo
viewport coordinate conversion
overlay placement
redraw scheduling
```

Core tests should not require GPU, window creation, or platform services.

Primitive snapshot tests may verify generated draw lists:

```rust
let output = run_ui_test(|ui| {
    ui.button("Analyze");
});

assert_snapshot!(output.primitives);
```

Renderer snapshot tests should prefer deterministic resource inventories and
backend command streams over pixel images. Backend-neutral resource snapshots
belong in `kinetik-ui-render`; Vello command snapshots belong in
`kinetik-ui-vello`. See [render-snapshots.md](../render-snapshots.md).

Interaction tests may simulate input:

```rust
let mut harness = UiTestHarness::new();
harness.pointer_move(pos);
harness.pointer_down(MouseButton::Primary);
harness.pointer_up(MouseButton::Primary);

assert!(harness.response("analyze_button").clicked);
```

Visual tests may render showcase scenes to images and compare snapshots.

Pixel-perfect visual tests should be used carefully because they can be brittle, but they are useful for catching major regressions.

## 30. Continuous Integration

The repository should include GitHub Actions workflows for formatting, linting, testing, building, docs, examples, and showcase checks.

CI should run on:

```text
pull_request events
push events to main
```

CI should not run on every feature-branch push unless that branch has an open pull request.

Default workflow trigger:

```yaml
on:
  pull_request:
  push:
    branches: [main]
```

Required checks:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
cargo doc --workspace --all-features --no-deps
```

Recommended platform matrix:

```text
Windows
Linux
macOS
```

At minimum, core tests should run on every supported CI platform.

Renderer-related checks should be split:

```text
core tests
  no GPU/window required

renderer smoke tests
  backend-specific, may be headless/offscreen where possible

showcase build
  compile showcase app without opening a window

visual regression tests
  optional, controlled, and stable enough to avoid noisy failures
```

Example workflow:

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, windows-latest, macos-latest]

    runs-on: ${{ matrix.os }}

    steps:
      - uses: actions/checkout@v4

      - uses: dtolnay/rust-toolchain@stable
        with:
          components: rustfmt, clippy

      - uses: Swatinem/rust-cache@v2

      - run: cargo fmt --all -- --check

      - run: cargo clippy --workspace --all-targets --all-features -- -D warnings

      - run: cargo test --workspace --all-features

      - run: cargo build --workspace --all-features

      - run: cargo check --workspace --examples --all-features

      - run: cargo doc --workspace --all-features --no-deps
        env:
          RUSTDOCFLAGS: -D warnings
```

## 31. Showcase Application

The toolkit should include a showcase application that acts as:

- Living documentation.
- Component gallery.
- Interaction testbed.
- Theme tuning surface.
- Visual regression target.
- LLM-readable usage reference.
- Realistic editor-layout exercise.

Showcase areas:

```text
Component gallery
  labels, buttons, icon buttons, toggles, checkboxes, radios,
  sliders, dropdowns, fields, tabs, menus, overlays

Inspector/property grid
  dense labeled rows, numeric controls, dropdowns, sections,
  disabled states, validation states

Media/editor layout
  Dock -> Frames -> Panels layout similar to editor applications

Viewport demo
  texture surface, zoom/pan, rulers, crosshair, overlays, fit/100% controls

Table stress test
  virtualized table with many rows, selection, sorting, scrolling

Text input states
  single-line, multi-line, numeric, search, undo, selection, focus

Action system demo
  same actions invoked through menu, toolbar, shortcut, command palette

Theme states
  normal, hover, pressed, focused, disabled, selected, danger, warning

DPI checks
  logical scaling, text crispness, stroke alignment
```

The showcase should use ordinary toolkit APIs, not special internal shortcuts.

## 32. Performance Requirements

The toolkit should be designed for crisp editor interaction.

Performance principles:

- Avoid continuous redraw when idle.
- Redraw immediately on input.
- Avoid blocking the UI thread.
- Cache text shaping and layout.
- Cache glyph resources.
- Virtualize large lists/tables/trees.
- Avoid per-frame full texture uploads.
- Avoid unnecessary allocation in hot paths.
- Batch or group render primitives where the backend benefits.
- Keep layout deterministic and simple.
- Expose profiling hooks early.

Performance targets should be validated in the showcase and tests:

```text
idle UI
  no unnecessary redraw loop

pointer hover
  visible feedback by next presented frame

dragging
  display-refresh-rate redraw where possible

large table
  only visible rows are measured/painted

viewport playback
  texture updates only when content changes

text fields
  typing should not reshape unrelated text
```

## 33. Implementation Workflow

The architecture should support phased issue-based PRs.

Phases describe implementation sequencing, not product scope judgment.

Recommended PR sequence:

```text
Phase 1: Repository and CI
  workspace crates
  formatting/lint/test/doc workflow
  basic crate boundaries

Phase 2: Core Geometry and DPI
  Rect, Point, Size, Vec2
  logical/physical conversion
  scale factor handling
  tests

Phase 3: Input and Frame Runtime
  UiInput
  begin_frame/end_frame
  UiMemory skeleton
  FrameOutput
  redraw requests

Phase 4: Widget Identity
  WidgetId
  ID stack
  scoped IDs
  duplicate detection
  tests

Phase 5: Measurement and Layout
  sizing rules
  row/column/stack/grid
  padding/align/spacer/separator
  measurement tests

Phase 6: Render Primitives
  primitive enum
  clipping/layer commands
  primitive snapshots
  renderer trait

Phase 7: Interaction Primitives
  hit_test
  pressable
  focusable
  draggable
  selectable
  pointer capture
  tests

Phase 8: Theme System
  base tokens
  semantic tokens
  component recipes
  default theme
  theme resolution tests

Phase 9: Basic Components
  label
  button
  icon button
  checkbox
  radio
  toggle
  slider
  panel

Phase 10: Text Subsystem
  cosmic-text integration boundary
  text layout cache
  single-line field
  numeric field
  search field
  text undo model

Phase 11: Platform Adapter
  winit integration
  window events
  input normalization
  cursor requests
  redraw scheduling

Phase 12: Vello Renderer
  render primitive translation
  text/glyph rendering path
  clipping/layers
  image support
  renderer smoke tests

Phase 13: Action System
  action descriptors
  shortcut routing
  action surfaces
  invocation queue
  command palette foundation

Phase 14: Overlays and Menus
  overlay layer
  tooltips
  popovers
  dropdowns
  custom menus
  context menus

Phase 15: Dock, Frame, Panel
  Dock layout model
  Frames
  passive Panels
  split/tab behavior
  frame focus

Phase 16: Collections
  virtual list
  table
  tree foundation
  selection model
  sorting/filtering hooks

Phase 17: Viewport Surfaces
  TextureId/TextureHandle
  viewport component
  fit/zoom/pan helpers
  coordinate conversion
  overlay drawing

Phase 18: Accessibility Semantics
  semantic node tree
  roles/states/labels
  action affordances
  adapter integration path

Phase 19: Showcase App
  component gallery
  editor layout
  viewport demo
  table stress
  action demo
  theme states

Phase 20: Performance and Debug Tools
  profiling hooks
  layout debug overlay
  hitbox/focus debug overlay
  repaint reason tracking
  cache metrics
```

Each PR should include tests for deterministic behavior when the implemented area is testable.

Agent-oriented issue templates should include:

```text
Goal
Relevant spec sections
Expected APIs
Non-goals
Tests required
Example usage
Acceptance criteria
```

## 34. Open Design Questions

These questions are intentionally left for later design decisions:

```text
Should native OS menus ever be supported as an alternate action surface?
How deep should built-in animation support go?
How should application-level undo integrate with text-field undo?
Which native accessibility backend should the winit handoff translate to first?
What exact icon format should be preferred?
How much rich text is needed beyond labels and text fields?
Should layout eventually include wrap/flex-like behavior?
```

Open questions should become focused design notes or issues before implementation reaches the affected subsystem.
