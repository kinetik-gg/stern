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

Content below reflects the 2026-08-03 audit. Items 1, 5, and 8 were
re-verified 2026-08-23 against `main` after PRs #950–#953 landed (layout L0,
story harness, layout L1 builder seam, input-localization memo). Items
17, 18, 22, 29, 39, 45, and 47 were re-verified and closed or updated on
2026-08-23 by the issue #948 widget-polish batch (PR refs in each entry).
File and line references may drift as the code moves; if a reference looks
stale, trust the code and send a correction PR.

## Framework pillars — not started

1. **Layout engine — partially landed (RFC 0001 Phase L0/L1).**
   `crates/stern-core/src/layout/tree.rs` now provides the frame-local
   measure→arrange solver with a retained `MeasureCache` (#950), and
   `crates/stern-widgets/src/ui/builders.rs` is the L1 seam: content-sized
   builders measure shaped text through the frame's `TextLayoutStore` plus
   theme metrics, then delegate to the existing rect-first methods (#952).
   Still missing: the legacy flat widget surface has not migrated to
   builders (family re-pass work), no wrap-aware multi-line measurement in
   the solver path beyond single-line extents, baseline alignment is carried
   in `Measurement` but consumed by nothing, `retain_widgets` eviction is not
   wired to owner reconciliation ("Phase L1" note in `tree.rs`), content-level
   scope identity for localization memos is deferred to RFC 0001 §6, and the
   story families still compose rect-based dock/workspace layouts.
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

5. Closed in practice (#945, merged as PR #953): `Ui::push_primitive` input
   localization was
   `O(primitives × events)` per frame and is now `O(scope transitions ×
   events)` — draw primitives and layer markers skip re-localization
   entirely, and clip/transform scope changes are served from a per-frame
   memo keyed by (spatial state token, retained localization flags)
   (`crates/stern-core/src/runtime/ui.rs`, `refresh_scoped_input`;
   `crates/stern-core/src/runtime/spatial.rs`, `state_token`). Residual
   costs: every scope transition still clones the cached `UiInput`
   (including its event vector) into `FrameContext`, and state identity is
   instance-based (push/pop), not content-hashed, so re-entering a
   content-identical scope via a fresh push recomputes instead of hitting
   the memo. RFC 0001 §6's layout-node scope table remains the long-term
   home for content-level scope identity.
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
   instead of builders. Needs a builder API before any 1.0 talk. The RFC
   0001 Phase L1 seam (`crates/stern-widgets/src/ui/builders.rs`, #952)
   starts the migration with content-sized builders over the layout tree;
   the flat surface remains canonical until the family re-passes adopt it.
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
12. RESOLVED 2026-08-23 (PR #962, epic #948 dependability batch): library
    code no longer panics on these internal invariants. A selection gesture
    invoked without root event ordinals, or a captured gesture whose ordinal
    sidecar length mismatches localized input, now emits
    `FrameWarning::SelectionGestureOrdinalsMissing` /
    `GestureOrdinalSidecarMismatch` and deterministically skips gesture event
    processing for the frame with retained pointer state untouched
    (`crates/stern-core/src/interaction/press.rs`). Text-input owner
    transitions on an exhausted owner epoch now emit
    `FrameWarning::TextInputOwnerEpochExhausted` and refuse the change with
    owner, mode, platform state, and pending stop intact
    (`crates/stern-core/src/memory.rs`; warnings queue in `UiMemory` and are
    drained by `Ui::end_frame`). The two polygon `.expect`s in
    `crates/stern-core/src/runtime/spatial.rs` were provably unreachable
    behind their own guards and became equivalent non-panicking control flow
    with no warning site (nothing recoverable to report). `FrameWarning`
    gained three variants — breaking for exhaustive matchers; pre-alpha.
13. RESOLVED 2026-08-23 (PR #962, epic #948 dependability batch): `WidgetId`
    hashes through a pinned hand-rolled FNV-1a 64-bit scheme with
    little-endian integer encodings (`crates/stern-core/src/identity.rs`,
    offset basis `0xcbf29ce484222325`, prime `0x100000001b3`) instead of the
    unspecified `std DefaultHasher`, so derived IDs persist bit-for-bit
    across compiler releases, processes, and platforms and are safe to
    persist or compare across builds. Golden-value conformance tests pin
    exact outputs for representative keys. Documented residual limit: std's
    `Hash` implementations for key types must keep routing to stable inputs;
    if they drift, the golden tests fail loudly in CI.
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
    exceptions table in `docs/design-system-tokens.md`.
    Spacing, radii, and sizes are now mapped too: `SpacingStep`,
    `SpacingRole`, `SizeToken`, and the new `RadiusToken` each carry a
    `design_token_name()` method, with mapping tests in
    `crates/stern-core/src/theme/tests.rs` pinning `default_dark_theme()`'s
    `spacing`, `sizes`, and `radii` scales to the vendored `SPACING`,
    `SIZES`, and `RADII` tokens; every value already matched exactly (no
    divergences). A dedicated test
    (`default_dark_control_and_header_height_ladder_matches_geometry_spec`)
    pins the exact geometry-ladder values from
    `docs/visual-spec/00-language.md` (control heights 20/24/28/32, panel
    header 30, workspace bar 40) through `Theme::sizes`. That said,
    `Theme::sizes` is not consistently wired into layout: a repo-wide grep
    found only `sizes.icon.md` (icon rendering) and `sizes.workspace_bar`
    (`stern_widgets::chrome::ApplicationBar`) actually read by
    `stern-widgets` layout code; `sizes.control.*`, `sizes.row.*`,
    `sizes.tab`, `sizes.panel_header`, and `sizes.handle.*` have no widget
    consumer today. The dock tab strip
    (`crates/stern-widgets/src/dock/scene.rs`) hardcodes its own
    `DEFAULT_TAB_HEIGHT = 28.0` instead of reading `sizes.tab` — it
    currently matches `size.tab`, but is a separate literal, not derived
    from the theme, and can drift silently. Separately,
    `ControlMetrics::control_height` (28.0) and `::compact_control_height`
    (22.0) are an older, unrelated metric pair (used only for
    `padding_x`/`padding_y`, never as a height) that overlaps conceptually
    with the `SizeScale` control ladder but has no declared token
    correspondence — `compact_control_height`'s 22.0 matches no `SIZES`
    entry. Stern has no single authority for "the" control height; deciding
    whether `ControlMetrics` should be retired in favor of `SizeScale` (or
    given its own token) is a product decision, not made here (see
    `docs/design-system-tokens.md`). The hand-rolled theme values remain the
    runtime source of truth until every token group is mapped; strokes,
    durations, elevation, and icon tables are not yet wired. Independently,
    the design system's 486-requirement parity index
    (`../stern-design-system/generated/parity-index.json`) remains 100%
    `unverified`. Stern-side, `conformance/claims.json` now carries the
    first machine-validated claims against that requirement set: 63
    requirements from the foundations/behaviors scope, each citing the
    merged model-layer tests that exercise it, all capped at `partial`
    (validator: `conformance/tests/claims_contract.rs`;
    rules: `conformance/README.md`). The remaining 423 requirements are
    unclaimed — accessibility entirely (no OS bridge), everything outside
    foundations/behaviors, and every requirement without a genuinely
    matching merged test. Nothing is `verified` (no visual, platform, or
    scale evidence exists), and syncing these claims back into the
    design-system parity index is a later, owner-approved step (see
    `docs/catalogue-conformance-matrix.md`).
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
    RESOLVED for stern's value-drag callers by PR #932: `UiMemory` carries
    per-owner value-drag snapshot slots
    (`crates/stern-core/src/memory.rs`, `drag_value_snapshot`/
    `set_drag_value_snapshot`/`clear_drag_value_snapshot`) and
    `resolve_drag_value_cancellation`
    (`crates/stern-widgets/src/components/common.rs`) composes them with
    `draggable`, so slider and numeric-scrub drags restore the pre-drag
    value on cancellation (`PointerReleaseAll`, window-focus loss,
    Escape) and keep the committed value on ordinary release. The design
    system still has no first-class `value_transaction` primitive that a
    new caller would compose automatically — each future value-drag widget
    must call the resolver itself (see #20).
18. **No `value_transaction` composition for pointer-drag value editing;
    values leak on cancellation.** RESOLVED in practice by PR #932: the
    leak is closed for both production value-drag call sites —
    `slider_with_label_and_step`
    (`crates/stern-widgets/src/components/slider.rs`) and
    `numeric_scrub_input_*`
    (`crates/stern-widgets/src/components/numeric_inputs.rs`) snapshot the
    pre-drag value when a drag arms and restore it whenever
    `resolve_drag_value_cancellation` observes a cancelled interaction,
    including capture loss via `PointerReleaseAll`/focus loss. Headless
    tests reproduce mid-drag cancellation through the real `UiInput` event
    path for both controls
    (`tests/basic_component_conformance/slider_and_choice_keyboard.rs`,
    `tests/text_field_conformance/numeric_and_scrub.rs`). What remains of
    the original gap is naming/architecture only: the transaction is a
    shared helper over core snapshot slots, not the DS's named
    `value_transaction` contract, so nothing enforces its use on new
    value-drag widgets (see #17, #20).
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

## Visual conformance (Issue #910)

Found while conforming button family recipes (`docs/visual-spec/01-buttons.md`)
to `Theme::button_variant`. Not fixed here per that issue's non-goals (no
behavior changes beyond the recipe's own fill/border/text values, no layout
engine, no new components).

22. **Icon-button call sites hardcode `size.icon.md` (16px) instead of
    following D3's control-height-aware icon size.** RESOLVED by the #948
    widget-polish batch: `components::control_icon_size(height, theme)`
    (`crates/stern-widgets/src/components/common.rs`) resolves the D3
    ladder (sm 12 in controls ≤24 tall, md 16 from 28; the unspecified
    25-27 band splits at 26), and `static_icon_button`, the unsized
    `image_icon_button*` defaults, `action_button`'s leading icon (plus its
    label-width reservation in the Ui facade), and `paint_overlay_icon`
    are all driven from their control/row rect height. The explicit
    `image_icon_button_sized` variants keep caller-override semantics.
    Per-call-site tests pin compact/standard geometry
    (`components/tests/basic.rs`).
23. **No "busy" button state.** `01-buttons.md`'s default-variant table
    defines a `busy` row (muted `#999999` text, spinner icon rotating 1s
    linear) distinct from `disabled`. `ComponentState`
    (`crates/stern-core/src/theme/recipes.rs:7-21`) has no `busy` flag, and
    no button call site renders a spinner; `Theme::button_variant` cannot
    resolve a state that does not exist yet. Adding it is a new capability
    (a state flag plus an animated icon primitive), not a recipe value fix.
24. **No button-group/split-button composition.** `01-buttons.md`'s
    "Button group / split button" section specifies a fused row (shared 1px
    borders with collapsed adjacent edges, outer `radius.sm` only at the
    group's ends, inner radii 0, `border.default` dividers). No widget in
    `crates/stern-widgets` composes buttons into such a row — `ButtonRecipe`
    (`crates/stern-core/src/theme/recipes.rs:360-371`) always returns a
    uniform `CornerRadius`, and per-corner radius override is a layout
    concern the caller would own, which does not exist yet. Out of scope for
    a recipe-values-only issue.

## Visual conformance (Issue #911)

Found while conforming field family recipes (`docs/visual-spec/02-fields.md`)
to `Theme::text_field`. Not fixed here per that issue's non-goals (no new
`ComponentState` fields, no new widget capabilities, no layout engine).

25. **`read-only` field state is not modeled.** `02-fields.md`'s state table
    gives read-only fields a distinct S2 fill (`#141414`) and muted text
    (`#999999`), but `ComponentState`
    (`crates/stern-core/src/theme/recipes.rs:10-21`) has no `read_only`
    field, and `TextFieldAccess::ReadOnly`
    (`crates/stern-widgets/src/components/text_fields.rs:24-45`) is never
    threaded into the `ComponentState` passed to `Theme::text_field` — only
    `disabled` is. Read-only fields currently render identically to editable
    ones. Adding this needs either a new `ComponentState` field (150+ call
    sites across ~40 files construct this struct by naming every field, so
    it is a wide-blast-radius change) or a field-specific state parameter —
    a new capability, not a recipe value fix.
26. **`invalid` field state is not modeled.** `02-fields.md` specifies
    `border.invalid` plus a trailing 12px status icon (`status.danger.foreground`,
    7px right inset, 28px reserved right padding) for invalid fields. No
    `ComponentState` field, `TextFieldAccess` variant, or widget config
    signals a validation failure anywhere in `crates/stern-widgets`, and
    `text_geometry.rs`'s primitive list has no trailing-icon slot. Entirely
    new capability.
27. **No placeholder-text rendering for the canonical editable field.**
    `02-fields.md`'s idle-state table includes placeholder styling
    (`content.muted` `#999999`), but `text_field`/`multi_line_text_field`
    (`crates/stern-widgets/src/components/text_fields.rs`) have no
    placeholder-text concept at all — no config field, no muted fallback
    rendering when `TextEditState.text` is empty. (The unrelated `placeholder`
    field on `selector_fields.rs`/`overlays/dropdown.rs` is the Select/
    dropdown trigger's fallback label, part of the choice family, not this
    one.)
28. **Unit affix / field group is unimplemented.** `02-fields.md`'s "Unit
    affix / field group" section (affix cell fill/border/text, fused group
    borders, axis-prefix styling) has no implementation anywhere in
    `crates/stern-widgets` — no "affix" concept exists in the crate at all.
29. **IME composition underline reuses the selection color instead of
    `focus.ring`.** RESOLVED by the #948 widget-polish batch:
    `text_geometry.rs`'s composition-underline stroke now paints with
    `recipe.caret` (resolved from `colors.focus.ring`, the same source as
    the caret), and a primitive-level test pins the underline brush against
    the theme's focus.ring while asserting it is not the selection fill.
30. **Selected text is not repainted in `selection.foreground`.**
    `00-language.md` §Selection-vs-hover doctrine: selection is
    `selection.background` fill + `selection.foreground` (white) text.
    `text_geometry.rs` always paints every glyph run in `self.recipe.foreground`
    (lines 368, 382), never swapping to `selection.foreground` for the
    selected sub-range. Fixing this needs new per-run color plumbing through
    the text-shaping/paint pipeline — a new capability, not a value fix.
31. **Numeric scrub never requests the `ew-resize` cursor.** `02-fields.md`:
    "During scrub: ... cursor = ew-resize." `numeric_scrub_input`
    (`crates/stern-widgets/src/components/numeric_inputs.rs`) always
    inherits `CursorShape::Text` from the underlying canonical text field
    (`crates/stern-widgets/src/components/text_fields.rs`'s
    `with_hover_cursor(..., CursorShape::Text)`) — `CursorShape::ResizeHorizontal`
    already exists (`crates/stern-core/src/runtime/types.rs`) but nothing
    requests it during a scrub drag. A widget call-site change, not a
    recipe value.

## Visual conformance (Issue #912)

32. **Choice/slider/tab family (Issue #912)**: `Theme::tab` (also painted by dock/chrome document tab strips, family #914) was left unconformed to `03-choice-sliders-tabs.md`'s segmented/tab-strip table to avoid clobbering that concurrent family; no progress-bar widget/recipe exists yet; checkbox/radio glyphs and the slider thumb now resolve correct per-state colors (`CheckRecipe.mark`, new `SliderRecipe.thumb`) but have no paint primitive (`ComponentState` also has no `mixed` checkbox flag) — see PR body for the full before/after table.

## Visual conformance (Issue #913)

Found while conforming overlay family recipes
(`docs/visual-spec/04-overlays.md`) to `Theme::overlay_surface`,
`Theme::overlay_item`, and `Theme::command_palette_item`
(`crates/stern-core/src/theme/model.rs`) and their call sites in
`crates/stern-widgets/src/ui/overlays.rs`. Not fixed here per that issue's
non-goals (values only — no anatomy/placement rewrites, no new components).

33. **`OverlaySceneRowKind::Passive` conflates four different text roles
    04-overlays.md tables separately.** `crates/stern-widgets/src/overlays/
    scene.rs`'s `rows()` uses the same `OverlaySceneRow::passive`/
    `menu_label` constructors, and `paint_overlay_row`'s non-`Action` branch
    (`crates/stern-widgets/src/ui/overlays.rs`) paints all of them with the
    same `TextRole::Label` foreground, for: menu group headings (spec: meta
    9/mono, UPPERCASE +0.06em, muted), modal title (control-strong 11/600),
    modal body (body type), tooltip text (detail 10, secondary), and command
    palette's query row (body 12, primary). Distinguishing them needs new
    `OverlaySceneRowKind` variants (or an explicit text-role field) — a
    row-kind/anatomy change, not a recipe-values fix.
34. **Modal anatomy has no header/footer chrome.** 04-overlays.md's Modal
    section specifies a header (height 34, padding-inline 12, title
    control-strong, `border-b` `border.subtle`), body (padding 12), footer
    (height 44, padding-inline 12, right-aligned actions, `gap.group` 8,
    `border-t` `border.subtle`), and a leading danger icon (16,
    `status.danger.foreground`) in danger modals' title row.
    `OverlaySceneSurface::Modal`'s `rows()` arm (`crates/stern-widgets/src/
    overlays/scene.rs:369-403`) only emits a title passive row, an optional
    body passive row, and flat action rows — no header/footer bands, no
    borders between them, no danger icon. Building that anatomy is out of
    scope for a recipe-values-only issue.
35. **Command palette anatomy has no search-row grid or footer.**
    04-overlays.md specifies a search row (height 38, grid 18/flex/auto,
    leading search icon, trailing esc-hint, `border-b` `border.default`) and
    a footer (height 26, fill `surface.application` S1, `border-t`
    `border.subtle`, hint items gap 12). `OverlaySceneSurface::CommandPalette`
    (`crates/stern-widgets/src/overlays/scene.rs:340-368`) only emits a plain
    `"> {query}"` passive row and flat result rows — no search-row
    icon/esc-hint columns and no footer row at all. Out of scope here.
36. **Menu group-heading padding, separator inset, and submenu overlap
    offset aren't verified against 04-overlays.md's exact metrics.** The
    spec calls for group-heading padding 7/8/4, a full-width separator inset
    4 with 4 block margin, and submenus opening overlapping their trigger by
    -4. `RowLayout`/`placed_entry`
    (`crates/stern-widgets/src/overlays/scene.rs`,
    `crates/stern-widgets/src/overlays/placement.rs`) predate this issue and
    were not audited against these specific numbers — a layout/anatomy
    check, not a recipe-color fix.
37. **Typography scale steps aren't wired into `TextRole` at all (D5,
    background gap, not overlay-specific — surfaced concretely here).**
    00-language.md's five-step scale (body 12, control 11, control-strong
    11/600, detail 10, meta 9/mono) has no corresponding `TextRole::Detail`
    or `TextRole::Meta`/9px entry in `FontSizeScale`
    (`crates/stern-core/src/theme/model.rs`); every overlay row (menu items,
    tooltip text, shortcut column, palette results) renders through
    `TextRole::Label` regardless of which step the spec calls for. Adding
    the missing steps is a typography-system change tracked by D5, not a
    per-recipe value fix.
38. **`DragPreview` has no normative surface treatment in
    04-overlays.md.** `overlay_surface_tier`
    (`crates/stern-widgets/src/ui/overlays.rs`) falls it back to the `Menu`
    tier (`surface.overlay`/`border.default`/`radius.md`) and
    `overlay_elevation_level` keeps it at `ElevationLevel::Low`, both
    unchanged from before this issue — there is no spec table to conform to
    yet.
39. **Overlay row icons hardcode `size.icon.md` (16px) regardless of D3's
    control-height-aware sizing**, the same root cause as gap #22 above but
    a call site #910 didn't enumerate. RESOLVED by the same commit as #22:
    `paint_overlay_icon`
    (`crates/stern-widgets/src/ui/overlays.rs`) now resolves
    `control_icon_size` from the row/slot height (12 at the 24px compact
    row height), clamped to the slot, with menu-row geometry covered by the
    D3 tests and rendered evidence in the collections story.

## Visual conformance (Issue #914)

Found while conforming chrome/dock/inspector recipes (`docs/visual-spec/05-chrome-dock.md`,
`07-status-feedback-inspector.md` §inspector/§status-dots/§jobs). Not fixed here per that
issue's recipe-only scope. `Theme::tab` itself (deferred by #912 above) *is* conformed here,
closing that deferral.

40. **Chrome bars, dock frames, and tabs paint a full-perimeter border**
    where the density ladder specifies a single edge (e.g. panel header
    `border-b`, status bar `border-t`). `RectPrimitive` has one uniform
    `stroke`, no per-edge variant; fixing this needs either a directional-
    stroke primitive or the extra-thin-rect idiom already used for the
    property-grid status accent (`crates/stern-widgets/src/ui/property_grid.rs`)
    applied to every bar, not attempted broadly here.
41. **Inspector property rows never hover-promote to S4 or right-align
    their labels.** `07-status-feedback-inspector.md` §Inspector property
    rows: "Row hover: fill S4"; "Label: ... right-aligned". But
    `paint_property_grid_row` (`crates/stern-widgets/src/ui/property_grid.rs`)
    has no hover `ComponentState` plumbed in (only `PropertyGridAccess`) and
    left-aligns the label origin unconditionally.
42. **System-feedback job/diagnostic/feedback rows don't match 07's job-row
    anatomy.** `07-status-feedback-inspector.md` §Job list rows specifies
    name / progress-bar / percent / cancel-or-retry-button with a status dot
    replacing progress on completion; `paint_system_feedback_row`/
    `paint_job_progress` (`crates/stern-widgets/src/ui/chrome/system_feedback.rs`)
    instead share one generic row shape (left tone stripe + bottom progress
    track) across jobs, diagnostics, and feedback. A structural anatomy
    change, not a recipe fix.
43. **Dock frames don't take `radius.md` despite `00-language.md`'s "editor
    frames = radius.md" rule.** `paint_dock_frame`
    (`crates/stern-widgets/src/ui/dock.rs`) keeps `radii.none`: frames tile
    edge-to-edge via 1px splitters, so naive rounding would show
    gaps/overlaps at internal seams without per-seam corner suppression,
    which this recipe-only pass does not attempt.

## Visual conformance (Issue #915)

Found while conforming collection recipes (`docs/visual-spec/06-collections.md`)
to `Theme::row`/`Theme::table_header_row`/`Theme::asset_card` and their
`crates/stern-widgets/src/ui/{collections,outliner,virtual_table,virtual_tree,
asset_browser}.rs` consumers. Not fixed here per that issue's non-goals (no new
`ComponentState`/`TextRole` capabilities, no new render primitives beyond
composing existing `Rect`/`Line` shapes).

44. **No "meta" typography role.** `00-language.md`'s type scale documents a
    `meta` step (9px/1, 500-700 weight, mono, UPPERCASE +0.06em tracking) for
    "shortcuts, group headings, status bars, numeric readouts, badges" —
    which is exactly what column headers ("UPPERCASE +0.06em muted") and row
    meta/trailing text call for. `TextRole`
    (`crates/stern-core/src/theme/tokens.rs:1362-1373`) only has
    `Body`/`Label`/`Caption`/`Title`/`Monospace`, with no size/weight/
    letter-spacing/case-transform fields on `TextRoleMetrics` at all. The
    table header (`table_header_label`,
    `crates/stern-widgets/src/ui/virtual_table.rs:698`) and the asset card's
    kind line render with the closest existing roles (`Label`/`Monospace`)
    instead. This is the same structural gap D5 in `00-language.md`'s
    divergence table already names ("no size tokens exist... propose
    upstreaming as tokens") — a typography-scale capability, not a
    collection recipe value.
45. **Table sort indicator is a text glyph, not the spec's icon.**
    RESOLVED by the #948 widget-polish batch: `table_header_label` returns
    the plain column name again (the sorted state stays conveyed by the
    header's `Sorted ascending/descending` semantic value), and
    `paint_virtual_table_header` emits a real 12px `size.icon.sm` vendored
    Phosphor caret in each header's trailing padding — muted caret-down as
    the affordance on non-active columns; direction-aware caret-up/down in
    `focus.indicator` with secondary label text (via the existing
    `table_header_row` recipe) on the active sort column. Documented
    interpretation: the spec sentence "caret icon 12 muted" is implemented
    as a per-column affordance rather than hover-only, since the spec does
    not define a hover state for headers. Conformance tests pin icon
    identity, geometry, both tints, and arrow-free label text.
46. **Outliner visibility/lock toggle hover never promotes to
    `content.primary`.** `06-collections.md` §Tree rows: "Inline visibility/
    lock toggles: quiet icon buttons 16, muted → primary on hover; remain
    visible on selected rows in white at 78%." The selected-row 78%-white
    exception is implemented; the muted→primary hover promotion is not —
    `paint_outliner_row`'s `toggle_foreground`
    (`crates/stern-widgets/src/ui/outliner.rs:888-894`) only branches on
    `disabled`/`selected`, never on each icon's own hover `Response` (already
    captured earlier in the same function as `visibility`/`lock`, but not
    threaded through). Needs new per-icon color parameters on
    `paint_outliner_visibility`/`paint_outliner_lock`, deferred to avoid
    guessing how it composes with their existing on/off alpha encoding (see
    PR body).
47. **No scrollbar recipe or paint primitive for virtualized collections.**
    RESOLVED by the #948 widget-polish batch:
    `crates/stern-widgets/src/collections/scrollbar.rs` provides the
    deterministic thumb geometry (`vertical_thumb`: hidden when content
    fits or extents are invalid, proportional length with a documented
    12px floor so extreme content stays visible, position clamped to the
    inset track) and a `border.strong`/`radius.full`/inset-2 painter with
    no track primitive (the spec's track is transparent). All five
    virtualized paint paths — virtual list, virtual table, virtual tree,
    outliner, asset browser — emit the thumb after their content clip
    closes so it stays fixed to the container. The thumb is chrome only:
    it has no hit target or drag interaction; scrolling remains
    wheel/keyboard-driven, and adding interaction is a separate slice.
    Geometry unit tests plus scrolled-state primitive assertions in the
    list/tree conformance tests and rendered evidence in the new
    collections/sheet story pin the behavior.

## Story harness (Issue #943)

48. **Choice-control labels: toggles still need an explicit label rect.**
    RESOLVED for checkboxes/radios by issue #946: `checkbox_with_label` and
    `radio_button_with_label` now paint the label (control type, secondary,
    gap 6 per visual-spec 03) to the right of the box inside the caller's
    control rect, and every `*_with_label_target` variant paints into its
    explicit label rect (`crates/stern-widgets/src/components/choice.rs`,
    `choice_label_paint_region`/`push_choice_label`). REMAINING: the toggle
    track fills its whole control rect, so `toggle_with_label` without an
    explicit label rect has no label region and paints no text — callers
    must use `toggle_with_label_target` until the toggle track is
    content-sized (26x14 per visual-spec 03 §Switch) in the layout-era
    family re-pass (EPIC #948 Phase 4). The checkbox mixed-state 6x2 bar
    also remains unpainted: the API has no public mixed state.
49. **Story CPU raster path is its own baseline, not GPU-parity.**
    `apps/stern-stories/src/raster.rs` executes the sanitized
    `stern-vello` command stream with tiny-skia + swash. It approximates
    shadows with a triple box blur, ignores non-uniform command-transform
    scale on stroke widths, and skips textures without CPU snapshots.
    Goldens produced by it regress the CPU path only; GPU output review
    still needs a human looking at a live window (EPIC #948 owner pass).
50. **Stories live in the harness crate, not next to widget source.**
    `apps/stern-stories/src/stories/` mirrors widget families, but
    `crates/stern-widgets` does not yet declare stories beside each
    widget's implementation (the #943 ideal). Moving declarations into the
    widget crate needs a dependency-clean story-registration seam; deferred
    until the L1 builder work (#942) settles what a widget-owned story
    signature looks like.

## Platform surface (Issue #944)

51. **Surface extent is never clamped to the device texture limit
    (theoretical above 8K-UHD; found by the #944 4K investigation).** The
    resize path (`crates/stern-vello-winit/src/presenter.rs`,
    `configure_existing_surface` -> vello `RenderContext::resize_surface`)
    forwards the raw window physical extent to surface configuration and
    render-target creation without clamping against
    `max_texture_dimension_2d`. Vello's `RenderContext` requests
    `wgpu::Limits::default()` (8192), so every 4K/5K/8K-UHD monitor is
    fine; a window spanning more than 8192 physical pixels in one dimension
    (e.g. one window stretched across two 5K monitors) would fail surface
    configuration instead of presenting a clamped image. The rest of the
    resize -> reconfigure -> present ordering was verified sound during the
    same investigation: `present` preflights `resize` from the live
    `window.inner_size()` before acquiring, and `drive_present` rejects
    frames whose viewport or acquired texture extent mismatches the
    configured extent (`FrameExtentOutdated`/`AcquiredExtentOutdated`),
    reconfiguring and requesting the next frame rather than presenting
    stale geometry.

## Family re-pass (controls) — found by the controls/states story

52. **Button, checkbox, radio, toggle, text-field, and select recipes do not
    visually differentiate hover/focus from the default state.** The
    `controls/states` story seeds `UiMemory::set_hovered`/`set_focused` for
    one control per family and renders them side by side with the default
    state; sliders prove the seeding path works (white hover thumb,
    spec-correct full-row focus ring), but buttons render hover/focus
    identically to default, choice controls show no hover/focus delta, and
    the focused text field paints no focus ring. Fixing this is recipe-value
    work (`Theme::button_variant` / `Theme::text_field` hover and focused
    rows per `docs/visual-spec/01-buttons.md` and `02-fields.md`), deferred
    from the geometry-scoped family pass. Evidence:
    `apps/stern-stories/src/stories/controls_states.rs`.
