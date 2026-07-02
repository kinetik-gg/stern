//! Data-only viewport tool overlay conformance tests.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, Color, Point, Rect, ScaleFactor,
    SemanticActionKind, SemanticRole, SemanticValue, Size, TextureId, Vec2, WidgetId,
};
use kinetik_ui_widgets::{
    PanZoom, ViewportActionDescriptor, ViewportActionKind, ViewportActionTarget,
    ViewportCursorMetadata, ViewportCursorRequestSource, ViewportCursorShape, ViewportFit,
    ViewportGuideDescriptor, ViewportGuideId, ViewportGuideOrientation, ViewportGuidePlacement,
    ViewportOverlayDescriptor, ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace,
    ViewportPanZoomHudDescriptor, ViewportRulerDescriptor, ViewportRulerEdge, ViewportRulerId,
    ViewportSafeAreaDescriptor, ViewportSafeAreaId, ViewportSafeAreaSpace,
    ViewportSelectionTargetDescriptor, ViewportSelectionTargetId, ViewportSurface,
    ViewportToolDescriptor, ViewportToolId, ViewportToolSurfaceDescriptor,
    ViewportTransformDragCapture, ViewportTransformDragRequest, ViewportTransformDragStatus,
    ViewportTransformHandleId, ViewportTransformHandleKind, ViewportTransformHandleSet,
    hit_test_viewport_overlays, hit_test_viewport_overlays_at, hit_test_viewport_transform_handles,
    viewport_action_requests, viewport_action_semantics, viewport_action_widget_id,
    viewport_actions_semantics, viewport_cursor_request, viewport_guide_widget_id, viewport_guides,
    viewport_overlay_widget_id, viewport_rulers, viewport_safe_area_widget_id, viewport_safe_areas,
    viewport_selection_outlines, viewport_tool_semantics, viewport_tool_widget_id,
    viewport_transform_handles,
};

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= 0.001,
        "expected {actual} to be close to {expected}"
    );
}

fn assert_point_close(actual: Point, expected: Point) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

fn assert_rect_close(actual: Rect, expected: Rect) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
    assert_close(actual.width, expected.width);
    assert_close(actual.height, expected.height);
}

fn assert_vec_close(actual: Vec2, expected: Vec2) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

fn surface() -> ViewportSurface {
    let mut pan_zoom = PanZoom::default();
    pan_zoom.set_zoom(2.0);
    pan_zoom.pan_by(Vec2::new(15.0, -5.0));

    ViewportSurface {
        texture: TextureId::from_raw(7),
        source_size: Size::new(100.0, 50.0),
        bounds: Rect::new(10.0, 20.0, 300.0, 200.0),
        pan_zoom,
    }
}

fn selection_target(raw: u64) -> ViewportSelectionTargetDescriptor {
    ViewportSelectionTargetDescriptor::new(
        ViewportSelectionTargetId::from_raw(raw),
        Rect::new(10.0, 5.0, 20.0, 10.0),
    )
    .with_handle_size(10.0)
    .with_rotate_offset(20.0)
}

fn viewport_action(
    id: &'static str,
    label: &'static str,
    kind: ViewportActionKind,
) -> ViewportActionDescriptor {
    ViewportActionDescriptor::new(
        ActionDescriptor::new(id, label),
        kind,
        ViewportActionTarget::new(WidgetId::from_key("viewport")),
    )
}

#[path = "viewport_conformance/guides_and_semantics.rs"]
mod guides_and_semantics;
#[path = "viewport_conformance/overlay_actions.rs"]
mod overlay_actions;
#[path = "viewport_conformance/transforms.rs"]
mod transforms;
