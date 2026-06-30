//! Data-only viewport tool overlay conformance tests.

use kinetik_ui_core::{
    Point, Rect, ScaleFactor, SemanticRole, SemanticValue, Size, TextureId, Vec2, WidgetId,
};
use kinetik_ui_widgets::{
    PanZoom, ViewportCursorMetadata, ViewportCursorShape, ViewportFit, ViewportOverlayDescriptor,
    ViewportOverlayId, ViewportOverlayKind, ViewportOverlaySpace, ViewportSurface,
    ViewportToolDescriptor, ViewportToolId, ViewportToolSurfaceDescriptor,
    hit_test_viewport_overlays, hit_test_viewport_overlays_at, viewport_overlay_widget_id,
    viewport_tool_semantics, viewport_tool_widget_id,
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
        viewport_tool_semantics(
            root,
            surface,
            &ViewportToolDescriptor::new(ViewportToolId::from_raw(9), "Measure").active(true),
        )
        .id,
        tool_node.id
    );
}
