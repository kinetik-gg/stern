//! Scrollbar geometry and paint for virtualized collections.
//!
//! Implements the `docs/visual-spec/06-collections.md` §Virtualized viewport
//! scrollbar recipe (KNOWN-GAPS #47): a 6px-wide thumb in `border.strong`
//! with `radius.full` ends, a transparent track (no track primitive), inset
//! 2 from the viewport edge. The thumb is pure chrome — it owns no
//! interaction; scrolling stays input-driven.

use stern_core::{Primitive, Rect, RectPrimitive, Theme};

/// Thumb width per 06-collections.md ("Scrollbar: 6 wide thumb").
pub const THUMB_WIDTH: f32 = 6.0;
/// Inset from the viewport's right/top/bottom edges per the same section
/// ("track transparent, inset 2").
pub const EDGE_INSET: f32 = 2.0;
/// Shortest rendered thumb length. The spec fixes no floor; without one an
/// arbitrarily long collection renders a sub-pixel sliver, so the thumb is
/// floored at twice its width — enough to stay visible while remaining a
/// deterministic pure function of the window.
pub const MIN_THUMB_LENGTH: f32 = 12.0;

/// Geometry of the vertical scrollbar thumb for a scrolled viewport.
///
/// Returns `None` when there is nothing to scroll (`content_height` does not
/// exceed the viewport, or either extent is non-finite/non-positive) — a
/// fitting viewport paints no thumb.
#[must_use]
pub fn vertical_thumb(viewport: Rect, content_height: f32, scroll_offset: f32) -> Option<Rect> {
    let track_length = viewport.height - EDGE_INSET * 2.0;
    if !viewport.height.is_finite()
        || !viewport.width.is_finite()
        || !content_height.is_finite()
        || content_height <= viewport.height.max(0.0)
        || track_length <= 0.0
    {
        return None;
    }

    let max_scroll_offset = content_height - viewport.height;
    let fraction = if scroll_offset.is_finite() && scroll_offset > 0.0 {
        (scroll_offset / max_scroll_offset).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let visible_fraction = viewport.height / content_height;
    let thumb_length =
        (track_length * visible_fraction).clamp(MIN_THUMB_LENGTH.min(track_length), track_length);

    let x = viewport.max_x() - EDGE_INSET - THUMB_WIDTH;
    let y = viewport.y + EDGE_INSET + fraction * (track_length - thumb_length);
    Some(Rect::new(x, y, THUMB_WIDTH, thumb_length))
}

/// Emits the scrollbar thumb primitives for a virtualized-collection
/// viewport, resolving the recipe colors through `theme`. A fitting or
/// invalid viewport emits nothing.
///
/// Call this after the content clip closes so the thumb stays fixed to the
/// container instead of translating with the scroll offset.
pub fn push_vertical_thumb(
    out: &mut Vec<Primitive>,
    theme: &Theme,
    viewport: Rect,
    content_height: f32,
    scroll_offset: f32,
) {
    if let Some(rect) = vertical_thumb(viewport, content_height, scroll_offset) {
        out.push(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(stern_core::Brush::Solid(theme.colors.border.strong)),
            stroke: None,
            radius: theme.radii.full,
        }));
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use super::{EDGE_INSET, MIN_THUMB_LENGTH, THUMB_WIDTH, vertical_thumb};
    use stern_core::Rect;

    const VIEWPORT: Rect = Rect::new(10.0, 20.0, 200.0, 100.0);

    #[test]
    fn fitting_or_invalid_content_hides_the_thumb() {
        assert_eq!(vertical_thumb(VIEWPORT, 100.0, 0.0), None);
        assert_eq!(vertical_thumb(VIEWPORT, 50.0, 40.0), None);
        // Exactly-fitting content has no overflow.
        assert_eq!(vertical_thumb(VIEWPORT, VIEWPORT.height, 1.0), None);
        assert_eq!(vertical_thumb(VIEWPORT, f32::NAN, 0.0), None);
        assert_eq!(vertical_thumb(VIEWPORT, f32::INFINITY, 0.0), None);
        assert_eq!(
            vertical_thumb(Rect::new(10.0, 20.0, 200.0, f32::NAN), 500.0, 0.0),
            None
        );
    }

    #[test]
    fn thumb_geometry_is_inset_proportional_and_clamped() {
        let content = 400.0;
        let top = vertical_thumb(VIEWPORT, content, 0.0).expect("thumb at top");
        assert_eq!(top.width, THUMB_WIDTH);
        assert_eq!(top.x, VIEWPORT.max_x() - EDGE_INSET - THUMB_WIDTH);
        assert_eq!(top.y, VIEWPORT.y + EDGE_INSET);

        // Visible fraction of a 100-tall viewport over 400 content is 1/4
        // of the 96px track.
        assert!((top.height - (VIEWPORT.height - 4.0) * 0.25).abs() < f32::EPSILON);

        // Half-scroll moves the thumb halfway through the free track.
        let mid = vertical_thumb(VIEWPORT, content, 150.0).expect("thumb at middle");
        let travel = (VIEWPORT.height - 4.0) - top.height;
        assert!((mid.y - (VIEWPORT.y + EDGE_INSET + travel * 0.5)).abs() < f32::EPSILON);

        // Overscroll clamps the thumb to the bottom inset.
        let bottom = vertical_thumb(VIEWPORT, content, 10_000.0).expect("clamped thumb");
        assert_eq!(bottom.max_y(), VIEWPORT.max_y() - EDGE_INSET);

        // Negative offsets clamp to the top inset.
        let negative = vertical_thumb(VIEWPORT, content, -30.0).expect("clamped thumb");
        assert_eq!(negative, top);
    }

    #[test]
    fn very_long_collections_floor_the_thumb_length() {
        let thumb = vertical_thumb(VIEWPORT, 1_000_000.0, 0.0).expect("thumb");
        assert_eq!(thumb.height, MIN_THUMB_LENGTH);
    }
}
