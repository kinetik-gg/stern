//! Exact public focus-ring recipe and geometry conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    Brush, Color, CornerRadius, FocusRingRecipe, PathElement, PathPrimitive, Point, Primitive,
    Rect, Stroke, StrokeScale, ThemeColors, default_dark_theme,
};

const KAPPA: f32 = 0.552_284_8;

fn sentinel_recipe(primary_width: f32, separator_width: f32) -> FocusRingRecipe {
    FocusRingRecipe {
        primary: Stroke::new(primary_width, Brush::Solid(Color::rgb8(0x12, 0x34, 0x56))),
        separator: Stroke::new(separator_width, Brush::Solid(Color::rgb8(0xA1, 0xB2, 0xC3))),
    }
}

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("focus annulus must be a path primitive");
    };
    path
}

fn assert_clockwise_contour(elements: &[PathElement], rect: Rect, radius: CornerRadius) {
    assert_eq!(elements.len(), 10);
    let min_x = rect.min_x();
    let min_y = rect.min_y();
    let max_x = rect.max_x();
    let max_y = rect.max_y();
    assert_eq!(
        elements,
        [
            PathElement::MoveTo(Point::new(min_x + radius.top_left, min_y)),
            PathElement::LineTo(Point::new(max_x - radius.top_right, min_y)),
            PathElement::CubicTo {
                ctrl1: Point::new(max_x - radius.top_right * (1.0 - KAPPA), min_y),
                ctrl2: Point::new(max_x, min_y + radius.top_right * (1.0 - KAPPA)),
                to: Point::new(max_x, min_y + radius.top_right),
            },
            PathElement::LineTo(Point::new(max_x, max_y - radius.bottom_right)),
            PathElement::CubicTo {
                ctrl1: Point::new(max_x, max_y - radius.bottom_right * (1.0 - KAPPA),),
                ctrl2: Point::new(max_x - radius.bottom_right * (1.0 - KAPPA), max_y,),
                to: Point::new(max_x - radius.bottom_right, max_y),
            },
            PathElement::LineTo(Point::new(min_x + radius.bottom_left, max_y)),
            PathElement::CubicTo {
                ctrl1: Point::new(min_x + radius.bottom_left * (1.0 - KAPPA), max_y,),
                ctrl2: Point::new(min_x, max_y - radius.bottom_left * (1.0 - KAPPA),),
                to: Point::new(min_x, max_y - radius.bottom_left),
            },
            PathElement::LineTo(Point::new(min_x, min_y + radius.top_left)),
            PathElement::CubicTo {
                ctrl1: Point::new(min_x, min_y + radius.top_left * (1.0 - KAPPA)),
                ctrl2: Point::new(min_x + radius.top_left * (1.0 - KAPPA), min_y),
                to: Point::new(min_x + radius.top_left, min_y),
            },
            PathElement::Close,
        ]
    );
}

fn assert_counter_clockwise_contour(elements: &[PathElement], rect: Rect, radius: CornerRadius) {
    assert_eq!(elements.len(), 10);
    let min_x = rect.min_x();
    let min_y = rect.min_y();
    let max_x = rect.max_x();
    let max_y = rect.max_y();
    assert_eq!(
        elements,
        [
            PathElement::MoveTo(Point::new(min_x + radius.top_left, min_y)),
            PathElement::CubicTo {
                ctrl1: Point::new(min_x + radius.top_left * (1.0 - KAPPA), min_y),
                ctrl2: Point::new(min_x, min_y + radius.top_left * (1.0 - KAPPA)),
                to: Point::new(min_x, min_y + radius.top_left),
            },
            PathElement::LineTo(Point::new(min_x, max_y - radius.bottom_left)),
            PathElement::CubicTo {
                ctrl1: Point::new(min_x, max_y - radius.bottom_left * (1.0 - KAPPA),),
                ctrl2: Point::new(min_x + radius.bottom_left * (1.0 - KAPPA), max_y,),
                to: Point::new(min_x + radius.bottom_left, max_y),
            },
            PathElement::LineTo(Point::new(max_x - radius.bottom_right, max_y)),
            PathElement::CubicTo {
                ctrl1: Point::new(max_x - radius.bottom_right * (1.0 - KAPPA), max_y,),
                ctrl2: Point::new(max_x, max_y - radius.bottom_right * (1.0 - KAPPA),),
                to: Point::new(max_x, max_y - radius.bottom_right),
            },
            PathElement::LineTo(Point::new(max_x, min_y + radius.top_right)),
            PathElement::CubicTo {
                ctrl1: Point::new(max_x, min_y + radius.top_right * (1.0 - KAPPA)),
                ctrl2: Point::new(max_x - radius.top_right * (1.0 - KAPPA), min_y),
                to: Point::new(max_x - radius.top_right, min_y),
            },
            PathElement::LineTo(Point::new(min_x + radius.top_left, min_y)),
            PathElement::Close,
        ]
    );
}

fn point_on_cubic(from: Point, ctrl1: Point, ctrl2: Point, to: Point, t: f32) -> Point {
    let mt = 1.0 - t;
    Point::new(
        mt * mt * mt * from.x
            + 3.0 * mt * mt * t * ctrl1.x
            + 3.0 * mt * t * t * ctrl2.x
            + t * t * t * to.x,
        mt * mt * mt * from.y
            + 3.0 * mt * mt * t * ctrl1.y
            + 3.0 * mt * t * t * ctrl2.y
            + t * t * t * to.y,
    )
}

fn flattened_segments(elements: &[PathElement]) -> Vec<(Point, Point)> {
    let mut segments = Vec::new();
    let mut current = Point::ZERO;
    let mut start = Point::ZERO;
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                current = point;
                start = point;
            }
            PathElement::LineTo(point) => {
                segments.push((current, point));
                current = point;
            }
            PathElement::QuadTo { ctrl, to } => {
                let from = current;
                for step in 1_u8..=32 {
                    let t = f32::from(step) / 32.0;
                    let mt = 1.0 - t;
                    let point = Point::new(
                        mt * mt * from.x + 2.0 * mt * t * ctrl.x + t * t * to.x,
                        mt * mt * from.y + 2.0 * mt * t * ctrl.y + t * t * to.y,
                    );
                    segments.push((current, point));
                    current = point;
                }
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                let from = current;
                for step in 1_u8..=32 {
                    let point = point_on_cubic(from, ctrl1, ctrl2, to, f32::from(step) / 32.0);
                    segments.push((current, point));
                    current = point;
                }
            }
            PathElement::Close => {
                segments.push((current, start));
                current = start;
            }
        }
    }
    segments
}

fn winding_at(elements: &[PathElement], point: Point) -> i32 {
    flattened_segments(elements)
        .into_iter()
        .fold(0, |winding, (from, to)| {
            let cross = (to.x - from.x) * (point.y - from.y) - (point.x - from.x) * (to.y - from.y);
            if from.y <= point.y && to.y > point.y && cross > 0.0 {
                winding + 1
            } else if from.y > point.y && to.y <= point.y && cross < 0.0 {
                winding - 1
            } else {
                winding
            }
        })
}

fn path_bounds(elements: &[PathElement]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in elements.iter().flat_map(|element| match *element {
        PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
        PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
        PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
        PathElement::Close => Vec::new(),
    }) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

#[test]
fn focus_ring_visibility_and_default_tokens_are_exact() {
    let theme = default_dark_theme();

    assert_eq!(theme.focus_ring(false), None);
    let recipe: FocusRingRecipe = theme.focus_ring(true).expect("visible focus ring");
    assert_eq!(recipe.primary.width, 1.0);
    assert_eq!(
        recipe.primary.brush,
        Brush::Solid(Color::rgb8(0x4D, 0xB2, 0xFF))
    );
    assert_eq!(recipe.separator.width, 1.0);
    assert_eq!(
        recipe.separator.brush,
        Brush::Solid(Color::rgb8(0x0B, 0x0B, 0x0B))
    );
}

#[test]
fn focus_ring_uses_distinct_sentinel_tokens_and_exact_nonuniform_geometry() {
    let mut colors = ThemeColors::default_dark();
    colors.focus.indicator = Color::rgb8(0x12, 0x34, 0x56);
    colors.focus.separator = Color::rgb8(0xA1, 0xB2, 0xC3);
    let theme = default_dark_theme()
        .with_colors(colors)
        .with_strokes(StrokeScale::from_values(0.5, 1.5, 2.5, 3.5, 4.5));
    let recipe = theme.focus_ring(true).expect("visible focus ring");

    assert_eq!(recipe.primary.width, 3.5);
    assert_eq!(
        recipe.primary.brush,
        Brush::Solid(Color::rgb8(0x12, 0x34, 0x56))
    );
    assert_eq!(recipe.separator.width, 4.5);
    assert_eq!(
        recipe.separator.brush,
        Brush::Solid(Color::rgb8(0xA1, 0xB2, 0xC3))
    );

    let rect = Rect::new(10.0, 20.0, 30.0, 40.0);
    let radius = CornerRadius {
        top_left: 1.0,
        top_right: 2.0,
        bottom_right: 3.0,
        bottom_left: 4.0,
    };
    let [primary, separator] = recipe.primitives(rect, radius);
    let Primitive::Rect(primary) = primary else {
        panic!("expected primary rectangle");
    };
    let Primitive::Rect(separator) = separator else {
        panic!("expected separator rectangle");
    };

    assert_eq!(primary.rect, Rect::new(2.0, 12.0, 46.0, 56.0));
    assert_eq!(primary.fill, Some(recipe.primary.brush));
    assert_eq!(primary.stroke, None);
    assert_eq!(
        primary.radius,
        CornerRadius {
            top_left: 9.0,
            top_right: 10.0,
            bottom_right: 11.0,
            bottom_left: 12.0,
        }
    );
    assert_eq!(separator.rect, Rect::new(5.5, 15.5, 39.0, 49.0));
    assert_eq!(separator.fill, Some(recipe.separator.brush));
    assert_eq!(separator.stroke, None);
    assert_eq!(
        separator.radius,
        CornerRadius {
            top_left: 5.5,
            top_right: 6.5,
            bottom_right: 7.5,
            bottom_left: 8.5,
        }
    );
}

#[test]
fn focused_choice_and_slider_recipes_retain_neutral_base_borders() {
    use stern_core::ComponentState;

    let theme = default_dark_theme();
    let focused = ComponentState {
        focused: true,
        ..ComponentState::default()
    };
    let unfocused = ComponentState::default();

    assert_eq!(
        theme.checkbox(focused).border,
        theme.checkbox(unfocused).border
    );
    assert_eq!(
        theme.radio_button(focused).border,
        theme.radio_button(unfocused).border
    );
    assert_eq!(theme.toggle(focused).border, theme.toggle(unfocused).border);
    assert_eq!(theme.slider(focused).border, theme.slider(unfocused).border);
    assert_eq!(
        theme.checkbox(focused).border.brush,
        Brush::Solid(theme.colors.border.default)
    );
}

#[test]
fn outward_annuli_are_exact_hollow_primary_then_separator_paths() {
    let recipe = sentinel_recipe(2.25, 1.5);
    let rect = Rect::new(10.25, 20.5, 30.75, 40.25);
    let radius = CornerRadius {
        top_left: 3.0,
        top_right: 4.0,
        bottom_right: 5.0,
        bottom_left: 6.0,
    };
    let primitives = recipe.outward_annulus_primitives(rect, radius);
    let primary = path(&primitives[0]);
    let separator = path(&primitives[1]);

    assert_eq!(primary.fill, Some(recipe.primary.brush));
    assert_eq!(primary.stroke, None);
    assert_eq!(separator.fill, Some(recipe.separator.brush));
    assert_eq!(separator.stroke, None);
    assert_eq!(primary.elements.len(), 20);
    assert_eq!(separator.elements.len(), 20);
    assert_eq!(
        primary
            .elements
            .iter()
            .filter(|element| matches!(element, PathElement::Close))
            .count(),
        2
    );

    assert_clockwise_contour(
        &primary.elements[..10],
        Rect::new(6.5, 16.75, 38.25, 47.75),
        CornerRadius {
            top_left: 6.75,
            top_right: 7.75,
            bottom_right: 8.75,
            bottom_left: 9.75,
        },
    );
    assert_clockwise_contour(
        &separator.elements[..10],
        Rect::new(8.75, 19.0, 33.75, 43.25),
        CornerRadius {
            top_left: 4.5,
            top_right: 5.5,
            bottom_right: 6.5,
            bottom_left: 7.5,
        },
    );
    assert_counter_clockwise_contour(&primary.elements[10..], rect, radius);
    assert_counter_clockwise_contour(&separator.elements[10..], rect, radius);
    assert_eq!(primary.elements[10..], separator.elements[10..]);

    let interior = rect.center();
    let primary_band = Point::new(rect.min_x() - 1.5 - 2.25 * 0.5, interior.y);
    let separator_band = Point::new(rect.min_x() - 1.5 * 0.5, interior.y);
    assert_eq!(winding_at(&primary.elements, interior), 0);
    assert_eq!(winding_at(&separator.elements, interior), 0);
    assert_ne!(winding_at(&primary.elements, primary_band), 0);
    assert_eq!(winding_at(&separator.elements, primary_band), 0);
    assert_ne!(winding_at(&primary.elements, separator_band), 0);
    assert_ne!(winding_at(&separator.elements, separator_band), 0);
}

#[test]
fn transparent_ghost_composition_keeps_the_component_interior_unpainted() {
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let rect = Rect::new(3.25, 5.75, 30.5, 18.25);
    let primitives = recipe.outward_annulus_primitives(rect, CornerRadius::all(4.0));
    assert_clockwise_contour(
        &path(&primitives[0]).elements[..10],
        Rect::new(1.25, 3.75, 34.5, 22.25),
        CornerRadius::all(6.0),
    );
    assert_clockwise_contour(
        &path(&primitives[1]).elements[..10],
        Rect::new(2.25, 4.75, 32.5, 20.25),
        CornerRadius::all(5.0),
    );
    assert_counter_clockwise_contour(
        &path(&primitives[0]).elements[10..],
        rect,
        CornerRadius::all(4.0),
    );

    for interior in [
        rect.center(),
        Point::new(rect.min_x() + 8.0, rect.min_y() + 6.0),
    ] {
        assert_eq!(winding_at(&path(&primitives[0]).elements, interior), 0);
        assert_eq!(winding_at(&path(&primitives[1]).elements, interior), 0);
    }

    let inward = recipe.inward_annulus_primitives(rect, CornerRadius::all(4.0), 1.0);
    assert_clockwise_contour(
        &path(&inward[0]).elements[..10],
        Rect::new(4.25, 6.75, 28.5, 16.25),
        CornerRadius::all(3.0),
    );
    assert_clockwise_contour(
        &path(&inward[1]).elements[..10],
        Rect::new(5.25, 7.75, 26.5, 14.25),
        CornerRadius::all(2.0),
    );
    assert_counter_clockwise_contour(
        &path(&inward[0]).elements[10..],
        Rect::new(6.25, 8.75, 24.5, 12.25),
        CornerRadius::all(1.0),
    );
}

#[test]
fn inward_annuli_use_full_boundary_width_and_remain_inside_component_clips() {
    let recipe = sentinel_recipe(2.25, 1.5);
    let rect = Rect::new(10.25, 20.5, 30.75, 40.25);
    let radius = CornerRadius {
        top_left: 3.0,
        top_right: 4.0,
        bottom_right: 5.0,
        bottom_left: 6.0,
    };
    let primitives = recipe.inward_annulus_primitives(rect, radius, 1.25);
    let primary = path(&primitives[0]);
    let separator = path(&primitives[1]);

    assert_clockwise_contour(
        &primary.elements[..10],
        Rect::new(11.5, 21.75, 28.25, 37.75),
        CornerRadius {
            top_left: 1.75,
            top_right: 2.75,
            bottom_right: 3.75,
            bottom_left: 4.75,
        },
    );
    assert_clockwise_contour(
        &separator.elements[..10],
        Rect::new(13.75, 24.0, 23.75, 33.25),
        CornerRadius {
            top_left: 0.0,
            top_right: 0.5,
            bottom_right: 1.5,
            bottom_left: 2.5,
        },
    );
    let inner_rect = Rect::new(15.25, 25.5, 20.75, 30.25);
    let inner_radius = CornerRadius {
        top_left: 0.0,
        top_right: 0.0,
        bottom_right: 0.0,
        bottom_left: 1.0,
    };
    assert_counter_clockwise_contour(&primary.elements[10..], inner_rect, inner_radius);
    assert_counter_clockwise_contour(&separator.elements[10..], inner_rect, inner_radius);
    assert_eq!(primary.elements[10..], separator.elements[10..]);
    assert_eq!(primary.fill, Some(recipe.primary.brush));
    assert_eq!(primary.stroke, None);
    assert_eq!(separator.fill, Some(recipe.separator.brush));
    assert_eq!(separator.stroke, None);

    for edge_rect in [
        Rect::new(0.0, 0.0, 30.0, 16.0),
        Rect::new(70.0, 34.0, 30.0, 16.0),
    ] {
        for primitive in recipe.inward_annulus_primitives(edge_rect, CornerRadius::all(8.0), 1.25) {
            assert!(edge_rect.contains_rect(path_bounds(&path(&primitive).elements)));
        }
    }
}

#[test]
fn annulus_sanitization_normalizes_radii_and_collapses_tiny_geometry_finitely() {
    let recipe = sentinel_recipe(f32::NAN, -4.0);
    let radius = CornerRadius {
        top_left: f32::INFINITY,
        top_right: -1.0,
        bottom_right: 100.0,
        bottom_left: 100.0,
    };
    let outward = recipe.outward_annulus_primitives(Rect::new(10.0, 20.0, -4.0, -6.0), radius);
    for primitive in outward {
        let path = path(&primitive);
        assert_eq!(path.elements.len(), 20);
        assert!(path.fill.is_some());
        assert_eq!(path.stroke, None);
        for (from, to) in flattened_segments(&path.elements) {
            assert!(from.x.is_finite() && from.y.is_finite());
            assert!(to.x.is_finite() && to.y.is_finite());
        }
        assert_eq!(path_bounds(&path.elements), Rect::new(8.0, 17.0, 0.0, 0.0));
    }

    let tiny_rect = Rect::new(0.25, 0.5, 0.5, 0.25);
    let inward = sentinel_recipe(8.0, 9.0).inward_annulus_primitives(
        tiny_rect,
        CornerRadius::all(100.0),
        f32::INFINITY,
    );
    for primitive in inward {
        let path = path(&primitive);
        let bounds = path_bounds(&path.elements);
        assert!(tiny_rect.contains_rect(bounds));
        assert!(bounds.width >= 0.0 && bounds.height >= 0.0);
        for (from, to) in flattened_segments(&path.elements) {
            assert!(from.x.is_finite() && from.y.is_finite());
            assert!(to.x.is_finite() && to.y.is_finite());
        }
    }
}

#[test]
fn radius_normalization_preserves_large_finite_proportions_without_pair_overflow() {
    let unit = f32::from_bits(0x7E00_0000);
    let radius = CornerRadius {
        top_left: unit * 7.0,
        top_right: unit * 3.0,
        bottom_right: unit * 2.0,
        bottom_left: unit,
    };
    assert!(
        [
            radius.top_left,
            radius.top_right,
            radius.bottom_right,
            radius.bottom_left,
        ]
        .iter()
        .all(|value| value.is_finite())
    );
    assert!((radius.top_left + radius.top_right).is_infinite());
    assert!((radius.top_left + radius.bottom_left).is_infinite());

    let rect = Rect::new(2.25, 4.75, 100.0, 90.0);
    let primitives = sentinel_recipe(2.0, 1.0).outward_annulus_primitives(rect, radius);
    let normalized = CornerRadius {
        top_left: 70.0,
        top_right: 30.0,
        bottom_right: 20.0,
        bottom_left: 10.0,
    };
    assert_counter_clockwise_contour(&path(&primitives[0]).elements[10..], rect, normalized);
    assert_eq!(
        path(&primitives[0]).elements[10..],
        path(&primitives[1]).elements[10..]
    );

    let scale = f64::from(normalized.top_left) / f64::from(radius.top_left);
    for (output, input) in [
        (normalized.top_right, radius.top_right),
        (normalized.bottom_right, radius.bottom_right),
        (normalized.bottom_left, radius.bottom_left),
    ] {
        let ratio = f64::from(output) / f64::from(input);
        assert!((ratio / scale - 1.0).abs() <= 1.0e-12);
    }
    assert!(normalized.top_left > 0.0);
    assert!(normalized.top_right > 0.0);
    assert!(normalized.bottom_right > 0.0);
    assert!(normalized.bottom_left > 0.0);
    assert!(normalized.top_left + normalized.top_right <= rect.width);
    assert!(normalized.bottom_left + normalized.bottom_right <= rect.width);
    assert!(normalized.top_left + normalized.bottom_left <= rect.height);
    assert!(normalized.top_right + normalized.bottom_right <= rect.height);
}
