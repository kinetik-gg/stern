//! Public fixed-height virtual-tree composition conformance tests.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use stern_core::{
    Brush, Color, FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PathElement,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive,
    Rect, RepaintRequest, Response, ScaleFactor, SemanticActionKind, SemanticNode, SemanticRole,
    Size, TimeInfo, Transform, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    CollectionCursor, ItemId, Selection, TreeExpansion, TreeItem, TreeModel, TreeRow, Ui,
    VirtualTreeConfig, VirtualTreeOutput, VirtualTreeRow, VirtualTreeSelectionMode,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 160.0, 60.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 200.0, 100.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn item(raw: u64, parent: Option<u64>, has_children: bool) -> TreeItem {
    TreeItem {
        id: id(raw),
        parent: parent.map(id),
        has_children,
    }
}

fn roots(raw_ids: impl IntoIterator<Item = u64>) -> TreeModel {
    TreeModel::new(
        raw_ids
            .into_iter()
            .map(|raw| item(raw, None, false))
            .collect::<Vec<_>>(),
    )
}

fn nested_model() -> TreeModel {
    TreeModel::new(vec![
        item(10, None, true),
        item(11, Some(10), false),
        item(12, Some(10), true),
        item(13, Some(12), false),
        item(20, None, false),
    ])
}

fn config() -> VirtualTreeConfig {
    VirtualTreeConfig::new(BOUNDS, 20.0, 16.0)
        .label("Scene")
        .overscan(1)
        .selection_mode(VirtualTreeSelectionMode::Multiple)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 240.0),
            PhysicalSize::new(320, 240),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(
    x: f32,
    y: f32,
    pressed: bool,
    released: bool,
    modifiers: Modifiers,
    click_count: u8,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            primary: PointerButtonState::new(pressed, pressed, released),
            click_count,
            ..PointerInput::default()
        },
        keyboard: KeyboardInput {
            modifiers,
            events: Vec::new(),
        },
        ..UiInput::default()
    }
}

fn wheel_input(delta_y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(100.0, 10.0)),
            wheel_delta: Vec2::new(0.0, delta_y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key, modifiers: Modifiers, repeat: bool) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, repeat)],
        },
        ..UiInput::default()
    }
}

struct Run {
    tree_id: WidgetId,
    lower: Option<Response>,
    output: VirtualTreeOutput,
    callbacks: Vec<TreeRow>,
    frame: stern_core::FrameOutput,
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

fn linear_channel(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast_ratio(foreground: Color, background: Color) -> f32 {
    let luminance = |color: Color| {
        0.2126 * linear_channel(color.r)
            + 0.7152 * linear_channel(color.g)
            + 0.0722 * linear_channel(color.b)
    };
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

fn assert_tree_row_focus(frame: &stern_core::FrameOutput, rect: Rect, has_children: bool) -> usize {
    let theme = default_dark_theme();
    let base_index = frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == rect))
        .expect("virtual-tree row base");
    let Primitive::Rect(base) = &frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(base.radius, theme.radii.none);
    assert_eq!(
        base.stroke.expect("neutral row boundary").brush,
        Brush::Solid(theme.colors.border.subtle)
    );
    assert_eq!(
        base.stroke.expect("neutral row boundary").width,
        theme.strokes.hairline
    );
    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(rect, base.radius, base.stroke.unwrap().width);
    assert_eq!(frame.primitives[base_index + 1], expected[0]);
    assert_eq!(frame.primitives[base_index + 2], expected[1]);
    for primitive in &frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("virtual-tree focus must remain a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        let bounds = path_bounds(&path.elements);
        assert!(
            [
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                bounds.max_x(),
                bounds.max_y(),
            ]
            .into_iter()
            .all(f32::is_finite)
        );
        assert!(rect.contains_rect(bounds));
    }
    let content_index = base_index + 3;
    if has_children {
        assert!(matches!(
            frame.primitives[content_index],
            Primitive::Line(_)
        ));
        assert!(matches!(
            frame.primitives[content_index + 1],
            Primitive::Line(_)
        ));
        assert!(matches!(
            frame.primitives[content_index + 2],
            Primitive::Text(_)
        ));
    } else {
        assert!(matches!(
            frame.primitives[content_index],
            Primitive::Text(_)
        ));
    }
    base_index
}

fn primitives_without_focus_paths(frame: &stern_core::FrameOutput) -> Vec<Primitive> {
    frame
        .primitives
        .iter()
        .filter(|primitive| !matches!(primitive, Primitive::Path(_)))
        .cloned()
        .collect()
}

fn output_without_focus(mut output: VirtualTreeOutput) -> VirtualTreeOutput {
    for response in &mut output.responses {
        response.response.state.focused = false;
        if let Some(disclosure) = &mut response.disclosure_response {
            disclosure.state.focused = false;
        }
    }
    output
}

fn semantics_without_focus(frame: &stern_core::FrameOutput) -> Vec<SemanticNode> {
    frame
        .semantics
        .nodes()
        .iter()
        .cloned()
        .map(|mut node| {
            node.state.focused = false;
            node
        })
        .collect()
}

fn assert_focus_only_transition(focused: &Run, unfocused: &Run) {
    assert_eq!(focused.callbacks, unfocused.callbacks);
    assert_eq!(focused.frame.repaint, unfocused.frame.repaint);
    assert_eq!(
        output_without_focus(focused.output.clone()),
        unfocused.output
    );
    assert_eq!(
        primitives_without_focus_paths(&focused.frame),
        unfocused.frame.primitives
    );
    assert_eq!(
        semantics_without_focus(&focused.frame),
        unfocused.frame.semantics.nodes()
    );
    assert_eq!(
        focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        2
    );
}

#[allow(clippy::too_many_arguments)]
fn run_frame(
    model: &TreeModel,
    config: VirtualTreeConfig,
    expansion: &mut TreeExpansion,
    cursor: &mut CollectionCursor,
    selection: &mut Selection,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let tree = ui
        .prepare_virtual_tree("tree", config, model, expansion)
        .expect("valid tree");
    let tree_id = tree.widget_id();
    let lower_id = ui.make_id("lower");
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
        }
        tree.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid shared pointer plan");
    let lower_response = lower.then(|| ui.pressable("lower", LOWER, false));
    let mut callbacks = Vec::new();
    let output = ui.virtual_tree(&tree, cursor, selection, expansion, |row| {
        callbacks.push(row);
        VirtualTreeRow::new(format!("Row {}", row.id.raw()))
    });
    let frame = ui.finish_output();
    Run {
        tree_id,
        lower: lower_response,
        output,
        callbacks,
        frame,
    }
}

#[allow(clippy::cast_precision_loss, clippy::too_many_arguments)]
fn click_at(
    row: usize,
    x: f32,
    modifiers: Modifiers,
    click_count: u8,
    model: &TreeModel,
    expansion: &mut TreeExpansion,
    cursor: &mut CollectionCursor,
    selection: &mut Selection,
    memory: &mut UiMemory,
    lower: bool,
) -> Run {
    let y = row as f32 * 20.0 + 10.0;
    let _ = run_frame(
        model,
        config(),
        expansion,
        cursor,
        selection,
        memory,
        pointer_input(x, y, true, false, modifiers, click_count),
        lower,
    );
    run_frame(
        model,
        config(),
        expansion,
        cursor,
        selection,
        memory,
        pointer_input(x, y, false, true, modifiers, click_count),
        lower,
    )
}

#[allow(clippy::too_many_arguments)]
fn click_row(
    row: usize,
    modifiers: Modifiers,
    model: &TreeModel,
    expansion: &mut TreeExpansion,
    cursor: &mut CollectionCursor,
    selection: &mut Selection,
    memory: &mut UiMemory,
) -> Run {
    click_at(
        row, 100.0, modifiers, 1, model, expansion, cursor, selection, memory, false,
    )
}

#[test]
fn focused_first_middle_and_last_leaf_rows_add_only_exact_owned_annuli() {
    let model = roots(0..5);
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );

    for (target, target_y) in [(0_u64, 0.0_f32), (2, 40.0), (4, 80.0)] {
        for selected in [false, true] {
            let mut unfocused_selection = Selection::new();
            if selected {
                unfocused_selection.replace(id(target));
            }
            let unfocused = run_frame(
                &model,
                config(),
                &mut TreeExpansion::new(),
                &mut CollectionCursor::new(),
                &mut unfocused_selection,
                &mut UiMemory::new(),
                UiInput::default(),
                false,
            );

            let mut focused_selection = Selection::new();
            if selected {
                focused_selection.replace(id(target));
            }
            let mut focused_memory = UiMemory::new();
            focused_memory.focus(seed.tree_id.child(("virtual-tree-row", target)));
            let focused = run_frame(
                &model,
                config(),
                &mut TreeExpansion::new(),
                &mut CollectionCursor::new(),
                &mut focused_selection,
                &mut focused_memory,
                UiInput::default(),
                false,
            );

            assert_focus_only_transition(&focused, &unfocused);
            assert_tree_row_focus(&focused.frame, Rect::new(0.0, target_y, 160.0, 20.0), false);
        }
    }
}

#[test]
fn focused_collapsed_and_expanded_branches_preserve_disclosure_content() {
    let model = nested_model();
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.tree_id.child(("virtual-tree-row", 10_u64));

    for expanded in [false, true] {
        for selected in [false, true] {
            let mut unfocused_expansion = TreeExpansion::new();
            let mut focused_expansion = TreeExpansion::new();
            if expanded {
                unfocused_expansion.expand(id(10));
                focused_expansion.expand(id(10));
            }
            let mut unfocused_selection = Selection::new();
            let mut focused_selection = Selection::new();
            if selected {
                unfocused_selection.replace(id(10));
                focused_selection.replace(id(10));
            }
            let unfocused = run_frame(
                &model,
                config(),
                &mut unfocused_expansion,
                &mut CollectionCursor::new(),
                &mut unfocused_selection,
                &mut UiMemory::new(),
                UiInput::default(),
                false,
            );
            let mut memory = UiMemory::new();
            memory.focus(row_id);
            let focused = run_frame(
                &model,
                config(),
                &mut focused_expansion,
                &mut CollectionCursor::new(),
                &mut focused_selection,
                &mut memory,
                UiInput::default(),
                false,
            );

            assert_focus_only_transition(&focused, &unfocused);
            assert_tree_row_focus(&focused.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
            let semantic = focused
                .frame
                .semantics
                .get(row_id)
                .expect("branch semantic");
            assert_eq!(semantic.state.expanded, Some(expanded));
            assert_eq!(semantic.state.selected, selected);
            assert!(semantic.state.focused);
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn focused_branch_annuli_are_invariant_across_row_and_disclosure_interactions() {
    let model = nested_model();
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.tree_id.child(("virtual-tree-row", 10_u64));

    for expanded in [false, true] {
        for selected in [false, true] {
            let mut idle_expansion = TreeExpansion::new();
            if expanded {
                idle_expansion.expand(id(10));
            }
            let mut idle_selection = Selection::new();
            if selected {
                idle_selection.replace(id(10));
            }
            let mut idle_memory = UiMemory::new();
            idle_memory.focus(row_id);
            let idle = run_frame(
                &model,
                config(),
                &mut idle_expansion,
                &mut CollectionCursor::new(),
                &mut idle_selection,
                &mut idle_memory,
                UiInput::default(),
                false,
            );
            let idle_base =
                assert_tree_row_focus(&idle.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
            let idle_annuli = &idle.frame.primitives[idle_base + 1..=idle_base + 2];

            for (name, input) in [
                (
                    "row-hover",
                    pointer_input(100.0, 10.0, false, false, Modifiers::default(), 1),
                ),
                (
                    "row-press",
                    pointer_input(100.0, 10.0, true, false, Modifiers::default(), 1),
                ),
                (
                    "disclosure-hover",
                    pointer_input(8.0, 10.0, false, false, Modifiers::default(), 1),
                ),
                (
                    "disclosure-press",
                    pointer_input(8.0, 10.0, true, false, Modifiers::default(), 1),
                ),
            ] {
                let mut expansion = TreeExpansion::new();
                if expanded {
                    expansion.expand(id(10));
                }
                let mut selection = Selection::new();
                if selected {
                    selection.replace(id(10));
                }
                let mut memory = UiMemory::new();
                memory.focus(row_id);
                let run = run_frame(
                    &model,
                    config(),
                    &mut expansion,
                    &mut CollectionCursor::new(),
                    &mut selection,
                    &mut memory,
                    input,
                    false,
                );
                let base =
                    assert_tree_row_focus(&run.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
                assert_eq!(
                    &run.frame.primitives[base + 1..=base + 2],
                    idle_annuli,
                    "{name}, expanded={expanded}, selected={selected}"
                );
                let response = run.output.responses[0];
                match name {
                    "row-hover" => assert!(response.response.state.hovered),
                    "row-press" => assert!(response.response.state.pressed),
                    "disclosure-hover" => assert!(
                        response
                            .disclosure_response
                            .expect("branch disclosure")
                            .state
                            .hovered
                    ),
                    "disclosure-press" => assert!(
                        response
                            .disclosure_response
                            .expect("branch disclosure")
                            .state
                            .pressed
                    ),
                    _ => unreachable!(),
                }
                assert!(!run.output.selection_changed, "{name}");
                assert!(!run.output.expansion_changed, "{name}");
                assert_eq!(run.output.activated, None, "{name}");
                assert_eq!(run.output.cursor_target, None, "{name}");
                assert!(memory.is_focused(row_id), "{name}");
            }
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn completed_disclosure_click_preserves_active_selected_row_focus_and_annuli() {
    let model = nested_model();

    for initially_expanded in [false, true] {
        let mut expansion = TreeExpansion::new();
        if initially_expanded {
            expansion.expand(id(10));
        }
        let mut cursor = CollectionCursor::new();
        let mut selection = Selection::new();
        let mut memory = UiMemory::new();
        let selected = click_row(
            0,
            Modifiers::default(),
            &model,
            &mut expansion,
            &mut cursor,
            &mut selection,
            &mut memory,
        );
        let row_id = selected.tree_id.child(("virtual-tree-row", 10_u64));
        let disclosure_id = selected.tree_id.child(("virtual-tree-disclosure", 10_u64));
        assert_eq!(cursor.active(), Some(id(10)));
        assert_eq!(selection.selected(), vec![id(10)]);
        assert!(memory.is_focused(row_id));
        assert_eq!(expansion.is_expanded(id(10)), initially_expanded);

        let baseline = run_frame(
            &model,
            config(),
            &mut expansion,
            &mut cursor,
            &mut selection,
            &mut memory,
            UiInput::default(),
            false,
        );
        let baseline_item = baseline.output.responses[0];
        assert_eq!(baseline_item.response.id, row_id);
        assert!(baseline_item.response.state.focused);
        assert!(baseline_item.response.state.selected);
        assert_eq!(baseline_item.row.expanded, initially_expanded);
        let baseline_base =
            assert_tree_row_focus(&baseline.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
        let baseline_annuli =
            baseline.frame.primitives[baseline_base + 1..=baseline_base + 2].to_vec();

        let toggled = click_at(
            0,
            8.0,
            Modifiers::default(),
            1,
            &model,
            &mut expansion,
            &mut cursor,
            &mut selection,
            &mut memory,
            false,
        );
        let toggled_item = toggled.output.responses[0];
        let disclosure = toggled_item
            .disclosure_response
            .expect("branch disclosure response");
        assert_eq!(toggled_item.response.id, row_id);
        assert_eq!(disclosure.id, disclosure_id);
        assert!(!toggled_item.response.clicked);
        assert!(disclosure.clicked);
        assert!(toggled_item.response.state.focused);
        assert!(toggled_item.response.state.selected);
        assert_eq!(toggled_item.row.expanded, initially_expanded);
        assert!(!toggled.output.selection_changed);
        assert!(toggled.output.expansion_changed);
        assert_eq!(toggled.output.toggled, Some(id(10)));
        assert_eq!(toggled.output.cursor_target, baseline.output.cursor_target);
        assert_eq!(toggled.output.activated, None);
        assert_eq!(cursor.active(), Some(id(10)));
        assert_eq!(selection.selected(), vec![id(10)]);
        assert!(memory.is_focused(row_id));
        assert_eq!(expansion.is_expanded(id(10)), !initially_expanded);
        let toggled_semantic = toggled
            .frame
            .semantics
            .get(row_id)
            .expect("toggled row semantic");
        assert!(toggled_semantic.state.focused);
        assert!(toggled_semantic.state.selected);
        assert_eq!(
            toggled_semantic.state.expanded,
            Some(initially_expanded),
            "the click frame keeps its prepared projection"
        );
        let toggled_base =
            assert_tree_row_focus(&toggled.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
        assert_eq!(
            &toggled.frame.primitives[toggled_base + 1..=toggled_base + 2],
            baseline_annuli.as_slice()
        );

        let following = run_frame(
            &model,
            config(),
            &mut expansion,
            &mut cursor,
            &mut selection,
            &mut memory,
            UiInput::default(),
            false,
        );
        let following_item = following.output.responses[0];
        let following_disclosure = following_item
            .disclosure_response
            .expect("following branch disclosure response");
        assert_eq!(following_item.response.id, row_id);
        assert_eq!(following_disclosure.id, disclosure_id);
        assert!(!following_item.response.clicked);
        assert!(!following_disclosure.clicked);
        assert!(following_item.response.state.focused);
        assert!(following_item.response.state.selected);
        assert_eq!(following_item.row.expanded, !initially_expanded);
        assert!(!following.output.selection_changed);
        assert!(!following.output.expansion_changed);
        assert_eq!(following.output.toggled, None);
        assert_eq!(
            following.output.cursor_target,
            baseline.output.cursor_target
        );
        assert_eq!(following.output.activated, None);
        assert_eq!(cursor.active(), Some(id(10)));
        assert_eq!(selection.selected(), vec![id(10)]);
        assert!(memory.is_focused(row_id));
        assert_eq!(
            following
                .callbacks
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            if initially_expanded {
                vec![id(10), id(20)]
            } else {
                vec![id(10), id(11), id(12), id(20)]
            }
        );
        let following_semantic = following
            .frame
            .semantics
            .get(row_id)
            .expect("following row semantic");
        assert!(following_semantic.state.focused);
        assert!(following_semantic.state.selected);
        assert_eq!(following_semantic.state.expanded, Some(!initially_expanded));
        let following_base =
            assert_tree_row_focus(&following.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
        assert_eq!(
            &following.frame.primitives[following_base + 1..=following_base + 2],
            baseline_annuli.as_slice()
        );
    }
}

#[test]
fn disclosure_identity_isolated_from_row_focus_selection_activation_and_cursor() {
    let model = nested_model();
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.tree_id.child(("virtual-tree-row", 10_u64));
    let disclosure_id = seed.tree_id.child(("virtual-tree-disclosure", 10_u64));

    let mut disclosure_focus = UiMemory::new();
    disclosure_focus.focus(disclosure_id);
    let disclosure_focused = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut disclosure_focus,
        UiInput::default(),
        false,
    );
    assert!(
        disclosure_focused
            .frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
    assert!(
        !disclosure_focused.output.responses[0]
            .response
            .state
            .focused
    );
    assert!(
        !disclosure_focused
            .frame
            .semantics
            .get(row_id)
            .expect("row semantic")
            .state
            .focused
    );

    let mut disclosure_expansion = TreeExpansion::new();
    let mut disclosure_cursor = CollectionCursor::new();
    let mut disclosure_selection = Selection::new();
    let mut disclosure_memory = UiMemory::new();
    let toggled = click_at(
        0,
        8.0,
        Modifiers::default(),
        1,
        &model,
        &mut disclosure_expansion,
        &mut disclosure_cursor,
        &mut disclosure_selection,
        &mut disclosure_memory,
        false,
    );
    assert!(toggled.output.expansion_changed);
    assert_eq!(toggled.output.toggled, Some(id(10)));
    assert_eq!(toggled.output.activated, None);
    assert_eq!(toggled.output.cursor_target, None);
    assert_eq!(disclosure_cursor.active(), None);
    assert!(disclosure_selection.selected().is_empty());
    assert!(!disclosure_memory.is_focused(row_id));
    assert!(
        toggled
            .frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );

    let mut row_expansion = TreeExpansion::new();
    let mut row_cursor = CollectionCursor::new();
    let mut row_selection = Selection::new();
    let mut row_memory = UiMemory::new();
    let selected = click_row(
        0,
        Modifiers::default(),
        &model,
        &mut row_expansion,
        &mut row_cursor,
        &mut row_selection,
        &mut row_memory,
    );
    assert_eq!(row_cursor.active(), Some(id(10)));
    assert_eq!(row_selection.selected(), vec![id(10)]);
    assert!(row_memory.is_focused(row_id));
    assert!(!selected.output.expansion_changed);
    assert_eq!(selected.output.toggled, None);
    assert_eq!(selected.output.activated, None);
    assert!(row_expansion.expanded().is_empty());
    assert_tree_row_focus(&selected.frame, Rect::new(0.0, 0.0, 160.0, 20.0), true);
}

#[test]
#[allow(clippy::too_many_lines)]
fn selected_branch_states_inventory_the_exact_label_and_disclosure_exception() {
    let theme = default_dark_theme();
    assert_eq!(
        theme.colors.selection.background,
        Color::rgb8(0x0C, 0x8C, 0xE9)
    );
    assert_eq!(theme.colors.selection.foreground, Color::WHITE);
    let ratio = contrast_ratio(
        theme.colors.selection.foreground,
        theme.colors.selection.background,
    );
    assert!((ratio - 3.53).abs() < 0.01);
    assert!(
        ratio < 4.5,
        "known product exception is not AA normal-text compliance"
    );

    let model = nested_model();
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.tree_id.child(("virtual-tree-row", 10_u64));
    let mut orientation_glyphs = Vec::new();

    for expanded in [false, true] {
        let mut baseline_glyph = None;
        let mut baseline_label = None;
        for (name, input, focused) in [
            ("selected-only", UiInput::default(), false),
            (
                "selected+row-hovered",
                pointer_input(100.0, 10.0, false, false, Modifiers::default(), 1),
                false,
            ),
            (
                "selected+row-pressed",
                pointer_input(100.0, 10.0, true, false, Modifiers::default(), 1),
                false,
            ),
            (
                "selected+disclosure-hovered",
                pointer_input(8.0, 10.0, false, false, Modifiers::default(), 1),
                false,
            ),
            (
                "selected+disclosure-pressed",
                pointer_input(8.0, 10.0, true, false, Modifiers::default(), 1),
                false,
            ),
            ("selected+focused", UiInput::default(), true),
            (
                "selected+focused+row-hovered",
                pointer_input(100.0, 10.0, false, false, Modifiers::default(), 1),
                true,
            ),
            (
                "selected+focused+row-pressed",
                pointer_input(100.0, 10.0, true, false, Modifiers::default(), 1),
                true,
            ),
            (
                "selected+focused+disclosure-hovered",
                pointer_input(8.0, 10.0, false, false, Modifiers::default(), 1),
                true,
            ),
            (
                "selected+focused+disclosure-pressed",
                pointer_input(8.0, 10.0, true, false, Modifiers::default(), 1),
                true,
            ),
        ] {
            let mut expansion = TreeExpansion::new();
            if expanded {
                expansion.expand(id(10));
            }
            let mut selection = Selection::new();
            selection.replace(id(10));
            let mut memory = UiMemory::new();
            if focused {
                memory.focus(row_id);
            }
            let run = run_frame(
                &model,
                config(),
                &mut expansion,
                &mut CollectionCursor::new(),
                &mut selection,
                &mut memory,
                input,
                false,
            );
            let row_rect = Rect::new(0.0, 0.0, 160.0, 20.0);
            let base_index = run
                .frame
                .primitives
                .iter()
                .position(
                    |primitive| matches!(primitive, Primitive::Rect(base) if base.rect == row_rect),
                )
                .expect("selected branch base");
            let Primitive::Rect(base) = &run.frame.primitives[base_index] else {
                unreachable!()
            };
            assert_eq!(
                base.fill,
                Some(Brush::Solid(theme.colors.selection.background)),
                "{name}, expanded={expanded}"
            );
            assert_eq!(
                base.stroke.expect("neutral boundary").brush,
                Brush::Solid(theme.colors.border.subtle),
                "{name}, expanded={expanded}"
            );
            let content_index = base_index + if focused { 3 } else { 1 };
            let glyph = run.frame.primitives[content_index..content_index + 2].to_vec();
            for primitive in &glyph {
                let Primitive::Line(line) = primitive else {
                    panic!("branch disclosure must remain two lines");
                };
                assert_eq!(
                    line.stroke.brush,
                    Brush::Solid(theme.colors.selection.foreground),
                    "{name}, expanded={expanded}"
                );
            }
            let label = run.frame.primitives[content_index + 2].clone();
            let Primitive::Text(text) = &label else {
                panic!("branch label must remain after disclosure");
            };
            assert_eq!(text.text, "Row 10");
            assert_eq!(
                text.brush,
                Brush::Solid(theme.colors.selection.foreground),
                "{name}, expanded={expanded}"
            );

            if name == "selected-only" {
                baseline_glyph = Some(glyph.clone());
                baseline_label = Some(label.clone());
                orientation_glyphs.push(glyph.clone());
            } else {
                assert_eq!(
                    glyph,
                    baseline_glyph.clone().expect("baseline glyph"),
                    "{name}"
                );
                assert_eq!(
                    label,
                    baseline_label.clone().expect("baseline label"),
                    "{name}"
                );
            }
            assert_eq!(
                run.frame
                    .primitives
                    .iter()
                    .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                    .count(),
                if focused { 2 } else { 0 },
                "{name}, expanded={expanded}"
            );
            if focused {
                assert_tree_row_focus(&run.frame, row_rect, true);
            }
            let response = run.output.responses[0];
            assert!(response.response.state.selected, "{name}");
            assert_eq!(response.response.state.focused, focused, "{name}");
            assert_eq!(response.row.expanded, expanded, "{name}");
            assert!(!run.output.selection_changed, "{name}");
            assert!(!run.output.expansion_changed, "{name}");
            assert_eq!(run.output.activated, None, "{name}");
            let semantic = run.frame.semantics.get(row_id).expect("branch semantic");
            assert!(semantic.state.selected, "{name}");
            assert_eq!(semantic.state.focused, focused, "{name}");
            assert_eq!(semantic.state.expanded, Some(expanded), "{name}");
        }
    }
    assert_eq!(orientation_glyphs.len(), 2);
    assert_ne!(orientation_glyphs[0], orientation_glyphs[1]);
}

#[test]
fn disabled_retained_row_focus_suppresses_annuli_and_remains_non_focusable() {
    let theme = default_dark_theme();
    let model = nested_model();
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );
    let row_id = seed.tree_id.child(("virtual-tree-row", 10_u64));
    let mut memory = UiMemory::new();
    memory.focus(row_id);
    let mut selection = Selection::new();
    selection.replace(id(10));
    let disabled = run_frame(
        &model,
        config().disabled(true),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );

    assert!(disabled.output.responses[0].response.state.focused);
    assert!(disabled.output.responses[0].response.state.selected);
    assert!(
        disabled
            .frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::Path(_)))
    );
    let semantic = disabled
        .frame
        .semantics
        .get(row_id)
        .expect("disabled row semantic");
    assert!(!semantic.focusable);
    assert!(semantic.state.disabled);
    assert!(semantic.state.selected);
    let base_index = disabled
        .frame
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Rect(base) if base.rect == Rect::new(0.0, 0.0, 160.0, 20.0))
        })
        .expect("disabled branch base");
    let Primitive::Rect(base) = &disabled.frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(
        base.fill,
        Some(Brush::Solid(theme.colors.surface.control_disabled))
    );
    for primitive in &disabled.frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Line(line) = primitive else {
            panic!("disabled branch disclosure line");
        };
        assert_eq!(
            line.stroke.brush,
            Brush::Solid(theme.colors.content.disabled)
        );
    }
    let Primitive::Text(label) = &disabled.frame.primitives[base_index + 3] else {
        panic!("disabled branch label");
    };
    assert_eq!(label.brush, Brush::Solid(theme.colors.content.disabled));
    assert!(!disabled.output.selection_changed);
    assert!(!disabled.output.expansion_changed);
    assert_eq!(disabled.output.activated, None);
}

#[test]
#[allow(clippy::too_many_lines)]
fn fractional_scroll_preserves_clip_transform_and_full_logical_row_annuli() {
    let model = roots(0..20);
    let seed = run_frame(
        &model,
        config(),
        &mut TreeExpansion::new(),
        &mut CollectionCursor::new(),
        &mut Selection::new(),
        &mut UiMemory::new(),
        UiInput::default(),
        false,
    );

    for (target, logical_y, semantic_y, semantic_height) in
        [(0_u64, 0.0_f32, 0.0_f32, 9.5_f32), (3, 60.0, 49.5, 10.5)]
    {
        let mut unfocused_memory = UiMemory::new();
        let _ = run_frame(
            &model,
            config(),
            &mut TreeExpansion::new(),
            &mut CollectionCursor::new(),
            &mut Selection::new(),
            &mut unfocused_memory,
            wheel_input(-10.5),
            false,
        );
        let unfocused = run_frame(
            &model,
            config(),
            &mut TreeExpansion::new(),
            &mut CollectionCursor::new(),
            &mut Selection::new(),
            &mut unfocused_memory,
            UiInput::default(),
            false,
        );

        let row_id = seed.tree_id.child(("virtual-tree-row", target));
        let mut focused_memory = UiMemory::new();
        focused_memory.focus(row_id);
        let _ = run_frame(
            &model,
            config(),
            &mut TreeExpansion::new(),
            &mut CollectionCursor::new(),
            &mut Selection::new(),
            &mut focused_memory,
            wheel_input(-10.5),
            false,
        );
        let focused = run_frame(
            &model,
            config(),
            &mut TreeExpansion::new(),
            &mut CollectionCursor::new(),
            &mut Selection::new(),
            &mut focused_memory,
            UiInput::default(),
            false,
        );

        assert_focus_only_transition(&focused, &unfocused);
        assert_eq!(focused.output.window.visible_range, 0..4);
        assert_eq!(focused.output.window.materialized_range, 0..5);
        assert_eq!(
            focused.output.window.clamped_scroll_offset.to_bits(),
            10.5_f32.to_bits()
        );
        assert_eq!(
            focused
                .callbacks
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            (0..5).map(id).collect::<Vec<_>>()
        );
        assert!(matches!(
            focused.frame.primitives[1],
            Primitive::ClipBegin { rect, .. } if rect == BOUNDS
        ));
        assert_eq!(
            focused.frame.primitives[2],
            Primitive::TransformBegin(Transform::translation(Vec2::new(0.0, -10.5)))
        );
        assert!(matches!(
            focused.frame.primitives[focused.frame.primitives.len() - 2],
            Primitive::TransformEnd
        ));
        assert!(matches!(
            focused.frame.primitives[focused.frame.primitives.len() - 1],
            Primitive::ClipEnd { .. }
        ));
        assert_tree_row_focus(
            &focused.frame,
            Rect::new(0.0, logical_y, 160.0, 20.0),
            false,
        );
        let semantic = focused
            .frame
            .semantics
            .get(row_id)
            .expect("partially clipped row semantic");
        assert_eq!(semantic.bounds.x.to_bits(), 0.0_f32.to_bits());
        assert!((semantic.bounds.y - semantic_y).abs() < 0.000_01);
        assert_eq!(semantic.bounds.width.to_bits(), 160.0_f32.to_bits());
        assert!((semantic.bounds.height - semantic_height).abs() < 0.000_01);
    }
}

#[test]
fn ten_thousand_roots_materialize_only_the_bounded_window() {
    let model = roots(0..10_000);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let run = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );

    assert_eq!(run.output.window.visible_range, 0..3);
    assert_eq!(run.output.window.materialized_range, 0..5);
    assert_eq!(
        run.callbacks.iter().map(|row| row.id).collect::<Vec<_>>(),
        vec![id(0), id(1), id(2), id(3), id(4)]
    );
    assert_eq!(run.output.responses.len(), 5);
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        5
    );
    let root = run.frame.semantics.get(run.tree_id).expect("tree root");
    assert_eq!(root.role, SemanticRole::List);
    assert_eq!(root.children.len(), 3);
}

#[test]
fn wheel_freezes_geometry_then_projects_strict_visible_semantics() {
    let model = roots(0..20);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let current = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        wheel_input(-10.0),
        false,
    );
    assert_eq!(current.output.window.visible_range, 0..3);
    assert_eq!(current.output.scroll.offset.y.to_bits(), 10.0_f32.to_bits());

    let next = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(next.output.window.visible_range, 0..4);
    let root = next.frame.semantics.get(next.tree_id).expect("tree root");
    assert_eq!(root.children.len(), 4);
    let first = next
        .frame
        .semantics
        .get(next.tree_id.child(("virtual-tree-row", 0_u64)))
        .expect("first partial row");
    let last = next
        .frame
        .semantics
        .get(next.tree_id.child(("virtual-tree-row", 3_u64)))
        .expect("last partial row");
    assert_eq!(first.bounds.y.to_bits(), 0.0_f32.to_bits());
    assert_eq!(first.bounds.height.to_bits(), 10.0_f32.to_bits());
    assert_eq!(last.bounds.y.to_bits(), 50.0_f32.to_bits());
    assert_eq!(last.bounds.height.to_bits(), 10.0_f32.to_bits());
}

#[test]
fn disclosure_toggle_is_isolated_and_changes_next_frame_geometry() {
    let model = nested_model();
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    let toggled = click_at(
        0,
        8.0,
        Modifiers::default(),
        1,
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        true,
    );
    assert!(toggled.lower.is_some_and(|response| !response.clicked));
    assert!(toggled.output.expansion_changed);
    assert_eq!(toggled.output.toggled, Some(id(10)));
    assert!(selection.selected().is_empty());
    assert_eq!(cursor.active(), None);
    assert_eq!(
        toggled
            .callbacks
            .iter()
            .map(|row| row.id)
            .collect::<Vec<_>>(),
        vec![id(10), id(20)]
    );

    let expanded = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(
        expanded
            .callbacks
            .iter()
            .map(|row| (row.id, row.depth))
            .collect::<Vec<_>>(),
        vec![(id(10), 0), (id(11), 1), (id(12), 1), (id(20), 0)]
    );
    let branch = expanded
        .frame
        .semantics
        .get(expanded.tree_id.child(("virtual-tree-row", 10_u64)))
        .expect("expanded branch");
    assert_eq!(branch.state.expanded, Some(true));
    assert!(
        branch
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::Close)
    );
    let leaf = expanded
        .frame
        .semantics
        .get(expanded.tree_id.child(("virtual-tree-row", 11_u64)))
        .expect("leaf");
    assert_eq!(leaf.state.expanded, None);
    let origins = expanded
        .frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text.origin.x),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(origins[1] > origins[0]);
}

#[test]
fn row_click_selection_uses_flattened_visible_order() {
    let model = nested_model();
    let mut expansion = TreeExpansion::new();
    expansion.expand(id(10));
    expansion.expand(id(12));
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();

    click_row(
        0,
        Modifiers::default(),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    click_row(
        2,
        Modifiers::new(false, true, false, false),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    assert_eq!(selection.selected(), vec![id(10), id(12)]);

    click_row(
        0,
        Modifiers::default(),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    click_row(
        2,
        Modifiers::new(true, false, false, false),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    assert_eq!(selection.selected(), vec![id(10), id(11), id(12)]);
}

#[test]
fn vertical_and_page_keys_select_focus_and_reveal() {
    let model = roots(0..20);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    click_row(
        0,
        Modifiers::default(),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );

    let navigated = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::PageDown, Modifiers::default(), false),
        false,
    );
    assert_eq!(cursor.active(), Some(id(3)));
    assert_eq!(selection.selected(), vec![id(3)]);
    assert_eq!(
        navigated
            .output
            .cursor_target
            .map(|target| target.projected_index),
        Some(3)
    );

    let revealed = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(
        revealed.output.window.clamped_scroll_offset.to_bits(),
        20.0_f32.to_bits()
    );
    assert!(memory.is_focused(revealed.tree_id.child(("virtual-tree-row", 3_u64))));
    assert_eq!(revealed.frame.repaint, RepaintRequest::None);
}

#[test]
fn left_right_expand_enter_child_parent_and_collapse() {
    let model = nested_model();
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    click_row(
        0,
        Modifiers::default(),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );

    let opened = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::ArrowRight, Modifiers::default(), false),
        false,
    );
    assert!(opened.output.expansion_changed);
    assert_eq!(opened.output.toggled, Some(id(10)));
    assert_eq!(cursor.active(), Some(id(10)));

    let _ = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    let child = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::ArrowRight, Modifiers::default(), false),
        false,
    );
    assert_eq!(
        child.output.cursor_target.map(|target| target.id),
        Some(id(11))
    );
    assert_eq!(selection.selected(), vec![id(11)]);

    let parent = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::ArrowLeft, Modifiers::default(), false),
        false,
    );
    assert_eq!(
        parent.output.cursor_target.map(|target| target.id),
        Some(id(10))
    );
    let collapsed = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::ArrowLeft, Modifiers::default(), false),
        false,
    );
    assert!(collapsed.output.expansion_changed);
    assert!(!expansion.is_expanded(id(10)));
    assert_eq!(cursor.active(), Some(id(10)));
}

#[test]
fn enter_space_and_double_click_activate_once_and_reject_repeat() {
    let model = roots(0..3);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    click_row(
        1,
        Modifiers::default(),
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );

    let enter = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::Enter, Modifiers::default(), false),
        false,
    );
    assert_eq!(enter.output.activated, Some(id(1)));
    assert_eq!(
        enter
            .output
            .responses
            .iter()
            .filter(|item| item.response.keyboard_activated)
            .count(),
        1
    );
    let repeated = run_frame(
        &model,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        key_input(Key::Space, Modifiers::default(), true),
        false,
    );
    assert_eq!(repeated.output.activated, None);

    let double = click_at(
        2,
        100.0,
        Modifiers::default(),
        2,
        &model,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        false,
    );
    assert_eq!(double.output.activated, Some(id(2)));

    let nested = nested_model();
    let mut nested_expansion = TreeExpansion::new();
    let disclosure_double = click_at(
        0,
        8.0,
        Modifiers::default(),
        2,
        &nested,
        &mut nested_expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        false,
    );
    assert_eq!(disclosure_double.output.activated, None);
}

#[test]
fn stable_ids_and_focused_removal_repair_are_deterministic() {
    let original = roots([1, 2, 3]);
    let reordered = roots([3, 2, 1]);
    let removed = roots([3, 1]);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    let clicked = click_row(
        1,
        Modifiers::default(),
        &original,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
    );
    let focused = clicked.tree_id.child(("virtual-tree-row", 2_u64));
    assert!(memory.is_focused(focused));

    let reordered_run = run_frame(
        &reordered,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert!(memory.is_focused(focused));
    assert!(reordered_run.frame.semantics.get(focused).is_some());

    let repaired = run_frame(
        &removed,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert_eq!(cursor.active(), Some(id(1)));
    assert!(!memory.is_focused(focused));
    assert!(memory.is_focused(repaired.tree_id.child(("virtual-tree-row", 1_u64))));
    assert!(repaired.frame.semantics.get(focused).is_none());
}

#[test]
fn empty_disabled_malformed_and_invalid_inputs_are_inert() {
    let empty = roots([]);
    let mut expansion = TreeExpansion::new();
    let mut cursor = CollectionCursor::new();
    let mut selection = Selection::new();
    let mut memory = UiMemory::new();
    let empty_run = run_frame(
        &empty,
        config(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        UiInput::default(),
        false,
    );
    assert!(empty_run.output.responses.is_empty());
    assert_eq!(
        empty_run
            .frame
            .semantics
            .get(empty_run.tree_id)
            .expect("empty tree root")
            .role,
        SemanticRole::List
    );

    let model = nested_model();
    let disabled = config().disabled(true);
    let _ = run_frame(
        &model,
        disabled.clone(),
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(8.0, 10.0, true, false, Modifiers::default(), 1),
        true,
    );
    let released = run_frame(
        &model,
        disabled,
        &mut expansion,
        &mut cursor,
        &mut selection,
        &mut memory,
        pointer_input(8.0, 10.0, false, true, Modifiers::default(), 1),
        true,
    );
    assert!(released.lower.is_some_and(|response| !response.clicked));
    assert!(selection.selected().is_empty());
    assert!(expansion.expanded().is_empty());
    assert_eq!(released.output.scroll.delta, Vec2::ZERO);
    assert!(
        released
            .frame
            .semantics
            .get(released.tree_id)
            .expect("disabled root")
            .state
            .disabled
    );

    let theme = default_dark_theme();
    let mut invalid_memory = UiMemory::new();
    let ui = Ui::begin_frame(context(UiInput::default()), &mut invalid_memory, &theme);
    assert!(
        ui.prepare_virtual_tree(
            "bad-bounds",
            VirtualTreeConfig::new(Rect::new(0.0, 0.0, f32::NAN, 60.0), 20.0, 16.0),
            &model,
            &expansion,
        )
        .is_none()
    );
    assert!(
        ui.prepare_virtual_tree(
            "bad-row",
            VirtualTreeConfig::new(BOUNDS, 0.0, 16.0),
            &model,
            &expansion,
        )
        .is_none()
    );
    let malformed = TreeModel::new(vec![item(1, None, false), item(1, None, false)]);
    assert!(
        ui.prepare_virtual_tree("malformed", config(), &malformed, &expansion)
            .is_none()
    );
}
