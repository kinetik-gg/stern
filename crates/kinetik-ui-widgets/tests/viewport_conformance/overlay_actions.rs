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
fn content_screen_point_and_rect_conversions_round_trip_under_pan_zoom() {
    let surface = surface();
    let content_point = Point::new(20.0, 10.0);
    let screen_point = surface
        .content_to_screen(content_point)
        .expect("screen point");
    let round_trip_point = surface
        .screen_to_content(screen_point)
        .expect("content point");
    let content_rect = Rect::new(12.0, 8.0, 24.0, 10.0);
    let screen_rect = surface
        .content_rect_to_screen(content_rect)
        .expect("screen rect");
    let round_trip_rect = surface
        .screen_rect_to_content(screen_rect)
        .expect("content rect");

    assert_eq!(surface.pan_zoom.fit, ViewportFit::Zoom);
    assert_point_close(screen_point, Point::new(115.0, 85.0));
    assert_point_close(round_trip_point, content_point);
    assert_rect_close(screen_rect, Rect::new(99.0, 81.0, 48.0, 20.0));
    assert_rect_close(round_trip_rect, content_rect);
    assert!(screen_point.x.is_finite());
    assert!(screen_rect.width.is_finite());
}

#[test]
fn overlay_hit_testing_transforms_content_targets_and_rejects_invalid_descriptors() {
    let surface = surface();
    let content_overlay = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(20),
        ViewportOverlayKind::ToolRegion,
        Rect::new(10.0, 5.0, 20.0, 10.0),
        ViewportOverlaySpace::Content,
    );
    let invalid_overlay = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(1),
        ViewportOverlayKind::Guide,
        Rect::new(f32::NAN, 0.0, 40.0, 10.0),
        ViewportOverlaySpace::Screen,
    )
    .with_priority(100);
    let zero_overlay = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(2),
        ViewportOverlayKind::Guide,
        Rect::new(0.0, 0.0, 0.0, 10.0),
        ViewportOverlaySpace::Screen,
    )
    .with_priority(100);
    let hit = hit_test_viewport_overlays(
        surface,
        &[invalid_overlay, zero_overlay, content_overlay],
        Point::new(98.0, 80.0),
    )
    .expect("content overlay hit");

    assert_eq!(hit.overlay, ViewportOverlayId::from_raw(20));
    assert_eq!(hit.kind, ViewportOverlayKind::ToolRegion);
    assert_rect_close(hit.rect, Rect::new(95.0, 75.0, 40.0, 20.0));
    assert_point_close(
        hit.content_point.expect("content point"),
        Point::new(11.5, 7.5),
    );
    assert!(hit_test_viewport_overlays(surface, &[], Point::new(f32::NAN, 80.0)).is_none());
}

#[test]
fn overlay_hit_priority_and_id_tie_breaking_are_descriptor_order_independent() {
    let surface = surface();
    let low = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(2),
        ViewportOverlayKind::ContentBounds,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    );
    let high = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(40),
        ViewportOverlayKind::ToolRegion,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .with_priority(80);
    let tie_a = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(7),
        ViewportOverlayKind::Guide,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .with_priority(90);
    let tie_b = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(3),
        ViewportOverlayKind::Guide,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .with_priority(90);

    let ordered = hit_test_viewport_overlays(
        surface,
        &[low.clone(), high.clone(), tie_a.clone(), tie_b.clone()],
        Point::new(100.0, 80.0),
    )
    .expect("ordered hit");
    let reversed =
        hit_test_viewport_overlays(surface, &[tie_b, tie_a, high, low], Point::new(100.0, 80.0))
            .expect("reversed hit");

    assert_eq!(ordered.overlay, ViewportOverlayId::from_raw(3));
    assert_eq!(reversed.overlay, ordered.overlay);
    assert_eq!(ordered.priority, 90);
}

#[test]
fn cursor_metadata_is_reported_as_request_data_only() {
    let tool = ViewportToolDescriptor::new(ViewportToolId::from_raw(11), "Pan")
        .active(true)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Grab).with_label("Pan"));
    let disabled_tool = tool.clone().enabled(false);
    let overlay = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(50),
        ViewportOverlayKind::ToolRegion,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .with_tool(tool.id)
    .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair));
    let hit = hit_test_viewport_overlays(surface(), &[overlay], Point::new(100.0, 80.0))
        .expect("cursor hit");

    assert_eq!(
        tool.cursor_request().map(|cursor| &cursor.shape),
        Some(&ViewportCursorShape::Grab)
    );
    assert_eq!(
        hit.cursor.as_ref().map(|cursor| &cursor.shape),
        Some(&ViewportCursorShape::Crosshair)
    );
    assert_eq!(hit.tool, Some(ViewportToolId::from_raw(11)));
    assert!(disabled_tool.cursor_request().is_none());
}

#[test]
fn disabled_or_unavailable_overlays_do_not_emit_hit_requests() {
    let disabled = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(1),
        ViewportOverlayKind::ToolRegion,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .enabled(false);
    let unavailable = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(2),
        ViewportOverlayKind::ToolRegion,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .available(false);

    assert!(
        hit_test_viewport_overlays(surface(), &[disabled, unavailable], Point::new(100.0, 80.0))
            .is_none()
    );
}

#[test]
fn viewport_action_descriptors_preserve_order_state_and_context_metadata() {
    let viewport = WidgetId::from_key("main-viewport");
    let mut select = ActionDescriptor::new("viewport.tool.select", "Select");
    select.state.checked = Some(true);
    let fit_selection = ActionDescriptor::new("viewport.fit.selection", "Fit Selection");
    let mut overlay = ActionDescriptor::new("viewport.overlay.grid", "Grid");
    overlay.state.checked = Some(false);
    let actions = [
        ViewportActionDescriptor::new(
            select,
            ViewportActionKind::ActivateTool,
            ViewportActionTarget::new(viewport).with_tool(ViewportToolId::from_raw(1)),
        ),
        ViewportActionDescriptor::new(
            fit_selection,
            ViewportActionKind::FitSelection,
            ViewportActionTarget::new(viewport)
                .with_selection(ViewportSelectionTargetId::from_raw(7)),
        ),
        ViewportActionDescriptor::new(
            overlay,
            ViewportActionKind::ToggleOverlay,
            ViewportActionTarget::new(viewport).with_overlay(ViewportOverlayId::from_raw(3)),
        ),
    ];

    let requests = viewport_action_requests(
        &actions,
        ActionSource::CommandPalette,
        &ActionContext::Widget(viewport),
    );

    assert_eq!(
        requests
            .iter()
            .map(|request| request.action_id.clone())
            .collect::<Vec<_>>(),
        vec![
            ActionId::new("viewport.tool.select"),
            ActionId::new("viewport.fit.selection"),
            ActionId::new("viewport.overlay.grid"),
        ]
    );
    assert_eq!(requests[0].kind, ViewportActionKind::ActivateTool);
    assert_eq!(requests[0].source, ActionSource::CommandPalette);
    assert_eq!(requests[0].context, ActionContext::Widget(viewport));
    assert_eq!(requests[0].target.tool, Some(ViewportToolId::from_raw(1)));
    assert_eq!(
        requests[1].target.selection,
        Some(ViewportSelectionTargetId::from_raw(7))
    );
    assert_eq!(
        requests[2].target.overlay,
        Some(ViewportOverlayId::from_raw(3))
    );
    assert_eq!(requests[0].checked, Some(true));
    assert_eq!(requests[2].checked, Some(false));
    assert_eq!(
        requests[0].action_invocation().action_id,
        ActionId::new("viewport.tool.select")
    );
}

#[test]
fn disabled_and_hidden_viewport_actions_do_not_emit_requests() {
    let mut disabled = viewport_action("viewport.zoom.in", "Zoom In", ViewportActionKind::ZoomIn);
    disabled.action.state.enabled = false;
    let mut hidden = viewport_action("viewport.zoom.out", "Zoom Out", ViewportActionKind::ZoomOut);
    hidden.action.state.visible = false;
    let actual_size = viewport_action(
        "viewport.actual-size",
        "Actual Size",
        ViewportActionKind::ActualSize,
    );

    let requests = viewport_action_requests(
        &[disabled, hidden, actual_size],
        ActionSource::Button,
        &ActionContext::Editor,
    );

    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].action_id, ActionId::new("viewport.actual-size"));
    assert_eq!(requests[0].kind, ViewportActionKind::ActualSize);
}

#[test]
fn focus_fit_zoom_pan_and_overlay_requests_preserve_viewport_targets() {
    let viewport = WidgetId::from_key("scene-view");
    let selected = ViewportSelectionTargetId::from_raw(42);
    let overlay = ViewportOverlayId::from_raw(5);
    let actions = [
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.focus.selected", "Focus Selected"),
            ViewportActionKind::FocusSelected,
            ViewportActionTarget::new(viewport).with_selection(selected),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.fit.content", "Fit Content"),
            ViewportActionKind::FitContent,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.fit.selection", "Fit Selection"),
            ViewportActionKind::FitSelection,
            ViewportActionTarget::new(viewport).with_selection(selected),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.zoom.in", "Zoom In"),
            ViewportActionKind::ZoomIn,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.pan", "Pan"),
            ViewportActionKind::PanMode,
            ViewportActionTarget::new(viewport),
        ),
        ViewportActionDescriptor::new(
            ActionDescriptor::new("viewport.overlay.safe-area", "Safe Area"),
            ViewportActionKind::ToggleOverlay,
            ViewportActionTarget::new(viewport).with_overlay(overlay),
        ),
    ];

    let requests = viewport_action_requests(&actions, ActionSource::Button, &ActionContext::Editor);

    assert_eq!(
        requests
            .iter()
            .map(|request| request.kind)
            .collect::<Vec<_>>(),
        vec![
            ViewportActionKind::FocusSelected,
            ViewportActionKind::FitContent,
            ViewportActionKind::FitSelection,
            ViewportActionKind::ZoomIn,
            ViewportActionKind::PanMode,
            ViewportActionKind::ToggleOverlay,
        ]
    );
    assert!(
        requests
            .iter()
            .all(|request| request.target.viewport == viewport)
    );
    assert_eq!(requests[0].target.selection, Some(selected));
    assert_eq!(requests[2].target.selection, Some(selected));
    assert_eq!(requests[5].target.overlay, Some(overlay));
}

#[test]
fn viewport_action_semantics_expose_button_toggle_and_action_metadata() {
    let viewport = WidgetId::from_key("scene-view");
    let mut pan = ActionDescriptor::new("viewport.pan", "Pan");
    pan.state.checked = Some(true);
    pan.tooltip = Some("Pan viewport".to_owned());
    let pan_action = ViewportActionDescriptor::new(
        pan,
        ViewportActionKind::PanMode,
        ViewportActionTarget::new(viewport),
    );
    let fit_action = ViewportActionDescriptor::new(
        ActionDescriptor::new("viewport.fit.content", "Fit Content"),
        ViewportActionKind::FitContent,
        ViewportActionTarget::new(viewport),
    );

    let root = WidgetId::from_key("viewport-actions");
    let pan_node = viewport_action_semantics(root, Rect::new(0.0, 0.0, 24.0, 24.0), &pan_action)
        .expect("pan semantics");
    let fit_node = viewport_action_semantics(root, Rect::new(24.0, 0.0, 24.0, 24.0), &fit_action)
        .expect("fit semantics");
    let nodes = viewport_actions_semantics(
        root,
        Rect::new(0.0, 0.0, 48.0, 24.0),
        "Viewport actions",
        &[pan_action, fit_action],
        [
            (
                ActionId::new("viewport.pan"),
                Rect::new(0.0, 0.0, 24.0, 24.0),
            ),
            (
                ActionId::new("viewport.fit.content"),
                Rect::new(24.0, 0.0, 24.0, 24.0),
            ),
        ],
    );

    assert_eq!(pan_node.role, SemanticRole::Toggle);
    assert_eq!(pan_node.state.checked, Some(true));
    assert!(pan_node.state.selected);
    assert_eq!(pan_node.description.as_deref(), Some("Pan viewport"));
    assert!(pan_node.actions.iter().any(|action| {
        action.kind == SemanticActionKind::Invoke
            && action.action_id == Some(ActionId::new("viewport.pan"))
    }));
    assert_eq!(fit_node.role, SemanticRole::Button);
    assert!(fit_node.actions.iter().any(|action| {
        action.kind == SemanticActionKind::Invoke
            && action.action_id == Some(ActionId::new("viewport.fit.content"))
    }));
    assert_eq!(nodes[0].children.len(), 2);
    assert_eq!(
        viewport_action_widget_id(root, &ActionId::new("viewport.pan")),
        pan_node.id
    );
}

#[test]
fn viewport_cursor_request_priority_is_active_handle_hovered_handle_overlay_then_tool() {
    let surface = surface();
    let viewport = WidgetId::from_key("scene-view");
    let target = selection_target(11);
    let hovered_handle = hit_test_viewport_transform_handles(
        surface,
        std::slice::from_ref(&target),
        Point::new(135.0, 85.0),
    )
    .expect("hovered handle");
    let active_capture = ViewportTransformDragCapture::from_hit(&hovered_handle);
    let overlay = ViewportOverlayDescriptor::new(
        ViewportOverlayId::from_raw(3),
        ViewportOverlayKind::ToolRegion,
        Rect::new(90.0, 75.0, 70.0, 40.0),
        ViewportOverlaySpace::Screen,
    )
    .with_tool(ViewportToolId::from_raw(4))
    .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Crosshair));
    let hovered_overlay = hit_test_viewport_overlays(surface, &[overlay], Point::new(100.0, 80.0))
        .expect("hovered overlay");
    let tool = ViewportToolDescriptor::new(ViewportToolId::from_raw(4), "Pan")
        .active(true)
        .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Grab));

    let active = viewport_cursor_request(
        viewport,
        Some(&active_capture),
        Some(&hovered_handle),
        Some(&hovered_overlay),
        Some(&tool),
    )
    .expect("active cursor");
    let handle = viewport_cursor_request(
        viewport,
        None,
        Some(&hovered_handle),
        Some(&hovered_overlay),
        Some(&tool),
    )
    .expect("handle cursor");
    let overlay =
        viewport_cursor_request(viewport, None, None, Some(&hovered_overlay), Some(&tool))
            .expect("overlay cursor");
    let tool =
        viewport_cursor_request(viewport, None, None, None, Some(&tool)).expect("tool cursor");

    assert_eq!(active.source, ViewportCursorRequestSource::ActiveHandle);
    assert_eq!(active.cursor.shape, ViewportCursorShape::ResizeHorizontal);
    assert_eq!(active.handle, Some(active_capture.handle));
    assert_eq!(handle.source, ViewportCursorRequestSource::HoveredHandle);
    assert_eq!(handle.target, Some(ViewportSelectionTargetId::from_raw(11)));
    assert_eq!(overlay.source, ViewportCursorRequestSource::HoveredOverlay);
    assert_eq!(overlay.overlay, Some(ViewportOverlayId::from_raw(3)));
    assert_eq!(overlay.tool, Some(ViewportToolId::from_raw(4)));
    assert_eq!(tool.source, ViewportCursorRequestSource::ActiveTool);
    assert_eq!(tool.cursor.shape, ViewportCursorShape::Grab);
}
