//! Public Stern/Vello focus-ring translation conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    CornerRadius, PathElement, PathPrimitive, PhysicalSize, Point, Primitive, Rect, ScaleFactor,
    Size, ViewportInfo, default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("focus annulus must remain a path primitive");
    };
    path
}

fn contour_bounds(elements: &[PathElement]) -> Rect {
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

fn contour_endpoint_area(elements: &[PathElement]) -> f32 {
    let mut points = Vec::new();
    for element in elements {
        match *element {
            PathElement::MoveTo(point)
            | PathElement::LineTo(point)
            | PathElement::QuadTo { to: point, .. }
            | PathElement::CubicTo { to: point, .. } => points.push(point),
            PathElement::Close => {}
        }
    }
    points
        .iter()
        .zip(points.iter().cycle().skip(1))
        .take(points.len())
        .map(|(from, to)| from.x * to.y - to.x * from.y)
        .sum::<f32>()
        * 0.5
}

fn assert_translated_compound_paths(primitives: &[Primitive; 2]) {
    let translation = translate_primitives(primitives, &RenderResources::new());
    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands.len(), 2);
    for (command, expected) in translation.commands.iter().zip(primitives) {
        let RenderCommandKind::Path {
            elements,
            fill,
            stroke,
            ..
        } = &command.kind
        else {
            panic!("annulus must translate as one compound path command");
        };
        let expected = path(expected);
        assert_eq!(elements, &expected.elements);
        assert_eq!(*fill, expected.fill);
        assert_eq!(*stroke, None);
        assert_eq!(elements.len(), 20);
        assert_eq!(
            elements
                .iter()
                .filter(|element| matches!(element, PathElement::Close))
                .count(),
            2
        );
        let outer_area = contour_endpoint_area(&elements[..10]);
        let inner_area = contour_endpoint_area(&elements[10..]);
        if outer_area != 0.0 && inner_area != 0.0 {
            assert!(outer_area > 0.0);
            assert!(inner_area < 0.0);
        }
    }
}

fn band_spans(outer: Rect, inner: Rect) -> [f32; 4] {
    [
        inner.min_x() - outer.min_x(),
        inner.min_y() - outer.min_y(),
        outer.max_x() - inner.max_x(),
        outer.max_y() - inner.max_y(),
    ]
}

#[test]
fn nested_focus_fills_preserve_order_through_vello_at_release_scales() {
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let rect = Rect::new(10.0, 20.0, 20.0, 20.0);
    let primitives = recipe.primitives(rect, CornerRadius::all(4.0));

    let [Primitive::Rect(outer), Primitive::Rect(inner)] = &primitives else {
        panic!("focus recipe must emit exactly two rectangle primitives");
    };
    assert_eq!(outer.fill, Some(recipe.primary.brush));
    assert_eq!(outer.stroke, None);
    assert_eq!(inner.fill, Some(recipe.separator.brush));
    assert_eq!(inner.stroke, None);

    let resources = RenderResources::new();
    let translation = translate_primitives(&primitives, &resources);
    assert!(translation.diagnostics.is_empty());
    assert_eq!(translation.commands.len(), 2);

    for (command, expected) in translation.commands.iter().zip([outer, inner]) {
        let RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } = &command.kind
        else {
            panic!("focus contour must translate to a rectangle command");
        };
        assert_eq!(*rect, expected.rect);
        assert_eq!(*fill, expected.fill);
        assert_eq!(*stroke, None);
        assert_eq!(*radius, expected.radius);
    }

    for (scale, physical_size) in [
        (1.0_f32, PhysicalSize::new(80, 80)),
        (1.25, PhysicalSize::new(100, 100)),
        (1.5, PhysicalSize::new(120, 120)),
        (2.0, PhysicalSize::new(160, 160)),
    ] {
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(80.0, 80.0),
                physical_size,
                ScaleFactor::new(f64::from(scale)),
            ),
            primitives: &primitives,
            resources: &resources,
        });

        assert_eq!(output.primitive_count, 2);
        assert!(
            output.diagnostics.is_empty(),
            "focus encoding diagnostics at {scale}x: {:?}",
            output.diagnostics
        );

        let encoding = renderer.scene().encoding();
        assert_eq!(encoding.n_paths, 2, "separate contours at {scale}x");
        assert_eq!(encoding.draw_tags.len(), 2, "separate fills at {scale}x");
        assert!(
            encoding.draw_tags.iter().all(|tag| tag.0 == 0x44),
            "both contours must remain solid-color fills at {scale}x"
        );
        assert_eq!(
            encoding.draw_data,
            vec![0xFFFF_B24D, 0xFF0B_0B0B],
            "outer-primary then inner-separator color order at {scale}x"
        );
        assert!(
            encoding.styles.iter().all(|style| {
                style.flags_and_miter_limit & 0x8000_0000 == 0 && style.line_width == 0.0
            }),
            "focus contours must not become stroked paths at {scale}x"
        );
        assert_eq!(encoding.transforms.len(), 1);
        assert_eq!(encoding.transforms[0].matrix, [scale, 0.0, 0.0, scale]);
    }
}

#[test]
fn hollow_focus_annuli_preserve_geometry_and_vello_encoding_at_release_scales() {
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let resources = RenderResources::new();

    for origin in [
        Point::new(3.125, 7.375),
        Point::new(10.2, 20.6),
        Point::new(17.75, 2.25),
    ] {
        let rect = Rect::new(origin.x, origin.y, 36.5, 22.25);
        let radius = CornerRadius {
            top_left: 3.0,
            top_right: 4.0,
            bottom_right: 5.0,
            bottom_left: 6.0,
        };
        let placements = [
            recipe.outward_annulus_primitives(rect, radius),
            recipe.inward_annulus_primitives(rect, radius, 1.0),
        ];

        for primitives in placements {
            assert_translated_compound_paths(&primitives);
            let primary = path(&primitives[0]);
            let separator = path(&primitives[1]);
            assert_eq!(primary.fill, Some(recipe.primary.brush));
            assert_eq!(separator.fill, Some(recipe.separator.brush));
            assert_eq!(primary.elements[10..], separator.elements[10..]);

            let primary_outer = contour_bounds(&primary.elements[..10]);
            let separator_outer = contour_bounds(&separator.elements[..10]);
            let inner = contour_bounds(&primary.elements[10..]);
            assert!(primary_outer.contains_rect(separator_outer));
            assert!(separator_outer.contains_rect(inner));
            let primary_spans = band_spans(primary_outer, separator_outer);
            let separator_spans = band_spans(separator_outer, inner);
            assert_eq!(primary_spans, [recipe.primary.width; 4]);
            assert_eq!(separator_spans, [recipe.separator.width; 4]);

            for (scale, physical_size) in [
                (1.0_f32, PhysicalSize::new(96, 64)),
                (1.25, PhysicalSize::new(120, 80)),
                (1.5, PhysicalSize::new(144, 96)),
                (2.0, PhysicalSize::new(192, 128)),
            ] {
                for (span, expected) in primary_spans
                    .into_iter()
                    .zip([recipe.primary.width; 4])
                    .chain(separator_spans.into_iter().zip([recipe.separator.width; 4]))
                {
                    let physical_span = span * scale;
                    assert_eq!(physical_span, expected * scale);
                    assert!(physical_span >= 1.0);
                }

                let mut renderer = VelloRenderer::new();
                let output = renderer.submit_frame(RenderFrameInput {
                    viewport: ViewportInfo::new(
                        Size::new(96.0, 64.0),
                        physical_size,
                        ScaleFactor::new(f64::from(scale)),
                    ),
                    primitives: &primitives,
                    resources: &resources,
                });
                assert_eq!(output.primitive_count, 2);
                assert!(
                    output.diagnostics.is_empty(),
                    "annulus encoding diagnostics at {origin:?}, {scale}x"
                );

                let encoding = renderer.scene().encoding();
                assert_eq!(encoding.n_paths, 2);
                assert_eq!(encoding.draw_tags.len(), 2);
                assert!(encoding.draw_tags.iter().all(|tag| tag.0 == 0x44));
                assert_eq!(encoding.draw_data, vec![0xFFFF_B24D, 0xFF0B_0B0B]);
                assert!(encoding.styles.iter().all(|style| {
                    style.flags_and_miter_limit & 0x8000_0000 == 0 && style.line_width == 0.0
                }));
                assert_eq!(encoding.transforms.len(), 1);
                assert_eq!(encoding.transforms[0].matrix, [scale, 0.0, 0.0, scale]);
            }
        }
    }
}

#[test]
fn degenerate_inward_annuli_encode_without_renderer_diagnostics() {
    let recipe = default_dark_theme()
        .focus_ring(true)
        .expect("visible focus ring");
    let primitives = recipe.inward_annulus_primitives(
        Rect::new(8.25, 11.75, 0.25, 0.125),
        CornerRadius::all(64.0),
        32.0,
    );
    assert_translated_compound_paths(&primitives);

    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(32.0, 32.0),
            PhysicalSize::new(40, 40),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.scene().encoding().n_paths, 2);
}
