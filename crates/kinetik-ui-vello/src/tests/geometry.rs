use super::common::{assert_approx, assert_approx64};
use crate::{
    RenderImageSampling, crisp_rect_border_segments, quantize_stroke_width_to_device,
    root_transform, snap_axis_aligned_translation, snap_filled_path_elements_to_device,
    snap_image_rect_to_device, snap_point_to_device, snap_radius_to_device, snap_rect_to_device,
    snap_stroke_center_to_device, snap_stroked_line_to_device,
    snap_stroked_path_elements_to_device, snap_stroked_rect_to_device,
};
use kinetik_ui_core::{CornerRadius, PathElement, Point, Rect};
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
    assert_approx(quantize_stroke_width_to_device(1.0, 1.0), 1.0);
    assert_approx(quantize_stroke_width_to_device(1.0, 1.25), 0.8);
    assert_approx(quantize_stroke_width_to_device(1.0, 1.5), 1.333_333_4);
    assert_approx(quantize_stroke_width_to_device(2.0, 1.25), 2.4);
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
