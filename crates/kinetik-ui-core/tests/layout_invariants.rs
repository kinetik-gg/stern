//! Deterministic invariant coverage for primitive layout helpers.

use kinetik_ui_core::layout::{separator, spacer};
use kinetik_ui_core::{
    Alignment, Axis, Insets, LayoutItem, Measurement, Rect, SeparatorKind, Size, SizeRule,
    column_layout, fit_box, pad_rect, rect_from_size, row_layout, split_leading, split_trailing,
    stack_layout,
};

const EPSILON: f32 = 0.0001;

fn item(width: SizeRule, height: SizeRule, measured: Size) -> LayoutItem {
    LayoutItem::new(width, height, Measurement::new(measured))
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= EPSILON,
        "expected {actual} to be approximately {expected}",
    );
}

fn assert_rect_invariants(rect: Rect) {
    assert!(rect.x.is_finite(), "rect x must be finite: {rect:?}");
    assert!(rect.y.is_finite(), "rect y must be finite: {rect:?}");
    assert!(
        rect.width.is_finite(),
        "rect width must be finite: {rect:?}",
    );
    assert!(
        rect.height.is_finite(),
        "rect height must be finite: {rect:?}",
    );
    assert!(
        rect.width >= 0.0,
        "rect width must be non-negative: {rect:?}",
    );
    assert!(
        rect.height >= 0.0,
        "rect height must be non-negative: {rect:?}",
    );
}

fn assert_rects_invariants(rects: &[Rect]) {
    for rect in rects {
        assert_rect_invariants(*rect);
    }
}

fn assert_size_invariants(size: Size) {
    assert!(size.width.is_finite(), "width must be finite: {size:?}");
    assert!(size.height.is_finite(), "height must be finite: {size:?}");
    assert!(size.width >= 0.0, "width must be non-negative: {size:?}");
    assert!(size.height >= 0.0, "height must be non-negative: {size:?}");
}

fn assert_horizontal_monotonic(rects: &[Rect]) {
    let mut previous_max = f32::NEG_INFINITY;

    for rect in rects {
        assert!(
            rect.x + EPSILON >= previous_max,
            "row rects must advance monotonically: {rects:?}",
        );
        previous_max = rect.max_x();
    }
}

fn assert_vertical_monotonic(rects: &[Rect]) {
    let mut previous_max = f32::NEG_INFINITY;

    for rect in rects {
        assert!(
            rect.y + EPSILON >= previous_max,
            "column rects must advance monotonically: {rects:?}",
        );
        previous_max = rect.max_y();
    }
}

#[test]
fn layout_invariants_size_rule_resolution_sanitizes_invalid_inputs() {
    let cases = [
        (SizeRule::Fixed(12.0), 100.0, 20.0, 30.0, 12.0),
        (SizeRule::Fixed(-12.0), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::Fixed(f32::NAN), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::Fixed(f32::INFINITY), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::Fit, 100.0, 20.0, 30.0, 20.0),
        (SizeRule::Fit, 100.0, f32::NAN, 30.0, 0.0),
        (SizeRule::Fit, 100.0, -20.0, 30.0, 0.0),
        (SizeRule::Fill, 100.0, 20.0, 30.0, 100.0),
        (SizeRule::Fill, -100.0, 20.0, 30.0, 0.0),
        (SizeRule::Fill, f32::INFINITY, 20.0, 30.0, 0.0),
        (SizeRule::Percent(0.25), 100.0, 20.0, 30.0, 25.0),
        (SizeRule::Percent(-0.25), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::Percent(f32::NAN), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::Percent(0.5), f32::NAN, 20.0, 30.0, 0.0),
        (
            SizeRule::MinMax {
                min: 10.0,
                max: 30.0,
            },
            100.0,
            50.0,
            30.0,
            30.0,
        ),
        (
            SizeRule::MinMax {
                min: 30.0,
                max: 10.0,
            },
            100.0,
            20.0,
            30.0,
            20.0,
        ),
        (
            SizeRule::MinMax {
                min: 30.0,
                max: 10.0,
            },
            100.0,
            50.0,
            30.0,
            30.0,
        ),
        (
            SizeRule::MinMax {
                min: f32::NAN,
                max: 20.0,
            },
            100.0,
            50.0,
            30.0,
            20.0,
        ),
        (SizeRule::AspectRatio(2.0), 100.0, 20.0, 30.0, 60.0),
        (SizeRule::AspectRatio(-2.0), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::AspectRatio(f32::NAN), 100.0, 20.0, 30.0, 0.0),
        (SizeRule::AspectRatio(2.0), 100.0, 20.0, f32::INFINITY, 0.0),
    ];

    for (rule, available, measured, cross, expected) in cases {
        let actual = rule.resolve(available, measured, cross);

        assert!(actual.is_finite(), "resolved size must be finite: {rule:?}");
        assert!(
            actual >= 0.0,
            "resolved size must be non-negative: {rule:?}",
        );
        assert_approx(actual, expected);
    }
}

#[test]
fn layout_invariants_empty_child_lists_return_empty() {
    assert!(row_layout(Rect::new(0.0, 0.0, f32::NAN, -10.0), &[], f32::INFINITY).is_empty());
    assert!(column_layout(Rect::new(0.0, 0.0, -10.0, f32::NAN), &[], f32::NAN).is_empty());
}

#[test]
fn layout_invariants_spacing_cannot_make_child_sizes_negative() {
    let rect = Rect::new(0.0, 0.0, 10.0, 5.0);
    let items = [
        item(SizeRule::Fixed(4.0), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        item(SizeRule::Percent(0.5), SizeRule::Fill, Size::ZERO),
    ];

    let row = row_layout(rect, &items, 100.0);

    assert_rects_invariants(&row);
    assert_horizontal_monotonic(&row);
    assert_approx(row[0].width, 4.0);
    assert_approx(row[1].width, 0.0);
    assert_approx(row[2].width, 5.0);
}

#[test]
fn layout_invariants_fill_distribution_is_stable() {
    let rect = Rect::new(0.0, 0.0, 120.0, 24.0);
    let items = [
        item(SizeRule::Fixed(20.0), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Percent(0.25), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fit, SizeRule::Fill, Size::new(10.0, 8.0)),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
    ];

    let row = row_layout(rect, &items, 5.0);

    assert_rects_invariants(&row);
    assert_horizontal_monotonic(&row);
    assert_approx(row[0].width, 20.0);
    assert_approx(row[1].width, 30.0);
    assert_approx(row[2].width, 20.0);
    assert_approx(row[3].width, 10.0);
    assert_approx(row[4].width, 20.0);
    assert_approx(row[2].width, row[4].width);
    assert_approx(row[4].max_x(), rect.max_x());
}

#[test]
fn layout_invariants_row_and_column_are_axis_symmetric() {
    let row_rect = Rect::new(10.0, 20.0, 90.0, 30.0);
    let column_rect = Rect::new(10.0, 20.0, 30.0, 90.0);
    let row_items = [
        item(SizeRule::Fixed(20.0), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fit, SizeRule::Fill, Size::new(10.0, 8.0)),
        item(SizeRule::Percent(0.2), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
    ];
    let column_items = [
        item(SizeRule::Fill, SizeRule::Fixed(20.0), Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fit, Size::new(8.0, 10.0)),
        item(SizeRule::Fill, SizeRule::Percent(0.2), Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
    ];

    let row = row_layout(row_rect, &row_items, 4.0);
    let column = column_layout(column_rect, &column_items, 4.0);

    assert_rects_invariants(&row);
    assert_rects_invariants(&column);
    assert_horizontal_monotonic(&row);
    assert_vertical_monotonic(&column);

    for (row_rect, column_rect) in row.iter().zip(column.iter()) {
        assert_approx(row_rect.x, column_rect.y - 10.0);
        assert_approx(row_rect.y, column_rect.x + 10.0);
        assert_approx(row_rect.width, column_rect.height);
        assert_approx(row_rect.height, column_rect.width);
    }
}

#[test]
fn layout_invariants_percent_fixed_fit_fill_combinations_are_stable_in_tiny_parents() {
    let rect = Rect::new(0.0, 0.0, 3.0, 2.0);
    let items = [
        item(SizeRule::Fixed(4.0), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fit, SizeRule::Fill, Size::new(5.0, 3.0)),
        item(SizeRule::Percent(0.75), SizeRule::Fill, Size::ZERO),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
    ];

    let row = row_layout(rect, &items, 1.0);

    assert_rects_invariants(&row);
    assert_horizontal_monotonic(&row);
    assert_approx(row[0].width, 4.0);
    assert_approx(row[1].width, 5.0);
    assert_approx(row[2].width, 2.25);
    assert_approx(row[3].width, 0.0);
}

#[test]
fn layout_invariants_invalid_parent_sizes_measurements_and_spacing_sanitize() {
    let invalid_row_rect = Rect::new(0.0, 0.0, f32::NAN, -10.0);
    let invalid_column_rect = Rect::new(0.0, 0.0, -10.0, f32::INFINITY);
    let items = [
        item(SizeRule::Fixed(f32::NAN), SizeRule::Fill, Size::ZERO),
        item(
            SizeRule::Fit,
            SizeRule::AspectRatio(f32::INFINITY),
            Size::new(f32::INFINITY, f32::NAN),
        ),
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
    ];

    let row = row_layout(invalid_row_rect, &items, f32::INFINITY);
    let column = column_layout(invalid_column_rect, &items, f32::NAN);

    assert_rects_invariants(&row);
    assert_rects_invariants(&column);
    assert_horizontal_monotonic(&row);
    assert_vertical_monotonic(&column);
    assert!(
        row.iter()
            .all(|rect| rect.width == 0.0 && rect.height == 0.0)
    );
    assert!(
        column
            .iter()
            .all(|rect| rect.width == 0.0 && rect.height == 0.0)
    );
}

#[test]
fn layout_invariants_zero_size_parent_produces_finite_non_negative_children() {
    let rect = Rect::ZERO;
    let items = [
        item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        item(SizeRule::Percent(0.5), SizeRule::Fit, Size::new(5.0, 6.0)),
        item(
            SizeRule::AspectRatio(2.0),
            SizeRule::MinMax { min: 6.0, max: 2.0 },
            Size::new(8.0, 4.0),
        ),
    ];

    let row = row_layout(rect, &items, 2.0);
    let column = column_layout(rect, &items, 2.0);

    assert_rects_invariants(&row);
    assert_rects_invariants(&column);
}

#[test]
fn layout_invariants_aspect_ratio_row_policy_does_not_freeze_column_height() {
    let row_items = [item(SizeRule::AspectRatio(2.0), SizeRule::Fill, Size::ZERO)];
    let column_items = [item(SizeRule::Fill, SizeRule::AspectRatio(2.0), Size::ZERO)];

    let row = row_layout(Rect::new(0.0, 0.0, 100.0, 30.0), &row_items, 0.0);
    let column = column_layout(Rect::new(0.0, 0.0, 30.0, 100.0), &column_items, 0.0);

    assert_rects_invariants(&row);
    assert_rects_invariants(&column);
    assert_approx(row[0].width, 60.0);
    assert_approx(row[0].height, 30.0);
    assert_approx(column[0].width, 30.0);
    // TODO: vertical-axis aspect-ratio layout should preserve the documented
    // width/height contract; conformance must not freeze the current column
    // height behavior until that policy is implemented.
}

#[test]
fn layout_invariants_primitive_helpers_sanitize_invalid_rect_inputs() {
    let invalid_rect = Rect::new(f32::NAN, f32::INFINITY, -10.0, f32::NAN);

    assert_rect_invariants(rect_from_size(Size::new(f32::INFINITY, -5.0)));
    assert_rect_invariants(pad_rect(
        invalid_rect,
        Insets::new(f32::NAN, f32::INFINITY, -5.0, 4.0),
    ));
    assert_rect_invariants(fit_box(
        invalid_rect,
        Size::new(f32::NAN, f32::INFINITY),
        Alignment::Center,
        Alignment::Stretch,
    ));
    assert_rects_invariants(&stack_layout(invalid_rect, 2));

    let (leading, remaining) = split_leading(invalid_rect, Axis::Horizontal, f32::INFINITY);
    assert_rect_invariants(leading);
    assert_rect_invariants(remaining);

    let (remaining, trailing) = split_trailing(invalid_rect, Axis::Vertical, f32::NAN);
    assert_rect_invariants(remaining);
    assert_rect_invariants(trailing);
}

#[test]
fn layout_invariants_spacer_and_separator_measurements_sanitize_invalid_amounts() {
    assert_size_invariants(spacer(Axis::Horizontal, f32::NAN).desired);
    assert_size_invariants(spacer(Axis::Vertical, -4.0).desired);
    assert_size_invariants(separator(SeparatorKind::Horizontal, f32::INFINITY).desired);
    assert_size_invariants(separator(SeparatorKind::Vertical, -1.0).desired);
}
