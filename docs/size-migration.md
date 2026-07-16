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
geometry. It supplies the unsized bitmap and selectable-bitmap paths, registered
vector icons, and the missing-vector symbol. It is also the fallback when an
explicit bitmap or selectable-bitmap size is non-finite or not positive.

Valid explicit sizes passed to `image_icon_button_sized` and
`image_icon_selectable_button_sized` remain authoritative and are not replaced
by the theme default.

`ControlMetrics::icon_size` has been removed. External `ControlMetrics` struct
literals must delete that field. Its remaining `control_height`,
`compact_control_height`, `check_size`, `padding_x`, and `padding_y` fields keep
their existing defaults and customization behavior, but cannot affect icon
geometry.

The size foundation intentionally provides no legacy aliases, mirrors, or
forwarding methods. Adoption by other component families and reconciliation of
hardcoded geometry remain separate inventoried changes.
