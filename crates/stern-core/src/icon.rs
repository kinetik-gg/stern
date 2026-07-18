//! Library-neutral immutable vector icon definitions.

use core::{cmp::Ordering, hash::Hash};

use crate::{Color, FillRule, IconId, PathElement, Rect, StrokeCap, StrokeJoin};

/// Stroke metadata stored by an immutable icon path.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconStroke {
    /// Stroke width in icon view-box units.
    pub width: f32,
    /// Shape placed at open path ends.
    pub cap: StrokeCap,
    /// Shape used where path segments meet.
    pub join: StrokeJoin,
}

impl IconStroke {
    /// Creates an icon stroke with explicit style metadata.
    #[must_use]
    pub const fn new(width: f32, cap: StrokeCap, join: StrokeJoin) -> Self {
        Self { width, cap, join }
    }
}

/// One immutable theme-tinted path in an icon graphic.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconPath {
    /// Borrowed path elements in icon view-box coordinates.
    pub elements: &'static [PathElement],
    /// Fill rule when the path has a tint fill; `None` means no fill.
    pub fill: Option<FillRule>,
    /// Optional tint stroke metadata.
    pub stroke: Option<IconStroke>,
    /// Opacity applied to this path before layer opacity.
    pub opacity: f32,
}

impl IconPath {
    /// Creates an immutable icon path.
    #[must_use]
    pub const fn new(
        elements: &'static [PathElement],
        fill: Option<FillRule>,
        stroke: Option<IconStroke>,
        opacity: f32,
    ) -> Self {
        Self {
            elements,
            fill,
            stroke,
            opacity,
        }
    }
}

/// Ordered immutable group of icon paths with shared opacity.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconLayer {
    /// Borrowed paths in paint order.
    pub paths: &'static [IconPath],
    /// Opacity applied to every path in this layer.
    pub opacity: f32,
}

impl IconLayer {
    /// Creates an immutable icon layer.
    #[must_use]
    pub const fn new(paths: &'static [IconPath], opacity: f32) -> Self {
        Self { paths, opacity }
    }
}

/// Immutable backend-independent vector icon data.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconGraphic {
    /// Canonical coordinate space used by every layer and path.
    pub view_box: Rect,
    /// Borrowed layers in paint order.
    pub layers: &'static [IconLayer],
}

impl IconGraphic {
    /// Creates an immutable icon graphic.
    #[must_use]
    pub const fn new(view_box: Rect, layers: &'static [IconLayer]) -> Self {
        Self { view_box, layers }
    }
}

/// Copyable handle to one independently linkable immutable icon definition.
#[derive(Debug, Clone, Copy)]
pub struct StaticIcon {
    id: IconId,
    graphic: &'static IconGraphic,
}

impl StaticIcon {
    /// Creates a static icon handle.
    #[must_use]
    pub const fn new(id: IconId, graphic: &'static IconGraphic) -> Self {
        Self { id, graphic }
    }

    /// Returns the stable icon identity.
    #[must_use]
    pub const fn id(self) -> IconId {
        self.id
    }

    /// Returns the immutable vector graphic.
    #[must_use]
    pub const fn graphic(self) -> &'static IconGraphic {
        self.graphic
    }
}

impl PartialEq for StaticIcon {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl Eq for StaticIcon {}

impl PartialOrd for StaticIcon {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for StaticIcon {
    fn cmp(&self, other: &Self) -> Ordering {
        self.id.cmp(&other.id)
    }
}

impl Hash for StaticIcon {
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state);
    }
}

/// Draw command for a static theme-tinted icon.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IconPrimitive {
    /// Static icon definition.
    pub icon: StaticIcon,
    /// Destination rectangle in logical coordinates.
    pub rect: Rect,
    /// Straight-sRGB tint applied to every fill and stroke.
    pub tint: Color,
}

impl IconPrimitive {
    /// Creates a static icon draw command.
    #[must_use]
    pub const fn new(icon: StaticIcon, rect: Rect, tint: Color) -> Self {
        Self { icon, rect, tint }
    }
}

#[cfg(test)]
#[allow(clippy::float_cmp)]
mod tests {
    use std::collections::{BTreeSet, HashSet};

    use super::{IconGraphic, IconLayer, IconPath, IconStroke, StaticIcon};
    use crate::{FillRule, IconId, PathElement, Point, Rect, StrokeCap, StrokeJoin};

    static ELEMENTS: [PathElement; 2] = [
        PathElement::MoveTo(Point::new(0.0, 0.0)),
        PathElement::LineTo(Point::new(16.0, 16.0)),
    ];
    static PATHS: [IconPath; 1] = [IconPath::new(
        &ELEMENTS,
        Some(FillRule::EvenOdd),
        Some(IconStroke::new(1.5, StrokeCap::Round, StrokeJoin::Bevel)),
        0.75,
    )];
    static LAYERS: [IconLayer; 1] = [IconLayer::new(&PATHS, 0.5)];
    static GRAPHIC: IconGraphic = IconGraphic::new(Rect::new(0.0, 0.0, 16.0, 16.0), &LAYERS);
    static OTHER_GRAPHIC: IconGraphic = IconGraphic::new(Rect::new(0.0, 0.0, 1.0, 1.0), &[]);

    #[test]
    fn graphic_retains_borrowed_ordered_style_metadata() {
        let icon = StaticIcon::new(IconId::from_raw(7), &GRAPHIC);
        let path = icon.graphic().layers[0].paths[0];

        assert!(core::ptr::eq(path.elements, ELEMENTS.as_slice()));
        assert_eq!(path.fill, Some(FillRule::EvenOdd));
        assert_eq!(path.opacity, 0.75);
        assert_eq!(path.stroke.expect("stroke").cap, StrokeCap::Round);
        assert_eq!(icon.graphic().layers[0].opacity, 0.5);
    }

    #[test]
    fn handle_identity_ignores_graphic_float_data() {
        let first = StaticIcon::new(IconId::from_raw(7), &GRAPHIC);
        let same_id = StaticIcon::new(IconId::from_raw(7), &OTHER_GRAPHIC);
        let distinct = StaticIcon::new(IconId::from_raw(8), &GRAPHIC);

        assert_eq!(first, same_id);
        assert_ne!(first, distinct);
        assert_eq!(HashSet::from([first, same_id, distinct]).len(), 2);
        assert_eq!(BTreeSet::from([distinct, first, same_id]).len(), 2);
        assert_eq!(
            BTreeSet::from([distinct, first]).into_iter().next(),
            Some(first)
        );
    }

    #[test]
    fn empty_graphics_require_no_registration_or_catalog() {
        let icon = StaticIcon::new(IconId::from_raw(1), &OTHER_GRAPHIC);

        assert!(icon.graphic().layers.is_empty());
    }
}
