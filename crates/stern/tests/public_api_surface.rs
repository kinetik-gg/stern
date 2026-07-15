//! Consumer-view compile checks for the provisional facade and prelude.

mod current_prelude_inventory {
    #![allow(unused_imports)]

    use stern::prelude::{
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
    use stern::prelude::{
        WinitAccessibilityUpdate, WinitFrameClock, WinitInputAdapter, WinitPlatformRequests,
        frame_context_from_winit, viewport_from_winit,
    };

    #[cfg(feature = "render-vello")]
    use stern::prelude::{VelloRenderer, translate_primitives};

    #[test]
    fn every_current_prelude_export_is_importable() {
        // Resolving the imports above is the assertion. Referencing one item keeps this
        // dedicated inventory test meaningful without assigning behavior to the prelude.
        let _ = std::any::type_name::<UiState>();
    }
}

fn captured_selection_method(
    ui: &mut stern::core::Ui<'_>,
    id: stern::core::WidgetId,
    rect: stern::core::Rect,
    disabled: bool,
) -> stern::core::CapturedSelectionGesture {
    ui.captured_selection_gesture(id, rect, disabled)
}

fn captured_selection_modifiers(
    action: &stern::core::SelectionGestureAction,
) -> stern::core::Modifiers {
    action.modifiers
}

fn captured_domain_drag_method(
    ui: &mut stern::core::Ui<'_>,
    id: stern::core::WidgetId,
    rect: stern::core::Rect,
    disabled: bool,
) -> stern::core::CapturedDomainDragGesture {
    ui.captured_domain_drag_gesture(id, rect, disabled)
}

fn captured_domain_drag_action(
    action: &stern::core::DomainDragGestureAction,
) -> (
    Option<usize>,
    stern::core::DomainDragGesturePhase,
    Option<stern::core::Point>,
    stern::core::Vec2,
    u8,
    stern::core::Modifiers,
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
    ui: &mut stern::core::Ui<'_>,
    id: stern::core::WidgetId,
) -> Result<Option<Vec<stern::core::OrderedTextInputEvent>>, stern::core::InputStreamConflict> {
    ui.claim_ordered_text_input_events(id)
}

#[test]
fn facade_root_and_feature_qualified_paths_compile() {
    use stern::{UiState, core, render, text, widgets};

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
        let _ = stern::platform_winit::WinitFrameClock::new();
        let _ = std::any::type_name::<stern::platform_winit::WinitInputAdapter>();
        let qualified = [
            std::any::type_name::<stern::platform_winit::WinitPlatformRequests>(),
            std::any::type_name::<stern::platform_winit::WinitShellRequests>(),
            std::any::type_name::<stern::platform_winit::WinitShellOutcome>(),
            std::any::type_name::<stern::platform_winit::WinitRepaintScheduler>(),
            std::any::type_name::<stern::platform_winit::NativeWinitShellServices>(),
            std::any::type_name::<dyn stern::platform_winit::WinitShellServices>(),
        ];
        assert!(qualified.iter().all(|path| !path.is_empty()));
    }

    #[cfg(feature = "render-vello")]
    {
        let _ = stern::render_vello::VelloRenderer::new();
        let _ = stern::render_vello::translate_primitives;
    }

    #[cfg(feature = "vello-winit")]
    {
        let presenter = stern::vello_winit::VelloWindowPresenter::new(
            stern::vello_winit::VelloPresenterConfig::new(),
        )
        .expect("detached construction is GPU-free");
        let qualified = [
            std::any::type_name::<stern::vello_winit::PresenterDeviceScope>(),
            std::any::type_name::<stern::vello_winit::VelloPresentReport>(),
            std::any::type_name::<stern::vello_winit::VelloRecoveryOutcome>(),
        ];
        assert!(qualified.iter().all(|path| !path.is_empty()));
        assert!(presenter.window().is_none());
    }
}

#[test]
fn facade_supports_mutation_first_semantic_palette_customization() {
    use stern::core::{
        AccentColors, BorderColors, Color, ContentColors, FocusColors, OverlayColors,
        SelectionColors, SemanticColor, StatusColorFamilyColors, StatusColors, SurfaceColors,
        ThemeColors, default_dark_theme,
    };

    let groups = [
        std::any::type_name::<SurfaceColors>(),
        std::any::type_name::<ContentColors>(),
        std::any::type_name::<BorderColors>(),
        std::any::type_name::<SelectionColors>(),
        std::any::type_name::<FocusColors>(),
        std::any::type_name::<OverlayColors>(),
        std::any::type_name::<AccentColors>(),
        std::any::type_name::<StatusColorFamilyColors>(),
        std::any::type_name::<StatusColors>(),
    ];
    assert!(groups.iter().all(|name| !name.is_empty()));
    assert_eq!(SemanticColor::ALL.len(), 53);

    let original = default_dark_theme();
    let mut colors = ThemeColors::default_dark();
    colors.surface.application = Color::rgb8(0x12, 0x23, 0x34);
    colors.content.primary = Color::rgb8(0x45, 0x56, 0x67);
    colors.accent.default = Color::rgb8(0x78, 0x89, 0x9A);

    let customized = original.with_colors(colors);
    assert_eq!(
        customized.color(SemanticColor::SurfaceApplication),
        Color::rgb8(0x12, 0x23, 0x34)
    );
    assert_eq!(
        customized.color(SemanticColor::ContentPrimary),
        Color::rgb8(0x45, 0x56, 0x67)
    );
    assert_eq!(
        customized.color(SemanticColor::AccentDefault),
        Color::rgb8(0x78, 0x89, 0x9A)
    );
    assert_eq!(customized.spacing, original.spacing);
    assert_eq!(customized.controls, original.controls);
}

#[test]
#[allow(clippy::float_cmp)]
fn facade_exposes_typed_elevation_construction_and_resolution() {
    use stern::core::{Color, ElevationLevel, ElevationScale, Vec2, default_dark_theme};

    let scale = ElevationScale::new(10.0, 20.0, 30.0, 40.0);
    let theme = default_dark_theme().with_elevation(scale);
    assert_eq!(theme.elevation.get(ElevationLevel::None), 10.0);
    assert_eq!(theme.elevation.get(ElevationLevel::Low), 20.0);
    assert_eq!(theme.elevation.get(ElevationLevel::Medium), 30.0);
    assert_eq!(theme.elevation.get(ElevationLevel::High), 40.0);

    assert_eq!(theme.elevation_shadow(ElevationLevel::None, 4.0), None);
    let medium = theme
        .elevation_shadow(ElevationLevel::Medium, 4.0)
        .expect("medium elevation recipe");
    assert_eq!(medium.offset, Vec2::new(0.0, 6.0));
    assert_eq!(medium.blur_radius, 18.0);
    assert_eq!(medium.spread, 0.0);
    assert_eq!(medium.radius, 4.0);
    assert_eq!(medium.color, Color::rgba(0.0, 0.0, 0.0, 0.42));
}

#[test]
#[allow(clippy::float_cmp)]
fn qualified_core_facades_expose_exact_spacing_construction_and_lookup() {
    let direct = stern_core::SpacingScale::new(
        101.0, 103.0, 107.0, 109.0, 113.0, 127.0, 131.0, 137.0, 139.0,
    );
    assert_eq!(direct.get(stern_core::SpacingStep::Eight), 139.0);
    assert_eq!(
        direct.resolve(stern_core::SpacingRole::CompactInlineControlPadding),
        109.0
    );

    let facade = stern::core::SpacingScale::new(
        201.0, 203.0, 207.0, 209.0, 211.0, 223.0, 227.0, 229.0, 233.0,
    );
    assert_eq!(facade.get(stern::core::SpacingStep::Zero), 201.0);
    assert_eq!(
        facade.resolve(stern::core::SpacingRole::InspectorLabelValueGap),
        211.0
    );
    assert_eq!(stern_core::SpacingStep::ALL.len(), 9);
    assert_eq!(stern::core::SpacingRole::ALL.len(), 9);
}

#[test]
fn facade_exposes_exact_radius_construction_and_qualified_fields() {
    use stern::core::{ComponentState, CornerRadius, RadiusScale, default_dark_theme};

    let radii = RadiusScale::from_values(4.0, 8.0, 16.0, 2048.0);
    assert_eq!(radii.none, CornerRadius::all(0.0));
    assert_eq!(radii.sm, CornerRadius::all(4.0));
    assert_eq!(radii.md, CornerRadius::all(8.0));
    assert_eq!(radii.lg, CornerRadius::all(16.0));
    assert_eq!(radii.full, CornerRadius::all(2048.0));

    let theme = default_dark_theme().with_radii(radii);
    assert_eq!(theme.radii, radii);
    assert_eq!(theme.radius, radii.sm);
    assert_eq!(
        theme.button(ComponentState::default()).radius,
        theme.radii.sm
    );
    assert_eq!(
        theme.tab(ComponentState::default()).radius,
        theme.radii.none
    );
    assert_eq!(
        theme.radio_button(ComponentState::default()).radius,
        theme.radii.full
    );
}

#[test]
fn facade_primary_recipe_consumes_custom_accent_state_roles() {
    let mut colors = stern::core::ThemeColors::default_dark();
    colors.accent.default = stern::core::Color::rgb8(1, 2, 3);
    colors.accent.hover = stern::core::Color::rgb8(4, 5, 6);
    colors.accent.pressed = stern::core::Color::rgb8(7, 8, 9);
    let theme = stern::core::default_dark_theme().with_colors(colors);

    let cases = [
        (
            stern::core::ComponentState::default(),
            colors.accent.default,
        ),
        (
            stern::core::ComponentState {
                hovered: true,
                ..stern::core::ComponentState::default()
            },
            colors.accent.hover,
        ),
        (
            stern::core::ComponentState {
                selected: true,
                hovered: true,
                ..stern::core::ComponentState::default()
            },
            colors.accent.default,
        ),
        (
            stern::core::ComponentState {
                pressed: true,
                selected: true,
                hovered: true,
                ..stern::core::ComponentState::default()
            },
            colors.accent.pressed,
        ),
    ];

    for (state, expected) in cases {
        assert_eq!(
            theme
                .button_variant(stern::core::ButtonVariant::Primary, state)
                .background,
            stern::core::Brush::Solid(expected)
        );
    }
}

#[cfg(feature = "vello-winit")]
#[test]
fn facade_vello_winit_module_preserves_direct_crate_identities_without_prelude_exports() {
    fn same_type<T>(_: Option<T>, _: Option<T>) {}

    same_type::<stern::vello_winit::wgpu::Device>(None, None::<stern_vello_winit::wgpu::Device>);
    same_type::<stern::vello_winit::AaConfig>(None, None::<stern_vello_winit::AaConfig>);
}

#[test]
fn canonical_liveness_incarnation_surface_compiles_and_reports_typed_statuses() {
    use stern::core::{
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
    use stern::core::{LivenessGeneration, LivenessRegistry, WidgetId};

    let target = WidgetId::from_key("compatibility");
    let mut registry = LivenessRegistry::new();
    let token = registry.mark_live(target);
    let generation: LivenessGeneration = token.generation();

    assert!(registry.is_live(target));
    assert_eq!(registry.current_generation(target), Some(generation));
}

#[test]
fn canonical_advanced_widget_modules_compile() {
    use stern::widgets::{
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
    use stern::text::{TextAffinity, TextCaret, TextEditState};

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
fn shaped_navigation_api_is_qualified_and_state_methods_are_typed() {
    use stern::text::{
        CosmicTextEngine, SHAPED_TEXT_GEOMETRY_EPSILON, ShapedCaretStop, ShapedTextNavigation,
        TextAffinity, TextCaret, TextEditState, TextLayoutKey, TextNavigationError,
        TextNavigationOutcome, TextStyle,
    };
    type Operation = fn(&mut TextEditState, &ShapedTextNavigation) -> TextNavigationOutcome;
    type KeyOperation = fn(
        &mut TextEditState,
        &stern::core::KeyEvent,
        &ShapedTextNavigation,
    ) -> Option<TextNavigationOutcome>;

    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&TextLayoutKey::new(
        "ab",
        TextStyle::new("Inter", 14.0, 20.0),
        100.0,
        false,
    ));
    let navigation: Result<ShapedTextNavigation, TextNavigationError> = layout.navigation("ab");
    let navigation = navigation.expect("valid public shaped navigation");
    let stops: &[ShapedCaretStop] = navigation.caret_stops();
    assert_eq!(navigation.source(), "ab");
    assert!(navigation.matches_source("ab"));
    assert!(std::hint::black_box(SHAPED_TEXT_GEOMETRY_EPSILON) > 0.0);
    assert!(!stops.is_empty());
    let _ = navigation.caret_rect(TextCaret::new(0, TextAffinity::After));
    let _ = navigation.hit_test_caret(0.0, 0.0);
    let _ = navigation.selection_rects(0..1);

    let operations: [Operation; 8] = [
        TextEditState::move_visual_left,
        TextEditState::move_visual_right,
        TextEditState::extend_visual_left,
        TextEditState::extend_visual_right,
        TextEditState::move_visual_word_left,
        TextEditState::move_visual_word_right,
        TextEditState::extend_visual_word_left,
        TextEditState::extend_visual_word_right,
    ];
    assert_eq!(operations.len(), 8);
    let key_operation: KeyOperation = TextEditState::apply_visual_navigation_key;
    let _ = key_operation;
}

#[test]
fn root_widget_compatibility_exports_remain_source_compatible() {
    use stern::widgets::{
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
    use stern::{core, text, widgets};

    let contract_paths = [
        std::any::type_name::<text::TextLayoutCache>(),
        std::any::type_name::<text::TextLayoutStore>(),
        std::any::type_name::<text::TextLayoutChange>(),
        std::any::type_name::<text::TextLayoutChangeCursor>(),
        std::any::type_name::<text::TextLayoutChanges<'static>>(),
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

#[test]
#[allow(clippy::float_cmp)]
fn qualified_facade_stroke_types_construct_and_expose_exact_roles() {
    let strokes: stern::core::StrokeScale =
        stern::core::StrokeScale::from_values(0.75, 1.25, 2.5, 3.5, 4.5);
    let focus: stern::core::FocusStrokeScale = strokes.focus;
    let theme = stern::core::default_dark_theme().with_strokes(strokes);

    assert_eq!(strokes.hairline, 0.75);
    assert_eq!(strokes.default, 1.25);
    assert_eq!(strokes.emphasis, 2.5);
    assert_eq!(focus.primary, 3.5);
    assert_eq!(focus.separator, 4.5);
    assert_eq!(theme.strokes, strokes);
    assert_eq!(theme.border_width, strokes.default);
}

#[test]
#[allow(clippy::float_cmp)]
fn qualified_facade_exposes_focus_ring_recipe_without_prelude_expansion() {
    use stern::core::{
        Brush, Color, CornerRadius, FocusRingRecipe, Primitive, Rect, StrokeScale, ThemeColors,
        default_dark_theme,
    };

    let mut colors = ThemeColors::default_dark();
    colors.focus.indicator = Color::rgb8(0x12, 0x34, 0x56);
    colors.focus.separator = Color::rgb8(0xA1, 0xB2, 0xC3);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(StrokeScale::from_values(0.5, 1.5, 2.5, 3.5, 4.5));
    let recipe: FocusRingRecipe = theme.focus_ring(true).expect("visible focus ring");
    let primitives = recipe.primitives(Rect::new(10.0, 20.0, 30.0, 40.0), theme.radii.sm);
    let outward: fn(FocusRingRecipe, Rect, CornerRadius) -> [Primitive; 2] =
        FocusRingRecipe::outward_annulus_primitives;
    let inward: fn(FocusRingRecipe, Rect, CornerRadius, f32) -> [Primitive; 2] =
        FocusRingRecipe::inward_annulus_primitives;
    let annulus_rect = Rect::new(10.25, 20.5, 30.75, 40.25);
    let outward = outward(recipe, annulus_rect, theme.radii.sm);
    let inward = inward(recipe, annulus_rect, theme.radii.sm, theme.strokes.default);

    assert_eq!(recipe.primary.width, 3.5);
    assert_eq!(
        recipe.primary.brush,
        Brush::Solid(Color::rgb8(0x12, 0x34, 0x56))
    );
    assert_eq!(recipe.separator.width, 4.5);
    assert_eq!(
        recipe.separator.brush,
        Brush::Solid(Color::rgb8(0xA1, 0xB2, 0xC3))
    );
    assert_eq!(primitives.len(), 2);
    assert!(outward.iter().all(|primitive| {
        matches!(
            primitive,
            Primitive::Path(path) if path.fill.is_some() && path.stroke.is_none()
        )
    }));
    assert!(inward.iter().all(|primitive| {
        matches!(
            primitive,
            Primitive::Path(path) if path.fill.is_some() && path.stroke.is_none()
        )
    }));
    assert_eq!(theme.focus_ring(false), None);
}

#[test]
fn retained_text_layout_lifecycle_surface_is_additive() {
    use stern::text;

    let request = text::TextLayoutKey::new(
        "layout",
        text::TextStyle::new("Inter", 12.0, 16.0),
        80.0,
        false,
    );
    let mut store = text::TextLayoutStore::new();
    let cursor: text::TextLayoutChangeCursor = store.change_cursor();
    let transient = store.shape_transient(&request);
    assert!(transient.line_count >= 1);
    let id = store
        .try_layout_id(request)
        .expect("small layout is retained");
    assert!(store.touch_layout(id));
    assert!(store.retained_payload_bytes() > 0);
    store.advance_generation();
    assert_eq!(store.generation(), 1);
    let changes: text::TextLayoutChanges<'_> = store.changes_since(cursor);
    let dirty: Vec<text::TextLayoutChange> = changes.iter().collect();
    assert_eq!(dirty.len(), 1);
    assert_eq!(dirty[0].id(), id);
    assert!(store.stored_layout(id).is_some());

    let mut cache = text::TextLayoutCache::new();
    cache.advance_generation();
    assert_eq!(cache.generation(), 1);
    assert_eq!(cache.retained_payload_bytes(), 0);
}

#[test]
fn incremental_text_resource_sync_is_qualified_and_caller_owned() {
    use stern::{UiState, render, text};

    let mut state = UiState::new();
    let id = state.text_layouts_mut().layout_id(text::TextLayoutKey::new(
        "resource",
        text::TextStyle::new("Inter", 12.0, 16.0),
        80.0,
        false,
    ));
    let mut resources = render::RenderResources::new();
    let mut sync = render::TextLayoutResourceSync::new();
    let report: render::TextLayoutResourceSyncReport =
        state.reconcile_text_layouts(&mut resources, &mut sync);

    assert_eq!(report.kind, render::TextLayoutResourceSyncKind::Full);
    assert_eq!(report.added, 1);
    assert!(resources.has_text_layout(id));
    assert_eq!(resources.text_layout_count(), 1);
    assert_eq!(
        resources.retained_text_layout_payload_bytes(),
        Some(state.text_layouts().retained_payload_bytes())
    );
    assert!(
        state
            .reconcile_text_layouts(&mut resources, &mut sync)
            .is_noop()
    );
}

#[cfg(feature = "vello-winit")]
#[test]
fn facade_native_texture_api_is_qualified_only() {
    use stern::vello_winit::{
        PresenterDeviceScope, VelloNativeTextureRegistration, VelloNativeTextureUpdateOutcome,
        VelloNativeTextureValidationError, VelloPresenterError, VelloWindowPresenter, wgpu,
    };

    let register: fn(
        &mut VelloWindowPresenter,
        &PresenterDeviceScope,
        &stern::render::TextureResource,
        &wgpu::Texture,
        u64,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> =
        VelloWindowPresenter::register_native_texture;
    let update: fn(
        &mut VelloWindowPresenter,
        &VelloNativeTextureRegistration,
        u64,
    ) -> Result<VelloNativeTextureUpdateOutcome, VelloPresenterError> =
        VelloWindowPresenter::update_native_texture;
    let replace: fn(
        &mut VelloWindowPresenter,
        &VelloNativeTextureRegistration,
        &stern::render::TextureResource,
        &wgpu::Texture,
        u64,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> =
        VelloWindowPresenter::replace_native_texture;
    let remove: fn(
        &mut VelloWindowPresenter,
        &VelloNativeTextureRegistration,
    ) -> Result<(), VelloPresenterError> = VelloWindowPresenter::remove_native_texture;
    let qualified = [
        std::any::type_name::<VelloNativeTextureRegistration>(),
        std::any::type_name::<VelloNativeTextureUpdateOutcome>(),
        std::any::type_name::<VelloNativeTextureValidationError>(),
    ];

    assert!(qualified.iter().all(|name| !name.is_empty()));
    let _ = (register, update, replace, remove);
}
