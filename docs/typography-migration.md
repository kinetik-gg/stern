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

## Space Mono loading alignment

The bundled monospace face now follows the semantic Mono family authority.
This is a prerelease breaking change:

- `DEFAULT_MONOSPACE_FONT_FAMILY` changed from `"Geist Mono"` to
  `"Space Mono"`.
- Public `fonts::GEIST_UPSTREAM_COMMIT` and `fonts::GEIST_MONO_VARIABLE` were
  removed without compatibility aliases.
- Public `fonts::SPACE_MONO_UPSTREAM_COMMIT` and
  `fonts::SPACE_MONO_REGULAR` expose the exact pinned replacement authority.

The default text engine loads Space Mono Regular from upstream revision
`329858c2c4dbd3476f972a4ae00624b018cf4b81`. Named `"Space Mono"`, the public
default, generic `"monospace"`, and the `"mono"` alias all resolve through
those same bundled bytes. Inter and generic sans-serif resolution are
unchanged.

Applications must expect monospace glyph metrics, measured widths, wrapping,
layout geometry, and any derived snapshots or hashes to change. Review stored
goldens and application-owned layout assumptions instead of treating the new
face as metrically interchangeable with Geist Mono.

## Space Grotesk Brand loading alignment

The semantic Brand family now resolves through one exact bundled Space
Grotesk variable face in the default text engine. Public
`fonts::SPACE_GROTESK_UPSTREAM_COMMIT` pins revision
`03507d024a01282884232081fc6011c09ff4e849`, and public
`fonts::SPACE_GROTESK_VARIABLE` exposes the `136,676` bytes from upstream path
`fonts/ttf/SpaceGrotesk[wght].ttf`, blob
`a1b2e6c26093066510a31147e7aec9abdc8d6c5e`, and SHA-256
`ACAD6DE1FC93436F5C0F1F4137751EF04F1AEA3063E7036535970FFCFBD79F72`.
The face identifies the typographic family as `Space Grotesk` and contains the
normal variable `wght 300-700` axis.

The exact upstream `OFL.txt`, blob
`cb512b9af44ff61e75e1aad387b7424cdfab36a3`, is bundled beside the face as
`LICENSE-Space-Grotesk.txt`. Its `4,495` bytes have SHA-256
`564CE565C371C5E5BBF286006565A7C9AA55A9F56E7CA58D56E05D649DD61A72`.
The project root remains MIT licensed; the OFL applies to this bundled
third-party font asset.

Qualified public-facade evidence resolves `FontFamilyRole::Brand` through the
default theme, passes the resulting `"Space Grotesk"` name into public
`TextStyle`, shapes text, and verifies that every glyph run uses the public
bundled bytes. Loading the additional face does not change named/default Inter
resolution or generic sans-serif resolution, and it does not change named,
default, generic `"monospace"`, or `"mono"` Space Mono resolution.

Brand text uses Space Grotesk metrics, so measured widths, wrapping, layout
geometry, baselines, overflow points, stored snapshots, and derived hashes can
differ from measurements made with another family. This slice proves only the
deterministic font-byte boundary; it does not establish optical suitability or
accept any geometry or visual result.

There is intentionally no `DEFAULT_BRAND_FONT_FAMILY`, Brand `TextRole`, Title
remapping, fallback stack, or platform discovery. Although the bundled face
contains a weight axis, the current shaping API does not select or transport
that axis.

## Deliberate limits

The semantic foundation still does not transport weights or features through
`FontToken`, text primitives, text layout, shaping, or renderers. In
particular, storing `"tabular-nums"` does not enable or prove tabular figures
in any consumer.

The Space Mono follow-up advances only deterministic Mono text-system
alignment for `STERN-TYP-000`, which remains Partial. Exact asset and license
provenance makes `STERN-TYP-006` Partial. The Space Grotesk follow-up advances
only the corresponding deterministic Brand text-system byte alignment and
exact asset/license provenance; both requirements remain Partial.
`STERN-TYP-001` and `STERN-TYP-003` are preserved without advancing;
`STERN-TYP-002`, `STERN-TYP-004`, `STERN-TYP-005`, and `STERN-TYP-007` do not
advance. All typography parity records remain unverified, and nothing is
Accepted.

This bounded evidence does not prove platform or non-Latin fallback,
failed-load layout stability, IME behavior, weight or feature transport,
tabular figures, widget adoption, truncation, optical baselines, overflow,
DPI legibility, renderer or browser output, or GPU/manual visual review.
