//! Production Vello transport evidence for actual focused Outliner rows.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    ComponentState, FrameContext, PathElement, PathPrimitive, PhysicalSize, Point, PointerOrder,
    Primitive, Rect, ScaleFactor, Size, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};
use stern_widgets::outliner::{OutlinerConfig, OutlinerState};
use stern_widgets::{ItemId, OutlinerItem, OutlinerModel, OutlinerRowZones, Ui};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn model() -> OutlinerModel {
    OutlinerModel::new(vec![
        OutlinerItem::new(id(10), "Focused row").with_has_children(true),
        OutlinerItem::new(id(20), "Sibling row"),
    ])
}

fn context() -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::new(320, 180),
            ScaleFactor::ONE,
        ),
        UiInput::default(),
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn outliner_frame(
    memory: &mut UiMemory,
    bounds: Rect,
    state: &mut OutlinerState,
    model: &OutlinerModel,
) -> (stern_core::FrameOutput, WidgetId, OutlinerRowZones) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(), memory, &theme);
    let scene = ui
        .prepare_outliner(
            "vello-outliner",
            OutlinerConfig::new(bounds, 28.25, 16.5).overscan(0),
            model,
            state,
        )
        .expect("valid outliner");
    let root = scene.widget_id();
    let row = scene
        .rows()
        .iter()
        .find(|row| row.row.id == id(10))
        .expect("focused row geometry")
        .clone();
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100), state);
    })
    .expect("valid pointer plan");
    let output = ui.outliner(&scene, state, |_target| Vec::new());
    assert_eq!(output.responses.len(), 2);
    (ui.finish_output(), root, row)
}

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("outliner focus must be a compound path");
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
fn actual_outliner_focus_translates_as_contained_fill_only_annuli_at_release_scales() {
    let theme = default_dark_theme();
    let resources = RenderResources::new();
    let model = model();
    for (case, origin) in [
        Point::new(3.125_f32, 7.375_f32),
        Point::new(10.2_f32, 20.6_f32),
        Point::new(17.75_f32, 2.25_f32),
        Point::new(5.375_f32, 13.625_f32),
    ]
    .into_iter()
    .enumerate()
    {
        let bounds = Rect::new(origin.x, origin.y, 244.5, 112.75);
        let selected = case % 2 == 0;
        let mut state = OutlinerState::new();
        if selected {
            state.selection.replace(id(10));
        }
        let mut memory = UiMemory::new();
        let (_, root, _) = outliner_frame(&mut memory, bounds, &mut state, &model);
        let row_id = root.child(("outliner-row", 10_u64));
        memory.focus(row_id);
        let (frame, focused_root, row) = outliner_frame(&mut memory, bounds, &mut state, &model);
        assert_eq!(focused_root, root);
        assert!(memory.is_focused(row_id));
        assert!(matches!(
            frame.primitives[1],
            Primitive::ClipBegin { rect, .. } if rect == bounds
        ));
        assert!(matches!(
            frame.primitives[frame.primitives.len() - 1],
            Primitive::ClipEnd { .. }
        ));
        assert!(
            frame
                .primitives
                .iter()
                .all(|primitive| !matches!(primitive, Primitive::TransformBegin { .. }))
        );

        let base_index = frame
            .primitives
            .iter()
            .position(
                |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == row.rect),
            )
            .expect("focused outliner base");
        let Primitive::Rect(base) = &frame.primitives[base_index] else {
            unreachable!()
        };
        let state = ComponentState {
            hovered: false,
            pressed: false,
            focused: true,
            disabled: false,
            selected,
        };
        let recipe = theme.row(state);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, recipe.radius);
        let primary = path(&frame.primitives[base_index + 1]);
        let separator = path(&frame.primitives[base_index + 2]);
        assert!(matches!(
            frame.primitives[base_index + 3],
            Primitive::Line(_)
        ));
        assert!(matches!(
            frame.primitives[base_index + 4],
            Primitive::Line(_)
        ));
        assert!(matches!(
            frame.primitives[base_index + 5],
            Primitive::Rect(visibility) if visibility.rect != row.rect
        ));
        assert!(matches!(
            frame.primitives[base_index + 6],
            Primitive::Rect(_)
        ));
        assert!(
            frame.primitives[base_index + 7..=base_index + 9]
                .iter()
                .all(|primitive| matches!(primitive, Primitive::Line(_)))
        );
        assert!(matches!(
            frame.primitives[base_index + 10],
            Primitive::Text(ref label) if label.text == row.row.label
        ));
        let focus = theme.focus_ring(true).expect("focus recipe");
        assert_eq!(primary.fill, Some(focus.primary.brush));
        assert_eq!(separator.fill, Some(focus.separator.brush));
        assert_eq!(primary.stroke, None);
        assert_eq!(separator.stroke, None);
        assert_eq!(primary.elements.len(), 20);
        assert_eq!(separator.elements.len(), 20);
        assert_eq!(
            frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            2
        );

        let full_translation = translate_primitives(&frame.primitives, &resources);
        assert!(full_translation.diagnostics.is_empty());
        let translation =
            translate_primitives(&frame.primitives[base_index..=base_index + 2], &resources);
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
                ..
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
        assert!(row.rect.contains_rect(primary_outer));
        assert!(primary_outer.contains_rect(separator_outer));
        assert!(separator_outer.contains_rect(inner));
        assert_eq!(primary.elements[10..], separator.elements[10..]);
        let primary_spans = band_spans(primary_outer, separator_outer);
        let separator_spans = band_spans(separator_outer, inner);
        assert_eq!(primary_spans, [theme.strokes.focus.primary; 4]);
        assert_eq!(separator_spans, [theme.strokes.focus.separator; 4]);

        for (scale, physical_size) in [
            (1.0_f32, PhysicalSize::new(320, 180)),
            (1.25, PhysicalSize::new(400, 225)),
            (1.5, PhysicalSize::new(480, 270)),
            (2.0, PhysicalSize::new(640, 360)),
        ] {
            for span in primary_spans.into_iter().chain(separator_spans) {
                assert!(span * scale >= 1.0);
            }
            let focus_primitives = &frame.primitives[base_index + 1..=base_index + 2];
            let mut renderer = VelloRenderer::new();
            let submission = renderer.submit_frame(RenderFrameInput {
                viewport: ViewportInfo::new(
                    Size::new(320.0, 180.0),
                    physical_size,
                    ScaleFactor::new(f64::from(scale)),
                ),
                primitives: focus_primitives,
                resources: &resources,
            });
            assert_eq!(submission.primitive_count, 2);
            assert!(submission.diagnostics.is_empty());
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
