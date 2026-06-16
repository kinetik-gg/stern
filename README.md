# Kinetik UI

Kinetik UI is a Rust UI toolkit for editor-style desktop applications.

It is designed for dense, tool-oriented interfaces: docked frames, passive panels, inspectors, property grids, tables, media viewports, timelines, menus, command palettes, and action-driven controls.

The architecture favors deterministic behavior, crisp rendering, reusable component patterns, and clear boundaries between UI runtime, application state, rendering, platform integration, and domain work.

## Architecture

Kinetik UI separates the system into small, explicit layers:

```text
Application
  Business state, documents, domain rendering, jobs, and action handling.

Kinetik UI Runtime
  Frame lifecycle, layout, widget identity, UI memory, interaction state,
  action dispatch, semantic nodes, and draw-list generation.

Widgets / Components
  Reusable controls built from layout, behavior primitives, theme recipes,
  render primitives, and semantic nodes.

Renderer Backend
  Backend-independent render contracts, primitive rendering, and texture compositing.

Platform Adapter
  Window events, input, DPI, cursor, clipboard, and redraw scheduling.
```

The main editor hierarchy is:

```text
DockArea
  -> Frame
      -> Panel
          -> Components
              -> Primitives
```

- `DockArea` arranges editor regions.
- `Frame` owns docked/sub-window behavior such as active state, resizing, closing, tabbing, merging, and focus.
- `Panel` is a passive content surface that receives space from its parent.

## Design Principles

- Components are not primitives.
- Behavior primitives are visually neutral.
- Appearance comes from theme tokens and style recipes.
- Renderers consume render primitives, not widget types.
- Applications own business logic and execute actions.
- The UI presents actions and dispatches invocations.
- Layout uses logical units; rendering targets physical pixels.
- Heavy work runs outside widget calls.
- Core behavior should be testable without a window or GPU.

Example composition:

```text
Button    = pressable + label/icon layout + button style recipe
TabHeader = selectable + pressable + tab style recipe
MenuItem  = pressable + row layout + menu style recipe
Slider    = draggable + value mapping + track/thumb style recipe
```

## Crates

The intended workspace layout is:

```text
kinetik-ui-core
  Platform-independent runtime types and core behavior.

kinetik-ui-widgets
  Components and editor patterns built on core primitives.

kinetik-ui-render
  Renderer backend traits, frame contracts, diagnostics, resource payloads, and handles.

kinetik-ui-vello
  Vello renderer backend.

kinetik-ui-winit
  winit platform adapter.

kinetik-ui
  Facade crate.

kinetik-ui-showcase
  Component gallery, editor-layout examples, and visual regression surface.
```

`kinetik-ui-core` must remain independent of renderer, windowing, and operating-system APIs.

## Documentation

- [Architecture specification](docs/specs.md)
- [Renderer snapshot strategy](docs/render-snapshots.md)
- [Release policy](docs/release.md)
- [Agent workflow](AGENTS.md)

The architecture specification is the primary reference for terminology, subsystem boundaries, test expectations, and phase-based PR workflow.

## Checks

Repository checks are expected to include:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
cargo doc --workspace --all-features --no-deps
```

GitHub Actions runs these checks on pull requests and pushes to `main`.

## License

Kinetik UI is licensed under the [MIT License](LICENSE).
