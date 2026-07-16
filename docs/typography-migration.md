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

## Tabular-number shaping transport

The qualified text API now transports Stern's bounded numeric feature through
layout and production shaping. This is a prerelease breaking struct-shape
change: public `TextStyle` literals must initialize the new
`features: TextFeatureSet` field. Existing constructor calls remain compatible
because `TextStyle::new(...)` selects `TextFeatureSet::NONE`.

`TextFeatureSet` is an opaque fixed-size value with only two public
authorities: `NONE` and `TABULAR_NUMBERS`. The latter maps to OpenType
`tnum=1`; Stern does not expose arbitrary OpenType tags or a generic feature
registry. Opt in through the qualified facade:

```rust
let theme = stern::core::default_dark_theme();
assert_eq!(
    theme
        .typography
        .features
        .get(stern::core::FontFeatureToken::Numeric),
    "tabular-nums",
);

let features = stern::text::TextFeatureSet::resolve_semantic(
    theme.typography.features,
    stern::core::FontFeatureToken::Numeric,
)
.unwrap_or_default();
let style = stern::text::TextStyle::new(
    theme.font_family(stern::core::FontFamilyRole::Ui),
    12.0,
    16.0,
)
.with_features(features);
```

`FontFeatureScale` remains the sole semantic token authority and continues to
resolve `FontFeatureToken::Numeric` to `"tabular-nums"` by default.
`TextFeatureSet` is the lower-level shaping mechanism selected after semantic
resolution; it does not duplicate or replace the theme token value.

Feature identity participates in `TextStyle` equality and hashing, and
therefore in `TextLayoutKey`, the compatibility cache, retained layout IDs,
and retained renderer resources through their existing composed style field.
Applications that previously stored public `TextStyle` literals must add
`features: TextFeatureSet::NONE` to preserve prior shaping and identity.

## Retained numeric component adoption

Canonical `Ui::numeric_input`, `Ui::numeric_scrub_input`, and vector numeric
subfields now resolve `FontFeatureToken::Numeric` through
`TextFeatureSet::resolve_semantic(...)` when a retained `TextLayoutStore` is
attached. The exact default `"tabular-nums"` value selects
`TABULAR_NUMBERS`; unsupported customized values fail soft to `NONE` rather
than becoming arbitrary OpenType tags.

This is a prerelease breaking rendering-behavior change without a widget
signature change. Numeric drafts can measure differently because digit
advances are now tabular. The same feature-bearing style is used for entry hit
geometry, caret and selection navigation, final retained shaping, renderer
resource reconciliation, and registered Vello encoding. Editable, read-only,
and disabled scrub states keep the same rendering feature; vector numeric
subfields inherit the scrub runtime. Generic text, search, path, and vector
axis labels remain feature-disabled.

Applications with snapshots or geometry derived from retained numeric fields
should refresh those expectations. Applications that customize
`theme.typography.features.numeric` to an unsupported value receive
feature-disabled numeric shaping. Direct low-level widget helpers and
layoutless/store-rejected compatibility rendering are unchanged and are not
covered by this adoption contract.

Deterministic conformance uses the exact bundled Inter variable face. Its
default digit advances are observably proportional, while enabled digits
`0-9` and equivalent-length changing numeric strings have equal advances
within `0.001` logical unit. Feature-bearing layouts retain the same UTF-8
ranges, line topology, Inter byte authority, bounded store/cache behavior, and
retained renderer-resource reconciliation.

## Non-destructive end ellipsis

The qualified text API now provides `TextOverflow::{Visible, EndEllipsis}` on
`TextLayoutKey`. `TextLayoutKey::new(...)` selects `Visible`, preserving the
existing glyph topology and unbounded nonwrapping presentation. Display-only
callers opt in explicitly:

```rust
let request = stern::text::TextLayoutKey::new(
    "The complete caller-owned source remains here",
    stern::text::TextStyle::new("Inter", 12.0, 16.0),
    96.0,
    false,
)
.with_overflow(stern::text::TextOverflow::EndEllipsis);
```

`EndEllipsis` is honored only for a finite positive width, disabled wrapping,
and single-line source. The production engine delegates that exact case to
pinned `cosmic-text` `Ellipsize::End` with a one-line limit. It does not build
a shortened string. Nonpositive or nonfinite widths, wrapping requests, and
multiline sources retain their existing visible or wrapping behavior.

The overflow policy participates in key equality, hashing, compatibility-cache
ordering, retained layout IDs, change reconciliation, and renderer-resource
identity. The byte-exact source and explicit policy remain in the key held by
the store and renderer resource. Only positioned shaped glyphs may omit hidden
source content. The engine-generated ellipsis glyph has an empty source range
at the elision grapheme boundary and sets `ShapedGlyph::elided`; callers can
query the aggregate with `ShapedTextLayout::is_elided()`.

This is a prerelease breaking public-shape change. External `TextLayoutKey`
literals must add `overflow: TextOverflow::Visible`, and external
`ShapedGlyph` literals must add `elided: false`. Exhaustive matches over
`TextNavigationError` must handle `ElidedLayout`. Navigation construction
returns that error before ordinary cluster validation because hidden source
graphemes cannot have byte-accurate caret or selection interpolation. Full-fit
and visible layouts preserve existing navigation.

No widget opts into ellipsis in this slice, and `TextPrimitive`, theme tokens,
render commands, editable field behavior, accessible values, copy behavior,
and tooltip behavior are unchanged. Registered renderer resources and Vello
consume the shaped topology already authorized by the retained layout ID.
Deterministic CPU encoding at `1.0`, `1.25`, `1.5`, and `2.0` is evidence of
topology transport, not raster or visual acceptance.

## Deliberate limits

The semantic foundation still does not transport weights through `FontToken`,
text primitives, text layout, shaping, or renderers. Numeric feature adoption
does not change `FontToken`, `TextRole`, or `TextPrimitive`; the accepted
retained layout ID remains the component-to-renderer authority. Generic text
behavior remains feature-disabled.

The Space Mono follow-up advances only deterministic Mono text-system
alignment for `STERN-TYP-000`, which remains Partial. Exact asset and license
provenance makes `STERN-TYP-006` Partial. The Space Grotesk follow-up advances
only the corresponding deterministic Brand text-system byte alignment and
exact asset/license provenance; both requirements remain Partial.
The retained numeric follow-up advances `STERN-TYP-002` only to stronger
bounded Partial for canonical retained numeric inputs, numeric scrubs, and
vector numeric subfields, including registered Vello glyph encoding. It is not
Accepted because direct/layoutless compatibility paths, timelines, frame
counters, timecodes, and tables do not consume the feature and no visual
acceptance was performed. `STERN-TYP-001` and `STERN-TYP-003` are preserved
without advancing. The non-destructive end-ellipsis follow-up advances only
`STERN-TYP-004` to bounded Partial for explicit retained single-line requests;
it has no component adoption or visual evidence. `STERN-TYP-005` and
`STERN-TYP-007` do not advance. All typography parity records remain
unverified, and nothing is Accepted.

This bounded evidence does not prove direct/layoutless component parity,
platform or non-Latin fallback, failed-load layout stability, IME behavior,
weight transport, start/middle/multiline or component truncation, optical
baselines, DPI legibility, renderer pixels, browser output, or GPU/manual
visual review.
