# Semantic Font-Family Migration

Stern `1.0.0-rc.2.dev` replaces the five resolved `FontToken` values stored in
`TypographyScale` with one semantic family authority, four exact foundation
scales, and five logical metric records. This is a prerelease breaking
struct-shape change. External `TypographyScale` literals must add `families`,
`sizes`, `line_heights`, `weights`, and `features`, then replace their stored
`FontToken` values with `TextRoleMetrics`.

## Exact family roles

The default typography foundation exposes three distinct roles:

| Role | Default family | Intended boundary |
| --- | --- | --- |
| `FontFamilyRole::Ui` | Inter | Dense controls, labels, menus, panels, and body copy |
| `FontFamilyRole::Brand` | Space Grotesk | Product identity and rare display moments |
| `FontFamilyRole::Mono` | Space Mono | Code, technical identifiers, and fixed-format values |

`FontFamilyRole::ALL` contains that exact order. `FontFamilyScale::get` and
`Theme::font_family` provide typed lookup without component-local family names.

## Exact foundation tokens

The foundation scales retain the exact token order and values from the pinned
design-system contract:

| Size token | Default |
| --- | ---: |
| `FontSizeToken::Ui` | 12 |
| `FontSizeToken::Dense` | 11 |
| `FontSizeToken::Metadata` | 10 |
| `FontSizeToken::Section` | 14 |
| `FontSizeToken::Dialog` | 16 |
| `FontSizeToken::Heading` | 20 |

| Line-height token | Default |
| --- | ---: |
| `FontLineHeightToken::Ui` | 16 |
| `FontLineHeightToken::Dense` | 15 |
| `FontLineHeightToken::Metadata` | 14 |

| Weight token | Default |
| --- | ---: |
| `FontWeightToken::Regular` | 400 |
| `FontWeightToken::Medium` | 500 |
| `FontWeightToken::Semibold` | 600 |
| `FontWeightToken::Bold` | 700 |

`FontFeatureToken::Numeric` resolves to the semantic feature value
`"tabular-nums"`. Each token enum exposes `ALL` in the table order. The
matching scale stores the customizable value once and exposes typed `get`
lookup through `theme.typography`.

These scales are foundation metadata only. They are deliberately separate from
the five resolved text-role recipes, so adding them does not change existing
component typography.

## Text-role mapping

`TypographyScale` stores only `TextRoleMetrics { size, line_height }` for its
five text roles. Resolution through `Theme::font` combines those metrics with
one semantic family:

| Text role | Family role | Size | Line height |
| --- | --- | ---: | ---: |
| `Body` | UI | 12 | 17 |
| `Label` | UI | 12 | 16 |
| `Caption` | UI | 11 | 15 |
| `Title` | UI | 14 | 19 |
| `Monospace` | Mono | 12 | 17 |

`Title` deliberately remains UI typography. The Brand family is public and
customizable but is not assigned to an existing `TextRole` by this migration.

## Updating a struct literal

Construct the semantic families and logical metrics separately:

```rust
use stern::core::{
    FontFamilyRole, FontFamilyScale, FontFeatureScale, FontFeatureToken,
    FontLineHeightScale, FontSizeScale, FontSizeToken, FontWeightScale, TextRole,
    TextRoleMetrics, TypographyScale, default_dark_theme,
};

let typography = TypographyScale {
    families: FontFamilyScale::new("Inter", "Space Grotesk", "Space Mono"),
    sizes: FontSizeScale::new(12.0, 11.0, 10.0, 14.0, 16.0, 20.0),
    line_heights: FontLineHeightScale::new(16.0, 15.0, 14.0),
    weights: FontWeightScale::new(400, 500, 600, 700),
    features: FontFeatureScale::new("tabular-nums"),
    body: TextRoleMetrics::new(12.0, 17.0),
    label: TextRoleMetrics::new(12.0, 16.0),
    caption: TextRoleMetrics::new(11.0, 15.0),
    title: TextRoleMetrics::new(14.0, 19.0),
    monospace: TextRoleMetrics::new(12.0, 17.0),
};
let theme = default_dark_theme().with_typography(typography);

assert_eq!(theme.font_family(FontFamilyRole::Brand), "Space Grotesk");
assert_eq!(theme.typography.sizes.get(FontSizeToken::Heading), 20.0);
assert_eq!(
    theme.typography.features.get(FontFeatureToken::Numeric),
    "tabular-nums",
);
assert_eq!(theme.font(TextRole::Title).family, "Inter");
assert_eq!(theme.font(TextRole::Monospace).family, "Space Mono");
```

`FontToken`, `TextRole`, `Theme::font`, and widget-facing resolved recipes keep
their existing signatures. `Theme::with_typography` continues to mirror only
the Body size into the legacy `Theme::text_size` compatibility field.

## Deliberate limits

This migration establishes deterministic theme authority only. It does not
transport weights or features through `FontToken`, text primitives, text
layout, shaping, or renderers. In particular, storing `"tabular-nums"` does not
enable or prove tabular figures in any consumer. It also does not bundle or
download new fonts, change the text engine, prove that a platform can load a
named family, define fallback behavior, change glyph metrics or baseline
placement, or provide browser, Vello, DPI, or visual review evidence. Existing
font assets and their license records are unchanged. Those concerns require
separate evidence before any typography requirement can be accepted.
