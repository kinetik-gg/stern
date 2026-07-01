use kinetik_ui_core::{Rect, Size};

use super::{OverlayEntry, OverlayId, OverlayKind};

/// Preferred popover placement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlacement {
    /// Place below the anchor.
    Below,
    /// Place above the anchor.
    Above,
    /// Place to the right of the anchor.
    Right,
    /// Place to the left of the anchor.
    Left,
}

/// Popover positioning request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PopoverRequest {
    /// Anchor rectangle.
    pub anchor: Rect,
    /// Overlay size.
    pub size: Size,
    /// Preferred placement.
    pub placement: PopoverPlacement,
    /// Gap between anchor and overlay.
    pub offset: f32,
    /// Whether the output should be clamped inside viewport bounds.
    pub fit_viewport: bool,
}

/// Places a popover in logical units.
#[must_use]
pub fn place_popover(request: PopoverRequest, viewport: Rect) -> Rect {
    let viewport = sanitize_rect(viewport).max_zero();
    let size = Size::new(
        sanitize_extent(request.size.width),
        sanitize_extent(request.size.height),
    );
    let anchor = sanitize_rect(request.anchor);
    let offset = sanitize_extent(request.offset);
    let preferred = popover_rect(anchor, size, request.placement, offset);
    if !request.fit_viewport {
        return preferred;
    }

    let mut rect = preferred;
    for placement in placement_candidates(request.placement) {
        let candidate = popover_rect(anchor, size, placement, offset);
        let adjusted = clamp_popover_cross_axis(candidate, placement, viewport);
        if placement_axis_fits(candidate, placement, viewport) && viewport.contains_rect(adjusted) {
            rect = adjusted;
            break;
        }
    }

    clamp_rect_to_viewport(rect, viewport)
}

pub(crate) fn placed_entry(
    id: OverlayId,
    kind: OverlayKind,
    request: PopoverRequest,
    viewport: Rect,
) -> OverlayEntry {
    OverlayEntry::new(id, kind, place_popover(request, viewport))
}

fn popover_rect(anchor: Rect, size: Size, placement: PopoverPlacement, offset: f32) -> Rect {
    match placement {
        PopoverPlacement::Below => {
            Rect::new(anchor.x, anchor.max_y() + offset, size.width, size.height)
        }
        PopoverPlacement::Above => Rect::new(
            anchor.x,
            anchor.y - offset - size.height,
            size.width,
            size.height,
        ),
        PopoverPlacement::Right => {
            Rect::new(anchor.max_x() + offset, anchor.y, size.width, size.height)
        }
        PopoverPlacement::Left => Rect::new(
            anchor.x - offset - size.width,
            anchor.y,
            size.width,
            size.height,
        ),
    }
}

fn placement_candidates(preferred: PopoverPlacement) -> [PopoverPlacement; 4] {
    match preferred {
        PopoverPlacement::Below => [
            PopoverPlacement::Below,
            PopoverPlacement::Above,
            PopoverPlacement::Right,
            PopoverPlacement::Left,
        ],
        PopoverPlacement::Above => [
            PopoverPlacement::Above,
            PopoverPlacement::Below,
            PopoverPlacement::Right,
            PopoverPlacement::Left,
        ],
        PopoverPlacement::Right => [
            PopoverPlacement::Right,
            PopoverPlacement::Left,
            PopoverPlacement::Below,
            PopoverPlacement::Above,
        ],
        PopoverPlacement::Left => [
            PopoverPlacement::Left,
            PopoverPlacement::Right,
            PopoverPlacement::Below,
            PopoverPlacement::Above,
        ],
    }
}

fn placement_axis_fits(rect: Rect, placement: PopoverPlacement, viewport: Rect) -> bool {
    match placement {
        PopoverPlacement::Below | PopoverPlacement::Above => {
            rect.y >= viewport.y && rect.max_y() <= viewport.max_y()
        }
        PopoverPlacement::Right | PopoverPlacement::Left => {
            rect.x >= viewport.x && rect.max_x() <= viewport.max_x()
        }
    }
}

fn clamp_popover_cross_axis(rect: Rect, placement: PopoverPlacement, viewport: Rect) -> Rect {
    match placement {
        PopoverPlacement::Below | PopoverPlacement::Above => Rect::new(
            clamp_origin(rect.x, rect.width, viewport.x, viewport.max_x()),
            rect.y,
            rect.width,
            rect.height,
        ),
        PopoverPlacement::Right | PopoverPlacement::Left => Rect::new(
            rect.x,
            clamp_origin(rect.y, rect.height, viewport.y, viewport.max_y()),
            rect.width,
            rect.height,
        ),
    }
}

fn clamp_rect_to_viewport(rect: Rect, viewport: Rect) -> Rect {
    Rect::new(
        clamp_origin(rect.x, rect.width, viewport.x, viewport.max_x()),
        clamp_origin(rect.y, rect.height, viewport.y, viewport.max_y()),
        rect.width,
        rect.height,
    )
}

fn clamp_origin(origin: f32, extent: f32, min: f32, max: f32) -> f32 {
    let max_origin = max - extent;
    if max_origin < min {
        min
    } else {
        origin.clamp(min, max_origin)
    }
}

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        sanitize_coordinate(rect.x),
        sanitize_coordinate(rect.y),
        sanitize_extent(rect.width),
        sanitize_extent(rect.height),
    )
}

fn sanitize_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn sanitize_extent(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}
