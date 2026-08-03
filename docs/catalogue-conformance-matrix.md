# Component Conformance Status

This file previously carried a per-capability matrix (S10-S13 rows) that
pointed at two things that no longer exist: an in-crate registry
(`stern-widgets::COMPONENT_CONFORMANCE_MATRIX` / its `taxonomy` module,
removed in PR #888) and showcase fixtures plus hand-authored evidence
packets (removed in PR #890). The matrix also used a capability vocabulary
(`ALPHA-00`, axes `M`/`P`/`I`/`A11y`/`PF`/`LW`, and the
`Stable`/`Experimental`/`Planned` statuses) that was invented in this
repository and defined nowhere outside these docs. None of that is restored
here.

## Current honest status

- Behavioral model-layer tests exist and pass: roughly 2,500 tests across
  the workspace (`cargo test --workspace --all-features`).
- No component has verified paint, platform, or accessibility evidence.
  Nothing in this repository is verified `Stable` by any standard.
- The intended verification ledger is the design system's parity index, not
  this repository: `../stern-design-system/generated/parity-index.json`
  (486 requirements, currently all `unverified`).

## Known gaps

Wiring stern's behavioral coverage into that parity ledger is future work.
See [`KNOWN-GAPS.md`](../KNOWN-GAPS.md) for the tracked gap list. That file
may not exist yet; a dangling link here is acceptable until it is added.
