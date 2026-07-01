//! Viewport texture surfaces and editor overlay primitives.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, Brush, ClipId,
    Color, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive, ScaleFactor,
    SemanticAction, SemanticNode, SemanticRole, SemanticValue, Size, Stroke, TextPrimitive,
    TextureId, TexturePrimitive, Vec2, WidgetId,
};

mod surface;
pub use surface::*;
mod actions;
pub use actions::*;
mod tools;
pub use tools::*;
mod overlays;
pub use overlays::*;
mod guides;
pub use guides::*;
mod hud;
pub use hud::*;
mod legacy;
pub use legacy::*;

#[cfg(test)]
mod tests {
    use super::{
        Crosshair, Guide, PanZoom, ViewportComposition, ViewportFit, ViewportSurface,
        guide_primitives, ruler_ticks,
    };
    use kinetik_ui_core::{
        ClipId, Color, Point, Primitive, Rect, ScaleFactor, Size, TextureId, Vec2,
    };

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_close(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() <= 0.001,
            "expected {actual} to be close to {expected}"
        );
    }

    fn assert_rect_close(actual: Rect, expected: Rect) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
        assert_close(actual.width, expected.width);
        assert_close(actual.height, expected.height);
    }

    fn assert_point_close(actual: Point, expected: Point) {
        assert_close(actual.x, expected.x);
        assert_close(actual.y, expected.y);
    }

    fn assert_edge_aligned(value: f32, scale_factor: ScaleFactor) {
        let physical = f64::from(value) * scale_factor.value();
        assert!(
            (physical - physical.round()).abs() <= 0.001,
            "{value} -> {physical}"
        );
    }

    fn assert_rect_edges_aligned(rect: Rect, scale_factor: ScaleFactor) {
        for edge in [rect.x, rect.y, rect.max_x(), rect.max_y()] {
            assert_edge_aligned(edge, scale_factor);
        }
    }

    fn surface() -> ViewportSurface {
        ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: Size::new(400.0, 200.0),
            bounds: Rect::new(0.0, 0.0, 200.0, 200.0),
            pan_zoom: PanZoom::default(),
        }
    }

    fn unsnapped_content_rect(surface: ViewportSurface, scale_factor: ScaleFactor) -> Rect {
        let bounds = surface.effective_bounds();
        let source = surface.effective_source_size().expect("source");
        let scale = surface.content_scale_at(scale_factor);
        let width = source.width * scale;
        let height = source.height * scale;

        Rect::new(
            bounds.x + (bounds.width - width) * 0.5 + surface.pan_zoom.pan.x,
            bounds.y + (bounds.height - height) * 0.5 + surface.pan_zoom.pan.y,
            width,
            height,
        )
    }

    fn expected_content_scale_at(surface: ViewportSurface, native_scale: f32) -> f32 {
        match surface.pan_zoom.fit {
            ViewportFit::Fit => {
                let width_scale = surface.bounds.width / surface.source_size.width;
                let height_scale = surface.bounds.height / surface.source_size.height;
                width_scale.min(height_scale)
            }
            ViewportFit::Fill => {
                let width_scale = surface.bounds.width / surface.source_size.width;
                let height_scale = surface.bounds.height / surface.source_size.height;
                width_scale.max(height_scale)
            }
            ViewportFit::ActualSize => native_scale,
            ViewportFit::Zoom => surface.pan_zoom.zoom * native_scale,
        }
    }

    fn expected_unsnapped_content_rect(surface: ViewportSurface, content_scale: f32) -> Rect {
        let width = surface.source_size.width * content_scale;
        let height = surface.source_size.height * content_scale;

        Rect::new(
            surface.bounds.x + (surface.bounds.width - width) * 0.5 + surface.pan_zoom.pan.x,
            surface.bounds.y + (surface.bounds.height - height) * 0.5 + surface.pan_zoom.pan.y,
            width,
            height,
        )
    }

    fn expected_snapped_content_rect(
        surface: ViewportSurface,
        scale_factor: ScaleFactor,
        content_scale: f32,
    ) -> Rect {
        scale_factor
            .snap_rect_to_physical_grid(expected_unsnapped_content_rect(surface, content_scale))
    }

    fn expected_screen_point(content_rect: Rect, content_scale: f32, point: Point) -> Point {
        Point::new(
            content_rect.x + point.x * content_scale,
            content_rect.y + point.y * content_scale,
        )
    }

    #[test]
    fn fit_mode_preserves_aspect_ratio() {
        let rect = surface().content_rect();

        assert_approx(rect.width, 200.0);
        assert_approx(rect.height, 100.0);
        assert_approx(rect.y, 50.0);
    }

    #[test]
    fn fill_mode_preserves_aspect_ratio_and_covers_bounds() {
        let mut surface = surface();
        surface.pan_zoom.fill();
        let rect = surface.content_rect();

        assert_approx(rect.width, 400.0);
        assert_approx(rect.height, 200.0);
        assert_approx(rect.x, -100.0);
        assert_approx(rect.y, 0.0);
    }

    #[test]
    fn pan_zoom_supports_actual_size_custom_zoom_and_pan() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();
        assert_approx(surface.content_rect().width, 400.0);

        surface.pan_zoom.set_zoom(0.5);
        surface.pan_zoom.pan_by(Vec2::new(10.0, 5.0));
        let rect = surface.content_rect();

        assert_eq!(surface.pan_zoom.fit, ViewportFit::Zoom);
        assert_approx(rect.x, 10.0);
        assert_approx(rect.y, 55.0);
    }

    #[test]
    fn actual_size_maps_source_pixels_to_physical_pixels() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();

        for scale_value in [1.0_f32, 1.25, 1.5, 2.0] {
            let scale_factor = ScaleFactor::new(f64::from(scale_value));
            let rect = surface.content_rect_at(scale_factor);
            let expected_scale = 1.0 / scale_value;

            assert_close(surface.content_scale_at(scale_factor), expected_scale);
            assert_close(rect.width * scale_value, surface.source_size.width);
            assert_close(rect.height * scale_value, surface.source_size.height);
            assert_rect_edges_aligned(rect, scale_factor);
        }
    }

    #[test]
    fn content_rect_at_delegates_valid_snapping_to_core_policy() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.35, 0.65, 205.0, 153.0);
        surface.pan_zoom.actual_size();
        surface.pan_zoom.pan_by(Vec2::new(0.4, -0.2));

        for scale_value in [1.25, 1.5, 2.0] {
            let scale_factor = ScaleFactor::new(scale_value);
            let expected = scale_factor
                .snap_rect_to_physical_grid(unsnapped_content_rect(surface, scale_factor));

            assert_rect_close(surface.content_rect_at(scale_factor), expected);
        }
    }

    #[test]
    fn zoom_mode_maps_zoom_to_physical_scale() {
        let mut surface = surface();
        surface.pan_zoom.set_zoom(1.0);

        assert_approx(surface.content_scale_at(ScaleFactor::new(2.0)), 0.5);
        assert_approx(surface.content_rect_at(ScaleFactor::new(2.0)).width, 200.0);
    }

    #[test]
    fn pan_zoom_sanitizes_invalid_zoom_and_pan() {
        let mut surface = surface();
        surface.pan_zoom.set_zoom(f32::NAN);
        surface.pan_zoom.pan_by(Vec2::new(f32::INFINITY, 4.0));
        let rect = surface.content_rect();

        assert_eq!(surface.pan_zoom.fit, ViewportFit::Zoom);
        assert_approx(surface.content_scale(), 1.0);
        assert_approx(rect.x, -100.0);
        assert_approx(rect.y, 4.0);
    }

    #[test]
    fn invalid_surface_sizes_emit_zero_sized_texture_rect() {
        let surface = ViewportSurface {
            texture: TextureId::from_raw(1),
            source_size: Size::new(f32::NAN, 200.0),
            bounds: Rect::new(10.0, 20.0, f32::INFINITY, 200.0),
            pan_zoom: PanZoom::default(),
        };
        let rect = surface.content_rect();

        assert_approx(rect.x, 10.0);
        assert_approx(rect.y, 20.0);
        assert_approx(rect.width, 0.0);
        assert_approx(rect.height, 0.0);
        assert!(surface.screen_to_content(Point::new(10.0, 20.0)).is_none());
    }

    #[test]
    fn viewport_coordinate_conversions_round_trip() {
        let surface = surface();
        let screen = surface
            .content_to_screen(Point::new(100.0, 50.0))
            .expect("screen");
        let content = surface.screen_to_content(screen).expect("content");
        let local = surface
            .screen_to_viewport(screen)
            .and_then(|point| surface.viewport_to_content(point))
            .expect("local content");
        let rect = surface
            .content_rect_to_screen(Rect::new(100.0, 50.0, 20.0, 10.0))
            .expect("rect");

        assert_approx(screen.x, 50.0);
        assert_approx(screen.y, 75.0);
        assert_approx(content.x, 100.0);
        assert_approx(content.y, 50.0);
        assert_approx(local.x, 100.0);
        assert_approx(local.y, 50.0);
        assert_approx(rect.x, 50.0);
        assert_approx(rect.y, 75.0);
        assert_approx(rect.width, 10.0);
        assert_approx(rect.height, 5.0);
    }

    #[test]
    fn fractional_scale_coordinate_conversions_round_trip() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.5, 203.0, 177.0);
        surface.pan_zoom.set_zoom(1.35);
        surface.pan_zoom.pan_by(Vec2::new(7.25, -3.5));

        for (scale_factor, scale_value) in [
            (ScaleFactor::new(1.25), 1.25_f32),
            (ScaleFactor::new(1.5), 1.5_f32),
        ] {
            let content_scale = expected_content_scale_at(surface, 1.0 / scale_value);
            let content_rect = expected_snapped_content_rect(surface, scale_factor, content_scale);

            for point in [
                Point::new(0.0, 0.0),
                Point::new(123.25, 77.5),
                Point::new(399.0, 199.0),
            ] {
                let expected_screen = expected_screen_point(content_rect, content_scale, point);
                let expected_viewport = Point::new(
                    expected_screen.x - surface.bounds.x,
                    expected_screen.y - surface.bounds.y,
                );
                let screen = surface
                    .content_to_screen_at(point, scale_factor)
                    .expect("screen");
                let content = surface
                    .screen_to_content_at(expected_screen, scale_factor)
                    .expect("content");
                let local = surface
                    .viewport_to_content_at(expected_viewport, scale_factor)
                    .expect("local content");

                assert_point_close(screen, expected_screen);
                assert_point_close(content, point);
                assert_point_close(local, point);
            }
        }
    }

    #[test]
    fn texture_surface_emits_texture_primitive() {
        assert!(matches!(
            surface().texture_primitive(),
            Primitive::Texture(_)
        ));
    }

    #[test]
    fn texture_surface_emits_scale_aware_native_rect() {
        let mut surface = surface();
        surface.pan_zoom.actual_size();

        let Primitive::Texture(texture) = surface.texture_primitive_at(ScaleFactor::new(2.0))
        else {
            panic!("expected texture primitive");
        };

        assert_approx(texture.rect.width, 200.0);
        assert_approx(texture.rect.height, 100.0);
    }

    #[test]
    fn ruler_ticks_change_with_zoom() {
        assert!(ruler_ticks(0.0, 100.0, 2.0).len() > ruler_ticks(0.0, 100.0, 0.5).len());
    }

    #[test]
    fn ruler_ticks_handle_reversed_and_invalid_ranges() {
        assert_eq!(ruler_ticks(100.0, 0.0, 1.0), ruler_ticks(0.0, 100.0, 1.0));
        assert!(ruler_ticks(0.0, f32::NAN, 1.0).is_empty());
        assert!(ruler_ticks(0.0, 100.0, f32::NAN).is_empty());
        assert!(ruler_ticks(0.0, 1_000_000.0, 2.0).len() <= 4097);
    }

    #[test]
    fn guide_primitives_emit_lines() {
        let primitives = guide_primitives(
            Rect::new(0.0, 0.0, 100.0, 100.0),
            &[Guide::Horizontal(50.0), Guide::Vertical(25.0)],
            Color::WHITE,
        );

        assert_eq!(primitives.len(), 2);
        assert!(matches!(primitives[0], Primitive::Line(_)));
    }

    #[test]
    fn crosshair_emits_lines_and_label_inside_bounds() {
        let crosshair = Crosshair {
            visible: true,
            position: Point::new(50.0, 50.0),
            label: Some("50,50".to_owned()),
            color: Color::WHITE,
        };

        let primitives = crosshair.primitives(Rect::new(0.0, 0.0, 100.0, 100.0));

        assert_eq!(primitives.len(), 3);
    }

    #[test]
    fn surface_content_overlays_transform_to_screen_space() {
        let surface = surface();
        let guide = surface.content_guide_primitives(&[Guide::Vertical(200.0)], Color::WHITE);
        let crosshair = Crosshair {
            visible: true,
            position: Point::new(200.0, 100.0),
            label: None,
            color: Color::WHITE,
        };
        let crosshair_primitives = surface.content_crosshair_primitives(&crosshair);

        let Primitive::Line(line) = &guide[0] else {
            panic!("expected guide line");
        };
        assert_approx(line.from.x, 100.0);
        assert_approx(line.from.y, 50.0);
        assert_approx(line.to.y, 150.0);

        let Primitive::Line(horizontal) = &crosshair_primitives[0] else {
            panic!("expected crosshair horizontal line");
        };
        assert_approx(horizontal.from.y, 100.0);
        assert_approx(horizontal.to.y, 100.0);
    }

    #[test]
    fn scale_aware_content_overlays_share_texture_rect() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();

        let Primitive::Texture(texture) = surface.texture_primitive_at(ScaleFactor::new(1.5))
        else {
            panic!("expected texture primitive");
        };
        let guide = surface.content_guide_primitives_at(
            &[Guide::Vertical(200.0)],
            Color::WHITE,
            ScaleFactor::new(1.5),
        );
        let Primitive::Line(line) = &guide[0] else {
            panic!("expected guide line");
        };

        assert_approx(line.from.y, texture.rect.y);
        assert_approx(line.to.y, texture.rect.max_y());
        assert!(line.from.x >= texture.rect.x);
        assert!(line.from.x <= texture.rect.max_x());
    }

    #[test]
    fn scale_aware_content_rect_overlays_snap_to_physical_pixels() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let scale_factor = ScaleFactor::new(1.25);
        let content_rect = surface.content_rect_at(scale_factor);
        let content_scale = surface.content_scale_at(scale_factor);
        let content_overlay = Rect::new(23.0, 17.0, 41.0, 19.0);
        let expected = scale_factor.snap_rect_to_physical_grid(Rect::new(
            content_rect.x + content_overlay.x * content_scale,
            content_rect.y + content_overlay.y * content_scale,
            content_overlay.width * content_scale,
            content_overlay.height * content_scale,
        ));

        let overlay = surface
            .content_rect_to_screen_at(content_overlay, scale_factor)
            .expect("overlay rect");

        assert_rect_close(overlay, expected);
        assert_rect_edges_aligned(overlay, scale_factor);
    }

    #[test]
    fn scale_aware_guides_and_crosshair_align_with_snapped_texture_rect() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let scale_factor = ScaleFactor::new(1.5);
        let content_scale = expected_content_scale_at(surface, 1.0 / 1.5);
        let content_rect = expected_snapped_content_rect(surface, scale_factor, content_scale);
        let expected_horizontal_y = content_rect.y + 100.0 * content_scale;
        let expected_vertical_x = content_rect.x + 200.0 * content_scale;
        let viewport_bounds = surface.bounds;

        let guides = surface.content_guide_primitives_at(
            &[Guide::Horizontal(100.0), Guide::Vertical(200.0)],
            Color::WHITE,
            scale_factor,
        );
        assert_eq!(guides.len(), 2);

        let Primitive::Line(horizontal_guide) = &guides[0] else {
            panic!("expected horizontal guide");
        };
        assert_close(horizontal_guide.from.x, content_rect.x);
        assert_close(horizontal_guide.to.x, content_rect.max_x());
        assert_close(horizontal_guide.from.y, expected_horizontal_y);
        assert_close(horizontal_guide.to.y, expected_horizontal_y);
        assert!(horizontal_guide.from.y >= content_rect.y);
        assert!(horizontal_guide.from.y <= content_rect.max_y());
        assert_edge_aligned(horizontal_guide.from.y, scale_factor);

        let Primitive::Line(vertical_guide) = &guides[1] else {
            panic!("expected vertical guide");
        };
        assert_close(vertical_guide.from.y, content_rect.y);
        assert_close(vertical_guide.to.y, content_rect.max_y());
        assert_close(vertical_guide.from.x, expected_vertical_x);
        assert_close(vertical_guide.to.x, expected_vertical_x);
        assert!(vertical_guide.from.x >= content_rect.x);
        assert!(vertical_guide.from.x <= content_rect.max_x());
        assert_edge_aligned(vertical_guide.from.x, scale_factor);

        let crosshair = Crosshair {
            visible: true,
            position: Point::new(200.0, 100.0),
            label: None,
            color: Color::WHITE,
        };
        let crosshair_primitives =
            surface.content_crosshair_primitives_at(&crosshair, scale_factor);
        assert_eq!(crosshair_primitives.len(), 2);
        let Primitive::Line(horizontal_crosshair) = &crosshair_primitives[0] else {
            panic!("expected crosshair horizontal line");
        };
        let Primitive::Line(vertical_crosshair) = &crosshair_primitives[1] else {
            panic!("expected crosshair vertical line");
        };
        let expected_crosshair_screen =
            expected_screen_point(content_rect, content_scale, crosshair.position);

        assert_close(horizontal_crosshair.from.x, viewport_bounds.x);
        assert_close(horizontal_crosshair.to.x, viewport_bounds.max_x());
        assert_close(horizontal_crosshair.from.y, expected_crosshair_screen.y);
        assert_close(horizontal_crosshair.to.y, expected_crosshair_screen.y);
        assert_close(vertical_crosshair.from.x, expected_crosshair_screen.x);
        assert_close(vertical_crosshair.to.x, expected_crosshair_screen.x);
        assert_close(vertical_crosshair.from.y, viewport_bounds.y);
        assert_close(vertical_crosshair.to.y, viewport_bounds.max_y());
        assert!(horizontal_crosshair.from.y >= content_rect.y);
        assert!(horizontal_crosshair.from.y <= content_rect.max_y());
        assert!(vertical_crosshair.from.x >= content_rect.x);
        assert!(vertical_crosshair.from.x <= content_rect.max_x());
        assert_edge_aligned(horizontal_crosshair.from.y, scale_factor);
        assert_edge_aligned(vertical_crosshair.from.x, scale_factor);
    }

    #[test]
    fn invalid_scale_factor_preserves_viewport_rect_behavior() {
        let mut surface = surface();
        surface.bounds = Rect::new(0.25, 0.25, 201.0, 201.0);
        surface.pan_zoom.actual_size();
        let invalid_scale = ScaleFactor::new(0.0);

        let rect = surface.content_rect_at(invalid_scale);
        let overlay = surface
            .content_rect_to_screen_at(Rect::new(20.0, 10.0, 40.0, 20.0), invalid_scale)
            .expect("overlay rect");

        assert_rect_close(rect, unsnapped_content_rect(surface, invalid_scale));
        assert!(rect.width > 0.0);
        assert!(rect.height > 0.0);
        assert!(overlay.width > 0.0);
        assert!(overlay.height > 0.0);
    }

    #[test]
    fn composition_orders_clip_texture_guides_crosshair() {
        let composition = ViewportComposition {
            surface: surface(),
            guides: vec![Guide::Horizontal(50.0)],
            crosshair: Some(Crosshair {
                visible: true,
                position: Point::new(50.0, 50.0),
                label: None,
                color: Color::WHITE,
            }),
            clip: ClipId::from_raw(1),
        };
        let primitives = composition.primitives();

        assert!(matches!(primitives[0], Primitive::ClipBegin { .. }));
        assert!(matches!(primitives[1], Primitive::Texture(_)));
        assert!(matches!(primitives.last(), Some(Primitive::ClipEnd { .. })));
    }
}
