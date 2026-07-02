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
fn selection_outlines_and_transform_handles_track_content_screen_conversion() {
    let surface = surface();
    let target = selection_target(11).with_label("Layer 11");
    let outlines = viewport_selection_outlines(surface, std::slice::from_ref(&target));
    let handles = viewport_transform_handles(surface, std::slice::from_ref(&target));
    let handle = |kind| {
        handles
            .iter()
            .find(|handle| handle.kind == kind)
            .expect("handle")
    };

    assert_eq!(outlines.len(), 1);
    assert_eq!(outlines[0].target, ViewportSelectionTargetId::from_raw(11));
    assert_eq!(outlines[0].label.as_deref(), Some("Layer 11"));
    assert_rect_close(outlines[0].content_rect, Rect::new(10.0, 5.0, 20.0, 10.0));
    assert_rect_close(outlines[0].screen_rect, Rect::new(95.0, 75.0, 40.0, 20.0));

    assert_rect_close(
        handle(ViewportTransformHandleKind::Move).handle_screen_rect,
        Rect::new(95.0, 75.0, 40.0, 20.0),
    );
    assert_rect_close(
        handle(ViewportTransformHandleKind::ResizeTopLeft).handle_screen_rect,
        Rect::new(90.0, 70.0, 10.0, 10.0),
    );
    assert_rect_close(
        handle(ViewportTransformHandleKind::ResizeRight).handle_screen_rect,
        Rect::new(130.0, 80.0, 10.0, 10.0),
    );
    assert_rect_close(
        handle(ViewportTransformHandleKind::Rotate).handle_screen_rect,
        Rect::new(110.0, 50.0, 10.0, 10.0),
    );
    assert_rect_close(
        handle(ViewportTransformHandleKind::Pivot).handle_screen_rect,
        Rect::new(110.0, 80.0, 10.0, 10.0),
    );
    assert_eq!(
        handle(ViewportTransformHandleKind::ResizeTopLeft)
            .cursor
            .shape,
        ViewportCursorShape::ResizeTopLeftBottomRight
    );
}

#[test]
fn transform_handle_priority_and_ties_are_descriptor_order_independent() {
    let surface = surface();
    let lower_priority = selection_target(2).with_priority(1);
    let topmost = selection_target(9).with_priority(5);
    let ordered = hit_test_viewport_transform_handles(
        surface,
        &[lower_priority.clone(), topmost.clone()],
        Point::new(95.0, 75.0),
    )
    .expect("ordered hit");
    let reversed = hit_test_viewport_transform_handles(
        surface,
        &[topmost, lower_priority],
        Point::new(95.0, 75.0),
    )
    .expect("reversed hit");

    assert_eq!(ordered.target, ViewportSelectionTargetId::from_raw(9));
    assert_eq!(reversed.target, ordered.target);
    assert_eq!(ordered.kind, ViewportTransformHandleKind::ResizeTopLeft);

    let move_and_pivot = hit_test_viewport_transform_handles(
        surface,
        &[selection_target(4)],
        Point::new(115.0, 85.0),
    )
    .expect("specific handle hit");
    assert_eq!(move_and_pivot.kind, ViewportTransformHandleKind::Pivot);

    let tie = hit_test_viewport_transform_handles(
        surface,
        &[selection_target(7), selection_target(3)],
        Point::new(115.0, 85.0),
    )
    .expect("tie hit");
    assert_eq!(tie.target, ViewportSelectionTargetId::from_raw(3));
    assert_eq!(tie.kind, ViewportTransformHandleKind::Pivot);
}

#[test]
fn transform_drag_capture_preserves_identity_and_reports_deltas_without_mutation() {
    let surface = surface();
    let target = selection_target(11);
    let original = target.clone();
    let hit = hit_test_viewport_transform_handles(
        surface,
        std::slice::from_ref(&target),
        Point::new(135.0, 85.0),
    )
    .expect("resize handle hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);
    let request = ViewportTransformDragRequest::update(
        surface,
        std::slice::from_ref(&target),
        &capture,
        Point::new(145.0, 95.0),
    );

    assert_eq!(hit.kind, ViewportTransformHandleKind::ResizeRight);
    assert_eq!(request.status, ViewportTransformDragStatus::Active);
    assert_eq!(
        request.handle,
        ViewportTransformHandleId::new(
            ViewportSelectionTargetId::from_raw(11),
            ViewportTransformHandleKind::ResizeRight,
        )
    );
    assert_eq!(request.target, ViewportSelectionTargetId::from_raw(11));
    assert_rect_close(
        request.source_content_rect,
        Rect::new(10.0, 5.0, 20.0, 10.0),
    );
    assert_eq!(
        request.current_content_rect,
        Some(Rect::new(10.0, 5.0, 20.0, 10.0))
    );
    assert_point_close(request.pointer_origin_screen, Point::new(135.0, 85.0));
    assert_point_close(request.pointer_current_screen, Point::new(145.0, 95.0));
    assert_vec_close(request.screen_delta, Vec2::new(10.0, 10.0));
    assert_vec_close(request.content_delta, Vec2::new(5.0, 5.0));
    assert_eq!(target, original);
}

#[test]
fn transform_drag_update_reports_invalid_pointer_as_noop_error_data() {
    let surface = surface();
    let target = selection_target(11);
    let hit = hit_test_viewport_transform_handles(
        surface,
        std::slice::from_ref(&target),
        Point::new(135.0, 85.0),
    )
    .expect("resize handle hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);
    let request = ViewportTransformDragRequest::update(
        surface,
        std::slice::from_ref(&target),
        &capture,
        Point::new(f32::NAN, 95.0),
    );

    assert_eq!(request.status, ViewportTransformDragStatus::InvalidPointer);
    assert!(request.is_noop());
    assert_eq!(
        request.current_content_rect,
        Some(Rect::new(10.0, 5.0, 20.0, 10.0))
    );
    assert_point_close(
        request.pointer_current_screen,
        capture.pointer_origin_screen,
    );
    assert_vec_close(request.screen_delta, Vec2::ZERO);
    assert_vec_close(request.content_delta, Vec2::ZERO);
}

#[test]
fn transform_drag_update_reports_invalid_scale_as_noop_error_data() {
    let surface = surface();
    let target = selection_target(11);
    let hit = hit_test_viewport_transform_handles(
        surface,
        std::slice::from_ref(&target),
        Point::new(135.0, 85.0),
    )
    .expect("resize handle hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);
    let invalid_surface = ViewportSurface {
        source_size: Size::new(0.0, 50.0),
        ..surface
    };
    let request = ViewportTransformDragRequest::update(
        invalid_surface,
        std::slice::from_ref(&target),
        &capture,
        Point::new(145.0, 95.0),
    );

    assert_eq!(request.status, ViewportTransformDragStatus::InvalidScale);
    assert!(request.is_noop());
    assert_eq!(
        request.current_content_rect,
        Some(Rect::new(10.0, 5.0, 20.0, 10.0))
    );
    assert_eq!(request.pointer_current_content, None);
    assert_vec_close(request.screen_delta, Vec2::new(10.0, 10.0));
    assert_vec_close(request.content_delta, Vec2::ZERO);
}

#[test]
fn transform_drag_update_rejects_invalid_current_target_geometry() {
    let surface = surface();
    let target = selection_target(11);
    let hit = hit_test_viewport_transform_handles(
        surface,
        std::slice::from_ref(&target),
        Point::new(135.0, 85.0),
    )
    .expect("resize handle hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);

    for content_rect in [
        Rect::new(f32::NAN, 5.0, 20.0, 10.0),
        Rect::new(10.0, 5.0, 0.0, 10.0),
    ] {
        let mut current_target = selection_target(11);
        current_target.content_rect = content_rect;

        let request = ViewportTransformDragRequest::update(
            surface,
            &[current_target],
            &capture,
            Point::new(145.0, 95.0),
        );

        assert_eq!(
            request.status,
            ViewportTransformDragStatus::UnavailableTarget
        );
        assert!(request.is_noop());
        assert_eq!(request.current_content_rect, None);
        assert_vec_close(request.screen_delta, Vec2::new(10.0, 10.0));
        assert_vec_close(request.content_delta, Vec2::new(5.0, 5.0));
    }
}

#[test]
fn disabled_read_only_and_unavailable_targets_suppress_transform_requests() {
    let surface = surface();
    let disabled = selection_target(1).enabled(false);
    let read_only = selection_target(2).read_only(true);
    let unavailable = selection_target(3).available(false);
    let unselected = selection_target(4).selected(false);

    assert!(
        hit_test_viewport_transform_handles(
            surface,
            &[disabled, read_only, unavailable, unselected],
            Point::new(115.0, 85.0),
        )
        .is_none()
    );

    let active = selection_target(10);
    let hit = hit_test_viewport_transform_handles(surface, &[active], Point::new(135.0, 85.0))
        .expect("active hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);
    let read_only_current = selection_target(10).read_only(true);
    let request = ViewportTransformDragRequest::update(
        surface,
        &[read_only_current],
        &capture,
        Point::new(140.0, 90.0),
    );

    assert_eq!(
        request.status,
        ViewportTransformDragStatus::UnavailableTarget
    );
    assert!(request.is_noop());
}

#[test]
fn stale_target_drag_requests_preserve_capture_metadata_as_noop_error_data() {
    let surface = surface();
    let target = selection_target(11);
    let hit = hit_test_viewport_transform_handles(surface, &[target], Point::new(135.0, 85.0))
        .expect("active hit");
    let capture = ViewportTransformDragCapture::from_hit(&hit);
    let request =
        ViewportTransformDragRequest::update(surface, &[], &capture, Point::new(145.0, 95.0));

    assert_eq!(request.status, ViewportTransformDragStatus::StaleTarget);
    assert!(request.is_noop());
    assert_eq!(
        request.handle,
        ViewportTransformHandleId::new(
            ViewportSelectionTargetId::from_raw(11),
            ViewportTransformHandleKind::ResizeRight,
        )
    );
    assert_eq!(request.current_content_rect, None);
    assert_rect_close(
        request.source_content_rect,
        Rect::new(10.0, 5.0, 20.0, 10.0),
    );
    assert_vec_close(request.screen_delta, Vec2::new(10.0, 10.0));
    assert_vec_close(request.content_delta, Vec2::new(5.0, 5.0));
}

#[test]
fn handle_sets_can_limit_available_transform_metadata() {
    let surface = surface();
    let target = selection_target(20).with_handles(ViewportTransformHandleSet::move_only());
    let handles = viewport_transform_handles(surface, std::slice::from_ref(&target));
    let hit = hit_test_viewport_transform_handles(surface, &[target], Point::new(115.0, 85.0))
        .expect("move hit");

    assert_eq!(handles.len(), 1);
    assert_eq!(handles[0].kind, ViewportTransformHandleKind::Move);
    assert_eq!(hit.kind, ViewportTransformHandleKind::Move);
}
