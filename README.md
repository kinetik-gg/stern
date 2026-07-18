# Stern

Stern is a Rust UI toolkit for editor-style desktop applications.

> **Project status:** Stern is pre-alpha and has not been published to a
> package registry. The workspace is preparing a planned
> `1.0.0-rc.2.dev` package baseline; that version string does not mean a tag,
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

Stern separates the system into small, explicit layers:

```text
Application
  Business state, documents, domain rendering, jobs, and action handling.

Stern Runtime
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
stern-core
  Platform-independent runtime types and core behavior.

stern-widgets
  Components and editor patterns built on core primitives.

stern-render
  Renderer backend traits, frame contracts, diagnostics, resource payloads, and handles.

stern-text
  Text shaping, layout, editing state, and bundled font assets.

stern-vello
  Vello renderer backend.

stern-winit
  winit platform adapter.

stern-vello-winit
  Concrete Vello/winit window presenter, surface lifecycle, and recovery policy.

stern
  Facade crate.

stern-demo
  Component gallery, editor-layout examples, and visual regression surface.
```

`stern-core` must remain independent of renderer, windowing, and operating-system APIs.

### Using the current source tree

The crates are currently unpublished. From an application next to a Stern
checkout, depend on the facade with a local path:

```toml
[dependencies]
stern = { path = "../stern/crates/stern", features = ["vello-winit"] }
stern-icons-phosphor = { path = "../stern/crates/stern-icons-phosphor" }
```

Then start from the prelude:

```rust
use stern::prelude::*;
use stern_icons_phosphor as phosphor;

ui.icon_button(
    "save",
    save_rect,
    phosphor::regular::FLOPPY_DISK,
    "Save project",
    false,
);
```

Use checkout-relative paths for lower-level integration boundaries as well:

```toml
[dependencies]
stern-render = { path = "../stern/crates/stern-render" } # custom renderer contracts
stern-vello = { path = "../stern/crates/stern-vello" }   # Vello backend
stern-winit = { path = "../stern/crates/stern-winit" }   # winit platform adapter
stern-vello-winit = { path = "../stern/crates/stern-vello-winit" } # live presenter
```

### Future registry use

Only after all eight library crates are published may applications replace those paths
with exact prerelease requirements:

```toml
[dependencies]
stern = { version = "=1.0.0-rc.2.dev", features = ["vello-winit"] }
```

Lower-level registry dependencies would likewise use
`=1.0.0-rc.2.dev`. Package dry-runs and generated-archive checks establish
packageability only; they do not create a tag, publish a crate, or constitute
alpha acceptance.

The default facade stack includes the composite `vello-winit` feature. Its
presenter remains qualified at `stern::vello_winit` and is deliberately
absent from the prelude. The complete application-owned event-loop example at
`stern-vello-winit/examples/one_window.rs` creates a texture on the
presenter's device, updates it through GPU queue submissions, and composites it
without a CPU snapshot or readback:

```text
cargo run -p stern-vello-winit --example one_window
```

The example recreates and re-registers its producer texture when the presenter
reports a new device scope. This demonstrates GPU-copy interoperability; it is
not a zero-copy claim.

The `ef7c2f9` crate consolidation renamed the old `stern-render-vello`
crate to `stern-vello` and the old `stern-platform-winit` crate to
`stern-winit`. See [crate migration notes](docs/crate-migration.md).

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

Stern is licensed under the [MIT License](LICENSE).

Bundled font assets in `crates/stern-text/assets/fonts/` are licensed
separately under the SIL Open Font License 1.1; see
`crates/stern-text/assets/THIRD_PARTY.md`.
