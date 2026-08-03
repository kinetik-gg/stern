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

1. Accent colors (done): `default_dark_accent_group_matches_design_system_tokens`
   asserts that `ThemeColors::default_dark()`'s accent group equals the
   `color.accent.*` entries.
2. Remaining semantic colors, then metrics (spacing, radii, sizes, strokes,
   durations, elevation) and typography/icon tables: planned. Where current
   theme values differ from the tokens, reconciling them is a product decision
   taken explicitly per group, not a silent side effect of wiring.

Once every group is mapped, the hand-rolled definitions can be derived from
`generated_tokens` directly and the mapping tests collapse into the drift
test.
