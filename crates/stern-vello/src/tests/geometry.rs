use super::common::{assert_approx, assert_approx64};
use crate::{
    RenderImageSampling, crisp_rect_border_segments, quantize_stroke_width_to_device,
    root_transform, snap_axis_aligned_translation, snap_filled_path_elements_to_device,
    snap_image_rect_to_device, snap_point_to_device, snap_radius_to_device, snap_rect_to_device,
    snap_stroke_center_to_device, snap_stroked_line_to_device,
    snap_stroked_path_elements_to_device, snap_stroked_rect_to_device, viewport_device_scale,
};
use stern_core::{
    CornerRadius, PathElement, PhysicalSize, Point, Primitive, Rect, ScaleFactor, Size,
    ViewportInfo, default_dark_theme,
};
use vello::kurbo::Affine;

#[test]
fn renderer_snaps_geometry_to_device_pixel_grid() {
    let point = snap_point_to_device(Point::new(10.2, 20.6), 2.0);
    let rect = snap_rect_to_device(Rect::new(1.2, 2.2, 9.1, 10.1), 2.0);
    let radius = snap_radius_to_device(
        CornerRadius {
            top_left: 2.0,
            top_right: 3.2,
            bottom_right: 0.0,
            bottom_left: -1.0,
        },
        1.25,
    );

    assert_eq!(point, Point::new(10.0, 20.5));
    assert_eq!(rect, Rect::new(1.0, 2.0, 9.5, 10.5));
    assert_eq!(
        radius,
        CornerRadius {
            top_left: 2.4,
            top_right: 3.2,
            bottom_right: 0.0,
            bottom_left: -1.0,
        }
    );
}

#[test]
fn image_rect_snapping_aligns_all_sampling_modes_to_device_bounds() {
    let rect = Rect::new(3.2, 4.2, 14.0, 14.0);
    let icon = snap_image_rect_to_device(rect, RenderImageSampling::UiIcon, 1.25);
    let smooth = snap_image_rect_to_device(rect, RenderImageSampling::Smooth, 1.25);
    let high_quality = snap_image_rect_to_device(rect, RenderImageSampling::HighQuality, 1.25);

    assert_approx(icon.x, 3.2);
    assert_approx(icon.y, 4.0);
    assert!((icon.width - 14.4).abs() < 0.000_01);
    assert!((icon.height - 14.4).abs() < 0.000_01);
    assert_eq!(smooth, icon);
    assert_eq!(high_quality, icon);
    assert!((icon.width * 1.25 - 18.0).abs() < 0.000_01);
    assert!((smooth.width * 1.25 - 18.0).abs() < 0.000_01);
    assert!((high_quality.width * 1.25 - 18.0).abs() < 0.000_01);
}

#[test]
fn renderer_snaps_stroke_centers_to_physical_pixel_coverage() {
    let one_px = snap_stroke_center_to_device(10.0, 1.0, 1.0);
    let one_px_fractional_scale = snap_stroke_center_to_device(10.0, 1.0, 1.25);
    let two_px = snap_stroke_center_to_device(10.0, 1.0, 2.0);
    let horizontal =
        snap_stroked_line_to_device(Point::new(0.2, 10.0), Point::new(20.2, 10.0), 1.0, 1.0);
    let rect = snap_stroked_rect_to_device(Rect::new(0.1, 0.1, 20.2, 12.2), 1.0, 1.0);
    let fractional_rect = snap_stroked_rect_to_device(Rect::new(0.0, 0.0, 20.0, 12.0), 1.0, 1.25);

    assert_approx(one_px, 10.5);
    assert_approx(one_px_fractional_scale, 10.0);
    assert_approx(two_px, 10.0);
    assert_eq!(horizontal.0, Point::new(0.0, 10.5));
    assert_eq!(horizontal.1, Point::new(20.0, 10.5));
    assert_eq!(rect, Rect::new(0.5, 0.5, 19.0, 11.0));
    assert_eq!(fractional_rect, Rect::new(0.4, 0.4, 19.2, 11.2));
}

#[test]
fn square_rect_borders_are_segmented_on_physical_pixels() {
    let segments = crisp_rect_border_segments(Rect::new(0.0, 0.0, 20.0, 12.0), 1.0, 1.25);

    assert_eq!(
        segments,
        vec![
            Rect::new(0.0, 0.0, 20.0, 0.8),
            Rect::new(0.0, 11.2, 20.0, 0.8),
            Rect::new(0.0, 0.8, 0.8, 10.4),
            Rect::new(19.2, 0.8, 0.8, 10.4),
        ]
    );
    for segment in segments {
        for value in [
            segment.x * 1.25,
            segment.y * 1.25,
            segment.width * 1.25,
            segment.height * 1.25,
        ] {
            assert!((value - value.round()).abs() <= 0.000_01, "{value}");
        }
    }
}

#[test]
fn square_rect_border_segments_collapse_tiny_rectangles() {
    assert_eq!(
        crisp_rect_border_segments(Rect::new(0.0, 0.0, 1.0, 1.0), 1.0, 1.25),
        vec![Rect::new(0.0, 0.0, 0.8, 0.8)]
    );
}

#[test]
fn renderer_snaps_line_based_stroked_paths_to_device_pixels() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 10.3)),
        PathElement::MoveTo(Point::new(4.2, 1.2)),
        PathElement::LineTo(Point::new(4.2, 11.2)),
        PathElement::Close,
    ];

    let snapped = snap_stroked_path_elements_to_device(&elements, 1.0, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(0.0, 10.0)),
            PathElement::LineTo(Point::new(20.0, 10.0)),
            PathElement::MoveTo(Point::new(4.4, 1.6)),
            PathElement::LineTo(Point::new(4.4, 11.2)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_snaps_filled_line_based_paths_to_device_pixels() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 30.3)),
        PathElement::Close,
    ];

    let snapped = snap_filled_path_elements_to_device(&elements, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(0.0, 10.4)),
            PathElement::LineTo(Point::new(20.0, 10.4)),
            PathElement::LineTo(Point::new(20.0, 30.4)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_snaps_closed_stroked_polygon_vertices() {
    let elements = vec![
        PathElement::MoveTo(Point::new(10.2, 0.2)),
        PathElement::LineTo(Point::new(20.2, 10.2)),
        PathElement::LineTo(Point::new(10.2, 20.2)),
        PathElement::LineTo(Point::new(0.2, 10.2)),
        PathElement::Close,
    ];

    let snapped = snap_stroked_path_elements_to_device(&elements, 1.0, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(10.4, 0.0)),
            PathElement::LineTo(Point::new(20.0, 10.4)),
            PathElement::LineTo(Point::new(10.4, 20.0)),
            PathElement::LineTo(Point::new(0.0, 10.4)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_leaves_curved_stroked_paths_unsnapped() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::QuadTo {
            ctrl: Point::new(5.2, 4.2),
            to: Point::new(20.2, 10.3),
        },
    ];

    assert_eq!(
        snap_stroked_path_elements_to_device(&elements, 1.0, 1.25),
        elements
    );
}

#[test]
fn renderer_quantizes_stroke_widths_to_physical_pixels() {
    for scale in [1.0_f64, 1.25, 1.5, 2.0] {
        for logical_width in [1.0_f32, 2.0] {
            let expected_physical_width = (f64::from(logical_width) * scale).round().max(1.0);
            let quantized = quantize_stroke_width_to_device(logical_width, scale);
            let actual_physical_width = f64::from(quantized) * scale;

            assert!(
                (actual_physical_width - expected_physical_width).abs() <= 0.000_01,
                "{actual_physical_width} != {expected_physical_width} at {scale}x"
            );
        }
    }
}

#[test]
fn nested_focus_contours_snap_contiguously_at_release_scales() {
    let base = Rect::new(10.0, 20.0, 20.0, 20.0);
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let [outer, inner] = recipe.primitives(base, CornerRadius::all(4.0));
    let Primitive::Rect(outer) = outer else {
        panic!("outer focus contour must be a rectangle");
    };
    let Primitive::Rect(inner) = inner else {
        panic!("inner focus contour must be a rectangle");
    };

    for scale in [1.0_f64, 1.25, 1.5, 2.0] {
        let snapped_outer = snap_rect_to_device(outer.rect, scale);
        let snapped_inner = snap_rect_to_device(inner.rect, scale);
        let snapped_base = snap_rect_to_device(base, scale);

        assert!(
            snapped_outer.contains_rect(snapped_inner),
            "outer contour must contain inner contour at {scale}x"
        );
        assert!(
            snapped_inner.contains_rect(snapped_base),
            "inner contour must contain base at {scale}x"
        );

        for rect in [snapped_outer, snapped_inner, snapped_base] {
            for edge in [rect.min_x(), rect.min_y(), rect.max_x(), rect.max_y()] {
                let physical = f64::from(edge) * scale;
                assert!(
                    (physical - physical.round()).abs() <= 0.000_01,
                    "edge {edge} is off the physical grid at {scale}x"
                );
            }
        }

        for (band, outer, inner) in [
            ("primary", snapped_outer, snapped_inner),
            ("separator", snapped_inner, snapped_base),
        ] {
            for logical_width in [
                inner.min_x() - outer.min_x(),
                inner.min_y() - outer.min_y(),
                outer.max_x() - inner.max_x(),
                outer.max_y() - inner.max_y(),
            ] {
                let physical_width = f64::from(logical_width) * scale;
                assert!(
                    (physical_width - physical_width.round()).abs() <= 0.000_01,
                    "{band} band is not pixel-aligned at {scale}x"
                );
                assert!(
                    physical_width >= 1.0 - 0.000_01,
                    "{band} band must cover at least one physical pixel at {scale}x"
                );
            }
        }
    }
}

#[test]
fn renderer_uses_half_pixel_centers_for_odd_strokes_and_integer_centers_for_even_strokes() {
    for scale in [1.0_f64, 1.25, 1.5, 2.0] {
        for logical_width in [1.0_f32, 2.0] {
            let physical_width = (f64::from(logical_width) * scale).round().max(1.0);
            let snapped = snap_stroke_center_to_device(10.37, logical_width, scale);
            let physical_center = f64::from(snapped) * scale;
            let fractional = physical_center - physical_center.floor();
            let expected_fractional = if physical_width % 2.0 == 0.0 {
                0.0
            } else {
                0.5
            };
            let direct_error = (fractional - expected_fractional).abs();
            let wrapped_error = (1.0 - direct_error).abs();

            assert!(
                direct_error.min(wrapped_error) <= 0.000_01,
                "physical center {physical_center} has wrong parity for width {physical_width}"
            );
        }
    }
}

#[test]
fn renderer_derives_device_scale_from_each_release_viewport() {
    for (scale, physical_size) in [
        (1.0, PhysicalSize::new(800, 600)),
        (1.25, PhysicalSize::new(1000, 750)),
        (1.5, PhysicalSize::new(1200, 900)),
        (2.0, PhysicalSize::new(1600, 1200)),
    ] {
        let viewport = ViewportInfo::new(
            Size::new(800.0, 600.0),
            physical_size,
            ScaleFactor::new(scale),
        );

        assert_approx64(viewport_device_scale(viewport), scale);
    }
}

#[test]
fn renderer_snaps_axis_aligned_transform_translation_to_device_pixels() {
    let transform =
        snap_axis_aligned_translation(root_transform(2.0) * Affine::translate((0.25, 0.25)));

    let coeffs = transform.as_coeffs();
    assert_approx64(coeffs[0], 2.0);
    assert_approx64(coeffs[1], 0.0);
    assert_approx64(coeffs[2], 0.0);
    assert_approx64(coeffs[3], 2.0);
    assert_approx64(coeffs[4], 1.0);
    assert_approx64(coeffs[5], 1.0);
}
