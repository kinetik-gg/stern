//! Production Vello evidence for actual focused virtual-tree row surfaces.

#![allow(clippy::float_cmp, clippy::too_many_lines)]

use std::time::Duration;

use stern_core::{
    Brush, FrameContext, PathElement, PathPrimitive, PhysicalSize, Point, PointerOrder, Primitive,
    Rect, ScaleFactor, Size, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId,
    default_dark_theme,
};
use stern_vello::{
    RenderCommandKind, RenderFrameInput, RenderResources, VelloRenderer, translate_primitives,
};
use stern_widgets::{
    CollectionCursor, ItemId, Selection, TreeExpansion, TreeItem, TreeModel, Ui, VirtualTreeConfig,
    VirtualTreeRow,
};

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn tree_model() -> TreeModel {
    TreeModel::new(vec![
        TreeItem {
            id: id(10),
            parent: None,
            has_children: true,
        },
        TreeItem {
            id: id(11),
            parent: Some(id(10)),
            has_children: false,
        },
        TreeItem {
            id: id(20),
            parent: None,
            has_children: false,
        },
    ])
}

fn context() -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(256.0, 128.0),
            PhysicalSize::new(256, 128),
            ScaleFactor::ONE,
        ),
        UiInput::default(),
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn virtual_tree_frame(
    memory: &mut UiMemory,
    bounds: Rect,
    model: &TreeModel,
    expansion: &mut TreeExpansion,
    selection: &mut Selection,
) -> (stern_core::FrameOutput, WidgetId) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(), memory, &theme);
    let tree = ui
        .prepare_virtual_tree(
            "vello-virtual-tree",
            VirtualTreeConfig::new(bounds, 24.25, 16.0).overscan(0),
            model,
            expansion,
        )
        .expect("valid virtual tree");
    let row_id = tree.row_widget_id(id(10));
    ui.resolve_pointer_targets(|plan| {
        tree.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let output = ui.virtual_tree(
        &tree,
        &mut CollectionCursor::new(),
        selection,
        expansion,
        |row| VirtualTreeRow::new(format!("Row {}", row.id.raw())),
    );
    assert!(!output.responses.is_empty());
    (ui.finish_output(), row_id)
}

fn path(primitive: &Primitive) -> &PathPrimitive {
    let Primitive::Path(path) = primitive else {
        panic!("virtual-tree focus must be a compound path");
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
fn actual_virtual_tree_focus_translates_as_contained_fill_only_annuli_at_release_scales() {
    let theme = default_dark_theme();
    let resources = RenderResources::new();
    let model = tree_model();
    for (case, origin) in [
        Point::new(3.125, 7.375),
        Point::new(10.2, 20.6),
        Point::new(17.75, 2.25),
    ]
    .into_iter()
    .enumerate()
    {
        let bounds = Rect::new(origin.x, origin.y, 128.5, 72.75);
        let row_rect = Rect::new(origin.x, origin.y, bounds.width, 24.25);
        let selected = case % 2 == 0;
        let expanded = case % 2 == 1;
        let mut expansion = TreeExpansion::new();
        if expanded {
            expansion.expand(id(10));
        }
        let mut selection = Selection::new();
        if selected {
            selection.replace(id(10));
        }
        let mut memory = UiMemory::new();
        let (_, row_id) =
            virtual_tree_frame(&mut memory, bounds, &model, &mut expansion, &mut selection);
        memory.focus(row_id);
        let (frame, focused_row_id) =
            virtual_tree_frame(&mut memory, bounds, &model, &mut expansion, &mut selection);
        assert_eq!(focused_row_id, row_id);
        assert!(matches!(
            frame.primitives[1],
            Primitive::ClipBegin { rect, .. } if rect == bounds
        ));
        assert!(matches!(frame.primitives[2], Primitive::TransformBegin(_)));
        assert!(matches!(
            frame.primitives[frame.primitives.len() - 2],
            Primitive::TransformEnd
        ));
        assert!(matches!(
            frame.primitives[frame.primitives.len() - 1],
            Primitive::ClipEnd { .. }
        ));

        let base_index = frame
            .primitives
            .iter()
            .position(
                |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == row_rect),
            )
            .expect("focused branch base");
        let Primitive::Rect(base) = &frame.primitives[base_index] else {
            unreachable!()
        };
        assert_eq!(base.radius, theme.radii.none);
        assert_eq!(
            base.stroke.expect("neutral boundary").brush,
            Brush::Solid(theme.colors.border.subtle)
        );
        assert_eq!(
            base.stroke.expect("neutral boundary").width,
            theme.strokes.hairline
        );
        assert_eq!(
            base.fill,
            Some(Brush::Solid(if selected {
                theme.colors.selection.background
            } else {
                theme.colors.surface.sunken
            }))
        );
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
            Primitive::Text(_)
        ));
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
        assert_eq!(
            frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            2
        );

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
        assert!(row_rect.contains_rect(primary_outer));
        assert!(primary_outer.contains_rect(separator_outer));
        assert!(separator_outer.contains_rect(inner));
        assert_eq!(primary.elements[10..], separator.elements[10..]);
        let primary_spans = band_spans(primary_outer, separator_outer);
        let separator_spans = band_spans(separator_outer, inner);
        assert_eq!(primary_spans, [theme.strokes.focus.primary; 4]);
        assert_eq!(separator_spans, [theme.strokes.focus.separator; 4]);

        for (scale, physical_size) in [
            (1.0_f32, PhysicalSize::new(256, 128)),
            (1.25, PhysicalSize::new(320, 160)),
            (1.5, PhysicalSize::new(384, 192)),
            (2.0, PhysicalSize::new(512, 256)),
        ] {
            for span in primary_spans.into_iter().chain(separator_spans) {
                assert!(span * scale >= 1.0);
            }
            let focus_primitives = &frame.primitives[base_index + 1..=base_index + 2];
            let mut renderer = VelloRenderer::new();
            let submission = renderer.submit_frame(RenderFrameInput {
                viewport: ViewportInfo::new(
                    Size::new(256.0, 128.0),
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
