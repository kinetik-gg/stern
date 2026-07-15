# Spacing migration

Stern `1.0.0-rc.2.dev` replaces the provisional five-step spacing API with
the exact compact spacing foundation. This is a prerelease breaking change.
There are no compatibility fields, aliases, or forwarding methods.

## Exact ladder

`SpacingScale` now stores every normative step directly:

| Field | `SpacingStep` | Default |
| --- | --- | ---: |
| `zero` | `Zero` | 0 |
| `one` | `One` | 2 |
| `two` | `Two` | 4 |
| `three` | `Three` | 6 |
| `four` | `Four` | 8 |
| `five` | `Five` | 12 |
| `six` | `Six` | 16 |
| `seven` | `Seven` | 24 |
| `eight` | `Eight` | 32 |

Construct custom scales with all nine values in ascending step order:

```rust
use stern::core::SpacingScale;

let spacing = SpacingScale::new(
    0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0,
);
```

The former defaults correspond to `xs -> one`, `sm -> two`, `md -> four`,
`lg -> five`, and `xl -> six`. Migrate by intent rather than preserving those
names: use `SpacingScale::get(SpacingStep)` for an exact ladder choice or a
semantic role where one applies.

## Semantic roles

`SpacingScale::resolve(SpacingRole)` derives each value from the configured
ladder. Stern does not store or duplicate semantic spacing numbers.

| `SpacingRole` | Step | Default |
| --- | --- | ---: |
| `IconLabelGap` | `Two` | 4 |
| `TightControlGap` | `Two` | 4 |
| `CompactInlineControlPadding` | `Three` | 6 |
| `DefaultInlineControlPadding` | `Four` | 8 |
| `BlockControlPadding` | `Two` | 4 |
| `InspectorLabelValueGap` | `Four` | 8 |
| `GroupGap` | `Four` | 8 |
| `PanelPadding` | `Four` | 8 |
| `SectionGap` | `Six` | 16 |

For example:

```rust
use stern::core::{SpacingRole, SpacingScale, default_dark_theme};

let spacing = SpacingScale::new(
    0.0, 3.0, 5.0, 7.0, 9.0, 13.0, 17.0, 25.0, 33.0,
);
let theme = default_dark_theme().with_spacing(spacing);

assert_eq!(theme.spacing.resolve(SpacingRole::PanelPadding), 9.0);
assert_eq!(theme.spacing.resolve(SpacingRole::SectionGap), 17.0);
```

`Theme::with_spacing` remains the replacement API and changes only the spacing
scale. Colors, radii, strokes, typography, opacity, elevation, duration,
control metrics, and legacy theme mirrors remain unchanged.

This foundation change does not migrate widget layout consumers, control or
icon sizes, structural dimensions, renderer behavior, or platform behavior.
