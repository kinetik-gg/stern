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
| Legacy `Theme` scalar fields versus token groups | New work uses grouped token surfaces including `Theme::radii`, `Theme::strokes`, `Theme::sizes`, `Theme::controls`, and `Theme::typography`. Typography stores semantic UI, Brand, and Mono family authority separately from per-role logical metrics, plus exact customizable size, line-height, weight, and feature foundation scales. Qualified foundation lookup is `theme.typography.<scale>.get(token)`; `Theme::font` remains the resolved compatibility boundary and `Theme::font_family` exposes typed family lookup. Title remains UI and Brand has no current `TextRole`. Foundation weight and feature metadata does not expand `FontToken` or text/render transport. Default icon geometry uses `Theme::sizes.icon.md`, while checkbox and radio recipes resolve their private exact `14.0` indicator dimension. Removed `ControlMetrics::{icon_size, check_size}` fields have no compatibility aliases. `radius`, `border_width`, and `text_size` remain compatible. | Complete theme-token migration and representative component paint proof precede deprecation or removal. Current typography evidence proves deterministic theme authority plus bounded Space Mono asset loading and named/default/generic byte alignment. The numeric `"tabular-nums"` value does not prove consumer adoption or shaped tabular figures; no evidence is claimed for fallback, glyph-metric suitability, DPI legibility, renderer output, or visual review. Current selection-indicator evidence covers direct visual geometry and full-label bounds only; it does not establish mixed-state mark anatomy or renderer baselines. |
| Dock `PanelId` versus `PanelInstanceId` | New instance-oriented APIs use `widgets::dock::PanelInstanceId`; the convertible legacy `PanelId` remains compatible. | Dock interaction, persistence round-trip, and public workflow evidence establish whether a migration can be enforced. |
| `ActionContext`, `ActionPriority`, and `ActionRoutingContext` | Keep all three compatible and provisional; do not claim that their current overlap is final. | Action-routing, input precedence, modal/text reservation, and public workflow evidence must establish one non-contradictory public model. |

No item in this ledger is deprecated by this policy. Migration notes, if any,
belong to the final API review, not to this inventory.
