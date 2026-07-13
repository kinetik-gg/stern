# Provisional Public API Policy

This document freezes an inventory, not the final alpha API. The current
`kinetik-ui` facade and default prelude are **provisional Experimental** during
the alpha-readiness campaign. Prelude inclusion is a convenience decision and
never implies Stable conformance. Candidate-for-alpha-stable is a separate
product decision from conformance status.

REND-04A: **Complete / Accepted**; REND-04B: **next**; integrated REND-04 remains **Current / Authorized**.

No tag, package publication, deployment, release, or alpha-readiness claim is made by REND-04A.

The application-facing native texture API is qualified-only; none of its symbols are exported by `kinetik_ui::prelude`.

- `kinetik_ui::vello_winit::VelloNativeTextureRegistration`
- `kinetik_ui::vello_winit::VelloNativeTextureUpdateOutcome`
- `kinetik_ui::vello_winit::VelloNativeTextureValidationError`
- `kinetik_ui::vello_winit::VelloWindowPresenter::register_native_texture`
- `kinetik_ui::vello_winit::VelloWindowPresenter::update_native_texture`
- `kinetik_ui::vello_winit::VelloWindowPresenter::replace_native_texture`
- `kinetik_ui::vello_winit::VelloWindowPresenter::remove_native_texture`
- `kinetik_ui::vello_winit::VelloPresenterError::{NativeTextureAlreadyRegistered, NativeTextureNotRegistered, StaleNativeTextureRegistration, NativeTextureRevisionRegressed, InvalidNativeTexture, NativeTextureGenerationExhausted}`
- `kinetik_ui::vello_winit::VelloNativeTextureValidationError::{ResourceIdMismatch, ZeroExtent, NonIntegralResourceExtent, ResourceExtentMismatch, UnsupportedFormat, MissingCopySourceUsage, UnsupportedDimension, UnsupportedArrayLayers, UnsupportedMipLevels, UnsupportedSampleCount}`
- `kinetik_ui::render_vello::VelloNativeTextureRegistry`
- `kinetik_ui::render_vello::VelloNativeTextureScope`
- `kinetik_ui::render_vello::VelloNativeTextureScope::new`
- `kinetik_ui::render_vello::VelloNativeTextureRegistry::{new, begin_native_texture_update, stage_native_texture, dirty_native_texture, replace_native_texture_image, retire_native_texture, commit_native_texture, clear_native_textures}`
- `kinetik_ui::render_vello::VelloRenderer::submit_frame_with_native_textures`

An API may be classified Stable only after accepted behavioral evidence proves
every capability axis required by that API: Model, Paint, Input,
Accessibility, Platform, and Live Workflow as applicable. Metadata-only or
fixture-only evidence proves none of those axes. Planned APIs never enter the
default prelude.

Final facade and prelude curation is gated on `SHOW-02`, the coherent editor
workflow built only from public APIs. Stage 1 keeps existing compatibility
paths without deprecation and makes no export additions, removals, or moves.

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

Every symbol currently re-exported from `kinetik_ui::prelude` appears below.
The classification and promotion decisions apply to the whole named group;
individual members may require narrower or additional evidence at final
curation.

| Group | Current classification | Canonical path for new code | `SHOW-02` candidacy | Promotion prerequisite |
| --- | --- | --- | --- | --- |
| Facade state | Provisional Experimental | `kinetik_ui::UiState` or `kinetik_ui::prelude::UiState` | Yes: retained application state | Public frame lifecycle, shaped-text resource registration, and presenter workflow proof |
| Core | Provisional Experimental | `kinetik_ui::core`; current common subset is also in `kinetik_ui::prelude` | Yes: runtime, input, actions, semantics, theme, and primitives | Accepted behavioral evidence for every required axis, including action/input and semantic-output proof |
| Text | Provisional Experimental | `kinetik_ui::text`; current common subset is also in `kinetik_ui::prelude` | Yes: desktop text editing and shaped layout storage | Text editing, shaping, clipboard/IME, lifecycle, and public workflow proof required by the selected types |
| Render | Provisional Experimental | `kinetik_ui::render`; current common subset is also in `kinetik_ui::prelude` | Yes: backend-independent frame and resource contract | Presenter, resource-lifetime, diagnostic, and external-texture workflow proof |
| Winit | Provisional Experimental, feature-gated | `kinetik_ui::platform_winit` | Yes: supported window and platform loop | Winit input, IME/clipboard/cursor/platform-request, accessibility boundary, and redraw-loop proof; no new prelude exports before presenter proof |
| Vello | Provisional Experimental, feature-gated | `kinetik_ui::render_vello` | Yes: supported 2D backend and presenter | Surface acquisition/recovery, resize/scale, presentation, diagnostics, and Vello-backed workflow proof; no new prelude exports before presenter proof |
| Vello/Winit presenter | Provisional Experimental, feature-gated, qualified only | `kinetik_ui::vello_winit` | Yes: supported one-window live presenter | REND-04A is Complete / Accepted; REND-04B is next for real-GPU evidence; access remains qualified only, and no presenter item enters the prelude |
| Widgets | Provisional Experimental | Common composition path: `kinetik_ui::widgets`; advanced APIs use the qualified modules listed below | Yes: controls and viewport surface; exact final subset deferred | Public paint/input/accessibility/platform/live-workflow evidence for each selected component |

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

These qualified exports are present at `kinetik_ui::vello_winit` when the
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
use kinetik_ui::widgets::{
    asset_browser, chrome, collection_actions, collections, dock, inline_edit,
    inspector, node_graph, outliner, overlays, taxonomy, timeline, viewport,
};
```

Existing root compatibility items under `kinetik_ui::widgets`, such as
`kinetik_ui::widgets::Dock`, remain source-compatible in Stage 1 but are
noncanonical for new documentation and examples. Their canonical advanced
form is `kinetik_ui::widgets::dock::Dock`, and the same module-qualified rule
applies to every module above. `node_graph` and `timeline` remain Experimental
and deferred from the supported alpha vertical slice unless later behavioral
evidence changes that decision.

Platform and renderer implementations stay qualified under
`kinetik_ui::platform_winit`, `kinetik_ui::render_vello`, and
`kinetik_ui::vello_winit`. Existing platform/Vello prelude compatibility
exports remain intact, while the presenter stays qualified-only until the
Stage 7 API decision.

Later stages may add new qualified APIs when a subsystem needs them. Any
prelude expansion requires an explicit policy decision plus accepted evidence;
final narrowing waits for `SHOW-02`.

## Final-Review Ledger

The following contracts remain compatible and Experimental during Stage 1.
This ledger prevents their presence from being mistaken for endorsement and
records the evidence needed for the Stage 7 API decision.

| Contract under review | Stage 1 canonical guidance | Evidence required before final action |
| --- | --- | --- |
| `text::TextLayoutCache` versus shaped `text::TextLayoutStore` | Use `TextLayoutStore` for retained shaped layouts and renderer resources. `TextLayoutCache` remains a module-qualified approximate measurement compatibility API. | `TEXT-01` through `TEXT-03`, renderer resource lifetime, and `SHOW-02` determine deprecation/removal and migration wording. |
| Legacy viewport `Guide`, `Crosshair`, and `ViewportComposition` helpers versus surface/descriptor paths | Keep legacy helpers compatible but noncanonical. New work starts with `widgets::viewport::ViewportSurface` and the relevant `ViewportGuideDescriptor`, `ViewportOverlayDescriptor`, or `ViewportToolSurfaceDescriptor`. | Viewport composition, external texture, pointer transform, painter, and public workflow proof determine the final retained set. |
| Legacy `Theme` scalar fields versus token groups | New work uses `Theme::radii`, `Theme::controls`, and `Theme::typography`; `radius`, `border_width`, and `text_size` remain compatible. | Complete theme-token migration and representative component paint proof precede deprecation or removal. |
| Dock `PanelId` versus `PanelInstanceId` | New instance-oriented APIs use `widgets::dock::PanelInstanceId`; the convertible legacy `PanelId` remains compatible. | Dock interaction, persistence round-trip, and `SHOW-02` establish whether a migration can be enforced. |
| `ActionContext`, `ActionPriority`, and `ActionRoutingContext` | Keep all three compatible and provisional; do not claim that their current overlap is final. | Action-routing, input precedence, modal/text reservation, and `SHOW-02` behavioral proof must establish one non-contradictory public model. |

No item in this ledger is deprecated by this policy. Migration notes, if any,
belong to final `API-01` after `SHOW-02`, not to the Stage 1 inventory checkpoint.
