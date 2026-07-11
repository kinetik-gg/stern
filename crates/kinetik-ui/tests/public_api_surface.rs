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

fn captured_domain_drag_method(
    ui: &mut kinetik_ui::core::Ui<'_>,
    id: kinetik_ui::core::WidgetId,
    rect: kinetik_ui::core::Rect,
    disabled: bool,
) -> kinetik_ui::core::CapturedDomainDragGesture {
    ui.captured_domain_drag_gesture(id, rect, disabled)
}

fn captured_domain_drag_action(
    action: &kinetik_ui::core::DomainDragGestureAction,
) -> (
    Option<usize>,
    kinetik_ui::core::DomainDragGesturePhase,
    Option<kinetik_ui::core::Point>,
    kinetik_ui::core::Vec2,
    u8,
    kinetik_ui::core::Modifiers,
    bool,
) {
    (
        action.ordinal,
        action.phase,
        action.position,
        action.delta,
        action.click_count,
        action.modifiers,
        action.release_clicked,
    )
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
        std::any::type_name::<core::CapturedDomainDragGesture>(),
        std::any::type_name::<core::CapturedSelectionGesture>(),
        std::any::type_name::<core::DomainDragGestureAction>(),
        std::any::type_name::<core::DomainDragGesturePhase>(),
        std::any::type_name::<core::LivenessIncarnation>(),
        std::any::type_name::<core::LivenessRegistry>(),
        std::any::type_name::<core::LivenessRemovalStatus>(),
        std::any::type_name::<core::LivenessToken>(),
        std::any::type_name::<core::LivenessUpdateStatus>(),
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
    let _ = captured_domain_drag_method;
    let _ = captured_domain_drag_action;
    let phases = [
        core::DomainDragGesturePhase::Press,
        core::DomainDragGesturePhase::Move,
        core::DomainDragGesturePhase::Release,
        core::DomainDragGesturePhase::Cancel,
    ];
    assert_eq!(phases.len(), 4);
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
fn canonical_liveness_incarnation_surface_compiles_and_reports_typed_statuses() {
    use kinetik_ui::core::{
        LivenessRegistry, LivenessRemovalStatus, LivenessTargetId, LivenessUpdateStatus, WidgetId,
    };

    let widget = WidgetId::from_key("preview");
    let target = LivenessTargetId::new(widget);
    let mut registry = LivenessRegistry::new();
    let first = registry.mark_present(target);
    assert_eq!(first.target(), target);
    assert!(registry.is_present(target));
    assert!(registry.is_active(target));
    assert_eq!(
        registry.current_incarnation(target),
        Some(first.incarnation())
    );

    assert!(matches!(
        registry.cancel(first),
        LivenessUpdateStatus::Cancelled {
            target: cancelled_target,
            incarnation,
        } if cancelled_target == target && incarnation == first.incarnation()
    ));

    let replacement = registry.restart(target);
    assert!(matches!(
        registry.validate(first),
        LivenessUpdateStatus::StaleIncarnation {
            target: stale_target,
            token_incarnation,
            current_incarnation,
        } if stale_target == target
            && token_incarnation == first.incarnation()
            && current_incarnation == replacement.incarnation()
    ));
    assert_eq!(registry.remove(target), LivenessRemovalStatus::Removed);
    assert_eq!(
        registry.remove(target),
        LivenessRemovalStatus::AlreadyAbsent
    );
}

#[allow(deprecated)]
#[test]
fn deprecated_liveness_generation_aliases_remain_importable() {
    use kinetik_ui::core::{LivenessGeneration, LivenessRegistry, WidgetId};

    let target = WidgetId::from_key("compatibility");
    let mut registry = LivenessRegistry::new();
    let token = registry.mark_live(target);
    let generation: LivenessGeneration = token.generation();

    assert!(registry.is_live(target));
    assert_eq!(registry.current_generation(target), Some(generation));
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
fn additive_text_caret_api_is_qualified_and_legacy_offsets_remain_compatible() {
    use kinetik_ui::text::{TextAffinity, TextCaret, TextEditState};

    let mut state = TextEditState::new("ab");
    state.set_caret(1);
    assert_eq!(state.caret(), 1);

    state.set_caret_position(TextCaret::new(1, TextAffinity::Before));
    assert_eq!(
        state.caret_position(),
        TextCaret::new(1, TextAffinity::Before)
    );
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
