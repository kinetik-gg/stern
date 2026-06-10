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
        match self {
            Self::Fixed(value) => value,
            Self::Fit => measured,
            Self::Fill => available,
            Self::Percent(value) => available * value,
            Self::MinMax { min, max } => measured.clamp(min, max),
            Self::AspectRatio(ratio) => cross * ratio,
        }
        .max(0.0)
    }
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
    Rect::new(
        rect.x + insets.left,
        rect.y + insets.top,
        rect.width - insets.left - insets.right,
        rect.height - insets.top - insets.bottom,
    )
    .max_zero()
}

/// Fits a size inside a rectangle according to alignment.
#[must_use]
pub fn fit_box(rect: Rect, size: Size, horizontal: Alignment, vertical: Alignment) -> Rect {
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
    match axis {
        Axis::Horizontal => Measurement::new(Size::new(amount, 0.0)),
        Axis::Vertical => Measurement::new(Size::new(0.0, amount)),
    }
}

/// Returns the measurement of a separator.
#[must_use]
pub fn separator(kind: SeparatorKind, thickness: f32) -> Measurement {
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

    let main_available = axis_size(axis, rect.size());
    let cross_available = axis_cross_size(axis, rect.size());
    let total_spacing = spacing.max(0.0) * (items.len().saturating_sub(1) as f32);
    let available_without_spacing = (main_available - total_spacing).max(0.0);
    let fill_count = items
        .iter()
        .filter(|item| item.main_rule(axis) == SizeRule::Fill)
        .count();

    let reserved = items
        .iter()
        .filter(|item| item.main_rule(axis) != SizeRule::Fill)
        .map(|item| {
            item.main_rule(axis).resolve(
                available_without_spacing,
                item.measured_main(axis),
                cross_available,
            )
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
            item.main_rule(axis).resolve(
                available_without_spacing,
                item.measured_main(axis),
                cross_available,
            )
        };
        let cross = item
            .cross_rule(axis)
            .resolve(cross_available, item.measured_cross(axis), main)
            .min(cross_available)
            .max(0.0);

        output.push(rect_from_axes(axis, rect, cursor, main, cross));
        cursor += main + spacing.max(0.0);
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
        fit_box, pad_rect, row_layout, separator, spacer, stack_layout,
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
    fn padding_clamps_negative_inner_size() {
        assert_eq!(
            pad_rect(Rect::new(0.0, 0.0, 10.0, 8.0), Insets::all(6.0)),
            Rect::new(6.0, 6.0, 0.0, 0.0)
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
    fn percent_layout_uses_available_space_after_spacing() {
        let rect = Rect::new(0.0, 0.0, 100.0, 10.0);
        let items = [
            item(SizeRule::Percent(0.5), SizeRule::Fill, Size::ZERO),
            item(SizeRule::Fill, SizeRule::Fill, Size::ZERO),
        ];

        assert_eq!(
            row_layout(rect, &items, 10.0),
            vec![
                Rect::new(0.0, 0.0, 45.0, 10.0),
                Rect::new(55.0, 0.0, 45.0, 10.0),
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
