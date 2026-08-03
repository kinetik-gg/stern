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
2. **Application shell.** First slice delivered: `crates/stern-app` owns the
   winit event loop, window, input adaptation, platform requests, repaint
   scheduling, retained UI state, and automatic GPU recovery behind an
   eframe-class `App` trait (`crates/stern-app/examples/hello_stern.rs` is a
   complete app in ~60 lines; `apps/stern-demo/src/bin/native_shell.rs` now
   runs on it). Remaining: multi-window; window chrome/options beyond title
   and initial/min size (icon, decorations, position, resizability);
   device-scope access for native GPU texture producers, for which
   `crates/stern-vello-winit/examples/one_window.rs` stays the manual
   application-owned path; and the accessibility/font gaps tracked as items
   3 and 4.
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

## Behavior primitive conformance (Issue #904)

Full mapping in `docs/design-system-primitives-map.md`. These are the
genuine mismatches found there against
`../stern-design-system/src/behaviors/primitives-and-contracts.md`, not
doc drift.

17. **`draggable`'s cancellation invariant is not honored for value edits.**
    The design system's `draggable` primitive promises cancellation
    "restores the pre-drag value or geometry"
    (`../stern-design-system/src/behaviors/primitives-and-contracts.md`,
    primitive table), but stern's `draggable`
    (`crates/stern-core/src/interaction/drag_select.rs:97-138`) only owns
    pointer capture, threshold, delta, and drag-source identity — it has no
    concept of a caller value to restore. The DS explicitly expects this to
    be closed by composing `draggable` + `value_transaction`
    (`primitives-and-contracts.md:25`, the `scrubbable` note), but no
    stern caller does. See #18 for where this actually bites.
18. **No `value_transaction` composition for pointer-drag value editing;
    values leak on cancellation.** `slider`/`slider_with_label_and_step`
    (`crates/stern-widgets/src/components/slider.rs:115-187`) composes
    `draggable` (line 127) and then mutates `*value` directly from the
    absolute pointer x-position every frame the gesture is active
    (`slider.rs:130-136`), with no captured starting value. The public
    entry point `numeric_scrub_input`
    (`crates/stern-widgets/src/components/numeric_inputs.rs:287-299`)
    delegates to `numeric_scrub_input_with_text_layouts_and_caret_visibility`
    (`numeric_inputs.rs:328-397`), which similarly accumulates
    `draggable`'s `drag_delta.x` onto the value every frame
    (`numeric_inputs.rs:363-383`). Neither path snapshots a "starting
    value" before the drag begins, so a cancelled or interrupted drag
    (pointer capture lost via `PointerReleaseAll`/window-focus loss, or the
    widget disabled mid-drag) leaves `*value` at whatever the last preview
    position produced instead of restoring it — violating
    STERN-PRIM-002 ("every cancellable direct-manipulation contract must
    preserve its starting value... and restore it on cancellation"). The
    *typed-text* editing path already gets this right —
    `NumericInputPolicy{draft, commit_requested, revert_requested}`
    (`numeric_inputs.rs:139-158`, resolved at
    `crates/stern-widgets/src/components/text_fields.rs:576-587`) commits or
    reverts via `restore_text_draft` (`text_fields.rs:561`) — proving the
    begin/change/commit/cancel pattern is understood; it was just never
    added to the drag-scrub call sites. No fix applied here per this issue's
    non-goals (no behavior changes, no new primitives).
19. **`roving_focus` has two independent, divergent implementations instead
    of one canonical primitive** — the exact failure mode STERN-PRIM-001
    exists to prevent. `CollectionCursor::navigate`
    (`crates/stern-widgets/src/collections/navigation.rs:114-146`) tracks a
    stable `ItemId`, does not wrap at the ends
    (`saturating_sub`/`saturating_add`/`.min(last_index)`), and
    deterministically reconciles a disappeared active item via
    `CollectionCursor::reconcile` (`navigation.rs:91-107`). `moved_index`
    (`crates/stern-widgets/src/overlays/navigation.rs:97-119`), used by
    menus/menu bars/command palette, tracks a bare `usize` position with no
    stable identity and wraps at both ends via modulo arithmetic
    (`overlays/navigation.rs:107-116`), with no reconciliation concept at
    all. Same behavior primitive, two different identity models (stable ID
    vs. raw index) and two different boundary policies (clamp vs. wrap),
    each used by a different widget family. This is naming/behavior drift
    that invites real bugs (e.g. a widget migrated from one family to the
    other silently changes wrap behavior and identity stability). No
    refactor applied here per this issue's non-goals.
20. **No `two_axis_value` primitive; `vector2_scrub_input` is two
    independent 1-axis fields, not a joint 2D drag.**
    `vector2_scrub_input`/`VectorScrubInputOutput<const N: usize>`
    (`crates/stern-widgets/src/components/vector_color_fields.rs:14-84,163-192`)
    composes `N` separate `numeric_scrub_input` calls side by side
    (OR-ing `scrubbed`/`value_changed` across axes,
    `vector_color_fields.rs:356-406`), each with the same
    missing-restore-on-cancel gap as #18. The design system's own text says
    "a two-axis color selector is `two_axis_value` over a tagged color
    projection" (`primitives-and-contracts.md:25`), describing a single
    joint pointer-drag surface (e.g. a saturation/brightness square) — a
    grep for "saturation", "2d drag", "joystick", "xy pad" across the
    workspace found no such control anywhere. Two side-by-side X/Y number
    fields are a reasonable UI in their own right but are not the primitive
    the design system names; flagged here rather than built, per this
    issue's non-goals (no new primitives).
21. **`multi-value-state` contract is a bare boolean, not a value wrapper.**
    The design system's `multi-value-state` contract asks for "zero/one/many
    source values, common value when equal, mixed flag, and explicit write
    policy" — the shape needed to edit a property across a multi-selection
    where values may differ. stern only has a `bool` mixed/indeterminate
    flag in three unrelated places: `ActionState.mixed`
    (`crates/stern-core/src/actions.rs:46`), `RowCheckState::Mixed(bool)`
    (`crates/stern-widgets/src/overlays/scene.rs:719`), and
    `AccessibilityNode.mixed` (`crates/stern-core/src/accessibility/model.rs:91`).
    There is no generic `Same(T) | Mixed` (or equivalent) type anywhere,
    including in the property-grid code
    (`crates/stern-widgets/src/inspector/property_grid.rs`,
    `crates/stern-widgets/src/inspector/row.rs`) where multi-selection
    property editing would need it most. Not built here per this issue's
    non-goals (no new contracts).
