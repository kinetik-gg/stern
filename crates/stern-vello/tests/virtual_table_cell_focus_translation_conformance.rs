//! Production Vello transport evidence for actual focused virtual-table body cells.

#![allow(clippy::cast_precision_loss, clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    ComponentState, FrameContext, PathElement, PathPrimitive, PhysicalSize, Point,
    PointerButtonState, PointerInput, PointerOrder, Primitive, Rect, ScaleFactor, Size, TimeInfo,
    Transform, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};
use stern_widgets::{
    CollectionProjection, ItemId, TableColumn, TableLayout, Ui, VirtualTableConfig,
    VirtualTableOutput, VirtualTableRow, VirtualTableSelection, VirtualTableSelectionMode,
    VirtualTableTarget,
};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 180.0),
            PhysicalSize::new(320, 180),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(point: Point, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(pressed, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

struct TableFrame {
    frame: stern_core::FrameOutput,
    root: WidgetId,
    output: VirtualTableOutput,
}

fn table_frame(
    memory: &mut UiMemory,
    selection: &mut VirtualTableSelection,
    bounds: Rect,
    input: UiInput,
) -> TableFrame {
    let theme = default_dark_theme();
    let projection =
        CollectionProjection::from_source_ids(&(1..=8_u64).map(id).collect::<Vec<_>>());
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
            sort: None,
        },
    )
    .label("Assets")
    .overscan(1)
    .selection_mode(VirtualTableSelectionMode::Cell);
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let table = ui
        .prepare_virtual_table("vello-virtual-table-cell", config, &projection)
        .expect("valid virtual table");
    let root = table.widget_id();
    ui.resolve_pointer_targets(|plan| {
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let output = ui.virtual_table(&table, selection, |item| {
        VirtualTableRow::new([
            format!("Row {} name", item.id.raw()),
            format!("Row {} kind", item.id.raw()),
            format!("Row {} size", item.id.raw()),
        ])
    });
    TableFrame {
        frame: ui.finish_output(),
        root,
        output,
    }
}

fn select_cell(
    memory: &mut UiMemory,
    selection: &mut VirtualTableSelection,
    bounds: Rect,
    row: usize,
    column: usize,
) -> TableFrame {
    let point = Point::new(
        bounds.x + column as f32 * 80.0 + 40.0,
        bounds.y + 20.25 + row as f32 * 20.0 + 10.0,
    );
    let _ = table_frame(memory, selection, bounds, pointer_input(point, true, false));
    table_frame(memory, selection, bounds, pointer_input(point, false, true))
}

fn cell_response(frame: &TableFrame, target: VirtualTableTarget) -> stern_core::Response {
    frame
        .output
        .selection_responses
        .iter()
        .find(|response| response.target == target)
        .unwrap_or_else(|| panic!("missing target {target:?}"))
        .response
}

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("table cell focus must be a compound path");
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
fn actual_table_cell_focus_submits_the_fractionally_scrolled_full_body_scope_at_release_scales() {
    let theme = default_dark_theme();
    let resources = RenderResources::new();
    for (origin, row, column, offset) in [
        (
            Point::new(3.125, 7.375),
            0_usize,
            0_usize,
            Vec2::new(13.25, 7.5),
        ),
        (Point::new(10.2, 20.6), 2, 1, Vec2::new(57.5, 31.25)),
        (Point::new(17.75, 2.25), 4, 2, Vec2::new(111.5, 69.5)),
    ] {
        let seed_bounds = Rect::new(origin.x, origin.y, 240.0, 140.0);
        let bounds = Rect::new(origin.x, origin.y, 128.5, 72.75);
        let mut selection = VirtualTableSelection::new();
        let mut memory = UiMemory::new();
        let selected = select_cell(&mut memory, &mut selection, seed_bounds, row, column);
        let target = VirtualTableTarget::Cell {
            row: id(row as u64 + 1),
            column: id([10_u64, 20, 30][column]),
        };
        let selected_response = cell_response(&selected, target);
        assert!(selected_response.state.focused);
        assert!(selected_response.state.selected);
        memory.set_scroll_offset(selected.root, offset);

        let focused = table_frame(&mut memory, &mut selection, bounds, UiInput::default());
        assert_eq!(focused.root, selected.root);
        assert_eq!(focused.output.window.offset, offset);
        let response = cell_response(&focused, target);
        assert!(response.state.focused);
        assert!(response.state.selected);
        assert!(memory.is_focused(response.id));

        let body_clip = Rect::new(
            bounds.x,
            bounds.y + 20.25,
            bounds.width,
            bounds.height - 20.25,
        );
        let body_begin = focused
            .frame
            .primitives
            .iter()
            .position(
                |primitive| matches!(primitive, Primitive::ClipBegin { rect, .. } if *rect == body_clip),
            )
            .expect("body clip begin");
        let Primitive::ClipBegin {
            id: body_clip_id, ..
        } = focused.frame.primitives[body_begin]
        else {
            unreachable!()
        };
        let body_end = focused
            .frame
            .primitives
            .iter()
            .enumerate()
            .skip(body_begin + 1)
            .find_map(|(index, primitive)| {
                matches!(primitive, Primitive::ClipEnd { id } if *id == body_clip_id)
                    .then_some(index)
            })
            .expect("body clip end");
        let body_scope = &focused.frame.primitives[body_begin..=body_end];
        assert!(matches!(
            body_scope[0],
            Primitive::ClipBegin { rect, .. } if rect == body_clip
        ));
        assert_eq!(
            body_scope[1],
            Primitive::TransformBegin(Transform::translation(Vec2::new(-offset.x, -offset.y)))
        );
        assert_eq!(body_scope[body_scope.len() - 2], Primitive::TransformEnd);
        assert!(matches!(
            body_scope[body_scope.len() - 1],
            Primitive::ClipEnd { .. }
        ));
        assert_eq!(
            body_scope
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::TransformBegin(_)))
                .count(),
            1
        );

        let base_index = focused
            .frame
            .primitives
            .iter()
            .position(
                |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == response.rect),
            )
            .expect("focused table cell base");
        assert!(body_begin < base_index && base_index + 3 < body_end);
        let Primitive::Rect(base) = &focused.frame.primitives[base_index] else {
            unreachable!()
        };
        let state = ComponentState {
            focused: true,
            selected: true,
            ..ComponentState::default()
        };
        let recipe = theme.row(state);
        assert_eq!(base.fill, Some(recipe.background));
        assert_eq!(base.stroke, Some(recipe.border));
        assert_eq!(base.radius, recipe.radius);
        let primary = path(&focused.frame.primitives[base_index + 1]);
        let separator = path(&focused.frame.primitives[base_index + 2]);
        assert!(matches!(
            focused.frame.primitives[base_index + 3],
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
            focused
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            2
        );

        let body_translation = translate_primitives(body_scope, &resources);
        assert!(body_translation.diagnostics.is_empty());
        let path_commands = body_translation
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
                ..
            } = &command.kind
            else {
                unreachable!()
            };
            assert_eq!(elements, &expected.elements);
            assert_eq!(*fill, expected.fill);
            assert_eq!(*stroke, None);
            assert_eq!(
                command.transform,
                Transform::translation(Vec2::new(-offset.x, -offset.y))
            );
            assert_eq!(command.clips.len(), 1);
            assert_eq!(command.clips[0].rect, body_clip);
            assert_eq!(command.clips[0].transform, Transform::IDENTITY);
        }

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
                primitives: body_scope,
                resources: &resources,
            });
            assert_eq!(submission.primitive_count, body_scope.len());
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
                        && transform.translation
                            == [(-offset.x * scale).round(), (-offset.y * scale).round()]
                }),
                "missing scaled, physical-pixel-snapped body translation: {:?}",
                encoding.transforms
            );
            for command in body_translation
                .commands
                .iter()
                .filter(|command| matches!(command.kind, RenderCommandKind::Path { .. }))
            {
                assert_eq!(
                    command.transform,
                    Transform::translation(Vec2::new(-offset.x, -offset.y))
                );
                assert_eq!(command.clips.len(), 1);
                assert_eq!(command.clips[0].rect, body_clip);
                assert_eq!(command.clips[0].transform, Transform::IDENTITY);
            }
        }
    }
}
