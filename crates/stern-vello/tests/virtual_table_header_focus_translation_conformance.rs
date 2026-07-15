//! Production Vello transport evidence for actual focused virtual-table headers.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    ComponentState, FrameContext, PathElement, PathPrimitive, PhysicalSize, Point, PointerOrder,
    Primitive, Rect, ScaleFactor, Size, TimeInfo, Transform, UiInput, UiMemory, Vec2, ViewportInfo,
    WidgetId, default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};
use stern_widgets::{
    CollectionProjection, ItemId, SortDirection, TableColumn, TableLayout, TableSort, Ui,
    VirtualTableConfig, VirtualTableRow, VirtualTableSelection,
};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
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

fn table_frame(
    memory: &mut UiMemory,
    bounds: Rect,
    sort: Option<TableSort>,
    column: ItemId,
) -> (stern_core::FrameOutput, WidgetId, stern_core::Response) {
    let theme = default_dark_theme();
    let projection = CollectionProjection::from_source_ids(&[id(1), id(2), id(3)]);
    let config = VirtualTableConfig::new(
        bounds,
        TableLayout {
            columns: vec![
                TableColumn::new(id(10), "Name", 80.0),
                TableColumn::new(id(20), "Kind", 80.0),
                TableColumn::new(id(30), "Size", 80.0),
            ],
            header_height: 20.25,
            row_height: 20.0,
            sort,
        },
    )
    .label("Assets")
    .overscan(0);
    let mut ui = Ui::begin_frame(context(), memory, &theme);
    let table = ui
        .prepare_virtual_table("vello-virtual-table", config, &projection)
        .expect("valid virtual table");
    let root = table.widget_id();
    ui.resolve_pointer_targets(|plan| {
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let output = ui.virtual_table(&table, &mut VirtualTableSelection::new(), |item| {
        VirtualTableRow::new([
            format!("Row {} name", item.id.raw()),
            format!("Row {} kind", item.id.raw()),
            format!("Row {} size", item.id.raw()),
        ])
    });
    let response = output
        .headers
        .iter()
        .find(|header| header.column == column)
        .expect("target header")
        .response;
    (ui.finish_output(), root, response)
}

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("table header focus must be a compound path");
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
fn actual_table_header_focus_translates_with_fractional_scroll_at_release_scales() {
    let theme = default_dark_theme();
    let resources = RenderResources::new();
    for (case, (origin, column, horizontal_scroll)) in [
        (Point::new(3.125, 7.375), id(10), 13.25_f32),
        (Point::new(10.2, 20.6), id(20), 57.5_f32),
        (Point::new(17.75, 2.25), id(30), 111.5_f32),
    ]
    .into_iter()
    .enumerate()
    {
        let bounds = Rect::new(origin.x, origin.y, 128.5, 72.75);
        let sort = (case == 1).then_some(TableSort {
            column,
            direction: SortDirection::Ascending,
        });
        let mut memory = UiMemory::new();
        let (_, root, response) = table_frame(&mut memory, bounds, sort, column);
        memory.focus(response.id);
        memory.set_scroll_offset(root, Vec2::new(horizontal_scroll, 0.0));
        let (frame, focused_root, response) = table_frame(&mut memory, bounds, sort, column);
        assert_eq!(focused_root, root);
        assert!(response.state.focused);
        assert!(!response.state.disabled);
        assert!(memory.is_focused(response.id));

        let header_clip = Rect::new(bounds.x, bounds.y, bounds.width, 20.25);
        assert!(matches!(
            frame.primitives[1],
            Primitive::ClipBegin { rect, .. } if rect == header_clip
        ));
        assert_eq!(
            frame.primitives[2],
            Primitive::TransformBegin(Transform::translation(Vec2::new(-horizontal_scroll, 0.0)))
        );
        let header_clip_end = frame
            .primitives
            .iter()
            .position(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
            .expect("header clip end");
        assert!(matches!(
            frame.primitives[header_clip_end - 1],
            Primitive::TransformEnd
        ));

        let base_index = frame
            .primitives
            .iter()
            .position(
                |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == response.rect),
            )
            .expect("focused table header base");
        let Primitive::Rect(base) = &frame.primitives[base_index] else {
            unreachable!()
        };
        let state = ComponentState {
            focused: true,
            selected: sort.is_some(),
            ..ComponentState::default()
        };
        let recipe = theme.row(state);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, recipe.radius);
        let primary = path(&frame.primitives[base_index + 1]);
        let separator = path(&frame.primitives[base_index + 2]);
        assert!(matches!(
            frame.primitives[base_index + 3],
            Primitive::Text(_)
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

        let header_scope = &frame.primitives[1..=header_clip_end];
        let header_translation = translate_primitives(header_scope, &resources);
        assert!(header_translation.diagnostics.is_empty());
        let path_commands = header_translation
            .commands
            .iter()
            .filter(|command| matches!(command.kind, RenderCommandKind::Path { .. }))
            .collect::<Vec<_>>();
        assert_eq!(path_commands.len(), 2);
        for (command, expected) in path_commands.into_iter().zip([primary, separator]) {
            let RenderCommandKind::Path {
                elements,
                fill,
                stroke,
            } = &command.kind
            else {
                unreachable!()
            };
            assert_eq!(elements, &expected.elements);
            assert_eq!(*fill, expected.fill);
            assert_eq!(*stroke, None);
            assert_eq!(
                command.transform,
                Transform::translation(Vec2::new(-horizontal_scroll, 0.0))
            );
            assert_eq!(command.clips.len(), 1);
            assert_eq!(command.clips[0].rect, header_clip);
            assert_eq!(command.clips[0].transform, Transform::IDENTITY);
        }

        let focus_translation =
            translate_primitives(&frame.primitives[base_index..=base_index + 2], &resources);
        assert!(focus_translation.diagnostics.is_empty());
        assert_eq!(focus_translation.commands.len(), 3);
        assert!(matches!(
            focus_translation.commands[0].kind,
            RenderCommandKind::Rect { .. }
        ));

        let primary_outer = contour_bounds(&primary.elements[..10]);
        let separator_outer = contour_bounds(&separator.elements[..10]);
        let inner = contour_bounds(&primary.elements[10..]);
        assert!(response.rect.contains_rect(primary_outer));
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
            let mut renderer = VelloRenderer::new();
            let submission = renderer.submit_frame(RenderFrameInput {
                viewport: ViewportInfo::new(
                    Size::new(320.0, 180.0),
                    physical_size,
                    ScaleFactor::new(f64::from(scale)),
                ),
                primitives: header_scope,
                resources: &resources,
            });
            assert_eq!(submission.primitive_count, header_scope.len());
            assert!(submission.diagnostics.is_empty());
            let encoding = renderer.scene().encoding();
            assert!(encoding.n_paths >= 2);
            let primary_draw = encoding
                .draw_data
                .iter()
                .position(|word| *word == 0xFFFF_B24D)
                .expect("primary focus draw");
            let separator_draw = encoding
                .draw_data
                .iter()
                .enumerate()
                .skip(primary_draw + 1)
                .find_map(|(index, word)| (*word == 0xFF0B_0B0B).then_some(index))
                .expect("separator focus draw after primary");
            assert_eq!(separator_draw, primary_draw + 3);
            assert!(
                encoding.transforms.iter().any(|transform| {
                    transform.matrix == [scale, 0.0, 0.0, scale]
                        && transform.translation == [(-horizontal_scroll * scale).round(), 0.0]
                }),
                "missing scaled, physical-pixel-snapped header translation: {:?}",
                encoding.transforms
            );
            for command in header_translation
                .commands
                .iter()
                .filter(|command| matches!(command.kind, RenderCommandKind::Path { .. }))
            {
                assert_eq!(
                    command.transform,
                    Transform::translation(Vec2::new(-horizontal_scroll, 0.0))
                );
                assert_eq!(command.clips.len(), 1);
                assert_eq!(command.clips[0].rect, header_clip);
                assert_eq!(command.clips[0].transform, Transform::IDENTITY);
            }
        }
    }
}
