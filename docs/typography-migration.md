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
remapping, fallback stack, or platform discovery. The qualified low-level
weight transport below can instance the bundled face without assigning a
semantic weight to Brand or any `TextRole`.

## Variable-font weight transport

Qualified retained text now transports an exact requested `u16` weight through
`TextStyle`, `TextLayoutKey`, deterministic cache/store identity, Cosmic Text
shaping, retained renderer resources, and both Vello glyph paths.
`TextStyle::new(...)` remains behavior-compatible by selecting Regular `400`;
callers resolve a value from the existing `FontWeightScale` and
`FontWeightToken` authority, then opt in with `TextStyle::with_weight(value)`.
Stern adds no second semantic weight enum or token scale.

This is a prerelease breaking public-shape change. External `TextStyle` literals
must add `weight: 400` to preserve constructor behavior. External
`ShapedGlyphRun` literals must add `normalized_coords: Vec::new()` when they
represent a static/default instance. The coordinate vector is renderer-ready
2.14 fixed-point selected-font axis state, not a semantic token surface.

The raw request remains in public style and retained-key identity. Cosmic Text
selects the face and applies its own variable-axis endpoint mapping; Stern does
not clamp, quantize to the four default tokens, substitute a family, synthesize
bold, or reshape for rendering. Inter preserves its full `opsz, wght` vector,
Space Grotesk preserves its `wght` vector, and static Space Mono preserves exact
bundled bytes with an empty vector. Store and renderer reachability metrics
count every coordinate vector's owned capacity and remain equal after full and
incremental reconciliation.

Deterministic evidence covers exact `400/500/600/700` vectors, raw out-of-range
key distinction with selected-face endpoint coordinates, features, end
ellipsis, Unicode/bidi/multiline topology, stable hot reuse, public facade
resolution, and registered Vello encoding through axis-aligned and general
affine paths at `1.0`, `1.25`, `1.5`, and `2.0` with no fallback-cache activity.
It does not adopt weight into `FontToken`, `TextRole`, `TextPrimitive`, or
layoutless text. The bounded canonical component adoption below consumes this
transport without expanding those public shapes.

## Retained property-grid section weight

Canonical retained `Ui::property_grid` section rows now resolve their family,
size, and line height from `TextRole::Title` and their requested weight from
`theme.typography.weights.get(FontWeightToken::Semibold)`. The default request
is exact UI-family Inter `14/19` at weight `600`; Title remains mapped to UI,
not Brand. A successfully admitted section owns one byte-exact complete-source,
nonwrapping, feature-disabled `Visible` layout with positive-zero width. The
selected bundled Inter face carries exact normalized coordinates `[0, 5_898]`.

No-store and rejected generic/layoutless fallbacks remain Regular `400`
because `TextPrimitive` still has no weight field. Ordinary property labels
retain Label `12/16`, Regular construction, and their existing `EndEllipsis`
policy; required markers, help/status glyphs, geometry, semantics, access,
interactions, and public shapes are unchanged. Custom themes continue to own
the UI family, Title metrics, and Semibold value, while selected-face
coordinates follow the actual chosen face rather than a component constant.

Deterministic CPU evidence covers strict admission and rejection, stable
retained identity, selected font bytes and coordinates, complete-source
semantics, and both registered Vello paths at `1.0`, `1.25`, `1.5`, and `2.0`.
Exact `14/600` rendering is bounded unindexed candidate type-scale/component
transport evidence only. `STERN-TYP-000`, `STERN-TYP-002`, `STERN-TYP-004`,
and `STERN-TYP-006` remain Partial without advancement;
`STERN-TYP-001`, `STERN-TYP-003`, `STERN-TYP-005`, and `STERN-TYP-007` do not
advance. No parity record is verified and nothing becomes Accepted. Browser,
raster, GPU, pixel, platform-font, failed-load, unsupported-script, IME,
optical-baseline, DPI-legibility, manual, and visual evidence remain
unverified; issue #653 remains the external browser/Vello visual-evidence
blocker.

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

Canonical retained `Ui::select_field` now opts selected values and placeholders
into this policy at the exact post-padding, post-disclosure text width. This is
a prerelease breaking rendering-behavior change even though public signatures
and exports are unchanged. Complete source remains in the primitive,
presentation, key, renderer resource, semantic description, and semantic
value; placeholder state remains unselected and the disclosure stays separate.
The public low-level `select_field(...)` path remains layoutless. Rejected
admission, invalid geometry, and multiline sources preserve complete visible
text. Registered component-to-Vello CPU encoding at `1.0`, `1.25`, `1.5`, and
`2.0` proves shaped topology transport without fallback-cache activity, not
raster or visual acceptance.

Canonical retained `Ui::property_grid` now opts only ordinary property-row main
labels into the same policy. The complete presentation source is `row.label`
plus `" *"` only for required properties; semantics continue to expose exact
undecorated `row.label`. The retained width uses the existing property-label
inset and leftmost trailing-glyph origin in this exact order:
`((label_rect.width - 6.0) - reserved_right).max(0.0)`. Help presence,
including `Some("")`, reserves `22.0`; otherwise an accented status reserves
`10.0`; otherwise the reservation is zero. `SpacingRole::IconLabelGap` is not
part of these fixed columns. Existing label/help/status origins, brushes,
baselines, callbacks, access, intents, ordering, virtualization, and value
controls are unchanged.

Admitted nonpositive-width and multiline requests retain a complete-source
`EndEllipsis` key and ID but shape visibly without a marker. Actual store
rejection falls through to complete-source generic visible or layoutless
attachment. Admitted section titles use the separately documented retained
Title + Semibold path; no-store or rejected generic fallback stays
complete-source Regular at the existing `label_rect.x + 8` origin. This is a
prerelease breaking rendering
policy change without a public signature or export change. Registered
property-label-to-Vello CPU evidence at `1.0`, `1.25`, `1.5`, and `2.0` proves
resource/topology transport and separate help/status glyphs, not pixels,
logical non-overlap at zero width, tooltip or copied-value behavior, or visual
acceptance.

Canonical retained `Ui::button` now opts only its existing final label
primitive into the same policy. The retained width is computed in this exact
order from the unchanged standard-button rectangle and theme control padding:
`let padding_x = theme.controls.padding_x; let raw_span = rect.width -
padding_x * 2.0_f32; let label_width = raw_span.max(0.0_f32);`. The key uses the
label primitive's exact family, size, and line height with wrapping disabled and
default features. Complete text remains in the primitive, retained key,
renderer resource, and semantic label. This is a prerelease breaking rendering
policy change without a public signature, export, theme, token, primitive, or
renderer-command change.

The existing `Ui::action_button` implementation is structurally unchanged and
inherits the policy only because it delegates its complete descriptor label to
`Ui::button`. Hidden actions remain absent, disabled actions remain visible and
inert, and pointer/keyboard activation retains the exact action ID, button
source, context, and FIFO ordering. Icon, shortcut, checked, tooltip, and
keyword descriptor metadata remains unused by this button topology; no
toolbar, menu, split, busy, disclosure, or icon-button behavior is added. The
standalone public `button(...)` component stays complete-source and layoutless,
and neighboring retained button-family consumers keep generic Visible policy.

Admitted nonpositive-width and multiline labels keep a complete-source
`EndEllipsis` key and ID but no generated marker. Strict rejection leaves the
store and change cursor unchanged and falls through to the existing
complete-source generic Visible or layoutless attachment. Invalid and
nonfinite rectangles follow the literal width expression while preserving the
existing geometry, semantic, response, cursor, repaint, focus, and action
topology; no geometry-validity or non-overlap claim is added. Registered
standard/action-button Vello CPU evidence at `1.0`, `1.25`, `1.5`, and `2.0`
proves resource and glyph topology without fallback-cache activity, not raster,
GPU, tooltip, copied-value, or visual acceptance.

Canonical retained `Ui::chrome_scene` now opts only final action-backed toolbar
row labels into the same policy. It uses the final overflow-projected row width,
not the requested item or surface width, in this exact operation order: `let
padding_x = theme.controls.padding_x; let raw_span = row.rect.width - padding_x
* 2.0_f32; let label_width = raw_span.max(0.0_f32);`. The unchanged final text
primitive supplies family, size, and line height; wrapping is disabled and
features remain default. Complete action label source remains in the descriptor,
primitive, retained key, renderer resource, and semantic label.

This prerelease rendering-policy change adds no public API, model, projection,
theme, primitive, renderer-command, or generic-attachment change. Hidden and
overflowed actions register no label, while the separate overflow-trigger
literal stays generic Visible/layoutless. Hover, press, focus, checked,
disabled, icon, shortcut, tooltip, and keyword metadata preserve retained
identity when source, effective width, and style are equal. Pointer and keyboard
activation preserve the existing action ID, button source, context, and FIFO
ordering. Equal retained text keys may share one ID across distinct actions;
the layout ID is not action identity.

Narrow/nonpositive spans and newline or Unicode paragraph sources retain the
complete-source explicit key without a generated marker. Strict admission
rejection leaves store accounting unchanged and falls through only to the
existing complete-source generic Visible or layoutless path. Menu, tab,
tab-close, status, overflow-trigger, overlay, command-palette, and system-
feedback text remain generic consumers. Registered chrome-toolbar-to-Vello CPU
evidence at `1.0`, `1.25`, `1.5`, and `2.0` proves exact resource/marker
topology, stable logical width and retained identity, coordinate scaling, and
zero fallback-cache activity. It does not prove browser, raster, pixel, GPU,
copied-value, tooltip, platform, DPI-legibility, manual, or visual acceptance.

Canonical retained `Ui::virtual_table` now opts only its final body-cell label
primitive into the same policy. It computes the retained span from the final
prepared and constraint-clamped cell rectangle, not the raw `TableColumn`
width, in this exact order: `let padding_x = theme.controls.padding_x; let
raw_span = rect.width - padding_x * 2.0_f32; let label_width =
raw_span.max(0.0_f32);`. The key uses the unchanged primitive family, size, and
line height with default features and wrapping disabled. Complete caller-owned
cell source remains in the primitive, key, renderer resource, and semantic
label. Equal complete source/style/effective-width keys may intentionally share
a retained layout ID without becoming row, column, or cell identity.

This prerelease rendering-policy change adds no table API or overflow
configuration. `VirtualTableConfig`, `TableColumn`, `VirtualTableRow`, prepared
geometry, stable identities, both selection/navigation modes, focus annuli,
sort, resize, two-axis scroll, virtualization, semantics, callback bounds, and
generic attachment remain unchanged. Missing cells keep empty-source explicit
policy, extra cells remain unpainted, and strict admission rejection preserves
the complete primitive and semantic source while leaving the label layoutless
when generic fallback also rejects. Header labels and sort arrows remain
generic Visible/layoutless consumers.

Registered virtual-table-to-Vello CPU evidence at `1.0`, `1.25`, `1.5`, and
`2.0` proves exact retained body-marker topology, separate complete header
resources, logical-width/ID stability, coordinate scaling, and zero fallback
cache activity. It does not prove raster or GPU output, browser behavior,
copied values, editing, tooltips, column-configurable or header overflow, or
visual acceptance.

## Deliberate limits

The semantic foundation still does not assign weights through `FontToken`,
`TextRole`, or text primitives. Qualified `TextStyle` requests transport exact
low-level weights through retained layout, shaping, and renderers. The
canonical retained property-grid section path is the sole semantic component
weight adopter; layoutless/generic text remains Regular `400`. Numeric feature
adoption does not change `FontToken`, `TextRole`, or `TextPrimitive`; the
accepted retained layout ID remains the component-to-renderer authority.
Generic text behavior remains feature-disabled.

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
without advancing. The retained select-trigger, property-label, standard and
delegated action-button, chrome-toolbar label, and virtual-table body-cell
adoptions advance only
`STERN-TYP-004` to stronger bounded Partial for canonical selected values,
placeholders, inspector property labels, standard/action button labels,
chrome-toolbar labels, and retained table body-cell labels with complete-source
semantics and registered Vello topology. `STERN-DEN-004` advances only to
bounded Partial for finite-positive computed property-label, button-label,
toolbar-label, and prepared body-cell spans; nonpositive spans retain visible
fail-safe behavior and make no endpoint or non-overlap claim. Other truncating
components and external visual evidence remain outstanding. Button action,
toolbar, chrome, and table-family evidence are regression-only and do not
advance any `STERN-ACT-*`, `STERN-TOOLBAR-001` through `STERN-TOOLBAR-006`,
`STERN-CHROME-001`, `STERN-CHROME-004`, `STERN-CHROME-005`, or `STERN-TBL-*`
requirement. Existing Partial evidence for `STERN-TYP-000`,
`STERN-TYP-002`, and `STERN-TYP-006` is preserved. `STERN-TYP-005`, `STERN-TYP-007`,
`STERN-INSPECT-001`, `STERN-PROP-001`, `STERN-TIP-001`, `STERN-TIP-002`, and
`STERN-OVERLAY-COMP-002` do not advance. All typography parity records remain
unverified, and nothing is Accepted.

This bounded evidence does not prove direct/layoutless component parity,
platform or non-Latin fallback, failed-load layout stability, IME behavior,
additional semantic role/component weight adoption, copied-value or tooltip
workflows, editable or other component truncation, start/middle/multiline
ellipsis, optical baselines, DPI legibility, renderer pixels, browser output,
or GPU/manual visual review.
