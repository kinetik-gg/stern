//! Application-facing facade for the Kinetik UI toolkit.
//!
//! This crate re-exports the current application stack so apps can start from a
//! single dependency while the implementation remains split across core,
//! widget, text, platform, and renderer crates. During the alpha-readiness
//! campaign, this facade and its [`prelude`] are provisional Experimental
//! surfaces: an item being reachable here is a convenience decision, not a
//! Stable conformance claim. Stable status requires accepted behavioral proof
//! for every capability axis required by that API.
//!
//! Advanced editor APIs should be imported through their qualified modules.
//! The exact provisional policy and compatibility rules are recorded in the
//! repository's `docs/public-api-policy.md`; final facade curation is gated on
//! the `SHOW-02` public-API editor workflow.

/// Platform-independent runtime, input, layout, actions, semantics, theme, and render primitives.
pub mod core {
    pub use kinetik_ui_core::*;
}

/// Text editing, shaping, and text layout cache types.
pub mod text {
    pub use kinetik_ui_text::*;
}

/// Backend-independent renderer contract and resource types.
pub mod render {
    pub use kinetik_ui_render::*;
}

/// Reusable widgets, editor models, overlays, docking, collections, and viewport helpers.
pub mod widgets {
    pub use kinetik_ui_widgets::*;
}

/// Winit platform adapter.
#[cfg(feature = "platform-winit")]
pub mod platform_winit {
    pub use kinetik_ui_winit::*;
}

/// Vello renderer boundary.
#[cfg(feature = "render-vello")]
pub mod render_vello {
    pub use kinetik_ui_vello::*;
}

/// Concrete Vello/Winit window presenter.
#[cfg(feature = "vello-winit")]
pub mod vello_winit {
    pub use kinetik_ui_vello_winit::*;
}

/// Retained application-facing toolkit state.
///
/// This owns the core UI memory and shaped text layout cache that most apps
/// should keep across frames. Applications still own domain state and action
/// execution; this state only stores toolkit/runtime concerns.
pub struct UiState {
    memory: core::UiMemory,
    text_layouts: text::TextLayoutStore,
    icons: widgets::IconLibrary,
}

impl UiState {
    /// Creates empty retained toolkit state.
    #[must_use]
    pub fn new() -> Self {
        Self {
            memory: core::UiMemory::new(),
            text_layouts: text::TextLayoutStore::new(),
            icons: widgets::IconLibrary::new(),
        }
    }

    /// Returns retained UI memory.
    #[must_use]
    pub const fn memory(&self) -> &core::UiMemory {
        &self.memory
    }

    /// Returns mutable retained UI memory.
    pub fn memory_mut(&mut self) -> &mut core::UiMemory {
        &mut self.memory
    }

    /// Returns the shaped text layout cache.
    #[must_use]
    pub const fn text_layouts(&self) -> &text::TextLayoutStore {
        &self.text_layouts
    }

    /// Returns mutable access to the shaped text layout cache.
    pub fn text_layouts_mut(&mut self) -> &mut text::TextLayoutStore {
        &mut self.text_layouts
    }

    /// Returns the vector icon library.
    #[must_use]
    pub const fn icons(&self) -> &widgets::IconLibrary {
        &self.icons
    }

    /// Returns mutable access to the vector icon library.
    pub fn icons_mut(&mut self) -> &mut widgets::IconLibrary {
        &mut self.icons
    }

    /// Clears cached shaped text layouts.
    pub fn clear_text_layouts(&mut self) {
        self.text_layouts.clear();
    }

    /// Starts a widget frame with shaped text layout caching enabled.
    #[must_use]
    pub fn begin_frame<'a>(
        &'a mut self,
        context: core::FrameContext,
        theme: &'a core::Theme,
    ) -> widgets::Ui<'a> {
        let Self {
            memory,
            text_layouts,
            icons,
        } = self;
        widgets::Ui::begin_frame_with_text_layouts(context, memory, theme, text_layouts)
            .with_icons(icons)
    }

    /// Registers cached shaped text layouts into renderer resources.
    pub fn register_text_layouts(&self, resources: &mut render::RenderResources) {
        resources.register_text_layouts(self.text_layouts.layouts());
    }

    /// Reconciles retained text layouts into one caller-owned resource registry.
    ///
    /// The sync state must remain paired with the exact registry passed here.
    /// Separate renderer consumers use independent registries and sync states.
    pub fn reconcile_text_layouts(
        &self,
        resources: &mut render::RenderResources,
        sync: &mut render::TextLayoutResourceSync,
    ) -> render::TextLayoutResourceSyncReport {
        resources.reconcile_text_layouts(&self.text_layouts, sync)
    }

    /// Creates renderer resources containing the cached shaped text layouts.
    #[must_use]
    pub fn text_render_resources(&self) -> render::RenderResources {
        let mut resources = render::RenderResources::new();
        self.register_text_layouts(&mut resources);
        resources
    }
}

impl Default for UiState {
    fn default() -> Self {
        Self::new()
    }
}

/// Provisional common imports for application UI code.
///
/// Prelude inclusion does not imply Stable conformance. This current surface is
/// classified Experimental until its required capability axes have accepted
/// behavioral proof. Planned APIs do not enter this prelude, and advanced
/// editor models should be imported from their qualified modules. The final
/// alpha prelude will be curated only after the `SHOW-02` public-API vertical
/// slice proves which imports form a coherent application path.
pub mod prelude {
    pub use crate::UiState;
    pub use crate::core::{
        AccessibilityAdapter, AccessibilityNode, AccessibilitySnapshot, ActionContext,
        ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionPriority, ActionQueue,
        ActionRouter, ActionRoutingContext, ActionSource, ActionState, Brush, Color, CursorShape,
        FrameContext, FrameOutput, FrameWarning, IconId, ImageId, Key, Modifiers, PathElement,
        PathPrimitive, PhysicalSize, PlatformRequest, Point, Primitive, Rect, RepaintRequest,
        ScaleFactor, SemanticTreeError, Shortcut, Size, TextureId, Theme, TimeInfo, UiInput,
        UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
    };
    #[cfg(feature = "platform-winit")]
    pub use crate::platform_winit::{
        WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        frame_context_from_winit, viewport_from_winit,
    };
    pub use crate::render::{
        ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput, RenderImage,
        RenderImageAlpha, RenderImageFormat, RenderImageSampling, RenderResources, RendererBackend,
        TextLayoutResource, TextureResource,
    };
    #[cfg(feature = "render-vello")]
    pub use crate::render_vello::{VelloRenderer, translate_primitives};
    pub use crate::text::{TextEditState, TextLayoutStore};
    pub use crate::widgets::{IconGraphic, IconLibrary, IconPath, Ui, ViewportSurface};
}

#[cfg(test)]
mod tests {
    use super::{UiState, core, prelude, render, text, widgets};

    #[test]
    fn facade_reexports_core_text_and_widgets() {
        let _input = core::UiInput::default();
        let _state = text::TextEditState::new("hello");
        let _panel = widgets::Panel::new(widgets::PanelId::from_raw(1), "Panel");
    }

    #[test]
    fn facade_reexports_backend_neutral_render_contract() {
        let resources = prelude::RenderResources::new();
        let _input = prelude::RenderFrameInput {
            viewport: prelude::ViewportInfo::new(
                prelude::Size::new(10.0, 10.0),
                prelude::PhysicalSize::new(10, 10),
                prelude::ScaleFactor::ONE,
            ),
            primitives: &[],
            resources: &resources,
        };

        assert!(!resources.has_image(prelude::ImageId::from_raw(1)));
    }

    #[test]
    fn facade_prelude_exposes_runtime_warning_types() {
        let warning = prelude::FrameWarning::InvalidSemanticTree {
            error: prelude::SemanticTreeError::MissingRoot,
        };

        assert!(matches!(
            warning,
            prelude::FrameWarning::InvalidSemanticTree {
                error: prelude::SemanticTreeError::MissingRoot
            }
        ));
    }

    #[test]
    fn facade_prelude_exposes_accessibility_snapshot_types() {
        let snapshot = prelude::AccessibilitySnapshot::default();
        let _node: Option<&prelude::AccessibilityNode> =
            snapshot.node(prelude::WidgetId::from_key("missing"));

        assert!(snapshot.nodes.is_empty());
    }

    #[test]
    fn facade_prelude_exposes_icon_customization_types() {
        let mut icons = prelude::IconLibrary::new();
        let icon = prelude::IconId::from_raw(1);
        icons.register(
            icon,
            prelude::IconGraphic::new(
                prelude::Rect::new(0.0, 0.0, 24.0, 24.0),
                [prelude::IconPath::stroked(
                    vec![
                        prelude::PathElement::MoveTo(prelude::Point::new(5.0, 12.0)),
                        prelude::PathElement::LineTo(prelude::Point::new(10.0, 17.0)),
                        prelude::PathElement::LineTo(prelude::Point::new(19.0, 7.0)),
                    ],
                    2.0,
                )],
            ),
        );

        assert!(icons.has_icon(icon));
    }

    #[test]
    fn facade_widgets_module_exposes_widget_prelude() {
        let theme = prelude::default_dark_theme();
        let output =
            widgets::prelude::label(prelude::Rect::new(0.0, 0.0, 80.0, 18.0), "Hello", &theme);

        assert_eq!(output.primitives.len(), 1);
    }

    #[cfg(feature = "render-vello")]
    #[test]
    fn facade_prelude_drives_vello_through_renderer_contract() {
        let resources = prelude::RenderResources::new();
        let mut renderer = prelude::VelloRenderer::new();

        let output = prelude::RendererBackend::render_frame(
            &mut renderer,
            prelude::RenderFrameInput {
                viewport: prelude::ViewportInfo::new(
                    prelude::Size::new(10.0, 10.0),
                    prelude::PhysicalSize::new(10, 10),
                    prelude::ScaleFactor::ONE,
                ),
                primitives: &[],
                resources: &resources,
            },
        )
        .expect("Vello frame submission is infallible before GPU presentation");

        assert_eq!(output.primitive_count, 0);
        assert!(output.diagnostics.is_empty());
    }

    #[test]
    fn prelude_contains_the_common_application_stack() {
        let theme = prelude::default_dark_theme();
        let viewport = prelude::ViewportInfo::new(
            prelude::Size::new(800.0, 600.0),
            prelude::PhysicalSize::new(1600, 1200),
            prelude::ScaleFactor::new(2.0),
        );
        let context = prelude::FrameContext::new(
            viewport,
            prelude::UiInput::default(),
            prelude::TimeInfo::default(),
        );
        let mut memory = prelude::UiMemory::new();
        let mut ui = prelude::Ui::begin_frame(context, &mut memory, &theme);

        ui.label(prelude::Rect::new(0.0, 0.0, 80.0, 18.0), "Hello");
        let output = ui.finish_output();

        assert_eq!(output.primitives.len(), 1);
    }

    #[test]
    fn ui_state_starts_text_layout_enabled_frames() {
        let theme = prelude::default_dark_theme();
        let viewport = prelude::ViewportInfo::new(
            prelude::Size::new(800.0, 600.0),
            prelude::PhysicalSize::new(1600, 1200),
            prelude::ScaleFactor::new(2.0),
        );
        let context = prelude::FrameContext::new(
            viewport,
            prelude::UiInput::default(),
            prelude::TimeInfo::default(),
        );
        let mut state = UiState::new();
        let mut ui = state.begin_frame(context, &theme);

        ui.label(prelude::Rect::new(0.0, 0.0, 80.0, 18.0), "Hello");
        let output = ui.finish_output();

        assert!(matches!(
            output.primitives.first(),
            Some(prelude::Primitive::Text(text)) if text.layout.is_some()
        ));
        assert_eq!(state.text_layouts().len(), 1);
    }

    #[cfg(feature = "platform-winit")]
    #[test]
    fn facade_reexports_winit_platform_feature() {
        let mut clock = prelude::WinitFrameClock::new();
        let time = clock.tick(std::time::Duration::from_millis(16));
        let output = prelude::FrameOutput::new();
        let update = prelude::WinitAccessibilityUpdate::from_frame_output(&output, None)
            .expect("empty semantic tree is valid");

        assert_eq!(time.frame_index, 0);
        assert!(update.snapshot.nodes.is_empty());
    }

    #[cfg(feature = "render-vello")]
    #[test]
    fn facade_reexports_vello_renderer_feature() {
        let resources = prelude::RenderResources::new();
        let translation = prelude::translate_primitives(&[], &resources);

        assert!(translation.commands.is_empty());
    }

    #[test]
    fn ui_state_exports_text_layouts_to_renderer_resources() {
        let theme = prelude::default_dark_theme();
        let viewport = prelude::ViewportInfo::new(
            prelude::Size::new(800.0, 600.0),
            prelude::PhysicalSize::new(1600, 1200),
            prelude::ScaleFactor::new(2.0),
        );
        let context = prelude::FrameContext::new(
            viewport,
            prelude::UiInput::default(),
            prelude::TimeInfo::default(),
        );
        let mut state = UiState::new();
        let mut ui = state.begin_frame(context, &theme);
        ui.label(prelude::Rect::new(0.0, 0.0, 80.0, 18.0), "Hello");
        let output = ui.finish_output();
        let layout = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                prelude::Primitive::Text(text) => text.layout,
                _ => None,
            })
            .expect("text layout handle");

        let resources = state.text_render_resources();

        assert!(resources.has_text_layout(layout));
    }

    #[test]
    fn ui_state_reconciles_independent_text_resource_consumers() {
        let mut state = UiState::new();
        let first = state.text_layouts_mut().layout_id(text::TextLayoutKey::new(
            "first",
            text::TextStyle::new("Inter", 12.0, 16.0),
            80.0,
            false,
        ));
        let mut first_resources = render::RenderResources::new();
        let mut second_resources = render::RenderResources::new();
        let mut first_sync = render::TextLayoutResourceSync::new();
        let mut second_sync = render::TextLayoutResourceSync::new();

        let first_report = state.reconcile_text_layouts(&mut first_resources, &mut first_sync);
        let second_report = state.reconcile_text_layouts(&mut second_resources, &mut second_sync);
        assert_eq!(first_report.kind, render::TextLayoutResourceSyncKind::Full);
        assert_eq!(second_report.kind, render::TextLayoutResourceSyncKind::Full);
        assert!(first_resources.has_text_layout(first));
        assert_eq!(first_resources.snapshot(), second_resources.snapshot());

        let second = state.text_layouts_mut().layout_id(text::TextLayoutKey::new(
            "second",
            text::TextStyle::new("Inter", 12.0, 16.0),
            80.0,
            false,
        ));
        assert_eq!(
            state
                .reconcile_text_layouts(&mut first_resources, &mut first_sync)
                .processed_changes,
            1
        );
        assert!(!second_resources.has_text_layout(second));
        assert_eq!(
            state
                .reconcile_text_layouts(&mut second_resources, &mut second_sync)
                .processed_changes,
            1
        );
        assert_eq!(first_resources.snapshot(), second_resources.snapshot());
    }

    #[test]
    fn ui_state_supplies_registered_icons_to_frames() {
        let theme = prelude::default_dark_theme();
        let viewport = prelude::ViewportInfo::new(
            prelude::Size::new(800.0, 600.0),
            prelude::PhysicalSize::new(1600, 1200),
            prelude::ScaleFactor::new(2.0),
        );
        let context = prelude::FrameContext::new(
            viewport,
            prelude::UiInput::default(),
            prelude::TimeInfo::default(),
        );
        let mut state = UiState::new();
        let icon = prelude::IconId::from_raw(7);
        state.icons_mut().register(
            icon,
            prelude::IconGraphic::new(
                prelude::Rect::new(0.0, 0.0, 24.0, 24.0),
                [prelude::IconPath::stroked(
                    vec![
                        prelude::PathElement::MoveTo(prelude::Point::new(5.0, 12.0)),
                        prelude::PathElement::LineTo(prelude::Point::new(10.0, 17.0)),
                        prelude::PathElement::LineTo(prelude::Point::new(19.0, 7.0)),
                    ],
                    2.0,
                )],
            ),
        );
        let mut ui = state.begin_frame(context, &theme);

        ui.icon_button(
            "apply",
            prelude::Rect::new(0.0, 0.0, 24.0, 24.0),
            icon,
            false,
        );
        let output = ui.finish_output();

        assert_eq!(output.primitives.len(), 2);
        assert!(matches!(output.primitives[1], prelude::Primitive::Path(_)));
    }
}
