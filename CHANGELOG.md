# Changelog

All notable changes to Kinetik UI will be documented in this file. Categories
follow the repository release policy.

## [Unreleased]

`0.1.0-alpha.1` is the planned first prerelease. It has not been dated, tagged,
published, or accepted as an alpha release.

### Changed

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
  move/extend methods reject stale maps transactionally and leave text,
  composition, and local undo/redo untouched. Existing shaped struct literals
  and byte-only geometry APIs remain compatible; retained widget wiring remains
  the following serialized text packet.
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

- Recorded dependency-aware package verification and the seven-crate publish
  order.
