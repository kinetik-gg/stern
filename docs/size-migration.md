# Exact Size Foundation Migration

Stern `1.0.0-rc.2.dev` adds one renderer-neutral `SizeScale` authority to
`Theme`. This is a prerelease breaking shape change: external `Theme` struct
literals must initialize the new `sizes` field.

## Exact inventory

The foundation stores all fourteen pinned logical-unit values:

| Typed token | Field | Default |
| --- | --- | ---: |
| `SizeToken::ControlXs` | `sizes.control.xs` | 20 |
| `SizeToken::ControlSm` | `sizes.control.sm` | 24 |
| `SizeToken::ControlMd` | `sizes.control.md` | 28 |
| `SizeToken::ControlLg` | `sizes.control.lg` | 32 |
| `SizeToken::RowCompact` | `sizes.row.compact` | 24 |
| `SizeToken::RowStandard` | `sizes.row.standard` | 28 |
| `SizeToken::Tab` | `sizes.tab` | 28 |
| `SizeToken::PanelHeader` | `sizes.panel_header` | 30 |
| `SizeToken::WorkspaceBar` | `sizes.workspace_bar` | 40 |
| `SizeToken::IconSm` | `sizes.icon.sm` | 12 |
| `SizeToken::IconMd` | `sizes.icon.md` | 16 |
| `SizeToken::IconLg` | `sizes.icon.lg` | 20 |
| `SizeToken::HandleVisual` | `sizes.handle.visual` | 1 |
| `SizeToken::HandleHit` | `sizes.handle.hit` | 7 |

`SizeToken::ALL` exposes that exact order. `SizeScale::get` resolves a typed
token without a string key.

## Customization

Start from the standard theme and replace only its size foundation:

```rust
use stern::core::{
    ControlSizeScale, HandleSizeScale, IconSizeScale, RowSizeScale, SizeScale,
    SizeToken, default_dark_theme,
};

let sizes = SizeScale::new(
    ControlSizeScale::new(20.0, 24.0, 28.0, 32.0),
    RowSizeScale::new(24.0, 28.0),
    28.0,
    30.0,
    40.0,
    IconSizeScale::new(12.0, 16.0, 20.0),
    HandleSizeScale::new(1.0, 7.0),
);
let theme = default_dark_theme().with_sizes(sizes);

assert_eq!(theme.sizes.get(SizeToken::PanelHeader), 30.0);
```

`Theme::with_sizes` replaces only `Theme::sizes`. Other theme groups and the
legacy scalar compatibility fields are preserved. Conversely,
`Theme::with_spacing` preserves a customized size foundation.

## Medium icon consumer authority

`Theme::sizes.icon.md` is the only production default for icon-button visual
geometry. It supplies the unsized bitmap and selectable-bitmap paths plus the
destination rectangle for direct `StaticIcon` vector primitives. It is also the
fallback when an explicit bitmap or selectable-bitmap size is non-finite or not
positive. Static icons require no registry and have no missing-lookup symbol.

Valid explicit sizes passed to `image_icon_button_sized` and
`image_icon_selectable_button_sized` remain authoritative and are not replaced
by the theme default.

`ControlMetrics::icon_size` has been removed. External `ControlMetrics` struct
literals must delete that field. Its remaining `control_height`,
`compact_control_height`, `padding_x`, and `padding_y` fields keep their
existing defaults and customization behavior, but cannot affect icon geometry.

## Selection indicator consumer authority

Checkbox and radio visible indicator geometry now resolves one private named
component-recipe dimension at exactly `14.0` logical units. `Theme::checkbox`
places that value in the unchanged public `CheckRecipe::size` field, and
`Theme::radio_button` continues to inherit the checkbox recipe before replacing
only its radius. The exact `14.0` value is not a size-foundation token and must
not be replaced with `sizes.icon.*` or a compatibility alias.

`ControlMetrics::check_size` has been removed. External `ControlMetrics` struct
literals must delete that field; there is no replacement customization hook.
The remaining `control_height`, `compact_control_height`, `padding_x`, and
`padding_y` fields remain unchanged and cannot affect checkbox or radio
indicator geometry.

Caller-provided control rectangles and full-label response and semantic bounds
remain authoritative. The visible indicator remains `14 x 14` logical units
through selected, unselected, hover, focus, and disabled states, and focus
layers remain additive. Unrounded physical-space transport is exactly `14.0`,
`17.5`, `21.0`, and `28.0` at scales `1.0`, `1.25`, `1.5`, and `2.0`; this does
not claim raster snapping or reviewed renderer baselines.

The size foundation intentionally provides no legacy aliases, mirrors, or
forwarding methods. Adoption by other component families and reconciliation of
hardcoded geometry remain separate inventoried changes.
