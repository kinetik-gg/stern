# Kinetik UI

Kinetik UI is a Rust UI toolkit for editor-style desktop applications.

> **Project status:** Kinetik UI is pre-alpha and has not been published to a
> package registry. The workspace is preparing a planned
> `0.1.0-alpha.1` package baseline; that version string does not mean a tag,
> publication, or accepted alpha release exists.

It is designed for dense, tool-oriented interfaces: docked frames, passive panels, inspectors, property grids, tables, media viewports, timelines, menus, command palettes, and action-driven controls.

The architecture favors deterministic behavior, crisp rendering, reusable component patterns, and clear boundaries between UI runtime, application state, rendering, platform integration, and domain work.

Catalogue claims use the ALPHA-00 capability vocabulary: **Model**, **Paint**,
**Input**, **Accessibility**, **Platform**, and **Live Workflow**. A surface is
`Stable` only when behavioral evidence proves every capability axis it
requires. `Experimental` and `Planned` surfaces may be incomplete, and
metadata-only evidence proves no capability axis. There are currently no
`Stable` catalogue entries.

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
Dock
  -> Frame
      -> Panel
          -> Components
              -> Primitives
```

- `Dock` arranges editor regions.
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

kinetik-ui-text
  Text shaping, layout, editing state, and bundled font assets.

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

### Using the current source tree

The crates are currently unpublished. From an application next to a Kinetik UI
checkout, depend on the facade with a local path:

```toml
[dependencies]
kinetik-ui = { path = "../kinetik-ui/crates/kinetik-ui", features = ["platform-winit", "render-vello"] }
```

Then start from the prelude:

```rust
use kinetik_ui::prelude::*;
```

Use checkout-relative paths for lower-level integration boundaries as well:

```toml
[dependencies]
kinetik-ui-render = { path = "../kinetik-ui/crates/kinetik-ui-render" } # custom renderer contracts
kinetik-ui-vello = { path = "../kinetik-ui/crates/kinetik-ui-vello" }   # Vello backend
kinetik-ui-winit = { path = "../kinetik-ui/crates/kinetik-ui-winit" }   # winit platform adapter
```

### Future registry use

Only after all seven crates are published may applications replace those paths
with exact prerelease requirements:

```toml
[dependencies]
kinetik-ui = { version = "=0.1.0-alpha.1", features = ["platform-winit", "render-vello"] }
```

Lower-level registry dependencies would likewise use
`=0.1.0-alpha.1`. Package dry-runs and generated-archive checks establish
packageability only; they do not create a tag, publish a crate, or constitute
alpha acceptance.

The `ef7c2f9` crate consolidation renamed the old `kinetik-ui-render-vello`
crate to `kinetik-ui-vello` and the old `kinetik-ui-platform-winit` crate to
`kinetik-ui-winit`. See [crate migration notes](docs/crate-migration.md).

## Documentation

- [Architecture specification](docs/specs.md)
- [Accessibility adapter boundary](docs/accessibility-adapters.md)
- [Crate split migration](docs/crate-migration.md)
- [Docking interactions](docs/docking-interactions.md)
- [Renderer snapshot strategy](docs/render-snapshots.md)
- [Showcase app plan](docs/showcase-plan.md)
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

Bundled font assets in `crates/kinetik-ui-text/assets/fonts/` are licensed
separately under the SIL Open Font License 1.1; see
`crates/kinetik-ui-text/assets/THIRD_PARTY.md`.
