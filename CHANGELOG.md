# Changelog

All notable changes to Stern will be documented in this file. Categories
follow the repository release policy.

## [Unreleased]

`1.0.0-rc.2.dev` is the planned first prerelease. It has not been dated, tagged,
published, or accepted as an alpha release.

### Changed

- **Breaking:** Replaced bundled Geist Mono with exact Space Mono Regular from
  revision `329858c2c4dbd3476f972a4ae00624b018cf4b81`. The public monospace
  default is now `"Space Mono"`; named/default/generic `"monospace"` and
  `"mono"` resolution use the same pinned bytes. Removed public
  `fonts::{GEIST_UPSTREAM_COMMIT, GEIST_MONO_VARIABLE}` without aliases and
  added the corresponding Space Mono authority. Applications should expect
  monospace metrics, layout geometry, snapshots, and derived hashes to change.
  This advances only Partial deterministic Mono alignment and asset/license
  provenance evidence; nothing is Accepted. See `docs/typography-migration.md`.
- **Breaking:** Added exact customizable font-size, line-height, weight, and
  feature foundation scales to `TypographyScale`. The defaults are the pinned
  six-size `12/11/10/14/16/20`, three-line-height `16/15/14`, and four-weight
  `400/500/600/700` inventories plus the numeric feature value
  `"tabular-nums"`. Existing text-role family, size, and line-height resolution
  is unchanged. The new weight and feature values are deterministic metadata
  only; they are not transported to text, shaping, widgets, or renderers and
  make no tabular-figure conformance claim. External `TypographyScale` struct
  literals must initialize the four new scales. See
  `docs/typography-migration.md`.
- **Breaking:** Replaced the five resolved `FontToken` values stored by
  `TypographyScale` with exact UI, Brand, and Mono semantic family roles plus
  five `TextRoleMetrics` values. `Theme::font` keeps its resolved `FontToken`
  result, while `Theme::font_family` exposes typed family lookup. Body, Label,
  Caption, and Title resolve to Inter; Monospace resolves to Space Mono; the
  exposed Space Grotesk Brand role is intentionally unadopted. External
  `TypographyScale` struct literals must migrate to the new shape. This change
  adds no font assets and makes no loading, fallback, renderer, or visual
  conformance claim. See `docs/typography-migration.md`.
- **Breaking:** Removed `ControlMetrics::check_size` and made one private exact
  `14.0` component-recipe dimension the sole visible indicator-size authority
  for checkbox and radio controls. `Theme::radio_button` continues to inherit
  the checkbox recipe size; caller-owned rectangles, full-label interaction and
  semantic bounds, focus layers, and component state behavior are unchanged.
  This migration adds no size token or alias. External `ControlMetrics` struct
  literals must delete `check_size`. See `docs/size-migration.md`.
- **Breaking:** Removed `ControlMetrics::icon_size` and made
  `Theme::sizes.icon.md` the sole default icon-size authority for unsized
  bitmap, selectable-bitmap, vector-library, and missing-vector icon buttons.
  Invalid explicit bitmap sizes now use that same themed fallback, while valid
  explicit sizes and the remaining four `ControlMetrics` fields are unchanged.
  See `docs/size-migration.md`.
- **Breaking:** Added the exact grouped 14-token `SizeScale` foundation at
  `Theme::sizes`, with typed `SizeToken` lookup and replacement through
  `Theme::with_sizes`. External `Theme` struct literals must initialize the new
  field. This prerelease foundation adds no aliases or mirrored values; the
  medium icon token now owns default icon-button geometry. See
  `docs/size-migration.md`.
- **Breaking:** Replaced `SpacingScale::{xs, sm, md, lg, xl}` and its
  five-value constructor with the exact nine-step `zero` through `eight`
  ladder, plus typed `SpacingStep` and `SpacingRole` inventories. Semantic
  roles resolve from the configured ladder; no legacy fields, aliases, or
  forwarding methods remain. Existing prerelease consumers must pass all nine
  values to `SpacingScale::new` and select exact or semantic roles. See
  `docs/spacing-migration.md`.
- **Breaking:** Removed `ControlMetrics::{border_width, focus_width,
  separator_width}` and added the exact shared `StrokeScale` ladder at
  `Theme::strokes`. Existing theme customization should migrate width roles to
  `Theme::with_strokes`; `Theme::border_width` remains only as a one-way legacy
  mirror of `strokes.default`. External `Theme` struct literals must initialize
  the new `strokes` field. See `docs/stroke-migration.md`.
- **Breaking:** Replaced `RadiusScale::{xs, pill}` and the provisional radius
  defaults with the exact `none`, `sm`, `md`, `lg`, and `full` ladder. The
  four-argument `RadiusScale::from_values(sm, md, lg, full)` now fixes `none`
  at zero, and direct consumers select the new roles by shape intent. No legacy
  aliases remain. See `docs/radius-migration.md`.
- **Breaking:** Replaced `ElevationScale::{flat, raised, overlay}` with the
  exact four-level `none`, `low`, `medium`, and `high` scale and changed
  `Theme::elevation_shadow` to accept `ElevationLevel` instead of an arbitrary
  `f32`. Overlay callers must now choose the semantic level that matches their
  real layering and input behavior. No legacy aliases remain. See
  `docs/elevation-migration.md`.
- **Breaking:** Replaced the provisional 19-field flat `ThemeColors` palette
  and broad `SemanticColor` variants with eight non-exhaustive grouped color
  families, 53 exact role keys, `SemanticColor::ALL`, and the explicit
  `ThemeColors::default_dark()` starting palette. Existing recipes and widget
  consumers now resolve deliberate semantic paths; applications must migrate
  by mutating the grouped palette before calling `Theme::with_colors`. No
  legacy field/variant mirror remains. See `docs/theme-color-migration.md`.
- Added stable-ID collection cursor navigation and public fixed-height virtual
  list/tree scenes with bounded large-data materialization, scrolling,
  keyboard focus/reveal, selection, expansion, theme primitives, and ordered
  semantics. Variable-height rows,
  custom row bodies, drag/drop, and inline rename remain later component work.
- Added a public fixed-height virtual table scene with headers and cells,
  two-axis retained scrolling, application-owned sort intents, stable row/cell
  selection, two-dimensional keyboard focus/reveal, constrained column resize
  requests, theme primitives, and ordered table semantics. Horizontal column
  virtualization, editing,
  multi-selection, grouped headers, and column reordering remain outside the
  MVP. `Ui::virtual_table` now takes caller-owned retained
  `VirtualTableSelection`, a provisional breaking alpha API change.
- Added stable-key chrome overflow projection and one public borrowed painted
  chrome scene over menu bar, toolbar, tab strip, and status bar models. The
  scene contributes clipped pointer targets, emits themed backend-independent
  primitives and ordered semantics, and returns typed menu, action, tab, and
  overflow intents. Toolbar icon polish and broader visual regression coverage
  remain incomplete.
- Added pure menu/dropdown keyboard, typeahead, reconciliation, and submenu
  intents plus one public painted overlay scene for menus, context menus,
  dropdowns, command palettes, modals, popovers, tooltips, and drag previews.
  The scene contributes to the caller's frame-wide pointer plan, emits themed
  backend-independent primitives and ordered semantics, and returns lifecycle
  and application-owned action intents. Menu-bar trigger and overflow painting
  are integrated with the public editor chrome.
- Added deterministic measured grid allocation and public keyed `Ui` row,
  column, grid, padding, stack, and scrolling containers, then dogfooded that
  seam through the facade example and Showcase layout preview without changing
  their established geometry. Broader
  retained/CSS-like layout remains outside the MVP.
- Added the qualified native-texture registration and Vello resolver foundation,
  including checked registration/revision identity, native-first texture command
  resolution, same-renderer lower bridge scoping, and device-lifetime invalidation.
- Added real-DX12 native-texture evidence for color/alpha, producer-handle
  lifetime, update/replace/remove, foreign-device validation, and recovery
  rebind, plus a runnable GPU producer and extracted package-consumer proof.
- Added the Experimental `stern-vello-winit` presenter boundary with a
  qualified facade feature, exact one-window acquire/render/blit/notify/present
  policy, zero-size handling, generation-scoped device borrowing, typed surface
  and device recovery, deterministic lifecycle evidence, and a runnable public
  example. The Showcase now adopts the public presenter while retaining
  application-owned input, shell, frame, and repaint work. The qualified
  native-texture path remains Experimental; this does not publish or accept an
  alpha release.
- Prepared package metadata and dependency constraints for the planned
  prerelease archives.
- Made Winit platform batches owned and consuming, with ordered shell services,
  same-owner IME rectangle updates, and stateful repaint replacement. This is a
  provisional breaking API change: callers must use `WinitPlatformRequests`
  getters and consume `apply_to_window`/`apply_to_window_ops`, then split the
  returned `WinitAppliedRequests` with `into_parts`.
- Added `PlatformRequest::UpdateTextInputRect`; exhaustive matches over the
  provisional public enum must handle the new same-owner IME geometry request.
- Added native text clipboard and hardened HTTP/HTTPS browser services to the
  Winit adapter; image clipboard features remain disabled.
- Normalized ordered line-wheel events to a fixed 40-logical-unit step while
  preserving exact logical pixel deltas and the legacy empty-stream magnitude.
  Added timestamped Winit click sequencing; the explicit-count method remains
  available and resets automatic history.
- Added a fixed four-current-scope-logical-unit drag threshold with latched
  release-click suppression. Canonical pointer transitions now resolve once in
  order, and `Ui::captured_selection_gesture` exposes original-root-ordinal
  selection actions without turning text selection into a domain drag source.
  `Ui::claim_ordered_text_input_events` supplies the matching ordinal-bearing
  editing stream so text fields need not parse pointer events again. Text
  composite numeric scrub fields resolve one domain-drag response without a
  second press pass. Global cancellation fences preserve earlier owner and
  wheel output even when primary and secondary owners differ, and planned drops
  use declared source intent plus immutable first-causal press/release geometry.
  Owner-mismatched plans and canonical unplanned drop commits now fail closed;
  passive hover also observes canonical focus-loss fences while pre-fence wheel
  deltas remain usable. Empty-stream legacy behavior remains compatible.
- Added event-time `modifiers` to the provisional public
  `SelectionGestureAction`. This is a source-breaking alpha API change:
  external struct literals must initialize `modifiers`, and patterns that do
  not name every field must use `..`. Canonical actions now report the modifier
  state at their original root ordinal; legacy snapshot actions use the
  snapshot modifier state.
- Added `Ui::captured_domain_drag_gesture` with DomainDrag-specific ordered
  actions and a causal `release_clicked` result on each release. Ordinary,
  transformed, and captured DomainDrag calls now share one exact first response
  per begun frame, deliver actions once, and keep public action ordinals
  separate from internal release/drop authority. Unframed standalone
  `draggable` calls remain uncached. This is a provisional breaking behavioral
  change for callers that resolve the same `WidgetId` more than once in a begun
  frame: use one authoritative call and share its `Response`, or use distinct
  widget IDs for genuinely distinct interactions.
- Separated frame-local async-owner presence from durable registry-scoped
  incarnation. Repeated presence marks now return one stable opaque token,
  while restart, exact-token cancellation, removal, same-ID reuse, foreign
  registries, observer delivery, and one-following-frame tombstone cleanup have
  deterministic typed outcomes. This is a provisional breaking API change:
  `LivenessToken::new` and observer token-refresh APIs were removed; Clone was
  removed from `UiMemory`, `LivenessRegistry`, `ObserverRegistry`, and
  `UiTestHarness`; generation/status terminology moved to incarnation; and
  `remove_live_target` now returns `LivenessRemovalStatus`. Mark owners present
  each frame, retain one token per incarnation, call `restart` for replacement
  work, and create a new observer subscription after restart or reentry.
- Completed canonical retained-`Ui` desktop text behavior. Added scalar-safe
  word movement, extension, deletion, and run selection; deterministic
  horizontal single-line and vertical wrapped-multiline `TextViewport`
  helpers; logical `TextInputOwnerMode`; and
  `TextFieldAccess::{Editable, ReadOnly, Disabled}` entry points. Canonical
  fields now merge root-ordinal pointer selection with exactly-once ordered
  editing input, retain viewport offsets between frames, publish visible
  clipped caret rectangles for editable IME, and give numeric scrub, search,
  path, and vector wrappers one shared transaction boundary. Read-only fields
  remain focusable, selectable, scrollable, navigable, and copyable without
  mutation or native IME; disabled fields remain non-interactive. Existing bool
  APIs stay source-compatible (`false` maps to Editable and `true` to Disabled),
  while explicit read-only behavior uses the retained `Ui` access/config APIs.
  Public free component functions retain their legacy compatible signatures and
  output shapes.
- Upgraded logical text editing to UAX #29 extended grapheme clusters and
  full-buffer word-bound segments. Combining sequences, emoji modifiers,
  regional-indicator flags, ZWJ emoji, and CRLF are atomic for navigation and
  deletion; explicit-line columns count graphemes; selections and composition
  ranges clamp to grapheme boundaries. Added qualified `TextCaret` and
  `TextAffinity` APIs with deterministic before/after association, stale-public-
  selection fallback, and undo/redo restoration. Existing byte-only setters
  remain compatible.
- Added source-bound `ShapedTextNavigation` derived from existing cosmic-text
  cluster ranges. It validates public shaped layouts all-or-nothing, subdivides
  multi-grapheme clusters by EGC count, preserves bidi/wrap affinity aliases,
  and supplies one authority for visual caret/word motion, hit testing, caret
  rectangles, and disjoint selection spans. New `TextEditState` visual
  move/extend methods and `apply_visual_navigation_key` reject stale maps
  transactionally and leave text, composition, and local undo/redo untouched.
  Canonical retained fields configured with `TextLayoutStore` now rebuild one
  exact display-source map after ordered input and use it for registered paint,
  visual keyboard movement, pointer selection, caret affinity, mixed-bidi
  selection, preedit underline/caret, viewport reveal, and native IME geometry.
  Pointer hits remain frozen to entry geometry while each horizontal key
  resolves the current post-mutation source. Read-only shares shaped navigation
  and copy without mutation or native IME; active preedit consumes horizontal
  model movement. Existing shaped struct literals, byte-only geometry APIs,
  free components, and construction without a retained layout store remain
  compatibility paths.
- Bounded text-field-local undo and redo to 128 combined snapshots and 4 MiB
  of retained UTF-8 snapshot text. Canonical ordered hardware insertion,
  unmodified Backspace, and unmodified Delete without active composition now
  coalesce contiguous runs in inclusive 4096-byte units without cloning a
  full-buffer snapshot for every fragment. Public direct edits, modified or
  active-preedit deletion, paste/cut, IME commits, selection replacement, word
  deletion, and multiline Enter remain atomic. Deterministic oldest/farthest
  eviction preserves nearest traversal targets; states over the byte limit form
  explicit one-way history barriers rather than allowing discontinuous undo
  jumps. No public text-editing API changed.
- Bounded retained shaped text layouts and the compatibility measurement cache
  to 32 MiB of checked owned key/layout payload with deterministic eviction
  after 120 idle frame generations. Added fallible retained admission,
  transient shaping, explicit held-ID touches, payload/generation metrics, and
  an incarnation-aware fixed 256 KiB dirty-ID journal for later incremental
  renderer reconciliation. Canonical field entry and event navigation now shape
  transiently so only final geometry is retained and rejected scrub previews do
  not churn the cache. Existing infallible admission and caller-owned layout
  handles remain compatible; strict saturation degrades canonical new text to
  layoutless fallback.
- Added per-renderer `TextLayoutResourceSync` reconciliation with deterministic
  full/reset reports, dirty-ID final-presence updates, text-only removal, and
  checked reachable-payload metrics. Sync state is deliberately non-clonable
  and caller-owned so independent registries cannot reuse a cursor without its
  matching resource state. No-change frames clone no text keys and mutate no
  resource maps. `UiState` now provides an additive reconciliation helper, and
  the showcase retains one resource registry, registers static media once, and
  incrementally reconciles text after each completed frame instead of cloning
  and rebuilding resources on access. Existing manual/full-snapshot
  registration APIs remain compatible.
- Made resolved `TextLayoutResource` layouts the sole Vello shaping and glyph-
  topology authority. Exact positive axis-aligned transforms now project each
  absolute logical glyph position through the full f64 affine, round once in
  f64, and only then narrow it to f32 while preserving exact scaled font size
  and non-uniform outline ratio. At identity command transforms, encoded glyph
  anchors and corresponding caret/selection edges now have strict positional
  parity at 1.25, 1.5, and 1.75 scale; the existing generic-rectangle band of
  at most 1.0001 physical pixels remains limited to fractional command
  translations. Every skewed, rotated, reflected, negative, or otherwise
  general affine stays on the raw transform path without hinting. Layoutless
  and missing-resource compatibility paint now uses a private logical-key
  `TextLayoutStore` with the accepted 32 MiB and 120-idle-generation bounds
  instead of a scale-keyed entry-count cache.
- Defined renderer-bound `Color` as straight sRGB plus straight alpha and made
  Vello translation diagnose and sanitize every invalid color occurrence before
  command snapshots. Peniko gradients now explicitly select sRGB with
  premultiplied-alpha interpolation. CPU image RGB bytes are documented as
  sRGB, and premultiplied image tint now applies tint alpha to RGB with one
  exact integer rounding. Public render-resource and snapshot APIs are
  unchanged.

### Documentation

- Distinguished current source-path use from future registry installation.

### Internal

- Recorded dependency-aware package verification and the eight-crate publish
  order.
