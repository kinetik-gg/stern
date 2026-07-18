# Provisional Public API Policy

This document freezes an inventory, not the final alpha API. The current
`stern` facade and default prelude are **provisional Experimental** during
pre-alpha development. Prelude inclusion is a convenience decision and
never implies Stable conformance. Candidate-for-alpha-stable is a separate
product decision from conformance status.

The public editor workflow now exercises the facade, presenter, Dock, viewport,
inspector, outliner, asset browser, and system-feedback surfaces. Final facade
and prelude curation remains open; no tag, package publication, deployment,
release, or alpha-readiness claim follows from the current implementation.

The application-facing native texture API is qualified-only; none of its symbols are exported by `stern::prelude`.

- `stern::vello_winit::VelloNativeTextureRegistration`
- `stern::vello_winit::VelloNativeTextureUpdateOutcome`
- `stern::vello_winit::VelloNativeTextureValidationError`
- `stern::vello_winit::VelloWindowPresenter::register_native_texture`
- `stern::vello_winit::VelloWindowPresenter::update_native_texture`
- `stern::vello_winit::VelloWindowPresenter::replace_native_texture`
- `stern::vello_winit::VelloWindowPresenter::remove_native_texture`
- `stern::vello_winit::VelloPresenterError::{NativeTextureAlreadyRegistered, NativeTextureNotRegistered, StaleNativeTextureRegistration, NativeTextureRevisionRegressed, InvalidNativeTexture, NativeTextureGenerationExhausted}`
- `stern::vello_winit::VelloNativeTextureValidationError::{ResourceIdMismatch, ZeroExtent, NonIntegralResourceExtent, ResourceExtentMismatch, UnsupportedFormat, MissingCopySourceUsage, UnsupportedDimension, UnsupportedArrayLayers, UnsupportedMipLevels, UnsupportedSampleCount}`
- `stern::render_vello::VelloNativeTextureRegistry`
- `stern::render_vello::VelloNativeTextureScope`
- `stern::render_vello::VelloNativeTextureScope::new`
- `stern::render_vello::VelloNativeTextureRegistry::{new, begin_native_texture_update, stage_native_texture, dirty_native_texture, replace_native_texture_image, retire_native_texture, commit_native_texture, clear_native_textures}`
- `stern::render_vello::VelloRenderer::submit_frame_with_native_textures`

An API may be classified Stable only after accepted behavioral evidence proves
every capability axis required by that API: Model, Paint, Input,
Accessibility, Platform, and Live Workflow as applicable. Metadata-only or
fixture-only evidence proves none of those axes. Planned APIs never enter the
default prelude.

Final facade and prelude curation is gated on the coherent editor workflow and
the remaining release evidence. Existing compatibility paths remain
undeprecated until that review makes an explicit migration decision.

## Exact Facade Root

The facade root is exactly: `UiState`, `core`, `text`, `render`, `widgets`,
`platform_winit`, `render_vello`, `vello_winit`, `prelude`.

`platform_winit` is available with the `platform-winit` feature and
`render_vello` is available with the `render-vello` feature. The concrete
`vello_winit` presenter is available through the composite `vello-winit`
feature, which enables both lower features and is enabled by default today.
The modules and `UiState` are provisional
Experimental until their applicable capability axes are proven through the
public vertical slice.

## Exact Default Prelude

Every symbol currently re-exported from `stern::prelude` appears below.
The classification and promotion decisions apply to the whole named group;
individual members may require narrower or additional evidence at final
curation.

### Qualified semantic theme palette

The grouped semantic palette remains available through the qualified
`stern::core` module without expanding the default prelude. New application
code starts from `stern::core::ThemeColors::default_dark()`, mutates public
fields on the non-exhaustive color groups, and applies the result with the
existing `Theme::with_colors` boundary. `SemanticColor::ALL` inventories the
53 exact resolver keys. See
[Semantic Theme Color Migration](theme-color-migration.md) for the deliberate
prerelease breaking field and variant cutover.

### Qualified bundled monospace authority

The qualified text API now exposes Space Mono as its exact bundled monospace
authority. `stern::text::DEFAULT_MONOSPACE_FONT_FAMILY` is `"Space Mono"`, and
`stern::text::fonts::{SPACE_MONO_UPSTREAM_COMMIT, SPACE_MONO_REGULAR}` expose
the pinned revision and bytes. The prerelease change removed
`fonts::{GEIST_UPSTREAM_COMMIT, GEIST_MONO_VARIABLE}` without aliases. None of
these qualified items is added to the default prelude.

Focused text evidence proves that named `"Space Mono"`, the public default,
generic `"monospace"`, and `"mono"` resolve through the same bundled bytes,
while the default theme Mono family matches the public text default. This
makes `STERN-TYP-000` Partial for deterministic Mono text-system alignment and
`STERN-TYP-006` Partial for exact asset/license provenance. No typography
requirement is Accepted, and this policy does not claim platform fallback,
renderer/browser output, GPU/manual review, or any other deferred capability.

### Qualified bundled Brand authority

The qualified text API exposes the exact bundled Space Grotesk variable face
through
`stern::text::fonts::{SPACE_GROTESK_UPSTREAM_COMMIT, SPACE_GROTESK_VARIABLE}`.
The revision is `03507d024a01282884232081fc6011c09ff4e849`; the public bytes
are the pinned `136,676`-byte upstream `fonts/ttf/SpaceGrotesk[wght].ttf`
asset with SHA-256
`ACAD6DE1FC93436F5C0F1F4137751EF04F1AEA3063E7036535970FFCFBD79F72`.
These qualified items are not added to the default prelude.

The default theme's `FontFamilyRole::Brand` result can be passed into public
`TextStyle`, and the default text engine resolves that `"Space Grotesk"` name
through the public bundled bytes. There is no Brand default alias, Brand
`TextRole`, Title remapping, or fallback-stack authority. Qualified callers may
use the low-level weight transport below without assigning Brand a semantic
role. Existing Inter and Space Mono named/default/generic resolution is
preserved.

This advances only deterministic Brand text-system byte alignment for
`STERN-TYP-000` and exact asset/license provenance for `STERN-TYP-006`; both
remain Partial. `STERN-TYP-001` and `STERN-TYP-003` are preserved without
advancing, `STERN-TYP-002`, `STERN-TYP-004`, `STERN-TYP-005`, and
`STERN-TYP-007` do not advance, all typography parity records remain
unverified, and nothing is Accepted. Measured geometry may change with the
family, but no baseline, overflow, widget-adoption, renderer, browser, GPU, or
manual visual evidence follows from this loading boundary.

### Qualified variable-font weight transport

The qualified text API exposes exact low-level weight transport through public
`TextStyle::weight` and `TextStyle::with_weight(u16)`. Constructors remain
Regular `400`. Callers resolve semantic values through the existing
`FontWeightScale`/`FontWeightToken` authority and pass the resulting number;
there is no second semantic enum or scale and no default-prelude expansion.

Weight participates in style/key equality and hashing, deterministic cache
ordering, retained IDs, and renderer-resource identity. Shaping passes the raw
request directly to Cosmic Text. Public `ShapedGlyphRun::normalized_coords`
then records the selected face's full renderer-ready 2.14 coordinate vector in
axis order. Raw out-of-range requests keep distinct key identity even when the
selected face maps them to the same endpoint. Static Space Mono retains exact
bundled bytes and an empty vector. Checked store and renderer accounting include
owned coordinate capacity, and both Vello transform paths consume the exact
slice without synthetic emboldening or fallback reconstruction.

Adding both public fields is a prerelease breaking struct-shape change;
constructors remain source-compatible and exactly equivalent to explicit
weight `400`. `FontToken`, `TextRole`, and the public `TextPrimitive` shape are
unchanged. The canonical retained property-grid section path below is the sole
semantic component weight adopter; layoutless/generic text remains Regular
`400`. Deterministic CPU evidence covers Inter, Space Grotesk, Space Mono,
Unicode/bidi/multiline, features, end ellipsis, resource reconciliation, the
retained property-grid section, and registered Vello scales `1.0`, `1.25`,
`1.5`, and `2.0`.
This is unindexed foundation transport evidence only: all numbered typography
requirements preserve their prior disposition, every parity record remains
unverified, and nothing becomes Accepted. No fallback, optical, raster, DPI,
browser, GPU, platform-font, manual, or visual claim is made.

### Qualified tabular-number shaping

The qualified text API exposes `stern::text::TextFeatureSet` as an opaque
fixed-size low-level shaping authority. Its only public values are `NONE` and
`TABULAR_NUMBERS`; the latter maps to OpenType `tnum=1` in the production text
engine. `stern::text::TextStyle::new(...)` remains feature-disabled, while
`TextStyle::with_features(...)` provides explicit opt-in. None of these items
is added to the default prelude.

`TextFeatureSet::resolve_semantic(FontFeatureScale, FontFeatureToken)` is the
qualified bridge from semantic theme authority to the bounded low-level set.
Only `FontFeatureToken::Numeric` with the exact `"tabular-nums"` value resolves
to `TABULAR_NUMBERS`; unsupported customized values return `None`. The method
does not expose arbitrary OpenType tags.

Adding public `TextStyle::features` is a prerelease breaking struct-shape
change. Feature identity is retained through `TextLayoutKey`, cache/store
lookup, layout IDs, and renderer text resources through their existing
composed style fields. `TextFeatureSet` does not expose arbitrary feature tags
or a generic registry; weight and selected-font coordinates use the separate
qualified transport above.

The default theme's existing `FontFeatureScale` remains the sole semantic
token authority: `FontFeatureToken::Numeric` resolves to `"tabular-nums"`.
`TextFeatureSet::TABULAR_NUMBERS` is only the low-level mechanism selected by
the qualified resolver; it is not a second token value.

Focused deterministic evidence proves that exact bundled Inter has unequal
default numeric advances, then produces equal enabled `0-9` advances and
equal widths for equivalent-length changing numeric strings within `0.001`
logical unit. It also proves preserved UTF-8 ranges, layout topology, family
bytes, bounded cache/store behavior, distinct retained IDs, and renderer
resource reconciliation. Canonical retained `Ui` numeric inputs, numeric
scrubs, and vector numeric subfields now use that resolver and preserve one
feature-bearing style through hit/navigation geometry, final shaping,
resource reconciliation, and registered Vello encoding at deterministic
`1.0`, `1.25`, `1.5`, and `2.0` scales. Generic text and vector axis labels
remain `NONE`.

This prerelease widget rendering-behavior change is breaking without changing
public widget signatures. Numeric measurements, caret positions, and derived
snapshots can change. This advances `STERN-TYP-002` only to stronger bounded
Partial. Direct/layoutless compatibility helpers, timelines, frame counters,
timecodes, and tables do not consume the feature, so the requirement is not
Accepted.
`STERN-TYP-000` and `STERN-TYP-006` preserve their existing Partial evidence;
`STERN-TYP-001` and `STERN-TYP-003` are preserved only;
`STERN-TYP-004`, `STERN-TYP-005`, and `STERN-TYP-007` do not advance. All
typography parity records remain unverified. This evidence makes no component,
fallback, failed-load, truncation, optical-baseline, overflow, non-Latin, IME,
DPI, renderer-pixel, browser, GPU, manual, visual, release, or acceptance
claim beyond the canonical retained numeric path.

### Qualified retained end ellipsis

The qualified `stern::text` API exposes
`TextOverflow::{Visible, EndEllipsis}`, `TextLayoutKey::with_overflow`,
`ShapedGlyph::elided`, `ShapedTextLayout::is_elided`, and
`TextNavigationError::ElidedLayout`. These symbols are not added to the default
prelude. `TextLayoutKey::new(...)` remains source-compatible and defaults to
`Visible`; public struct literals must add `overflow: TextOverflow::Visible`.
Public `ShapedGlyph` literals must add `elided: false`, and exhaustive error
matches must handle the new navigation variant.

`EndEllipsis` is a display-only retained layout policy for finite positive
width, nonwrapping, single-line requests. Pinned cosmic-text performs the end
elision with a one-line limit. The byte-exact source is never replaced: it and
the explicit overflow policy remain in `TextLayoutKey`, retained cache/store
identity, renderer resources, and reconciliation. Only the shaped presentation
may hide glyphs. The generated ellipsis glyph carries an empty source range at
an extended-grapheme seam and is distinguished from a literal source
`U+2026` by `elided`.

An elided layout rejects navigation explicitly because hidden graphemes cannot
provide complete byte-accurate caret and selection geometry. Full-fit and
visible layouts remain navigable. Invalid widths, wrapping requests, and
multiline sources preserve existing visible or wrapping behavior.

Deterministic evidence covers LTR, RTL, combining-mark, and emoji seams;
distinct stable retained identities and hot-frame accounting; complete-source
renderer reconciliation; and registered Vello glyph topology at `1.0`, `1.25`,
`1.5`, and `2.0` without fallback cache activity. It does not prove raster
pixels or visual acceptance.

Canonical retained `Ui::select_field` now applies the policy to complete
selected values and placeholders at the exact text rectangle while preserving
complete primitive and semantic text and a separate disclosure affordance.
This adoption adds no public API: the existing `Ui::select_field` signature is
unchanged, its retained helper is crate-private, the public low-level
`select_field(...)` remains layoutless, and no root, prelude, or facade export
is added.

Canonical retained `Ui::property_grid` now applies the same policy only to
`PropertyGridRowKind::Property` main labels at the exact existing label inset
and fixed help/status reservation. Complete required presentation text and
undecorated semantic text remain distinct; help/status glyphs retain their
existing generic visible/layoutless paths.

The same canonical method now resolves section primitive family, size, and
line height from `TextRole::Title` and requests the existing Semibold token only
for a strictly admitted complete-source `Visible` retained layout. It adds no
weight to `TextPrimitive`: no-store and rejected generic/layoutless fallback
remains Regular `400`. Ordinary labels retain their existing Label metrics,
Regular request, and `EndEllipsis` policy. This bounded adoption adds no public
API: the existing `Ui::property_grid` signature,
`PropertyGridConfig`, row/layout models, callbacks, output, access, intents,
semantics, exports, and qualified facade remain unchanged. No public overflow
configuration, copied-value API, tooltip API, helper, alias, prelude item,
`TextPrimitive` field, or renderer command is added, and generic retained-text
attachment keeps its existing fallback contract.

Canonical retained `Ui::button` now applies the policy only to the final
complete-source label primitive at
`(rect.width - theme.controls.padding_x * 2.0_f32).max(0.0_f32)`. Existing
`Ui::action_button` delegates to that path unchanged. This adoption adds no
public API: both method signatures, `ActionDescriptor`, `Response`, standalone
public `button(...)`, theme and spacing surfaces, exports, qualified modules,
facade root, and default prelude remain unchanged. The standalone component
stays layoutless. No public overflow configuration, helper, alias, copied-value
or tooltip API, descriptor presentation field, `TextPrimitive` field, renderer
command, toolbar/menu/split/busy/disclosure/icon-button behavior, or generic
attachment change is introduced. Hidden, disabled, pointer, keyboard, action
source/context/order, cursor, repaint, focus, semantic, and geometry behavior
remain the existing contracts; action evidence is regression-only.

Canonical retained `Ui::chrome_scene` now applies the policy only to final
complete-source toolbar-row label primitives at the final overflow-projected
span `(row.rect.width - theme.controls.padding_x * 2.0_f32).max(0.0_f32)`.
This adoption adds no public API: `Ui::chrome_scene`, `ChromeSceneConfig`,
toolbar/action/chrome/overflow models, `ActionDescriptor`, response and output
types, constructors, fields, methods, exports, qualified modules, facade root,
and default prelude remain unchanged. It adds no public overflow configuration,
copied-value or tooltip API, helper, alias, theme or spacing surface,
`TextPrimitive` field, renderer command, or generic attachment change.

Complete action label source remains in the existing descriptor, primitive,
retained key, renderer resource, and semantic label. Hidden and overflowed
actions register no toolbar label; the overflow trigger and menu, tab,
tab-close, status, overlay, command-palette, and system-feedback text remain
generic Visible/layoutless consumers. Projection, stable action identity and
order, row/surface geometry, focus, semantics, interaction states, and exact
button-source invocation routing remain existing contracts. Retained layout IDs
remain presentation-cache identity and may be shared by equal
source/style/effective-width actions. Toolbar and chrome evidence is
regression-only; no toolbar or chrome requirement advances.

Canonical retained `Ui::virtual_table` now applies the policy only to final
complete-source body-cell label primitives at the exact prepared-cell span
`(rect.width - theme.controls.padding_x * 2.0_f32).max(0.0_f32)`. This adoption
adds no public API: `Ui::virtual_table`, `VirtualTableConfig`, `TableColumn`,
`VirtualTableRow`, selection/output models, constructors, fields, methods,
exports, qualified modules, facade root, and default prelude remain unchanged.
It adds no public column overflow configuration, copied-value or editing API,
tooltip, helper, alias, theme or spacing surface, `TextPrimitive` field, or
renderer command. Header text and sort arrows remain generic Visible/layoutless
consumers, and generic retained attachment remains the only fallback authority.

Stable row/column/cell identities, both selection and navigation modes, focus
annuli, sort/resize outputs, two-axis scroll transforms, bounded
materialization, callbacks, semantics, and complete caller-owned cell labels
remain application-owned existing contracts. Retained layout IDs are
presentation-cache identity only and may be shared by equal
source/style/effective-width cells. Table-family evidence is regression-only;
there is no claim for copied values, editing, column-configurable overflow,
header overflow, auto-sizing, pinning, or numeric tabular shaping.

This advances only `STERN-TYP-004` to stronger bounded Partial. Component
evidence covers selection and placeholder states, property-label state and
fixed-column topology, standard and delegated action-button states, retained
chrome-toolbar labels, virtual-table body cells, exact retained identity and
rejection, and registered Vello topology.
`STERN-DEN-004` advances only to bounded Partial for finite-positive computed
property-label, button-label, toolbar-label, and prepared body-cell spans;
nonpositive spans make no endpoint or non-overlap claim. Existing Partial
evidence for `STERN-TYP-000`, `STERN-TYP-002`, and `STERN-TYP-006` is preserved,
while `STERN-TYP-001` and `STERN-TYP-003` do not advance. `STERN-DEN-003`,
`STERN-STA-001` through `STERN-STA-007`, button action routing, toolbar/chrome
behavior, and table-family requirements are regression-only; no `STERN-ACT-*`,
`STERN-TOOLBAR-001` through `STERN-TOOLBAR-006`, `STERN-CHROME-001`,
`STERN-CHROME-004`, `STERN-CHROME-005`, or `STERN-TBL-*` requirement advances.
This makes no claim about copied values,
tooltips, editable selection, other truncating components,
start/middle/multiline ellipsis, baseline behavior, browser output, GPU output,
or manual review. `STERN-TYP-005`, `STERN-TYP-007`,
`STERN-INSPECT-001`, `STERN-PROP-001`, `STERN-TIP-001`, `STERN-TIP-002`, and
`STERN-OVERLAY-COMP-002` do not advance. No typography parity record is
Accepted.

### Qualified size foundation

The grouped size foundation remains available through `stern::core` without
expanding the default prelude. New code customizes `stern::core::Theme::sizes`
through `Theme::with_sizes` and uses the typed `SizeToken` inventory for
lookup. `Theme::sizes.icon.md` is the only default and invalid-explicit
fallback authority for icon-button visual geometry; the prerelease migration
removed `ControlMetrics::icon_size` without an alias or mirror. Valid explicit
`*_sized` icon APIs remain authoritative. Checkbox and radio visible indicator
geometry instead uses one private exact `14.0` component-recipe dimension,
exposed through the unchanged `CheckRecipe::size` result. The prerelease
migration removed `ControlMetrics::check_size` without a token, alias, mirror,
or replacement customization hook; `Theme::radio_button` continues to inherit
the checkbox recipe size. Caller-owned rectangles and full-label interaction
and semantic bounds remain unchanged. See
[Exact Size Foundation Migration](size-migration.md).

### Qualified collection context snapshots and Escape focus return

Selection context targets own sorted, deduplicated item IDs, and collection
action metadata and requests retain the target acquired when a menu opens.
Bounded public headless evidence from issue #740/PR #743 separately proves
Asset Browser and Outliner retain stable target and selection snapshots through
command display and invocation when live selection changes after opening.
That evidence does not prove command-close focus restoration.
This advances only `STERN-CONTEXT-001` to bounded Partial; Candidate remains
Candidate.

The two new Asset Browser and Outliner tests in issue #746 alone prove that
Escape dismissal clears the retained context, restores focus to the exact
selected invoking item or row without implicit selection mutation: cursor and
selected item IDs remain byte-for-byte unchanged; no context request or frame
action is emitted; a following frame is requested; the context menu then
remains closed and selection-stable on that idle frame. The issue #746
tests alone advance only the
covered Asset Browser and Outliner portion of `STERN-CONTEXT-003` to bounded
Partial. Candidate remains Candidate.

Public `Key::ContextMenu` is a prerelease breaking enum addition; exhaustive
`Key` matches must handle it or add a wildcard. The two Asset Browser and
Outliner tests in issue #750 prove that a real focused selection converges on
the same target, ordered enabled/disabled command inventory, enabled command
request, and application action through secondary click, the Menu key, and
unmodified `Shift+F10`. Released Menu keys, unfocused collections, and disabled
collections fail closed. This advances only the covered portion of
`STERN-CONTEXT-002` to bounded Partial; Candidate remains Candidate.

Outside-click dismissal, target or owner destruction, focus loss, dynamic
command removal, other consumers, generic `MenuOverlay` focus return,
command-owned selection mutation, and context invocation beyond the Asset
Browser and Outliner remain unverified. Platform/native/browser/raster/GPU/
Vello/manual/visual evidence also remains unverified. `STERN-MENU-003` does not
advance, and nothing is Accepted.

### Qualified shortcut presentation

The additive `ShortcutPlatform`, `ShortcutModifier`, `ShortcutLabelToken`,
`ShortcutLabelLocalizer`, and `EnglishShortcutLabels` types are qualified
Experimental core API. `Shortcut::localized_label` and
`Shortcut::english_label` return owned presentation text without changing the
existing routing shape or storing a display, platform, or locale field. These
items are reachable through `stern::core` and deliberately absent from
`stern::prelude`.

The additive qualified Experimental widget entrypoint is
`stern_widgets::Ui::overlay_scene_with_menu_presentation`, also reachable as
`stern::widgets::Ui::overlay_scene_with_menu_presentation`. It borrows an
explicit platform and caller-owned localizer for one evaluation and calls
`Shortcut::localized_label`; neither input nor formatted text is stored. The
existing `Ui::overlay_scene` signature and label-only output remain unchanged,
and no shortcut-presentation type enters the prelude.

At post-inset row width `272.0` or greater, menu actions and section labels use
`8.0` outer padding on each side and reserve state `16.0`, gap `8.0`, icon
`16.0`, gap `8.0`, flexible label, gap `8.0`, status `16.0`, gap `8.0`,
shortcut `112.0`, gap `8.0`, and disclosure `16.0`. The threshold leaves a
`40.0` label slot. Default inset `4.0` requires surface width `280.0`; surface
width `272.0` yields a `264.0` row. Every narrower row, including the preceding
`f32` value, uses the legacy path without calling the caller-owned localizer.
Labels and present shortcut strings receive exact independent clips; submenu
rows paint one `›`. State, icon, and status slots stay reserved and unpainted.

Full-row responses and targets, stable identities, semantics, focus,
navigation, descriptor values, action source/context, and FIFO routing are
unchanged. `STERN-MENU-COMP-002` and `STERN-MENU-001` gain only bounded Partial
evidence; shortcut dispositions stay bounded Partial. Unverified areas are
active-platform and locale discovery, non-English translation quality,
sequential chords, check/radio/icon/status/destructive painting, mixed state,
right-to-left layout,
narrow-menu columns, work-area fitting and scrolling, platform-menu entry,
context-menu convergence, browser/raster/GPU output, manual review, and
reference-image equivalence. Maturity does not advance beyond bounded Partial.

Current core evidence remains bounded Partial for explicit Windows, macOS, and
Linux policy selection, the deterministic English fallback, caller-owned
localizer behavior, fail-closed tokens, and routing noninterference. The widget
entrypoint does not change that maturity.

| Group | Current classification | Canonical path for new code | Public workflow use | Promotion prerequisite |
| --- | --- | --- | --- | --- |
| Facade state | Provisional Experimental | `stern::UiState` or `stern::prelude::UiState` | Yes: retained application state | Public frame lifecycle, shaped-text resource registration, and presenter workflow proof |
| Core | Provisional Experimental | `stern::core`; current common subset is also in `stern::prelude` | Yes: runtime, input, actions, semantics, theme, and primitives | Accepted behavioral evidence for every required axis, including action/input and semantic-output proof |
| Text | Provisional Experimental | `stern::text`; current common subset is also in `stern::prelude` | Yes: desktop text editing and shaped layout storage | Text editing, shaping, clipboard/IME, lifecycle, and public workflow proof required by the selected types |
| Render | Provisional Experimental | `stern::render`; current common subset is also in `stern::prelude` | Yes: backend-independent frame and resource contract | Presenter, resource-lifetime, diagnostic, and external-texture workflow proof |
| Winit | Provisional Experimental, feature-gated | `stern::platform_winit` | Yes: supported window and platform loop | Winit input, IME/clipboard/cursor/platform-request, accessibility boundary, and redraw-loop proof; no new prelude exports before presenter proof |
| Vello | Provisional Experimental, feature-gated | `stern::render_vello` | Yes: supported 2D backend and presenter | Surface acquisition/recovery, resize/scale, presentation, diagnostics, and Vello-backed workflow proof; no new prelude exports before presenter proof |
| Vello/Winit presenter | Provisional Experimental, feature-gated, qualified only | `stern::vello_winit` | Yes: supported one-window live presenter | Real-GPU and packaged-example evidence exists; access remains qualified only, and no presenter item enters the prelude |
| Widgets | Provisional Experimental | Common composition path: `stern::widgets`; advanced APIs use the qualified modules listed below | Yes: controls and viewport surface; exact final subset deferred | Measured containers and qualified painted overlay, chrome, and collection seams are implemented; no prelude promotion, and public live-workflow evidence remains required |

### Facade state inventory

- `UiState`

### Core inventory

- `AccessibilityAdapter`
- `AccessibilityNode`
- `AccessibilitySnapshot`
- `ActionContext`
- `ActionDescriptor`
- `ActionIcon`
- `ActionId`
- `ActionInvocation`
- `ActionPriority`
- `ActionQueue`
- `ActionRouter`
- `ActionRoutingContext`
- `ActionSource`
- `ActionState`
- `EnglishShortcutLabels`
- `Brush`
- `Color`
- `CursorShape`
- `FrameContext`
- `FrameOutput`
- `FrameWarning`
- `IconId`
- `ImageId`
- `Key`
- `Modifiers`
- `ShortcutLabelLocalizer`
- `ShortcutLabelToken`
- `ShortcutModifier`
- `ShortcutPlatform`
- `PathElement`
- `PathPrimitive`
- `PhysicalSize`
- `PlatformRequest`
- `Point`
- `Primitive`
- `Rect`
- `RepaintRequest`
- `ScaleFactor`
- `SemanticTreeError`
- `Shortcut`
- `Size`
- `TextureId`
- `Theme`
- `TimeInfo`
- `UiInput`
- `UiMemory`
- `Vec2`
- `ViewportInfo`
- `WidgetId`
- `default_dark_theme`

### Text inventory

- `TextEditState`
- `TextLayoutStore`

### Render inventory

- `ImageResource`
- `RenderDiagnostic`
- `RenderFrameInput`
- `RenderFrameOutput`
- `RenderImage`
- `RenderImageAlpha`
- `RenderImageFormat`
- `RenderImageSampling`
- `RenderResources`
- `RendererBackend`
- `TextLayoutResource`
- `TextureResource`

### Winit inventory

These exports are present when the `platform-winit` feature is enabled:

- `WinitAccessibilityUpdate`
- `WinitFrameClock`
- `WinitInputAdapter`
- `WinitPlatformRequests`
- `frame_context_from_winit`
- `viewport_from_winit`

### Vello inventory

These exports are present when the `render-vello` feature is enabled:

- `VelloRenderer`
- `translate_primitives`

### Vello/Winit presenter inventory

These qualified exports are present at `stern::vello_winit` when the
`vello-winit` feature is enabled. None is re-exported from the prelude:

- `VelloWindowPresenter`
- `VelloPresenterConfig`
- `PresenterDeviceScope`
- `PresenterDevice`
- `VelloPresenterStatus`
- `VelloAttachmentStatus`
- `VelloAttachOutcome`
- `VelloSuspendOutcome`
- `VelloResizeOutcome`
- `VelloRecoveryKind`
- `VelloRecoveryOutcome`
- `VelloPresentReport`
- `VelloPresentStatus`
- `VelloRedrawGuidance`
- `VelloPresenterError`
- `PresenterGpuError`
- `PresenterGpuErrorKind`
- `InvalidColorChannel`
- `AaConfig`
- `wgpu`

### Widgets inventory

- `IconGraphic`
- `IconLibrary`
- `IconPath`
- `Ui`
- `ViewportSurface`

## Canonical Advanced Widget Imports

New documentation and examples use these qualified modules:

```rust
use stern::widgets::{
    asset_browser, chrome, collection_actions, collections, dock, inline_edit,
    inspector, node_graph, outliner, overlays, taxonomy, timeline, viewport,
};
```

Existing root compatibility items under `stern::widgets`, such as
`stern::widgets::Dock`, remain source-compatible but are
noncanonical for new documentation and examples. Their canonical advanced
form is `stern::widgets::dock::Dock`, and the same module-qualified rule
applies to every module above. `node_graph` and `timeline` remain Experimental
and deferred from the supported alpha vertical slice unless later behavioral
evidence changes that decision.

Platform and renderer implementations stay qualified under
`stern::platform_winit`, `stern::render_vello`, and
`stern::vello_winit`. Existing platform/Vello prelude compatibility
exports remain intact, while the presenter stays qualified-only until the
final API review.

Future changes may add new qualified APIs when a subsystem needs them. Any
prelude expansion requires an explicit policy decision plus accepted evidence;
final narrowing waits for the complete public-workflow and release evidence.

The qualified `stern::widgets::overlays` API provides scene,
navigation, paint, input, action-intent, and semantic contract. It does not add
overlay symbols to the default prelude. Menu-bar trigger/overflow painting was
delivered by the public editor chrome and is exercised by the Showcase.

The qualified `stern::widgets::chrome` API provides overflow and
scene contracts plus their public `Ui` paint/input/semantic integration. It
does not add chrome symbols to the default prelude.

The qualified `stern::widgets::collections` API provides navigation,
virtual-list, virtual-tree, and virtual-table contracts plus public `Ui`
paint/input/semantic integration. These symbols remain outside the default
prelude pending final API review.

## Final-Review Ledger

The following contracts remain compatible and Experimental.
This ledger prevents their presence from being mistaken for endorsement and
records the evidence needed for the final API decision.

| Contract under review | Current canonical guidance | Evidence required before final action |
| --- | --- | --- |
| `text::TextLayoutCache` versus shaped `text::TextLayoutStore` | Use `TextLayoutStore` for retained shaped layouts and renderer resources. `TextLayoutCache` remains a module-qualified approximate measurement compatibility API. | Desktop text behavior, renderer resource lifetime, and public workflow evidence determine deprecation/removal and migration wording. |
| Legacy viewport `Guide`, `Crosshair`, and `ViewportComposition` helpers versus surface/descriptor paths | Keep legacy helpers compatible but noncanonical. New work starts with `widgets::viewport::ViewportSurface` and the relevant `ViewportGuideDescriptor`, `ViewportOverlayDescriptor`, or `ViewportToolSurfaceDescriptor`. | Viewport composition, external texture, pointer transform, painter, and public workflow proof determine the final retained set. |
| Legacy `Theme` scalar fields versus token groups | New work uses grouped token surfaces including `Theme::radii`, `Theme::strokes`, `Theme::sizes`, `Theme::controls`, and `Theme::typography`. Typography stores semantic UI, Brand, and Mono family authority separately from per-role logical metrics, plus exact customizable size, line-height, weight, and feature foundation scales. Qualified foundation lookup is `theme.typography.<scale>.get(token)`; `Theme::font` remains the resolved compatibility boundary and `Theme::font_family` exposes typed family lookup. Title remains UI and Brand has no current `TextRole`. Foundation weight metadata does not expand `FontToken`; qualified callers may pass its exact value into low-level `TextStyle::with_weight`, which transports selected-font coordinates through retained shaping and rendering, with one bounded canonical retained property-grid section adoption. The numeric feature resolves through the qualified low-level `TextFeatureSet` bridge for canonical retained numeric fields without changing `FontToken` or primitives; generic components remain feature-disabled. Default icon geometry uses `Theme::sizes.icon.md`, while checkbox and radio recipes resolve their private exact `14.0` indicator dimension. Removed `ControlMetrics::{icon_size, check_size}` fields have no compatibility aliases. `radius`, `border_width`, and `text_size` remain compatible. | Complete theme-token migration and representative component paint proof precede deprecation or removal. Current typography evidence proves deterministic theme authority, bounded Space Mono and Space Grotesk asset loading with exact byte alignment for Mono and Brand, exact low-level selected-font weight-coordinate transport, bounded bundled-Inter `tnum=1` shaping with retained identity, canonical retained numeric input/scrub/vector adoption, and canonical retained property-grid section Semibold transport through registered Vello glyphs. Direct/layoutless helpers, additional semantic weight adoption, and other specified numeric consumers remain unverified; no evidence is claimed for fallback, glyph-metric suitability, DPI legibility, renderer pixels, or visual review. Current selection-indicator evidence covers direct visual geometry and full-label bounds only; it does not establish mixed-state mark anatomy or renderer baselines. |
| Dock `PanelId` versus `PanelInstanceId` | New instance-oriented APIs use `widgets::dock::PanelInstanceId`; the convertible legacy `PanelId` remains compatible. | Dock interaction, persistence round-trip, and public workflow evidence establish whether a migration can be enforced. |
| `ActionContext`, `ActionPriority`, and `ActionRoutingContext` | Keep all three compatible and provisional; do not claim that their current overlap is final. | Action-routing, input precedence, modal/text reservation, and public workflow evidence must establish one non-contradictory public model. |

No item in this ledger is deprecated by this policy. Migration notes, if any,
belong to the final API review, not to this inventory.
