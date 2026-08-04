# Component Conformance Status

Stern's conformance claims live in one place:
[`conformance/claims.json`](../conformance/claims.json). That manifest is
the single ledger; this page only explains how to read it. The rules,
status policy, exclusions, and vendored-schema provenance are in
[`conformance/README.md`](../conformance/README.md).

The governing design-system rule: **a claim without a validating record is
not a claim.** Every entry in the ledger cites the merged in-repo test(s)
that exercise its requirement, and the validator
(`conformance/tests/claims_contract.rs`, run by
`cargo test --workspace` and an explicit CI step) fails the build when a
claim names an unknown requirement, cites a test that does not exist, or
carries a status the evidence cannot support.

## Current honest status

- 63 of the design system's 486 requirements carry machine-validated
  claims, all `partial`: model-layer automated tests only. By family:
  focus/keyboard 8, interaction states 7, text editing 7, drag/scroll 7,
  overlays/menus/dialogs/palette 11, primitives/contracts 3, color
  management 4, colors 3, cursors 2, geometry (radii/borders) 4, icons 3,
  spacing 1, typography 3.
- Nothing is `verified`. No component has specimen, browser/Vello
  baseline, scale, platform, or accessibility evidence, and the validator
  rejects any claim that says otherwise.
- The remaining 423 requirements are unclaimed — including all of
  accessibility (no OS bridge exists) and everything outside
  `src/foundations/` and `src/behaviors/` (components, collections,
  patterns, principles, implementation), regardless of how much related
  test coverage exists. Absence from the ledger is the honest default.
- The design system's own ledger,
  `../stern-design-system/generated/parity-index.json`, still records all
  486 requirements as `unverified`. Syncing stern's claims into it is a
  later, owner-approved step; stern never writes into the design-system
  repository.

## Known gaps

See [`KNOWN-GAPS.md`](../KNOWN-GAPS.md) (item 15 tracks the token and
parity pipeline).
