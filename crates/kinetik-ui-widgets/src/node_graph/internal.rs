#[allow(clippy::wildcard_imports)]
use super::*;

#[allow(clippy::cast_precision_loss)]
pub(crate) fn port_label_width(label: &str, style: &NodeGraphStyle) -> f32 {
    label.chars().count() as f32 * style.port_label_size * 0.55
}

pub(crate) fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
    )
}

pub(crate) fn normalize_screen_rect(rect: Rect) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
    {
        return Rect::ZERO;
    }

    let min = sanitize_point(rect.origin());
    let max = sanitize_point(Point::new(
        finite_rect_extent(rect.x, rect.width),
        finite_rect_extent(rect.y, rect.height),
    ));
    Rect::from_min_max(
        Point::new(min.x.min(max.x), min.y.min(max.y)),
        Point::new(min.x.max(max.x), min.y.max(max.y)),
    )
}

pub(crate) fn sanitize_point(point: Point) -> Point {
    Point::new(finite_or_zero(point.x), finite_or_zero(point.y))
}

pub(crate) fn sanitize_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() && zoom > 0.0 {
        zoom.max(MIN_ZOOM)
    } else {
        DEFAULT_ZOOM
    }
}

pub(crate) fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

pub(crate) fn finite_sum(lhs: f32, rhs: f32) -> f32 {
    let sum = lhs + rhs;
    if sum.is_finite() {
        sum
    } else if sum.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

pub(crate) fn finite_product(lhs: f32, rhs: f32) -> f32 {
    let product = lhs * rhs;
    if product.is_finite() {
        product
    } else if product.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

pub(crate) fn finite_div(lhs: f32, rhs: f32) -> f32 {
    let quotient = lhs / rhs;
    finite_or_zero(quotient)
}

pub(crate) fn finite_rect_extent(origin: f32, size: f32) -> f32 {
    if origin.is_finite() && size.is_finite() {
        finite_or_zero(origin + size)
    } else {
        0.0
    }
}

pub(crate) fn box_selection_operations(
    intent: NodeGraphSelectionIntent,
    targets: &[NodeGraphSelectionTarget],
) -> Vec<NodeGraphSelectionOperation> {
    match intent {
        NodeGraphSelectionIntent::Replace => {
            let Some((first, rest)) = targets.split_first() else {
                return vec![NodeGraphSelectionOperation::Clear];
            };

            let mut operations = Vec::with_capacity(targets.len());
            operations.push(NodeGraphSelectionOperation::Replace(*first));
            operations.extend(
                rest.iter()
                    .copied()
                    .map(NodeGraphSelectionOperation::Extend),
            );
            operations
        }
        NodeGraphSelectionIntent::Add => targets
            .iter()
            .copied()
            .map(NodeGraphSelectionOperation::Extend)
            .collect(),
        NodeGraphSelectionIntent::Subtract => targets
            .iter()
            .copied()
            .map(NodeGraphSelectionOperation::Remove)
            .collect(),
    }
}

pub(crate) fn effective_snap_grid_size(grid_size: f32) -> Option<f32> {
    (grid_size.is_finite() && grid_size > 0.0).then_some(grid_size)
}

pub(crate) fn snap_graph_component(value: f32, grid_size: f32) -> f32 {
    finite_product((finite_div(value, grid_size)).round(), grid_size)
}
