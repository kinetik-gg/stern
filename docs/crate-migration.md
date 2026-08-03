# Crate Split Migration

Commit `ef7c2f9` consolidated the toolkit into the current crate graph and
introduced the application-facing `stern` facade crate. This was a
breaking crate-boundary change.

`stern-icon-atlas` and `stern-identity-scan` moved from `crates/` to `tools/`
and out of the product `[workspace]`: they are development tooling, not
product crates. Each now builds as its own standalone Cargo workspace; see
the README's Crates section.

## Which Crate To Depend On

The crates are currently unpublished. Most applications working from source
should depend on the facade by path (adjust the relative path for the location
of the checkout):

```toml
[dependencies]
stern = { path = "../stern/crates/stern", features = ["vello-winit"] }
```

After `0.2.0-alpha.1` has actually been published, the future registry form is:

```toml
[dependencies]
stern = { version = "=0.2.0-alpha.1", features = ["vello-winit"] }
```

The facade re-exports the common application stack through
`stern::prelude::*` and namespaced modules:

```rust
use stern::prelude::*;
```

Use lower-level crates only when building an integration boundary or a custom
backend. Today, use source paths:

```toml
[dependencies]
stern-core = { path = "../stern/crates/stern-core" }
stern-widgets = { path = "../stern/crates/stern-widgets" }
stern-render = { path = "../stern/crates/stern-render" }
stern-vello = { path = "../stern/crates/stern-vello" }
stern-winit = { path = "../stern/crates/stern-winit" }
stern-vello-winit = { path = "../stern/crates/stern-vello-winit" }
```

Once published, each lower-level registry dependency must use the exact
`=0.2.0-alpha.1` requirement. A package dry-run is not publication and is not
a reason to use the registry snippets early.

## Migration Map

| Before `ef7c2f9` | After `ef7c2f9` | Use for |
| --- | --- | --- |
| application code importing several toolkit crates directly | `stern` | Normal app code, examples, and common prelude imports |
| `stern-core` | `stern-core` | Platform-independent runtime, input, layout, IDs, actions, semantics, theme, and render primitives |
| `stern-widgets` | `stern-widgets` | Reusable widgets, editor models, overlays, collections, docking, and viewport helpers |
| renderer contracts inside lower-level code | `stern-render` | Backend-neutral renderer traits, diagnostics, frame contracts, and resource payloads |
| `stern-render-vello` | `stern-vello` | Vello renderer backend and primitive translation |
| `stern-platform-winit` | `stern-winit` | winit input normalization, platform requests, DPI, cursor, IME, redraw, and accessibility handoff data |
| `stern-text` | `stern-text` | Text editing, shaping, measurement, hit testing, and layout cache |
| no prior supported presenter crate | `stern-vello-winit` | Concrete Vello/winit surface, device, presentation, and recovery integration |

## Import Changes

Prefer facade imports in application code:

```rust
use stern::prelude::*;
```

When a boundary needs a specific layer, import that layer directly:

```rust
use stern_render::{RenderFrameInput, RenderResources, RendererBackend};
use stern_vello::VelloRenderer;
use stern_winit::{WinitInputAdapter, frame_context_from_winit};
use stern_vello_winit::{VelloPresenterConfig, VelloWindowPresenter};
```

## Boundary Rules

- `stern-core` remains free of winit, Vello, wgpu, OS APIs, and renderer
  backend types.
- Custom renderers should depend on `stern-render`, not widget crates.
- Vello-specific code should depend on `stern-vello`.
- winit shells should depend on `stern-winit` or enable the facade's
  `platform-winit` feature.
- Applications using the accepted live Vello window path should depend on
  `stern-vello-winit` directly or enable the facade's composite
  `vello-winit` feature. Presenter types remain under
  `stern::vello_winit`, not the prelude.
- Applications that want the full default stack can use the facade default
  features.
