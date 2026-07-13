//! Public fixed-height virtual-tree composition conformance tests.

use std::time::Duration;

use kinetik_ui_core::{
    FrameContext, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point,
    PointerButtonState, PointerInput, PointerOrder, PointerTarget, Primitive, Rect, RepaintRequest,
    Response, ScaleFactor, SemanticActionKind, SemanticRole, Size, TimeInfo, UiInput, UiMemory,
    Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use kinetik_ui_widgets::{
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
    frame: kinetik_ui_core::FrameOutput,
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
