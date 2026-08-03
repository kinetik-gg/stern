# Design-System Token Vendoring

The stern design system (the sibling `stern-design-system` repository) is the
source of truth for design tokens. Its generator emits
`generated/rust/stern_tokens.rs`: colors, spacing, radii, sizes, strokes,
durations, elevation, easings, shadows, font families, icon mappings, and
color-space descriptors as `&'static` slices, plus `STERN_VERSION` and
`SOURCE_SHA256` constants identifying the token source revision.

Stern vendors that file verbatim as
`crates/stern-core/src/theme/generated_tokens.rs` (public as
`stern_core::theme::generated_tokens`). The design-system repository is never
modified from stern, and stern never reads it at build time; adoption flows
one way, through re-vendoring.

## Vendoring contract

- The vendored file is the upstream file with a provenance comment block and
  module docs/lint allows inserted after the `// @generated` header. Everything
  after that header is byte-identical to upstream; the `mod` declaration in
  `crates/stern-core/src/theme.rs` carries `#[rustfmt::skip]` to keep it that
  way.
- Never hand-edit the vendored file. To update it, re-run the design-system
  generator (in that repository), copy the fresh
  `generated/rust/stern_tokens.rs` over the vendored body, and refresh the
  provenance block (date and `SOURCE_SHA256`).

## Drift test

`crates/stern-core/src/theme/tests.rs` contains
`vendored_tokens_match_design_system_output`. When
`../stern-design-system/generated/rust/stern_tokens.rs` exists relative to the
workspace root, the test compares the upstream `SOURCE_SHA256` and the full
generated body against the vendored copy and fails with a re-vendor message on
any mismatch. When the sibling checkout is absent (for example in CI), the
test skips silently apart from an `eprintln!` note, so the check runs wherever
the design system is checked out without making CI depend on it.

## Staged adoption

The hand-rolled values in `crates/stern-core/src/theme/` remain the runtime
source of truth while adoption is staged. Token groups are adopted one at a
time by adding mapping tests that pin theme values to the corresponding
`generated_tokens` entries:

1. All semantic colors (done): every one of the 53 `SemanticColor` resolver
   keys carries a `SemanticColor::design_token_name` mapping to its exact
   `color.*` token name (`crates/stern-core/src/theme/tokens.rs`), keyed off
   `docs/visual-spec/00-language.md`'s tier tables (surface, text, border,
   selection, focus, overlay, accent, status). One test per tier group in
   `crates/stern-core/src/theme/tests.rs` — plus a completeness guard over
   `SemanticColor::ALL` — asserts `ThemeColors::default_dark()` equals every
   mapped token. See [Exceptions and divergences](#exceptions-and-divergences)
   below.
2. Typography font families (done, with exceptions):
   `crates/stern-text/src/tests.rs` asserts every `generated_tokens::FONT_FAMILIES`
   role's primary family name against its stern-text authority
   (`font_family_role_primaries_match_stern_authority_constants`) and that
   each primary actually shapes through its bundled font asset
   (`design_system_font_family_primaries_shape_through_bundled_assets`).
   Sizes and weights are out of scope for now: `generated_tokens` carries no
   typography size/weight fields (only `role`/`primary`/`fallbacks` per
   family), so there is nothing upstream yet to pin stern's `TextRole`
   metrics against. See
   [Exceptions and divergences](#exceptions-and-divergences).
3. Metrics (spacing, radii, sizes, strokes, durations, elevation) and icon
   tables: planned. Where current theme values differ from the tokens,
   reconciling them is a product decision taken explicitly per group, not a
   silent side effect of wiring.

Once every group is mapped, the hand-rolled definitions can be derived from
`generated_tokens` directly and the mapping tests collapse into the drift
test.

## Exceptions and divergences

### Colors

Every `SemanticColor` key / `ThemeColors` field has a named
`generated_tokens::COLORS` counterpart — there are no color exceptions
(hand-rolled color values with no design-system token) to record.

Checking the mapping against `ThemeColors::default_dark()` also found no
divergences: every mapped theme value already equals its token exactly,
including the one case `docs/visual-spec/00-language.md` flags as
historically contested (`surface.control_pressed` / `border.hover`, its
divergence D1 — labs.css disagrees with the token, but the *token* is
normative, and stern's theme already used the token value).

### Typography

- **`brand` font family role.** `generated_tokens::FONT_FAMILIES` defines
  three roles: `ui` (Inter), `brand` (Space Grotesk), `mono` (Space Mono).
  stern-text exposes named authority constants for `ui`
  (`DEFAULT_FONT_FAMILY`) and `mono` (`DEFAULT_MONOSPACE_FONT_FAMILY`), but no
  equivalent constant for `brand`. Brand text still resolves correctly today —
  `CosmicTextEngine` shapes the literal `"Space Grotesk"` family name through
  the bundled `fonts::SPACE_GROTESK_VARIABLE` asset (see
  `named_space_grotesk_family_shapes_with_bundled_font` and
  `design_system_font_family_primaries_shape_through_bundled_assets` in
  `crates/stern-text/src/tests.rs`) — there is just no named constant an
  application can import instead of the string literal. This is a
  naming-convention gap, not a font-asset gap; no fonts were added to close
  it. Per `docs/visual-spec/00-language.md`, brand appears only in the
  application titlebar identity and frame/lab titles, so the blast radius of
  the missing constant is small.
- **Sizes and weights.** `docs/visual-spec/00-language.md` §Typography
  documents a normative 9/10/11/12px scale with per-step weights (divergence
  D5: "labs scale, propose upstreaming as tokens"). The vendored
  `generated_tokens::FONT_FAMILIES` carries no matching size or weight
  fields, so there is no upstream token yet to assert stern's `TextRole`
  metrics (`crates/stern-core/src/theme/tokens.rs`) against. This is a gap in
  the vendored token data, not something stern's mapping tests can close
  today.

There is nothing here for an owner to decide; this section stays as a record
for future groups (metrics, icons) to append to if their adoption finds
either kind of gap.
