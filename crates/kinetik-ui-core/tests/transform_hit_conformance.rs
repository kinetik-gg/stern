//! Transform and transformed hit-test conformance coverage.

use kinetik_ui_core::{
    Point, PointerInput, Rect, Transform, UiInput, Vec2, hit_test, hit_test_transformed,
};

const EPSILON: f32 = 0.0001;

fn input_at(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}

fn assert_close(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be within {EPSILON} of {expected}"
    );
}

fn assert_point_close(actual: Point, expected: Point) {
    assert_close(actual.x, expected.x);
    assert_close(actual.y, expected.y);
}

fn assert_transform_close(actual: Transform, expected: Transform) {
    assert_close(actual.m11, expected.m11);
    assert_close(actual.m12, expected.m12);
    assert_close(actual.m21, expected.m21);
    assert_close(actual.m22, expected.m22);
    assert_close(actual.dx, expected.dx);
    assert_close(actual.dy, expected.dy);
}

#[test]
fn transform_hit_identity_matches_rect_hit_test() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let hit = input_at(Point::new(25.0, 35.0));
    let miss = input_at(Point::new(45.0, 35.0));

    assert_eq!(
        hit_test_transformed(rect, Transform::IDENTITY, &hit),
        hit_test(rect, &hit)
    );
    assert_eq!(
        hit_test_transformed(rect, Transform::IDENTITY, &miss),
        hit_test(rect, &miss)
    );
    assert!(!hit_test_transformed(
        rect,
        Transform::IDENTITY,
        &UiInput::default()
    ));
}

#[test]
fn transform_hit_translation_maps_screen_points_to_local_rect_space() {
    let rect = Rect::new(0.0, 0.0, 20.0, 10.0);
    let transform = Transform::translation(Vec2::new(100.0, 50.0));

    assert!(hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(105.0, 55.0))
    ));
    assert!(!hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(95.0, 55.0))
    ));
}

#[test]
fn transform_hit_scale_preserves_max_edge_exclusivity_after_inverse_mapping() {
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let transform = Transform::scale(Vec2::new(2.0, 3.0));

    assert!(hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(0.0, 0.0))
    ));
    assert!(hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(19.999, 29.999))
    ));
    assert!(!hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(20.0, 15.0))
    ));
    assert!(!hit_test_transformed(
        rect,
        transform,
        &input_at(Point::new(10.0, 30.0))
    ));
}

#[test]
fn transform_hit_nested_composition_matches_vello_formula() {
    let parent = Transform {
        m11: 2.0,
        m12: 0.0,
        m21: 0.0,
        m22: 3.0,
        dx: 10.0,
        dy: 20.0,
    };
    let child = Transform::translation(Vec2::new(4.0, 5.0));
    let composed = parent.then(child);

    assert_transform_close(
        composed,
        Transform {
            m11: 2.0,
            m12: 0.0,
            m21: 0.0,
            m22: 3.0,
            dx: 18.0,
            dy: 35.0,
        },
    );
    assert_point_close(
        composed.transform_point(Point::new(3.0, 2.0)),
        Point::new(24.0, 41.0),
    );
    assert!(hit_test_transformed(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        composed,
        &input_at(Point::new(24.0, 41.0))
    ));
}

#[test]
fn transform_hit_inverse_round_trips_representative_points() {
    let transform = Transform {
        m11: 2.0,
        m12: 0.25,
        m21: -0.5,
        m22: 3.0,
        dx: 10.0,
        dy: -4.0,
    };
    let inverse = transform.try_inverse().expect("invertible transform");

    for point in [
        Point::ZERO,
        Point::new(3.0, 7.0),
        Point::new(-5.0, 11.0),
        Point::new(22.5, -13.25),
    ] {
        let screen = transform.transform_point(point);
        let local = inverse.transform_point(screen);
        assert_point_close(local, point);
    }
}

#[test]
fn transform_hit_near_zero_finite_determinant_stays_invertible() {
    let rect = Rect::new(0.0, 0.0, 10.0, 10.0);
    let transform = Transform::scale(Vec2::new(0.0001, 0.0001));
    let inverse = transform
        .try_inverse()
        .expect("finite non-zero determinant remains invertible");
    let local_point = Point::new(5.0, 5.0);
    let screen_point = transform.transform_point(local_point);

    assert!(inverse.is_finite());
    assert_point_close(inverse.transform_point(screen_point), local_point);
    assert!(hit_test_transformed(
        rect,
        transform,
        &input_at(screen_point)
    ));
    assert!(!hit_test_transformed(
        rect,
        transform,
        &input_at(transform.transform_point(Point::new(10.0, 5.0)))
    ));
}

#[test]
fn transform_hit_singular_transforms_miss_and_return_no_inverse() {
    let singular = Transform {
        m11: 1.0,
        m12: 2.0,
        m21: 2.0,
        m22: 4.0,
        dx: 0.0,
        dy: 0.0,
    };

    assert!(singular.is_finite());
    assert!(singular.try_inverse().is_none());
    assert!(singular.inverse().is_none());
    assert!(!hit_test_transformed(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        singular,
        &input_at(Point::new(5.0, 5.0))
    ));
}

#[test]
fn transform_hit_non_finite_transforms_and_points_miss_deterministically() {
    let non_finite = Transform {
        dx: f32::INFINITY,
        ..Transform::IDENTITY
    };

    assert!(!non_finite.is_finite());
    assert!(non_finite.try_inverse().is_none());
    assert!(!hit_test_transformed(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        non_finite,
        &input_at(Point::new(5.0, 5.0))
    ));

    let finite_with_huge_inverse = Transform {
        m11: 0.1,
        ..Transform::IDENTITY
    };
    assert!(finite_with_huge_inverse.try_inverse().is_some());
    assert!(!hit_test_transformed(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        finite_with_huge_inverse,
        &input_at(Point::new(f32::MAX, 5.0))
    ));
}

#[test]
fn transform_hit_rect_only_hit_test_preserves_max_edge_behavior() {
    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);

    assert!(hit_test(rect, &input_at(Point::new(10.0, 20.0))));
    assert!(hit_test(rect, &input_at(Point::new(39.999, 59.999))));
    assert!(!hit_test(rect, &input_at(Point::new(40.0, 30.0))));
    assert!(!hit_test(rect, &input_at(Point::new(20.0, 60.0))));
}
