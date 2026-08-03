# Known Gaps

This is the canonical gap ledger for stern. It exists because the project's
past failure mode was silence about gaps paired with noise about process:
inflated status claims, invented conformance vocabulary, and evidence
packets that verified nothing. This file inverts that. It is meant to be
blunt, current, and linkable.

Agents and contributors: when you find a gap, add it here instead of
scrubbing a TODO or leaving it undocumented elsewhere. Entries link to a
GitHub issue when one exists; entries without a linked issue are still real
and still tracked here.

Content below reflects the 2026-08-03 audit, re-verified against the tree
after Issues 01-04 landed. File and line references were checked with
`grep`/`Read` as of that verification and may drift as the code moves; if a
reference looks stale, trust the code and send a correction PR.

## Framework pillars — not started

1. **Layout engine.** `crates/stern-core/src/layout.rs` is single-pass rect
   splitting: `row_layout`/`column_layout`/`grid_layout` consume
   caller-supplied `Measurement` values, they do not run a measurement pass
   themselves. There is no top-down/bottom-up measure-then-arrange system, no
   content-driven sizing derived from a widget's own render output, no text
   wrapping integration, no baseline alignment, and no layout result cache.
   Every widget still takes a caller-computed `Rect`. Spec phase 5
   ("Measurement") in `docs/specs.md` is unbuilt.
2. **Application shell.** There is no runner type. The complete
   application-owned event loop lives in
   `crates/stern-vello-winit/examples/one_window.rs`, ~630 lines of
   hand-rolled winit + GPU-recovery plumbing that every application must
   reproduce or copy.
3. **Accessibility bridge.** `SemanticTree`/`AccessibilitySnapshot` exist and
   are tested (see `crates/stern-core`), but nothing consumes them into an
   OS accessibility tree. `crates/stern-winit/src/accessibility.rs` only
   translates `FrameOutput` into a `WinitAccessibilityUpdate` snapshot and
   explicitly punts the AccessKit/OS adapter step to application shells.
4. **Font fallback.** `FontSystem` loads only the bundled Inter, Space
   Grotesk, and Space Mono faces; there is no `load_system_fonts()` and no
   fallback chain. CJK, emoji, and Arabic text render as tofu
   (`crates/stern-text/src/engine.rs:236-244`, `bundled_font_system`).

## Performance (contradicts docs/specs/05 §32)

5. `Ui::push_primitive` calls `refresh_scoped_input` on every primitive
   (`crates/stern-core/src/runtime/ui.rs:551-555`), which calls
   `SpatialScope::localize_input`, which clones the full `UiInput` per call
   (`crates/stern-core/src/runtime/spatial.rs:45`, `let mut input =
   root.clone();`). That is an `O(primitives × events)` clone per frame.
6. Vello translation clones the clip stack into a fresh `Vec` for every
   render command (`crates/stern-vello/src/translation.rs:493-511`,
   `render_command`) and clones the text and font-family `String`s per text
   primitive (`translation.rs:182-183`). The whole scene is re-encoded every
   frame; there is no damage model and no stable primitive identity to hang
   one off later.
7. `crates/stern-core/src/perf.rs` defines `FrameTimings`, `FrameCounters`,
   `FrameMetrics`, and `AllocationBudget` as plain data types. Nothing in the
   runtime populates them from real measurements; the promised profiling
   hooks are absent.

## Design debt

8. Combinatorial flat widget API: roughly 135-150 `pub fn` methods spread
   across `crates/stern-widgets/src/ui/*.rs` (plus a few more in
   `ui/chrome/`, `ui.rs`, and per-domain files), with separate positional
   variants per checkbox/radio/toggle state and trailing positional bools
   instead of builders. Needs a builder API before any 1.0 talk.
9. Default pointer routing has no occlusion: `PointerRoute::allows` returns
   `true` for `PointerRoute::Unplanned` unconditionally
   (`crates/stern-core/src/memory.rs:24-31`), so every widget under a point
   is eligible unless an application opts into an explicit pointer plan.
   Correctness for overlapping widgets requires that opt-in.
10. Dual legacy-snapshot/canonical-stream input paths coexist and double the
    interaction code (`crates/stern-core/src/input.rs`, e.g. the
    canonical-vs-legacy projection and mismatch handling from roughly line
    650 onward, plus `legacy_text_events` and its callers). `docs/specs.md`
    §8 / `docs/specs/01-foundations.md` §8 ("Input Model") still documents
    both as if the split were the design, not residue.
11. Single-window architecture: `UiMemory`
    (`crates/stern-core/src/memory.rs`) holds one set of interaction
    singletons (focus, drag, text-input owner, etc.) with no multi-window
    identity or routing concept.
12. Library code panics on internal invariants instead of surfacing a
    `FrameWarning`: `crates/stern-core/src/interaction/press.rs:253`
    (`.expect("selection gestures require root event ordinals")` plus the
    adjacent `assert_eq!`), `crates/stern-core/src/runtime/spatial.rs:510`
    and `:597` (`.expect("non-empty polygon")`,
    `.expect("polygon has at least two points")`), and
    `crates/stern-core/src/memory.rs:964-1053` (several
    `.expect("text-input owner epoch overflowed")` calls in
    `set_text_input_owner`, `set_text_input_owner_mode`, and
    `clear_text_input_owner`).
13. `WidgetId` hashes with `std::collections::hash_map::DefaultHasher`
    (`crates/stern-core/src/identity.rs`), whose output is unspecified
    across Rust releases. Unsafe if IDs are ever persisted or compared
    across builds.
14. Command palette matching is substring-only and its lookups are `O(n)`
    per query: `matches_iter` does a `.to_lowercase().contains(&query)` scan
    over every entry and keyword, and `match_at`/`selected_match` walk that
    iterator by index rather than indexing a precomputed result
    (`crates/stern-widgets/src/overlays/command_palette.rs:72-105`). No
    fuzzy ranking, no cached match set.
15. Design-system token pipeline: staged adoption in progress. Stern now
    vendors `stern-design-system`'s generated tokens verbatim as
    `stern_core::theme::generated_tokens`, with a sibling-checkout drift
    test. All 53 `SemanticColor` resolver keys now carry a
    `SemanticColor::design_token_name` mapping to their exact `color.*`
    token, with per-tier mapping tests (surface, text, border, selection,
    focus, overlay, accent, status) plus a completeness guard pinning
    `ThemeColors::default_dark()` to the vendored tokens (see
    `docs/design-system-tokens.md`); every mapped value already matched its
    token exactly, so no visual changes and no divergences to record.
    Typography font families are also mapped now
    (`crates/stern-text/src/tests.rs`): every `generated_tokens::FONT_FAMILIES`
    role's primary name is pinned to its stern-text authority constant and
    shown to shape through its bundled asset. That mapping surfaced two data
    gaps rather than code gaps: the `brand` font family role (Space Grotesk)
    has no named stern-text authority constant (stern resolves it via a
    literal string match instead), and the vendored tokens carry no
    size/weight fields for typography at all, so the visual-spec's
    9/10/11/12px type scale (`docs/visual-spec/00-language.md` §Typography,
    divergence D5) has nothing upstream to pin against yet — see the
    exceptions table in `docs/design-system-tokens.md`. The hand-rolled
    theme values remain the runtime source of truth until every token group
    is mapped; metrics (spacing, radii, sizes, strokes, durations,
    elevation) and icon tables are not yet wired. Independently, the design
    system's 486-requirement parity index
    (`../stern-design-system/generated/parity-index.json`) remains 100%
    `unverified`; wiring stern's behavioral coverage into that ledger is
    future work (see `docs/catalogue-conformance-matrix.md`).
16. `docs/public-api-policy.md` describes a historical, now-retired
    conformance vocabulary (`ALPHA-00`, axes like `M`/`P`/`I`/`A11y`, and
    `Stable`/`Experimental`/`Planned` statuses removed elsewhere in this
    audit). It is pending a rewrite and should not be read as current
    policy until then.
