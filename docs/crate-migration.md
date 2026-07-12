# Crate Split Migration

Commit `ef7c2f9` consolidated the toolkit into the current crate graph and
introduced the application-facing `kinetik-ui` facade crate. This was a
breaking crate-boundary change.

## Which Crate To Depend On

The crates are currently unpublished. Most applications working from source
should depend on the facade by path (adjust the relative path for the location
of the checkout):

```toml
[dependencies]
kinetik-ui = { path = "../kinetik-ui/crates/kinetik-ui", features = ["vello-winit"] }
```

After `0.1.0-alpha.1` has actually been published, the future registry form is:

```toml
[dependencies]
kinetik-ui = { version = "=0.1.0-alpha.1", features = ["vello-winit"] }
```

The facade re-exports the common application stack through
`kinetik_ui::prelude::*` and namespaced modules:

```rust
use kinetik_ui::prelude::*;
```

Use lower-level crates only when building an integration boundary or a custom
backend. Today, use source paths:

```toml
[dependencies]
kinetik-ui-core = { path = "../kinetik-ui/crates/kinetik-ui-core" }
kinetik-ui-widgets = { path = "../kinetik-ui/crates/kinetik-ui-widgets" }
kinetik-ui-render = { path = "../kinetik-ui/crates/kinetik-ui-render" }
kinetik-ui-vello = { path = "../kinetik-ui/crates/kinetik-ui-vello" }
kinetik-ui-winit = { path = "../kinetik-ui/crates/kinetik-ui-winit" }
kinetik-ui-vello-winit = { path = "../kinetik-ui/crates/kinetik-ui-vello-winit" }
```

Once published, each lower-level registry dependency must use the exact
`=0.1.0-alpha.1` requirement. A package dry-run is not publication and is not
a reason to use the registry snippets early.

## Migration Map

| Before `ef7c2f9` | After `ef7c2f9` | Use for |
| --- | --- | --- |
| application code importing several toolkit crates directly | `kinetik-ui` | Normal app code, examples, and common prelude imports |
| `kinetik-ui-core` | `kinetik-ui-core` | Platform-independent runtime, input, layout, IDs, actions, semantics, theme, and render primitives |
| `kinetik-ui-widgets` | `kinetik-ui-widgets` | Reusable widgets, editor models, overlays, collections, docking, and viewport helpers |
| renderer contracts inside lower-level code | `kinetik-ui-render` | Backend-neutral renderer traits, diagnostics, frame contracts, and resource payloads |
| `kinetik-ui-render-vello` | `kinetik-ui-vello` | Vello renderer backend and primitive translation |
| `kinetik-ui-platform-winit` | `kinetik-ui-winit` | winit input normalization, platform requests, DPI, cursor, IME, redraw, and accessibility handoff data |
| `kinetik-ui-text` | `kinetik-ui-text` | Text editing, shaping, measurement, hit testing, and layout cache |
| no prior supported presenter crate | `kinetik-ui-vello-winit` | Concrete Vello/winit surface, device, presentation, and recovery integration |

## Import Changes

Prefer facade imports in application code:

```rust
use kinetik_ui::prelude::*;
```

When a boundary needs a specific layer, import that layer directly:

```rust
use kinetik_ui_render::{RenderFrameInput, RenderResources, RendererBackend};
use kinetik_ui_vello::VelloRenderer;
use kinetik_ui_winit::{WinitInputAdapter, frame_context_from_winit};
use kinetik_ui_vello_winit::{VelloPresenterConfig, VelloWindowPresenter};
```

## Boundary Rules

- `kinetik-ui-core` remains free of winit, Vello, wgpu, OS APIs, and renderer
  backend types.
- Custom renderers should depend on `kinetik-ui-render`, not widget crates.
- Vello-specific code should depend on `kinetik-ui-vello`.
- winit shells should depend on `kinetik-ui-winit` or enable the facade's
  `platform-winit` feature.
- Applications using the accepted live Vello window path should depend on
  `kinetik-ui-vello-winit` directly or enable the facade's composite
  `vello-winit` feature. Presenter types remain under
  `kinetik_ui::vello_winit`, not the prelude.
- Applications that want the full default stack can use the facade default
  features.
