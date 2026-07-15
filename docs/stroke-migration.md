# Stroke Width Migration

Stern `1.0.0-rc.2.dev` replaces the three provisional width fields in
`ControlMetrics` with one shared stroke foundation. This is a prerelease
breaking change: control sizing and padding remain in `ControlMetrics`, while
paint widths now come from `Theme::strokes`.

## Exact stroke roles

The default theme exposes these logical-unit widths:

| Role | Default | Intended use |
| --- | ---: | --- |
| `strokes.hairline` | `1` | Structural surfaces, dividers, rows, and separators |
| `strokes.default` | `1` | Ordinary control, panel, overlay, picker, icon, and affordance boundaries |
| `strokes.emphasis` | `2` | Strong discrete indicators such as the selected-tab indicator |
| `strokes.focus.primary` | `1` | Reserved primary focus layer |
| `strokes.focus.separator` | `1` | Reserved focus contrast separator |

The focus widths are foundation values only in this migration. They do not add
or change focus primitives.

## Customizing widths

Construct all five roles explicitly and replace the scale through
`Theme::with_strokes`:

```rust
use stern::core::{StrokeScale, default_dark_theme};

let strokes = StrokeScale::from_values(
    1.0, // hairline
    1.0, // default
    2.0, // emphasis
    1.0, // focus primary
    1.0, // focus separator
);
let theme = default_dark_theme().with_strokes(strokes);
```

## Removed `ControlMetrics` fields

| Removed field | Replacement |
| --- | --- |
| `controls.border_width` | `strokes.default` for ordinary boundaries, or `strokes.hairline` for structural boundaries |
| `controls.focus_width` | `strokes.focus.primary` |
| `controls.separator_width` | `strokes.hairline` for ordinary separators, or `strokes.focus.separator` for the reserved focus separator |

`Theme::with_controls` now updates only control sizes and padding. It does not
change any stroke role.

## Legacy mirror and struct literals

`Theme::border_width` remains temporarily available as a one-way compatibility
mirror. `default_dark_theme()` and `Theme::with_strokes` set it from
`strokes.default`, but recipes and widgets read `Theme::strokes` directly.
Writing `Theme::border_width` does not update the stroke scale and does not
change rendered widths.

External `Theme` struct literals must add the new `strokes: StrokeScale` field.
Prefer starting from `default_dark_theme()` and using `with_strokes` so the
legacy mirror stays synchronized.
