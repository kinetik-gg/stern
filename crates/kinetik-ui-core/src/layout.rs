//! Measurement-aware layout primitives.

use crate::{Rect, Size};

/// Layout axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Axis {
    /// Horizontal axis.
    Horizontal,
    /// Vertical axis.
    Vertical,
}

/// A rule for resolving a size along one axis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SizeRule {
    /// Exact logical size.
    Fixed(f32),
    /// Use the measured intrinsic size.
    Fit,
    /// Share remaining space with other fill items.
    Fill,
    /// Use a percentage of parent available space.
    Percent(f32),
    /// Clamp the measured intrinsic size to a range.
    MinMax {
        /// Minimum resolved size.
        min: f32,
        /// Maximum resolved size.
        max: f32,
    },
    /// Preserve an aspect ratio. The value is width divided by height.
    AspectRatio(f32),
}

impl SizeRule {
    /// Resolves this rule against available and measured sizes.
    #[must_use]
    pub fn resolve(self, available: f32, measured: f32, cross: f32) -> f32 {
        sanitize_size(match self {
            Self::Fixed(value) => value,
            Self::Fit => measured,
            Self::Fill => available,
            Self::Percent(value) => sanitize_size(available) * sanitize_size(value),
            Self::MinMax { min, max } => {
                let min = sanitize_size(min);
                let max = sanitize_size(max);
                let (min, max) = if min <= max { (min, max) } else { (max, min) };
                sanitize_size(measured).clamp(min, max)
            }
            Self::AspectRatio(ratio) => sanitize_size(cross) * sanitize_size(ratio),
        })
    }
}

fn sanitize_size(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn sanitize_origin(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        sanitize_origin(rect.x),
        sanitize_origin(rect.y),
        sanitize_size(rect.width),
        sanitize_size(rect.height),
    )
}

fn sanitize_insets(insets: Insets) -> Insets {
    Insets::new(
        sanitize_size(insets.left),
        sanitize_size(insets.right),
        sanitize_size(insets.top),
        sanitize_size(insets.bottom),
    )
}

/// Measured intrinsic size for a layout item.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Measurement {
    /// Desired logical size.
    pub desired: Size,
}

impl Measurement {
    /// Creates a measurement from a desired size.
    #[must_use]
    pub const fn new(desired: Size) -> Self {
        Self { desired }
    }
}

/// A child participating in layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutItem {
    /// Width sizing rule.
    pub width: SizeRule,
    /// Height sizing rule.
    pub height: SizeRule,
    /// Intrinsic measurement.
    pub measurement: Measurement,
}

impl LayoutItem {
    /// Creates a layout item.
    #[must_use]
    pub const fn new(width: SizeRule, height: SizeRule, measurement: Measurement) -> Self {
        Self {
            width,
            height,
            measurement,
        }
    }

    fn main_rule(self, axis: Axis) -> SizeRule {
        match axis {
            Axis::Horizontal => self.width,
            Axis::Vertical => self.height,
        }
    }

    fn cross_rule(self, axis: Axis) -> SizeRule {
        match axis {
            Axis::Horizontal => self.height,
            Axis::Vertical => self.width,
        }
    }

    fn measured_main(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.measurement.desired.width,
            Axis::Vertical => self.measurement.desired.height,
        }
    }

    fn measured_cross(self, axis: Axis) -> f32 {
        match axis {
            Axis::Horizontal => self.measurement.desired.height,
            Axis::Vertical => self.measurement.desired.width,
        }
    }
}

/// Insets used for padding and margins.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Insets {
    /// Left inset.
    pub left: f32,
    /// Right inset.
    pub right: f32,
    /// Top inset.
    pub top: f32,
    /// Bottom inset.
    pub bottom: f32,
}

impl Insets {
    /// Creates insets from individual sides.
    #[must_use]
    pub const fn new(left: f32, right: f32, top: f32, bottom: f32) -> Self {
        Self {
            left,
            right,
            top,
            bottom,
        }
    }

    /// Creates equal insets on every side.
    #[must_use]
    pub const fn all(value: f32) -> Self {
        Self::new(value, value, value, value)
    }

    /// Creates symmetric horizontal and vertical insets.
    #[must_use]
    pub const fn symmetric(horizontal: f32, vertical: f32) -> Self {
        Self::new(horizontal, horizontal, vertical, vertical)
    }
}

/// Alignment inside an available rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Alignment {
    /// Align to the start edge.
    Start,
    /// Align to the center.
    Center,
    /// Align to the end edge.
    End,
    /// Stretch to fill the available span.
    Stretch,
}

/// Separator orientation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeparatorKind {
    /// Horizontal separator line.
    Horizontal,
    /// Vertical separator line.
    Vertical,
}

/// Applies padding to a rectangle.
#[must_use]
pub fn pad_rect(rect: Rect, insets: Insets) -> Rect {
    let rect = sanitize_rect(rect);
    let insets = sanitize_insets(insets);

    Rect::new(
        rect.x + insets.left,
        rect.y + insets.top,
        rect.width - insets.left - insets.right,
        rect.height - insets.top - insets.bottom,
    )
    .max_zero()
}

/// Returns a rectangle whose origin is `(0, 0)` and size is `size`.
#[must_use]
pub fn rect_from_size(size: Size) -> Rect {
    Rect::new(
        0.0,
        0.0,
        sanitize_size(size.width),
        sanitize_size(size.height),
    )
}

/// Splits a rectangle into a leading band and remaining body along `axis`.
#[must_use]
pub fn split_leading(rect: Rect, axis: Axis, amount: f32) -> (Rect, Rect) {
    let rect = sanitize_rect(rect);
    let amount = sanitize_size(amount);
    match axis {
        Axis::Horizontal => {
            let leading = amount.min(rect.width);
            (
                Rect::new(rect.x, rect.y, leading, rect.height),
                Rect::new(
                    rect.x + leading,
                    rect.y,
                    (rect.width - leading).max(0.0),
                    rect.height,
                ),
            )
        }
        Axis::Vertical => {
            let leading = amount.min(rect.height);
            (
                Rect::new(rect.x, rect.y, rect.width, leading),
                Rect::new(
                    rect.x,
                    rect.y + leading,
                    rect.width,
                    (rect.height - leading).max(0.0),
                ),
            )
        }
    }
}

/// Splits a rectangle into a remaining body and trailing band along `axis`.
#[must_use]
pub fn split_trailing(rect: Rect, axis: Axis, amount: f32) -> (Rect, Rect) {
    let rect = sanitize_rect(rect);
    let amount = sanitize_size(amount);
    match axis {
        Axis::Horizontal => {
            let trailing = amount.min(rect.width);
            (
                Rect::new(
                    rect.x,
                    rect.y,
                    (rect.width - trailing).max(0.0),
                    rect.height,
                ),
                Rect::new(rect.max_x() - trailing, rect.y, trailing, rect.height),
            )
        }
        Axis::Vertical => {
            let trailing = amount.min(rect.height);
            (
                Rect::new(
                    rect.x,
                    rect.y,
                    rect.width,
                    (rect.height - trailing).max(0.0),
                ),
                Rect::new(rect.x, rect.max_y() - trailing, rect.width, trailing),
            )
        }
    }
}

/// Fits a size inside a rectangle according to alignment.
#[must_use]
pub fn fit_box(rect: Rect, size: Size, horizontal: Alignment, vertical: Alignment) -> Rect {
    let rect = sanitize_rect(rect);
    let size = Size::new(sanitize_size(size.width), sanitize_size(size.height));

    let width = if horizontal == Alignment::Stretch {
        rect.width
    } else {
        size.width.min(rect.width).max(0.0)
    };
    let height = if vertical == Alignment::Stretch {
        rect.height
    } else {
        size.height.min(rect.height).max(0.0)
    };

    let x = aligned_origin(rect.x, rect.width, width, horizontal);
    let y = aligned_origin(rect.y, rect.height, height, vertical);

    Rect::new(x, y, width, height)
}

/// Returns rectangles for children stacked over the same parent rectangle.
#[must_use]
pub fn stack_layout(rect: Rect, count: usize) -> Vec<Rect> {
    let rect = sanitize_rect(rect);

    vec![rect; count]
}

/// Lays out children horizontally.
#[must_use]
pub fn row_layout(rect: Rect, items: &[LayoutItem], spacing: f32) -> Vec<Rect> {
    linear_layout(Axis::Horizontal, rect, items, spacing)
}

/// Lays out children vertically.
#[must_use]
pub fn column_layout(rect: Rect, items: &[LayoutItem], spacing: f32) -> Vec<Rect> {
    linear_layout(Axis::Vertical, rect, items, spacing)
}

/// Returns the measurement of a spacer along an axis.
#[must_use]
pub fn spacer(axis: Axis, amount: f32) -> Measurement {
    let amount = sanitize_size(amount);

    match axis {
        Axis::Horizontal => Measurement::new(Size::new(amount, 0.0)),
        Axis::Vertical => Measurement::new(Size::new(0.0, amount)),
    }
}

/// Returns the measurement of a separator.
#[must_use]
pub fn separator(kind: SeparatorKind, thickness: f32) -> Measurement {
    let thickness = sanitize_size(thickness);

    match kind {
        SeparatorKind::Horizontal => Measurement::new(Size::new(0.0, thickness)),
        SeparatorKind::Vertical => Measurement::new(Size::new(thickness, 0.0)),
    }
}

fn aligned_origin(origin: f32, available: f32, actual: f32, alignment: Alignment) -> f32 {
    match alignment {
        Alignment::Start | Alignment::Stretch => origin,
        Alignment::Center => origin + (available - actual) * 0.5,
        Alignment::End => origin + available - actual,
    }
}

#[allow(clippy::cast_precision_loss)]
fn linear_layout(axis: Axis, rect: Rect, items: &[LayoutItem], spacing: f32) -> Vec<Rect> {
    if items.is_empty() {
        return Vec::new();
    }

    let rect = sanitize_rect(rect);
    let spacing = sanitize_size(spacing);
    let main_available = axis_size(axis, rect.size());
    let cross_available = axis_cross_size(axis, rect.size());
    let total_spacing = spacing * (items.len().saturating_sub(1) as f32);
    let available_without_spacing = (main_available - total_spacing).max(0.0);
    let fill_count = items
        .iter()
        .filter(|item| item.main_rule(axis) == SizeRule::Fill)
        .count();

    let reserved = items
        .iter()
        .filter(|item| item.main_rule(axis) != SizeRule::Fill)
        .map(|item| {
            item.main_rule(axis)
                .resolve(main_available, item.measured_main(axis), cross_available)
        })
        .sum::<f32>();

    let fill_size = if fill_count == 0 {
        0.0
    } else {
        ((available_without_spacing - reserved).max(0.0)) / fill_count as f32
    };

    let mut cursor = axis_origin(axis, rect);
    let mut output = Vec::with_capacity(items.len());

    for item in items {
        let main = if item.main_rule(axis) == SizeRule::Fill {
            fill_size
        } else {
            item.main_rule(axis)
                .resolve(main_available, item.measured_main(axis), cross_available)
        };
        let cross = item
            .cross_rule(axis)
            .resolve(cross_available, item.measured_cross(axis), main)
            .min(cross_available)
            .max(0.0);

        output.push(rect_from_axes(axis, rect, cursor, main, cross));
        cursor += main + spacing;
    }

    output
}

fn axis_size(axis: Axis, size: Size) -> f32 {
    match axis {
        Axis::Horizontal => size.width,
        Axis::Vertical => size.height,
    }
}

fn axis_cross_size(axis: Axis, size: Size) -> f32 {
    match axis {
        Axis::Horizontal => size.height,
        Axis::Vertical => size.width,
    }
}

fn axis_origin(axis: Axis, rect: Rect) -> f32 {
    match axis {
        Axis::Horizontal => rect.x,
        Axis::Vertical => rect.y,
    }
}

fn rect_from_axes(axis: Axis, parent: Rect, main_origin: f32, main: f32, cross: f32) -> Rect {
    match axis {
        Axis::Horizontal => Rect::new(main_origin, parent.y, main, cross),
        Axis::Vertical => Rect::new(parent.x, main_origin, cross, main),
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{
        Alignment, Axis, Insets, LayoutItem, Measurement, SeparatorKind, SizeRule, column_layout,
        fit_box, pad_rect, rect_from_size, row_layout, separator, spacer, split_leading,
        split_trailing, stack_layout,
    };
    use crate::{Rect, Size};

    fn item(width: SizeRule, height: SizeRule, measured: Size) -> LayoutItem {
        LayoutItem::new(width, height, Measurement::new(measured))
    }

    #[test]
    fn resolves_size_rules() {
        assert_eq!(SizeRule::Fixed(12.0).resolve(100.0, 20.0, 30.0), 12.0);
        assert_eq!(SizeRule::Fit.resolve(100.0, 20.0, 30.0), 20.0);
        assert_eq!(SizeRule::Fill.resolve(100.0, 20.0, 30.0), 100.0);
        assert_eq!(SizeRule::Percent(0.25).resolve(100.0, 20.0, 30.0), 25.0);
        assert_eq!(
            SizeRule::MinMax {
                min: 10.0,
                max: 30.0
            }
            .resolve(100.0, 50.0, 30.0),
            30.0
        );
        assert_eq!(SizeRule::AspectRatio(2.0).resolve(100.0, 20.0, 30.0), 60.0);
    }

    #[test]
    fn size_rules_sanitize_invalid_constraints() {
        assert_eq!(
            SizeRule::MinMax {
                min: 30.0,
                max: 10.0
            }
            .resolve(100.0, 20.0, 30.0),
            20.0
        );
        assert_eq!(SizeRule::Fixed(f32::NAN).resolve(100.0, 20.0, 30.0), 0.0);
        assert_eq!(SizeRule::Percent(f32::NAN).resolve(100.0, 20.0, 30.0), 0.0);
        assert_eq!(
            SizeRule::AspectRatio(f32::NAN).resolve(100.0, 20.0, 30.0),
            0.0
        );
    }

    #[test]
    fn padding_clamps_negative_inner_size() {
        assert_eq!(
            pad_rect(Rect::new(0.0, 0.0, 10.0, 8.0), Insets::all(6.0)),
            Rect::new(6.0, 6.0, 0.0, 0.0)
        );
    }

    #[test]
    fn rect_from_size_sanitizes_negative_sizes() {
        assert_eq!(
            rect_from_size(Size::new(-10.0, 30.0)),
            Rect::new(0.0, 0.0, 0.0, 30.0)
        );
    }

    #[test]
    fn split_leading_clamps_to_parent() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        assert_eq!(
            split_leading(rect, Axis::Vertical, 12.0),
            (
                Rect::new(10.0, 20.0, 100.0, 12.0),
                Rect::new(10.0, 32.0, 100.0, 38.0)
            )
        );
        assert_eq!(
            split_leading(rect, Axis::Horizontal, 120.0),
            (
                Rect::new(10.0, 20.0, 100.0, 50.0),
                Rect::new(110.0, 20.0, 0.0, 50.0)
            )
        );
    }

    #[test]
    fn split_trailing_clamps_to_parent() {
        let rect = Rect::new(10.0, 20.0, 100.0, 50.0);

        assert_eq!(
            split_trailing(rect, Axis::Horizontal, 15.0),
            (
                Rect::new(10.0, 20.0, 85.0, 50.0),
                Rect::new(95.0, 20.0, 15.0, 50.0)
            )
        );
        assert_eq!(
            split_trailing(rect, Axis::Vertical, 75.0),
            (
                Rect::new(10.0, 20.0, 100.0, 0.0),
                Rect::new(10.0, 20.0, 100.0, 50.0)
            )
        );
    }

    #[test]
    fn fit_box_aligns_content() {
        let rect = Rect::new(0.0, 0.0, 100.0, 50.0);

        assert_eq!(
            fit_box(
                rect,
                Size::new(20.0, 10.0),
                Alignment::Center,
                Alignment::End
            ),
            Rect::new(40.0, 40.0, 20.0, 10.0)
        );
        assert_eq!(
            fit_box(
                rect,
                Size::new(20.0, 10.0),
                Alignment::Stretch,
                Alignment::Stretch
            ),
            rect
        );
    }

    #[test]
    fn stack_layout_repeats_parent_rect() {
        let rect = Rect::new(1.0, 2.0, 3.0, 4.0);

        assert_eq!(stack_layout(rect, 3), vec![rect, rect, rect]);
    }

    #[test]
    fn row_layout_handles_fixed_fit_and_fill() {
        let rect = Rect::new(0.0, 0.0, 100.0, 20.0);
        let items = [
            item(SizeRule::Fixed(20.0), SizeRule::Fill, Size::ZERO),
            item(SizeRule::Fit, SizeRule::Fill, Size::new(10.0, 5.0)),
            item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        ];

        assert_eq!(
            row_layout(rect, &items, 5.0),
            vec![
                Rect::new(0.0, 0.0, 20.0, 20.0),
                Rect::new(25.0, 0.0, 10.0, 20.0),
                Rect::new(40.0, 0.0, 60.0, 20.0),
            ]
        );
    }

    #[test]
    fn column_layout_handles_fixed_fit_and_fill() {
        let rect = Rect::new(0.0, 0.0, 20.0, 100.0);
        let items = [
            item(SizeRule::Fill, SizeRule::Fixed(20.0), Size::ZERO),
            item(SizeRule::Fill, SizeRule::Fit, Size::new(5.0, 10.0)),
            item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        ];

        assert_eq!(
            column_layout(rect, &items, 5.0),
            vec![
                Rect::new(0.0, 0.0, 20.0, 20.0),
                Rect::new(0.0, 25.0, 20.0, 10.0),
                Rect::new(0.0, 40.0, 20.0, 60.0),
            ]
        );
    }

    #[test]
    fn percent_layout_uses_parent_available_space() {
        let rect = Rect::new(0.0, 0.0, 100.0, 10.0);
        let items = [
            item(SizeRule::Percent(0.5), SizeRule::Fill, Size::ZERO),
            item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        ];

        assert_eq!(
            row_layout(rect, &items, 10.0),
            vec![
                Rect::new(0.0, 0.0, 50.0, 10.0),
                Rect::new(60.0, 0.0, 40.0, 10.0),
            ]
        );
    }

    #[test]
    fn spacer_and_separator_have_axis_measurements() {
        assert_eq!(spacer(Axis::Horizontal, 12.0).desired, Size::new(12.0, 0.0));
        assert_eq!(
            separator(SeparatorKind::Vertical, 1.0).desired,
            Size::new(1.0, 0.0)
        );
    }
}
