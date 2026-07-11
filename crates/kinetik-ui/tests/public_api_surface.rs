//! Consumer-view compile checks for the provisional facade and prelude.

mod current_prelude_inventory {
    #![allow(unused_imports)]

    use kinetik_ui::prelude::{
        AccessibilityAdapter, AccessibilityNode, AccessibilitySnapshot, ActionContext,
        ActionDescriptor, ActionIcon, ActionId, ActionInvocation, ActionPriority, ActionQueue,
        ActionRouter, ActionRoutingContext, ActionSource, ActionState, Brush, Color, CursorShape,
        FrameContext, FrameOutput, FrameWarning, IconGraphic, IconId, IconLibrary, IconPath,
        ImageId, ImageResource, Key, Modifiers, PathElement, PathPrimitive, PhysicalSize,
        PlatformRequest, Point, Primitive, Rect, RenderDiagnostic, RenderFrameInput,
        RenderFrameOutput, RenderImage, RenderImageAlpha, RenderImageFormat, RenderImageSampling,
        RenderResources, RendererBackend, RepaintRequest, ScaleFactor, SemanticTreeError, Shortcut,
        Size, TextEditState, TextLayoutResource, TextLayoutStore, TextureId, TextureResource,
        Theme, TimeInfo, Ui, UiInput, UiMemory, UiState, Vec2, ViewportInfo, ViewportSurface,
        WidgetId, default_dark_theme,
    };

    #[cfg(feature = "platform-winit")]
    use kinetik_ui::prelude::{
        WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        frame_context_from_winit, viewport_from_winit,
    };

    #[cfg(feature = "render-vello")]
    use kinetik_ui::prelude::{VelloRenderer, translate_primitives};

    #[test]
    fn every_current_prelude_export_is_importable() {
        // Resolving the imports above is the assertion. Referencing one item keeps this
        // dedicated inventory test meaningful without assigning behavior to the prelude.
        let _ = std::any::type_name::<UiState>();
    }
}

fn captured_selection_method(
    ui: &mut kinetik_ui::core::Ui<'_>,
    id: kinetik_ui::core::WidgetId,
    rect: kinetik_ui::core::Rect,
    disabled: bool,
) -> kinetik_ui::core::CapturedSelectionGesture {
    ui.captured_selection_gesture(id, rect, disabled)
}

fn captured_selection_modifiers(
    action: &kinetik_ui::core::SelectionGestureAction,
) -> kinetik_ui::core::Modifiers {
    action.modifiers
}

fn ordered_text_input_method(
    ui: &mut kinetik_ui::core::Ui<'_>,
    id: kinetik_ui::core::WidgetId,
) -> Result<
    Option<Vec<kinetik_ui::core::OrderedTextInputEvent>>,
    kinetik_ui::core::InputStreamConflict,
> {
    ui.claim_ordered_text_input_events(id)
}

#[test]
fn facade_root_and_feature_qualified_paths_compile() {
    use kinetik_ui::{UiState, core, render, text, widgets};

    let _ = UiState::new();
    let paths = [
        std::any::type_name::<core::UiInput>(),
        std::any::type_name::<core::CapturedSelectionGesture>(),
        std::any::type_name::<core::OrderedTextInputEvent>(),
        std::any::type_name::<core::SelectionGestureAction>(),
        std::any::type_name::<core::SelectionGesturePhase>(),
        std::any::type_name::<text::TextLayoutStore>(),
        std::any::type_name::<render::RenderResources>(),
        std::any::type_name::<widgets::Ui<'static>>(),
    ];
    assert!(paths.iter().all(|path| !path.is_empty()));

    let _ = captured_selection_method;
    let _ = captured_selection_modifiers;
    let _ = ordered_text_input_method;

    #[cfg(feature = "platform-winit")]
    {
        let _ = kinetik_ui::platform_winit::WinitFrameClock::new();
        let _ = std::any::type_name::<kinetik_ui::platform_winit::WinitInputAdapter>();
        let qualified = [
            std::any::type_name::<kinetik_ui::platform_winit::WinitPlatformRequests>(),
            std::any::type_name::<kinetik_ui::platform_winit::WinitShellRequests>(),
            std::any::type_name::<kinetik_ui::platform_winit::WinitShellOutcome>(),
            std::any::type_name::<kinetik_ui::platform_winit::WinitRepaintScheduler>(),
            std::any::type_name::<kinetik_ui::platform_winit::NativeWinitShellServices>(),
            std::any::type_name::<dyn kinetik_ui::platform_winit::WinitShellServices>(),
        ];
        assert!(qualified.iter().all(|path| !path.is_empty()));
    }

    #[cfg(feature = "render-vello")]
    {
        let _ = kinetik_ui::render_vello::VelloRenderer::new();
        let _ = kinetik_ui::render_vello::translate_primitives;
    }
}

#[test]
fn canonical_advanced_widget_modules_compile() {
    use kinetik_ui::widgets::{
        asset_browser, chrome, collection_actions, collections, dock, inline_edit, inspector,
        node_graph, outliner, overlays, taxonomy, timeline, viewport,
    };

    let canonical_paths = [
        std::any::type_name::<asset_browser::AssetBrowserModel>(),
        std::any::type_name::<chrome::Toolbar>(),
        std::any::type_name::<collection_actions::CollectionContextTarget>(),
        std::any::type_name::<collections::Selection>(),
        std::any::type_name::<dock::Dock>(),
        std::any::type_name::<inline_edit::InlineEditSession>(),
        std::any::type_name::<inspector::PropertyGridLayout>(),
        std::any::type_name::<node_graph::NodeGraphDescriptor>(),
        std::any::type_name::<outliner::OutlinerModel>(),
        std::any::type_name::<overlays::OverlayStack>(),
        std::any::type_name::<taxonomy::ComponentConformanceStatus>(),
        std::any::type_name::<timeline::TimelineDescriptor>(),
        std::any::type_name::<viewport::ViewportSurface>(),
    ];

    assert!(canonical_paths.iter().all(|path| !path.is_empty()));
}

#[test]
fn root_widget_compatibility_exports_remain_source_compatible() {
    use kinetik_ui::widgets::{
        self, asset_browser, chrome, collection_actions, collections, dock, inline_edit, inspector,
        node_graph, outliner, overlays, taxonomy, timeline, viewport,
    };

    fn same_type<T>(_: Option<T>, _: Option<T>) {}

    same_type(
        None::<asset_browser::AssetBrowserModel>,
        None::<widgets::AssetBrowserModel>,
    );
    same_type(None::<chrome::Toolbar>, None::<widgets::Toolbar>);
    same_type(
        None::<collection_actions::CollectionContextTarget>,
        None::<widgets::CollectionContextTarget>,
    );
    same_type(None::<collections::Selection>, None::<widgets::Selection>);
    same_type(None::<dock::Dock>, None::<widgets::Dock>);
    same_type(
        None::<inline_edit::InlineEditSession>,
        None::<widgets::InlineEditSession>,
    );
    same_type(
        None::<inspector::PropertyGridLayout>,
        None::<widgets::PropertyGridLayout>,
    );
    same_type(
        None::<node_graph::NodeGraphDescriptor>,
        None::<widgets::NodeGraphDescriptor>,
    );
    same_type(
        None::<outliner::OutlinerModel>,
        None::<widgets::OutlinerModel>,
    );
    same_type(
        None::<overlays::OverlayStack>,
        None::<widgets::OverlayStack>,
    );
    same_type(
        None::<taxonomy::ComponentConformanceStatus>,
        None::<widgets::ComponentConformanceStatus>,
    );
    same_type(
        None::<timeline::TimelineDescriptor>,
        None::<widgets::TimelineDescriptor>,
    );
    same_type(
        None::<viewport::ViewportSurface>,
        None::<widgets::ViewportSurface>,
    );
}

#[test]
fn legacy_and_duplicate_contracts_remain_importable_for_stage_one() {
    use kinetik_ui::{core, text, widgets};

    let contract_paths = [
        std::any::type_name::<text::TextLayoutCache>(),
        std::any::type_name::<text::TextLayoutStore>(),
        std::any::type_name::<widgets::viewport::Guide>(),
        std::any::type_name::<widgets::viewport::Crosshair>(),
        std::any::type_name::<widgets::viewport::ViewportComposition>(),
        std::any::type_name::<widgets::viewport::ViewportSurface>(),
        std::any::type_name::<widgets::viewport::ViewportGuideDescriptor>(),
        std::any::type_name::<widgets::dock::PanelId>(),
        std::any::type_name::<widgets::dock::PanelInstanceId>(),
        std::any::type_name::<core::ActionContext>(),
        std::any::type_name::<core::ActionPriority>(),
        std::any::type_name::<core::ActionRoutingContext>(),
    ];
    assert!(contract_paths.iter().all(|path| !path.is_empty()));

    let theme = core::default_dark_theme();
    std::hint::black_box((theme.radius, theme.border_width, theme.text_size));
    std::hint::black_box((theme.radii, theme.controls, theme.typography));

    let legacy_panel = widgets::dock::PanelId::from_raw(7);
    let panel_instance: widgets::dock::PanelInstanceId = legacy_panel.into();
    assert_eq!(panel_instance.raw(), 7);
}
