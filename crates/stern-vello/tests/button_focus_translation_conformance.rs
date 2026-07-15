//! Production Vello translation evidence for actual focused button surfaces.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use stern_core::{
    Brush, PathElement, PathPrimitive, PhysicalSize, Point, Primitive, Rect, ScaleFactor, Size,
    UiInput, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};
use stern_widgets::button;

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("button focus must be a compound path");
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

fn band_spans(outer: Rect, inner: Rect) -> [f32; 4] {
    [
        inner.min_x() - outer.min_x(),
        inner.min_y() - outer.min_y(),
        outer.max_x() - inner.max_x(),
        outer.max_y() - inner.max_y(),
    ]
}

#[test]
fn actual_button_focus_translates_as_contained_fill_only_annuli_at_release_scales() {
    let theme = default_dark_theme();
    let resources = RenderResources::new();
    for (case, origin) in [
        Point::new(3.125, 7.375),
        Point::new(10.2, 20.6),
        Point::new(17.75, 2.25),
    ]
    .into_iter()
    .enumerate()
    {
        let id = WidgetId::from_key(format!("vello-button-{case}"));
        let rect = Rect::new(origin.x, origin.y, 42.5, 24.25);
        let mut memory = UiMemory::new();
        memory.focus(id);
        let output = button(
            id,
            rect,
            "Encode",
            &UiInput::default(),
            &mut memory,
            &theme,
            false,
        );
        assert_eq!(output.primitives.len(), 4);
        let Primitive::Rect(base) = &output.primitives[0] else {
            panic!("neutral base first");
        };
        assert_eq!(base.rect, rect);
        assert_eq!(
            base.stroke.expect("neutral boundary").brush,
            Brush::Solid(theme.colors.border.default)
        );
        let primary = path(&output.primitives[1]);
        let separator = path(&output.primitives[2]);
        assert!(matches!(output.primitives[3], Primitive::Text(_)));
        assert_eq!(
            primary.fill,
            Some(theme.focus_ring(true).unwrap().primary.brush)
        );
        assert_eq!(
            separator.fill,
            Some(theme.focus_ring(true).unwrap().separator.brush)
        );
        assert_eq!(primary.stroke, None);
        assert_eq!(separator.stroke, None);

        let translation = translate_primitives(&output.primitives[..3], &resources);
        assert!(translation.diagnostics.is_empty());
        assert_eq!(translation.commands.len(), 3);
        assert!(matches!(
            translation.commands[0].kind,
            RenderCommandKind::Rect { .. }
        ));
        for (command, expected) in translation.commands[1..].iter().zip([primary, separator]) {
            let RenderCommandKind::Path {
                elements,
                fill,
                stroke,
            } = &command.kind
            else {
                panic!("focus command must remain a path");
            };
            assert_eq!(elements, &expected.elements);
            assert_eq!(*fill, expected.fill);
            assert_eq!(*stroke, None);
        }

        let primary_outer = contour_bounds(&primary.elements[..10]);
        let separator_outer = contour_bounds(&separator.elements[..10]);
        let inner = contour_bounds(&primary.elements[10..]);
        assert!(rect.contains_rect(primary_outer));
        assert!(primary_outer.contains_rect(separator_outer));
        assert!(separator_outer.contains_rect(inner));
        assert_eq!(primary.elements[10..], separator.elements[10..]);
        let primary_spans = band_spans(primary_outer, separator_outer);
        let separator_spans = band_spans(separator_outer, inner);
        assert_eq!(primary_spans, [theme.strokes.focus.primary; 4]);
        assert_eq!(separator_spans, [theme.strokes.focus.separator; 4]);

        for (scale, physical_size) in [
            (1.0_f32, PhysicalSize::new(96, 64)),
            (1.25, PhysicalSize::new(120, 80)),
            (1.5, PhysicalSize::new(144, 96)),
            (2.0, PhysicalSize::new(192, 128)),
        ] {
            for span in primary_spans.into_iter().chain(separator_spans) {
                assert!(span * scale >= 1.0);
            }
            let focus_primitives = &output.primitives[1..3];
            let mut renderer = VelloRenderer::new();
            let frame = renderer.submit_frame(RenderFrameInput {
                viewport: ViewportInfo::new(
                    Size::new(96.0, 64.0),
                    physical_size,
                    ScaleFactor::new(f64::from(scale)),
                ),
                primitives: focus_primitives,
                resources: &resources,
            });
            assert_eq!(frame.primitive_count, 2);
            assert!(frame.diagnostics.is_empty());
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
