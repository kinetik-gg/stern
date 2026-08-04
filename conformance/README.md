# Parity Conformance Claims

This directory is stern's single machine-validated conformance ledger
against the design system's requirement index
(`../stern-design-system/generated/requirement-index.json`, 486
requirements).

The governing rule comes from the design system's conformance policy:

> A claim without a validating record is not a claim.

Every entry in [`claims.json`](claims.json) therefore names the merged,
in-repo test(s) that exercise the requirement. If no merged test genuinely
exercises a requirement, the requirement is simply absent from the manifest
— absence is the honest default, not an oversight. When in doubt, a claim
is left out.

## Claim format

`claims.json` is a hand-curated array, sorted by `requirementId`:

```json
{
  "requirementId": "STERN-KEY-004",
  "status": "partial",
  "tests": [
    "crates/stern-core/tests/focus_keyboard_conformance/traversal.rs::focus_keyboard_focus_traversal_wraps_forward_and_backward"
  ],
  "notes": "What the tests prove, and what they do not."
}
```

- `requirementId` — must exist in the design system's
  `generated/requirement-index.json`.
- `status` — one of the status values from the design system's
  parity-evidence schema (`unverified`, `partial`, `verified`). See the
  status policy below.
- `tests` — `path::test_fn` references, paths relative to the stern repo
  root. Every referenced file and test function must exist in this
  repository.
- `notes` — what the cited tests actually prove, and which parts of the
  requirement statement remain unproven.

## `partial` vs `verified`

The parity-evidence schema
([`parity-evidence.schema.json`](parity-evidence.schema.json), vendored —
see provenance below) defines three statuses:

- **`unverified`** — no validating record. Requirements in this state are
  not listed here at all; the design system's own
  `generated/parity-index.json` already records all 486 requirements as
  `unverified`.
- **`partial`** — some genuine validating record exists, but not the full
  evidence set the schema demands (specimens, automated tests, browser and
  Vello baselines, all four scales, all three platforms, review records).
  Everything stern can claim today is at most `partial`: the cited evidence
  is model-layer automated tests only. No visual baseline, platform, or
  scale evidence exists in this repository.
- **`verified`** — the complete evidence set. Nothing in stern qualifies,
  and the validator test fails any claim that says `verified` until visual
  and platform evidence pipelines exist.

## Exclusions

- **Accessibility (`src/foundations/accessibility.md`, STERN-ACC-\*)** is
  excluded entirely: stern has no OS accessibility bridge, so no test can
  genuinely exercise those requirements end to end.
- **Requirements outside `src/foundations/` and `src/behaviors/`** are out
  of scope for this manifest today (components, collections, patterns,
  principles, implementation contracts). Widening the scope is future work
  and requires the same test-backed discipline.
- **Anything without a merged test.** A requirement that stern's merged
  test suite does not genuinely exercise at the model layer is not listed,
  even when the implementation "obviously" behaves correctly.

## Validation

This directory is also a small dev-test crate (workspace member
`conformance`; its library exports nothing). Its
[`tests/claims_contract.rs`](tests/claims_contract.rs) machine-validates
the manifest on every `cargo test --workspace` run (and as an explicit CI
step):

- `claims.json` parses, is sorted, has unique requirement ids, and every
  claim carries at least one test reference and non-empty notes;
- every referenced test file exists in-repo and contains the named
  `#[test]` function;
- every status is a schema status value, and is `partial` (policy gate, see
  above);
- when `../stern-design-system` is checked out as a sibling of the stern
  workspace root, every `requirementId` must exist in
  `generated/requirement-index.json` and its `source` must be in scope
  (foundations/behaviors, never accessibility), and the vendored schema
  copy must still match the design-system original byte for byte. Without
  the sibling checkout these checks skip with a note, matching the theme
  token drift test.

## Vendored schema provenance

`parity-evidence.schema.json` is vendored verbatim from the read-only
design-system repository (same policy as
`crates/stern-core/src/theme/generated_tokens.rs`; JSON cannot carry a
header comment, so the provenance lives here):

- Source: stern-design-system repository,
  `schemas/parity-evidence.schema.json` (checked out as a sibling of the
  stern workspace root).
- Vendored: 2026-08-04.
- Do not edit; re-vendor from stern-design-system. The validator's drift
  check fails when this copy no longer matches the design-system original.

## Relationship to the design-system parity index

The design system's `generated/parity-index.json` remains the
design-system-side ledger and stays untouched: all 486 requirements are
`unverified` there. Syncing the claims in this directory back into that
index is a later, owner-approved step — this repository never writes into
`../stern-design-system`.
