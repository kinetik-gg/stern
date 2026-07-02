#[allow(unused_imports)]
use super::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, Color, PanZoom, Point, Rect,
    ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Size, TextureId, Vec2,
    ViewportActionDescriptor, ViewportActionKind, ViewportActionTarget, ViewportCursorMetadata,
    ViewportCursorRequestSource, ViewportCursorShape, ViewportFit, ViewportGuideDescriptor,
    ViewportGuideId, ViewportGuideOrientation, ViewportGuidePlacement, ViewportOverlayDescriptor,
    ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace, ViewportPanZoomHudDescriptor,
    ViewportRulerDescriptor, ViewportRulerEdge, ViewportRulerId, ViewportSafeAreaDescriptor,
    ViewportSafeAreaId, ViewportSafeAreaSpace, ViewportSelectionTargetDescriptor,
    ViewportSelectionTargetId, ViewportSurface, ViewportToolDescriptor, ViewportToolId,
    ViewportToolSurfaceDescriptor, ViewportTransformDragCapture, ViewportTransformDragRequest,
    ViewportTransformDragStatus, ViewportTransformHandleId, ViewportTransformHandleKind,
    ViewportTransformHandleSet, WidgetId, assert_close, assert_point_close, assert_rect_close,
    assert_vec_close, hit_test_viewport_overlays, hit_test_viewport_overlays_at,
    hit_test_viewport_transform_handles, selection_target, surface, viewport_action,
    viewport_action_requests, viewport_action_semantics, viewport_action_widget_id,
    viewport_actions_semantics, viewport_cursor_request, viewport_guide_widget_id, viewport_guides,
    viewport_overlay_widget_id, viewport_rulers, viewport_safe_area_widget_id, viewport_safe_areas,
    viewport_selection_outlines, viewport_tool_semantics, viewport_tool_widget_id,
    viewport_transform_handles,
};

#[test]
fn overlay_constructors_cover_texture_content_guide_and_tool_metadata() {
    let surface = surface();
    let texture =
        ViewportOverlayDescriptor::texture_surface(ViewportOverlayId::from_raw(1), surface);
    let content =
        ViewportOverlayDescriptor::content_bounds(ViewportOverlayId::from_raw(2), surface);
    let guide = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(3),
        ViewportOverlayKind::Guide,
        Rect::new(0.0, 20.0, 100.0, 1.0),
        ViewportOverlaySpace::Content,
    );
    let tool = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(4),
        ViewportOverlayKind::ToolRegion,
        Rect::new(0.0, 0.0, 20.0, 20.0),
        ViewportOverlaySpace::Viewport,
    )
    .with_label("Tool region");

    assert_eq!(texture.kind, ViewportOverlayKind::TextureSurface);
    assert_eq!(content.kind, ViewportOverlayKind::ContentBounds);
    assert_eq!(guide.kind, ViewportOverlayKind::Guide);
    assert_eq!(tool.kind, ViewportOverlayKind::ToolRegion);
    assert_eq!(tool.label.as_deref(), Some("Tool region"));
    assert_rect_close(
        texture
            .screen_rect(surface, ScaleFactor::ONE)
            .expect("texture rect"),
        surface.content_rect(),
    );
    assert_eq!(
        hit_test_viewport_overlays_at(surface, &[guide], Point::new(80.0, 105.5), ScaleFactor::ONE)
            .expect("guide hit")
            .kind,
        ViewportOverlayKind::Guide
    );
}

#[test]
fn guide_descriptors_resolve_deterministically_and_reject_invalid_inputs() {
    let surface = surface();
    let guides = vec![
        ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(9),
            ViewportGuideOrientation::Horizontal,
            ViewportGuidePlacement::Content(f32::NAN),
        ),
        ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(2),
            ViewportGuideOrientation::Vertical,
            ViewportGuidePlacement::Content(20.0),
        )
        .with_label("Action safe")
        .locked(true),
        ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(4),
            ViewportGuideOrientation::Horizontal,
            ViewportGuidePlacement::Content(10.0),
        ),
        ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(3),
            ViewportGuideOrientation::Horizontal,
            ViewportGuidePlacement::Screen(40.0),
        )
        .enabled(false),
        ViewportGuideDescriptor::new(
            ViewportGuideId::from_raw(8),
            ViewportGuideOrientation::Vertical,
            ViewportGuidePlacement::Content(120.0),
        ),
    ];

    let resolved = viewport_guides(surface, &guides);

    assert_eq!(
        resolved.iter().map(|guide| guide.id).collect::<Vec<_>>(),
        vec![
            ViewportGuideId::from_raw(4),
            ViewportGuideId::from_raw(3),
            ViewportGuideId::from_raw(2),
        ]
    );
    assert_close(resolved[0].screen_position, 85.0);
    assert_eq!(resolved[0].content_position, Some(10.0));
    assert_rect_close(resolved[0].screen_rect, Rect::new(10.0, 84.5, 300.0, 1.0));
    assert_close(resolved[1].screen_position, 40.0);
    assert!(!resolved[1].enabled);
    assert!(resolved[2].locked);
    assert_eq!(resolved[2].label.as_deref(), Some("Action safe"));
    assert!(matches!(
        resolved[2].primitive(Color::WHITE),
        kinetik_ui_core::Primitive::Line(_)
    ));
}

#[test]
fn safe_area_descriptors_clamp_to_content_and_viewport_bounds() {
    let surface = surface();
    let safe_areas = [
        ViewportSafeAreaDescriptor::new(
            ViewportSafeAreaId::from_raw(4),
            Rect::new(-10.0, 10.0, 60.0, 20.0),
            ViewportSafeAreaSpace::Content,
        )
        .with_label("Title safe"),
        ViewportSafeAreaDescriptor::new(
            ViewportSafeAreaId::from_raw(2),
            Rect::new(280.0, -10.0, 60.0, 80.0),
            ViewportSafeAreaSpace::Viewport,
        )
        .enabled(false),
        ViewportSafeAreaDescriptor::new(
            ViewportSafeAreaId::from_raw(9),
            Rect::new(f32::NAN, 5.0, 80.0, 10.0),
            ViewportSafeAreaSpace::Content,
        ),
    ];

    let resolved = viewport_safe_areas(surface, &safe_areas);

    assert_eq!(resolved.len(), 3);
    assert_eq!(resolved[0].id, ViewportSafeAreaId::from_raw(2));
    assert_rect_close(resolved[0].rect, Rect::new(280.0, 0.0, 20.0, 70.0));
    assert_rect_close(resolved[0].screen_rect, Rect::new(290.0, 20.0, 20.0, 70.0));
    assert!(!resolved[0].enabled);
    assert_eq!(resolved[1].id, ViewportSafeAreaId::from_raw(4));
    assert_rect_close(resolved[1].rect, Rect::new(0.0, 10.0, 50.0, 20.0));
    assert_rect_close(resolved[1].screen_rect, Rect::new(75.0, 85.0, 100.0, 40.0));
    assert_eq!(resolved[1].label.as_deref(), Some("Title safe"));
    assert!(resolved[2].screen_rect.x.is_finite());
    assert!(matches!(
        resolved[1].primitive(Color::WHITE, Color::WHITE),
        kinetik_ui_core::Primitive::Rect(_)
    ));
}

#[test]
fn ruler_overlay_descriptors_emit_bounded_ticks_labels_and_origin_metadata() {
    let surface = surface();
    let rulers = viewport_rulers(
        surface,
        &[
            ViewportRulerDescriptor::new(ViewportRulerId::from_raw(9), ViewportRulerEdge::Left)
                .with_thickness(20.0)
                .with_max_ticks(4),
            ViewportRulerDescriptor::new(ViewportRulerId::from_raw(2), ViewportRulerEdge::Top)
                .with_origin_content(0.0)
                .with_max_ticks(6)
                .with_label("Top ruler"),
        ],
    );

    assert_eq!(rulers.len(), 2);
    assert_eq!(rulers[0].id, ViewportRulerId::from_raw(2));
    assert_eq!(rulers[0].edge, ViewportRulerEdge::Top);
    assert_rect_close(rulers[0].rect, Rect::new(10.0, 20.0, 300.0, 18.0));
    assert_eq!(rulers[0].visible_content_range, (0.0, 100.0));
    assert_close(rulers[0].origin_screen_position, 75.0);
    assert_eq!(rulers[0].ticks.len(), 6);
    assert_eq!(rulers[0].ticks[0].label.as_deref(), Some("0"));
    assert!(
        rulers[0]
            .ticks
            .iter()
            .any(|tick| tick.label.as_deref() == Some("50"))
    );
    assert!(
        rulers[0]
            .ticks
            .windows(2)
            .all(|pair| pair[0].value <= pair[1].value)
    );
    assert!(
        rulers[0]
            .primitives(Color::WHITE, Color::WHITE, Color::WHITE)
            .len()
            > 1
    );
    assert_eq!(rulers[1].edge, ViewportRulerEdge::Left);
    assert_eq!(rulers[1].ticks.len(), 4);
}

#[test]
fn ruler_overlay_zero_max_ticks_emits_no_ticks_or_labels() {
    let surface = surface();
    let rulers = viewport_rulers(
        surface,
        &[
            ViewportRulerDescriptor::new(ViewportRulerId::from_raw(12), ViewportRulerEdge::Top)
                .with_max_ticks(0),
        ],
    );

    assert_eq!(rulers.len(), 1);
    assert!(rulers[0].ticks.is_empty());

    let primitives = rulers[0].primitives(Color::WHITE, Color::WHITE, Color::WHITE);
    assert_eq!(primitives.len(), 1);
    assert!(
        !primitives
            .iter()
            .any(|primitive| matches!(primitive, kinetik_ui_core::Primitive::Text(_)))
    );
}

#[test]
fn pan_zoom_hud_reports_state_and_target_metadata_without_actions() {
    let surface = surface();
    let hud = ViewportPanZoomHudDescriptor::new(WidgetId::from_key("viewport-hud"), "Viewport HUD")
        .with_focused_target(ViewportSelectionTargetId::from_raw(2))
        .with_selected_targets(&[
            ViewportSelectionTargetId::from_raw(7),
            ViewportSelectionTargetId::from_raw(2),
            ViewportSelectionTargetId::from_raw(7),
        ])
        .resolve(surface);

    assert_eq!(hud.fit, ViewportFit::Zoom);
    assert_close(hud.zoom, 2.0);
    assert_close(hud.effective_content_scale, 2.0);
    assert_vec_close(hud.pan, Vec2::new(15.0, -5.0));
    assert_eq!(hud.content_size, Size::new(100.0, 50.0));
    assert_eq!(
        hud.selected_targets,
        vec![
            ViewportSelectionTargetId::from_raw(2),
            ViewportSelectionTargetId::from_raw(7),
        ]
    );
    let semantics = hud.semantics(Rect::new(0.0, 0.0, 120.0, 24.0));
    assert_eq!(
        semantics.role,
        SemanticRole::Custom("viewport-pan-zoom-hud".to_owned())
    );
    assert!(semantics.actions.is_empty());
    assert!(matches!(
        semantics.state.value,
        Some(SemanticValue::Text(ref value)) if value.contains("Zoom zoom 2.000")
    ));
}

#[test]
fn semantic_metadata_exposes_stable_viewport_and_tool_identity() {
    let surface = surface();
    let root = WidgetId::from_key("scene-view");
    let tool = ViewportToolDescriptor::new(ViewportToolId::from_raw(9), "Measure")
        .active(true)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair));
    let descriptor = ViewportToolSurfaceDescriptor::new(root, "Scene View").with_active_tool(tool);
    let viewport_node = descriptor.semantics(surface);
    let tool_node = descriptor
        .active_tool_semantics(surface)
        .expect("tool semantics");

    assert_eq!(viewport_node.role, SemanticRole::Viewport);
    assert_eq!(viewport_node.label.as_deref(), Some("Scene View"));
    assert_eq!(
        viewport_node.children,
        vec![viewport_tool_widget_id(root, ViewportToolId::from_raw(9))]
    );
    assert!(matches!(
        viewport_node.state.value,
        Some(SemanticValue::Text(ref value)) if value.contains("Active tool 9")
    ));
    assert_eq!(
        tool_node.role,
        SemanticRole::Custom("viewport-tool".to_owned())
    );
    assert!(tool_node.state.selected);
    assert_eq!(
        tool_node.id,
        viewport_tool_widget_id(root, ViewportToolId::from_raw(9))
    );
    assert_eq!(
        viewport_overlay_widget_id(root, ViewportOverlayId::from_raw(3)),
        viewport_overlay_widget_id(root, ViewportOverlayId::from_raw(3))
    );
    assert_eq!(
        viewport_guide_widget_id(root, ViewportGuideId::from_raw(3)),
        viewport_guide_widget_id(root, ViewportGuideId::from_raw(3))
    );
    assert_eq!(
        viewport_safe_area_widget_id(root, ViewportSafeAreaId::from_raw(3)),
        viewport_safe_area_widget_id(root, ViewportSafeAreaId::from_raw(3))
    );
    assert_eq!(
        viewport_tool_semantics(
            root,
            surface,
            &ViewportToolDescriptor::new(ViewportToolId::from_raw(9), "Measure").active(true),
        )
        .id,
        tool_node.id
    );
}
