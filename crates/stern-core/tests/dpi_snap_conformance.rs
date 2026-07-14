//! Deterministic conformance for STERN-DPI-001 and 002, with partial
//! core-boundary evidence for STERN-DPI-003.

use stern_core::{PhysicalRect, PhysicalSize, Point, Rect, ScaleFactor, Size};

const EPSILON: f64 = 0.000_1;
const EPSILON_F32: f32 = 0.000_1;

fn assert_on_physical_grid(value: f32, scale: ScaleFactor) {
    let physical = f64::from(value) * scale.value();
    let rounded = physical.round();

    assert!(physical.is_finite(), "{physical} must be finite");
    assert!(
        (physical - rounded).abs() <= EPSILON,
        "{physical} must land on an integer physical pixel"
    );
}

fn assert_point_on_physical_grid(point: Point, scale: ScaleFactor) {
    assert_on_physical_grid(point.x, scale);
    assert_on_physical_grid(point.y, scale);
}

fn assert_rect_edges_on_physical_grid(rect: Rect, scale: ScaleFactor) {
    assert_on_physical_grid(rect.min_x(), scale);
    assert_on_physical_grid(rect.min_y(), scale);
    assert_on_physical_grid(rect.max_x(), scale);
    assert_on_physical_grid(rect.max_y(), scale);
}

fn assert_rect_is_finite_and_non_negative(rect: Rect) {
    assert!(rect.x.is_finite());
    assert!(rect.y.is_finite());
    assert!(rect.width.is_finite());
    assert!(rect.height.is_finite());
    assert!(rect.width >= 0.0);
    assert!(rect.height >= 0.0);
}

fn assert_point_near(actual: Point, expected: Point) {
    assert!((actual.x - expected.x).abs() <= EPSILON_F32);
    assert!((actual.y - expected.y).abs() <= EPSILON_F32);
}

fn assert_rect_near(actual: Rect, expected: Rect) {
    assert!((actual.x - expected.x).abs() <= EPSILON_F32);
    assert!((actual.y - expected.y).abs() <= EPSILON_F32);
    assert!((actual.width - expected.width).abs() <= EPSILON_F32);
    assert!((actual.height - expected.height).abs() <= EPSILON_F32);
}

#[test]
fn dpi_snap_point_lands_on_integer_physical_coordinates() {
    for scale in [
        ScaleFactor::new(1.0),
        ScaleFactor::new(1.25),
        ScaleFactor::new(1.5),
        ScaleFactor::new(2.0),
    ] {
        let snapped = scale.snap_point_to_physical_grid(Point::new(10.3, -7.7));

        assert_point_on_physical_grid(snapped, scale);
    }
}

#[test]
fn dpi_snap_rect_edges_land_each_edge_on_physical_grid() {
    for scale in [
        ScaleFactor::new(1.0),
        ScaleFactor::new(1.25),
        ScaleFactor::new(1.5),
        ScaleFactor::new(2.0),
    ] {
        let snapped = scale.snap_rect_to_physical_grid(Rect::new(10.25, 11.25, 5.5, 6.5));

        assert_rect_edges_on_physical_grid(snapped, scale);
        assert_rect_is_finite_and_non_negative(snapped);
    }
}

#[test]
fn dpi_snap_adjacent_rects_keep_shared_snapped_edges() {
    for scale in [
        ScaleFactor::new(1.0),
        ScaleFactor::new(1.25),
        ScaleFactor::new(1.5),
        ScaleFactor::new(2.0),
    ] {
        let left = Rect::new(2.4, 3.2, 7.3, 4.5);
        let right = Rect::new(left.max_x(), 3.2, 5.7, 4.5);

        let snapped_left = scale.snap_rect_to_physical_grid(left);
        let snapped_right = scale.snap_rect_to_physical_grid(right);

        assert!(
            (snapped_left.max_x() - snapped_right.min_x()).abs() <= f32::EPSILON,
            "shared edge must remain shared at scale {}",
            scale.value()
        );
        assert_rect_edges_on_physical_grid(snapped_left, scale);
        assert_rect_edges_on_physical_grid(snapped_right, scale);
    }
}

#[test]
fn dpi_foundation_dimensions_stay_logical_across_release_scales() {
    let logical = Size::new(800.0, 600.0);

    for (value, expected) in [
        (1.0, PhysicalSize::new(800, 600)),
        (1.25, PhysicalSize::new(1000, 750)),
        (1.5, PhysicalSize::new(1200, 900)),
        (2.0, PhysicalSize::new(1600, 1200)),
    ] {
        let scale = ScaleFactor::new(value);
        assert_eq!(scale.logical_size_to_physical(logical), expected);
        assert_eq!(scale.physical_size_to_logical(expected), logical);
    }
}

#[test]
fn dpi_design_pixel_equals_one_logical_unit_at_one_hundred_percent() {
    let scale = ScaleFactor::ONE;

    assert_eq!(
        scale.logical_rect_to_physical(Rect::new(0.0, 0.0, 1.0, 1.0)),
        PhysicalRect::new(0, 0, 1, 1)
    );
}

#[test]
fn dpi_snap_rect_policy_is_distinct_from_coverage_conversion() {
    let scale = ScaleFactor::new(1.25);
    let rect = Rect::new(10.25, 11.25, 5.5, 6.5);

    let coverage = scale.logical_rect_to_physical(rect);
    let snapped = scale.snap_rect_to_physical_grid(rect);

    assert_eq!(coverage, PhysicalRect::new(12, 14, 8, 9));
    assert_ne!(coverage, PhysicalRect::new(13, 14, 7, 8));
    assert_rect_near(snapped, Rect::new(10.4, 11.2, 5.6, 6.4));
    assert_on_physical_grid(snapped.min_x(), scale);
    assert_on_physical_grid(snapped.max_x(), scale);
}

#[test]
fn dpi_snap_invalid_points_return_zero() {
    assert_point_near(
        ScaleFactor::new(0.0).snap_point_to_physical_grid(Point::new(1.0, 2.0)),
        Point::ZERO,
    );
    assert_point_near(
        ScaleFactor::new(f64::NAN).snap_point_to_physical_grid(Point::new(1.0, 2.0)),
        Point::ZERO,
    );
    assert_point_near(
        ScaleFactor::new(1.25).snap_point_to_physical_grid(Point::new(f32::NAN, 2.0)),
        Point::ZERO,
    );
}

#[test]
fn dpi_snap_invalid_rects_return_finite_zero_rect() {
    let cases = [
        (ScaleFactor::new(0.0), Rect::new(1.0, 2.0, 3.0, 4.0)),
        (ScaleFactor::new(f64::NAN), Rect::new(1.0, 2.0, 3.0, 4.0)),
        (ScaleFactor::new(1.25), Rect::new(f32::NAN, 2.0, 3.0, 4.0)),
        (ScaleFactor::new(1.25), Rect::new(1.0, 2.0, -3.0, 4.0)),
        (ScaleFactor::new(1.25), Rect::new(1.0, 2.0, 3.0, -4.0)),
    ];

    for (scale, rect) in cases {
        let snapped = scale.snap_rect_to_physical_grid(rect);

        assert_rect_near(snapped, Rect::ZERO);
        assert_rect_is_finite_and_non_negative(snapped);
    }
}

#[test]
fn dpi_snap_zero_sized_rects_snap_deterministically() {
    let scale = ScaleFactor::new(1.5);
    let snapped = scale.snap_rect_to_physical_grid(Rect::new(2.2, 3.2, 0.0, 0.0));

    assert_rect_edges_on_physical_grid(snapped, scale);
    assert_rect_is_finite_and_non_negative(snapped);
}
