//! Public prepared-asset-browser MVP conformance tests.

use std::time::Duration;

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionSource, FrameContext, FrameOutput, ImageId,
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, PointerTarget, Primitive, Rect, RepaintRequest, Response,
    ScaleFactor, SemanticRole, Size, TimeInfo, UiInput, UiMemory, Vec2, ViewportInfo, WidgetId,
    default_dark_theme,
};
use kinetik_ui_widgets::asset_browser::{
    AssetBrowserConfig, AssetBrowserDropTargetKind, AssetBrowserItem, AssetBrowserItemRect,
    AssetBrowserLayout, AssetBrowserModel, AssetBrowserOutput, AssetBrowserRequest,
    AssetBrowserSort, AssetBrowserSortKey, AssetBrowserState, AssetBrowserViewMode,
    AssetIconFallback,
};
use kinetik_ui_widgets::{
    CollectionContextTarget, GridColumns, GridLayout, InlineEditCancelReason,
    InlineEditCommitReason, InlineEditRequest, ItemId, ListLayout, SortDirection, Ui,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 240.0, 120.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 280.0, 150.0);
const OUTSIDE: Rect = Rect::new(260.0, 180.0, 40.0, 30.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn asset(raw: u64, name: impl Into<String>, kind: impl Into<String>) -> AssetBrowserItem {
    AssetBrowserItem::new(id(raw), name, kind)
}

fn assets(raw_ids: impl IntoIterator<Item = u64>) -> AssetBrowserModel {
    AssetBrowserModel::new(
        raw_ids
            .into_iter()
            .map(|raw| asset(raw, format!("Asset {raw}"), "mesh"))
            .collect::<Vec<_>>(),
    )
}

fn layout(view_mode: AssetBrowserViewMode) -> AssetBrowserLayout {
    AssetBrowserLayout::new(
        view_mode,
        GridLayout {
            columns: GridColumns::Fixed(3),
            item_size: Size::new(72.0, 72.0),
            gap: 4.0,
        },
        ListLayout::new(28.0),
    )
    .with_overscan(1)
}

fn config(view_mode: AssetBrowserViewMode) -> AssetBrowserConfig {
    AssetBrowserConfig::new(BOUNDS, layout(view_mode))
        .label("Project assets")
        .selection_mode(kinetik_ui_widgets::asset_browser::AssetBrowserSelectionMode::Multiple)
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

fn primary_input(
    point: Point,
    down: bool,
    pressed: bool,
    released: bool,
    click_count: u8,
) -> UiInput {
    primary_input_with_modifiers(
        point,
        down,
        pressed,
        released,
        click_count,
        Modifiers::default(),
    )
}

fn primary_input_with_modifiers(
    point: Point,
    down: bool,
    pressed: bool,
    released: bool,
    click_count: u8,
    modifiers: Modifiers,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
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

fn secondary_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            secondary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn move_input(point: Point, delta: Vec2) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            delta,
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key) -> UiInput {
    key_input_with_modifiers(key, Modifiers::default())
}

fn key_input_with_modifiers(key: Key, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn typed_input(text: &str) -> UiInput {
    let event = KeyEvent::new(
        Key::Character(text.to_owned()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(text);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

struct Run {
    root: WidgetId,
    outside: WidgetId,
    items: Vec<AssetBrowserItemRect>,
    projected: Vec<ItemId>,
    lower: Option<Response>,
    output: AssetBrowserOutput,
    frame: FrameOutput,
}

#[allow(clippy::too_many_arguments)]
fn run_frame(
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
    reject_rename: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let scene = ui
        .prepare_asset_browser("project-assets", config, model, state)
        .expect("valid asset browser scene");
    let root = scene.widget_id();
    let items = scene.layout().items.clone();
    let projected = scene.projection().visible_ids();
    let lower_id = ui.make_id("lower");
    let outside = ui.make_id("outside");
    ui.register_id(outside);
    ui.resolve_pointer_targets(|plan| {
        if lower {
            plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
        }
        plan.target(PointerTarget::new(outside, OUTSIDE, PointerOrder::new(20)));
        scene.declare_pointer_targets(plan, PointerOrder::new(100), state);
    })
    .expect("valid shared pointer plan");
    let lower = lower.then(|| ui.pressable_with_id(lower_id, LOWER, false));
    let _ = ui.pressable_with_id(outside, OUTSIDE, false);
    let output = ui.asset_browser(
        &scene,
        state,
        |_target, _draft| reject_rename.then(|| "name already exists".to_owned()),
        |target| match target {
            CollectionContextTarget::Background(_) => vec![action("asset.create", "Create")],
            CollectionContextTarget::Item(_) => vec![action("asset.inspect", "Inspect")],
            CollectionContextTarget::Selection(_) => vec![action("asset.delete", "Delete")],
        },
    );
    let frame = ui.finish_output();
    Run {
        root,
        outside,
        items,
        projected,
        lower,
        output,
        frame,
    }
}

fn click(
    point: Point,
    click_count: u8,
    modifiers: Modifiers,
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        config.clone(),
        state,
        memory,
        primary_input_with_modifiers(point, true, true, false, click_count, modifiers),
        false,
        false,
    );
    run_frame(
        model,
        config,
        state,
        memory,
        primary_input_with_modifiers(point, false, false, true, click_count, modifiers),
        false,
        false,
    )
}

fn context_click(
    point: Point,
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        config.clone(),
        state,
        memory,
        secondary_input(point, true, true, false),
        false,
        false,
    );
    run_frame(
        model,
        config,
        state,
        memory,
        secondary_input(point, false, false, true),
        false,
        false,
    )
}

fn semantic_center(frame: &FrameOutput, label: &str) -> Point {
    frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing semantic node {label}"))
        .bounds
        .center()
}

#[test]
fn prepared_scene_bounds_ten_thousand_items_and_strictly_clips_paint_and_semantics() {
    let model = assets(0..10_000);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let hovered = run_frame(
        &model,
        config(AssetBrowserViewMode::Grid),
        &mut state,
        &mut memory,
        primary_input(Point::new(36.0, 36.0), false, false, false, 0),
        true,
        false,
    );

    assert_eq!(hovered.output.visible_range, 0..6);
    assert_eq!(hovered.output.materialized_range, 0..12);
    assert_eq!(hovered.items.len(), 12);
    assert_eq!(hovered.output.responses.len(), 12);
    assert!(
        hovered
            .lower
            .is_some_and(|response| !response.state.hovered)
    );
    assert_eq!(
        hovered
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Text(_)))
            .count(),
        36
    );
    let clip_begin = hovered
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::ClipBegin { .. }))
        .expect("asset paint clip begins");
    let clip_end = hovered
        .frame
        .primitives
        .iter()
        .rposition(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
        .expect("asset paint clip ends");
    assert!(clip_begin < clip_end);
    let root = hovered
        .frame
        .semantics
        .get(hovered.root)
        .expect("asset browser semantics root");
    assert_eq!(root.role, SemanticRole::Grid);
    assert_eq!(root.label.as_deref(), Some("Project assets"));
    assert_eq!(root.children.len(), 6);
    assert_eq!(
        root.children[0],
        hovered.root.child(("asset-browser-item", 0_u64))
    );
    hovered
        .frame
        .semantics
        .validate()
        .expect("valid strict semantic tree");
    assert!(hovered.frame.warnings.is_empty());
}

#[test]
fn dynamic_filter_sort_and_view_changes_preserve_selection_and_repair_focus_by_stable_id() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "Zeta Scene", "scene").with_tags(["world"]),
        asset(2, "alpha Rock", "mesh").with_tags(["stone"]),
        asset(3, "Beta", "material").with_tags(["SURFACE"]),
        asset(4, "alpha Sky", "image").with_tags(["environment"]),
        asset(5, "alpha Rock", "mesh").with_tags(["duplicate"]),
    ]);
    let alpha =
        config(AssetBrowserViewMode::List)
            .query("ALPHA")
            .sort(Some(AssetBrowserSort::new(
                AssetBrowserSortKey::Name,
                SortDirection::Ascending,
            )));
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let first = run_frame(
        &model,
        alpha.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    assert_eq!(first.projected, vec![id(2), id(5), id(4)]);
    let selected = click(
        first.items[0].rect.center(),
        1,
        Modifiers::default(),
        &model,
        alpha,
        &mut state,
        &mut memory,
    );
    assert_eq!(state.selection.selected(), vec![id(2)]);
    assert_eq!(state.cursor.active(), Some(id(2)));
    assert!(memory.is_focused(selected.root.child(("asset-browser-item", 2_u64))));

    let grid = run_frame(
        &model,
        config(AssetBrowserViewMode::Grid)
            .query("mesh")
            .sort(Some(AssetBrowserSort::new(
                AssetBrowserSortKey::Name,
                SortDirection::Ascending,
            ))),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    assert_eq!(grid.output.view_mode, AssetBrowserViewMode::Grid);
    assert_eq!(grid.projected, vec![id(2), id(5)]);
    assert_eq!(state.selection.selected(), vec![id(2)]);
    assert_eq!(state.cursor.active(), Some(id(2)));
    assert!(memory.is_focused(grid.root.child(("asset-browser-item", 2_u64))));

    let tag_filtered = run_frame(
        &model,
        config(AssetBrowserViewMode::List).query("surface"),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    assert_eq!(tag_filtered.projected, vec![id(3)]);
    assert_eq!(state.selection.selected(), vec![id(2)]);
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert!(memory.is_focused(tag_filtered.root.child(("asset-browser-item", 3_u64))));
}

#[test]
fn keyboard_range_navigation_skips_disabled_assets_and_keeps_focus_deterministic() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "First", "mesh"),
        asset(2, "Disabled", "mesh").disabled(true),
        asset(3, "Third", "mesh"),
        asset(4, "Fourth", "mesh"),
    ]);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        config(AssetBrowserViewMode::List),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let selected = click(
        idle.items[0].rect.center(),
        1,
        Modifiers::default(),
        &model,
        config(AssetBrowserViewMode::List),
        &mut state,
        &mut memory,
    );
    let ranged = run_frame(
        &model,
        config(AssetBrowserViewMode::List),
        &mut state,
        &mut memory,
        key_input_with_modifiers(Key::ArrowDown, Modifiers::new(true, false, false, false)),
        false,
        false,
    );

    assert!(ranged.output.selection_changed);
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert_eq!(state.selection.selected(), vec![id(1), id(3)]);
    assert!(memory.is_focused(selected.root.child(("asset-browser-item", 3_u64))));
    assert!(!ranged.output.responses[1].response.state.focused);
}

#[test]
fn thumbnail_and_fallback_paths_emit_backend_independent_primitives() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "Thumbnail", "image").with_thumbnail(ImageId::from_raw(77)),
        asset(2, "Fallback", "material").with_fallback(AssetIconFallback::new("material", "MAT")),
    ]);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let frame = run_frame(
        &model,
        config(AssetBrowserViewMode::List),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );

    assert_eq!(
        frame
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Image(image) if image.image == ImageId::from_raw(77)))
            .count(),
        1
    );
    assert!(
        frame
            .frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "MAT"))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn f2_and_name_double_click_cover_rename_draft_commit_cancel_focus_loss_and_conflict() {
    let model = AssetBrowserModel::new(vec![asset(1, "Stone", "material")]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let selected = click(
        idle.items[0].rect.center(),
        1,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let begin = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Function(2)),
        false,
        false,
    );
    assert!(matches!(
        begin.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Begin(request))]
            if request.target == id(1)
    ));
    assert_eq!(state.rename_target(), Some(id(1)));

    let drafted = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        typed_input("X"),
        false,
        false,
    );
    assert!(matches!(
        drafted.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::DraftEdit(draft))]
            if draft.target == id(1) && draft.draft_text == "StoneX"
    ));
    assert_eq!(state.rename_draft(), Some("StoneX"));
    let rename_id = selected.root.child(("inline-edit", 1_u64));
    assert_eq!(
        drafted
            .frame
            .semantics
            .get(rename_id)
            .expect("rename field semantics")
            .role,
        SemanticRole::TextField
    );

    let conflict = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Enter),
        false,
        true,
    );
    let conflict = conflict.output.rename_conflict.expect("caller conflict");
    assert_eq!(conflict.target, id(1));
    assert_eq!(conflict.draft_text, "StoneX");
    assert_eq!(conflict.message, "name already exists");
    assert_eq!(state.rename_target(), Some(id(1)));
    assert!(memory.is_focused(rename_id));

    let committed = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Enter),
        false,
        false,
    );
    assert!(matches!(
        committed.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Commit(commit))]
            if commit.target == id(1)
                && commit.draft_text == "StoneX"
                && commit.reason == InlineEditCommitReason::Enter
    ));
    assert_eq!(state.rename_target(), None);
    assert_eq!(model.item_by_id(id(1)).expect("app item").name, "Stone");

    let mut cancel_state = AssetBrowserState::new();
    let mut cancel_memory = UiMemory::new();
    let cancel_idle = run_frame(
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
        UiInput::default(),
        false,
        false,
    );
    let double = click(
        cancel_idle.items[0].name_rect.center(),
        2,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
    );
    assert!(matches!(
        double.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Begin(request))]
            if request.target == id(1)
    ));
    let cancelled = run_frame(
        &model,
        cfg.clone(),
        &mut cancel_state,
        &mut cancel_memory,
        key_input(Key::Escape),
        false,
        false,
    );
    assert!(matches!(
        cancelled.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(1) && cancel.reason == InlineEditCancelReason::Escape
    ));

    let mut focus_state = AssetBrowserState::new();
    let mut focus_memory = UiMemory::new();
    let focus_idle = run_frame(
        &model,
        cfg.clone(),
        &mut focus_state,
        &mut focus_memory,
        UiInput::default(),
        false,
        false,
    );
    let _ = click(
        focus_idle.items[0].rect.center(),
        1,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut focus_state,
        &mut focus_memory,
    );
    let focus_begin = run_frame(
        &model,
        cfg.clone(),
        &mut focus_state,
        &mut focus_memory,
        key_input(Key::Function(2)),
        false,
        false,
    );
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut focus_state,
        &mut focus_memory,
        typed_input("Y"),
        false,
        false,
    );
    focus_memory.focus(focus_begin.outside);
    let focus_lost = run_frame(
        &model,
        cfg,
        &mut focus_state,
        &mut focus_memory,
        UiInput::default(),
        false,
        false,
    );
    assert!(matches!(
        focus_lost.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Commit(commit))]
            if commit.target == id(1)
                && commit.draft_text == "StoneY"
                && commit.reason == InlineEditCommitReason::FocusLost
    ));
    assert_eq!(focus_state.rename_target(), None);
}

#[test]
#[allow(clippy::too_many_lines)]
fn selection_drag_rejects_self_and_disabled_targets_then_accepts_item_and_background() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
        asset(4, "Disabled", "mesh").disabled(true),
    ]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let one = idle.items[0].rect.center();
    let two = idle.items[1].rect.center();
    let three = idle.items[2].rect.center();
    let disabled = idle.items[3].rect.center();
    let _ = click(
        one,
        1,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let _ = click(
        two,
        1,
        Modifiers::new(false, true, false, false),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    assert_eq!(state.selection.selected(), vec![id(1), id(2)]);

    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(one, true, true, false, 1),
        false,
        false,
    );
    let self_rejected = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        move_input(two, Vec2::new(two.x - one.x, two.y - one.y)),
        false,
        false,
    );
    assert_eq!(
        self_rejected
            .output
            .drag_payload
            .as_ref()
            .map(|source| source.items.clone()),
        Some(vec![id(1), id(2)])
    );
    assert_eq!(self_rejected.output.drop_preview, None);
    let rejected_release = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(two, false, false, true, 1),
        false,
        false,
    );
    assert!(
        rejected_release
            .output
            .requests
            .iter()
            .all(|request| !matches!(request, AssetBrowserRequest::Drop(_)))
    );

    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(one, true, true, false, 1),
        false,
        false,
    );
    let disabled_rejected = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        move_input(disabled, Vec2::new(disabled.x - one.x, disabled.y - one.y)),
        false,
        false,
    );
    assert_eq!(disabled_rejected.output.drop_preview, None);
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(disabled, false, false, true, 1),
        false,
        false,
    );

    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(one, true, true, false, 1),
        false,
        false,
    );
    let accepted_preview = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        move_input(three, Vec2::new(three.x - one.x, three.y - one.y)),
        false,
        false,
    );
    assert!(matches!(
        accepted_preview.output.drop_preview.as_ref().map(|drop| drop.kind),
        Some(AssetBrowserDropTargetKind::Item { target }) if target == id(3)
    ));
    let accepted = run_frame(
        &model,
        cfg,
        &mut state,
        &mut memory,
        primary_input(three, false, false, true, 1),
        false,
        false,
    );
    assert!(matches!(
        accepted.output.requests.as_slice(),
        [AssetBrowserRequest::Drop(drop)]
            if drop.source.items == vec![id(1), id(2)]
                && drop.kind == AssetBrowserDropTargetKind::Item { target: id(3) }
    ));

    let grid_cfg = config(AssetBrowserViewMode::Grid);
    let grid = run_frame(
        &model,
        grid_cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let source = grid.items[0].rect.center();
    let background = Point::new(235.0, 70.0);
    let _ = run_frame(
        &model,
        grid_cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(source, true, true, false, 1),
        false,
        false,
    );
    let background_preview = run_frame(
        &model,
        grid_cfg.clone(),
        &mut state,
        &mut memory,
        move_input(
            background,
            Vec2::new(background.x - source.x, background.y - source.y),
        ),
        false,
        false,
    );
    assert!(matches!(
        background_preview
            .output
            .drop_preview
            .as_ref()
            .map(|drop| drop.kind),
        Some(AssetBrowserDropTargetKind::EmptySpace { index: 4 })
    ));
    let background_drop = run_frame(
        &model,
        grid_cfg,
        &mut state,
        &mut memory,
        primary_input(background, false, false, true, 1),
        false,
        false,
    );
    assert!(matches!(
        background_drop.output.requests.as_slice(),
        [AssetBrowserRequest::Drop(drop)]
            if drop.kind == AssetBrowserDropTargetKind::EmptySpace { index: 4 }
    ));
}

#[test]
#[allow(clippy::too_many_lines)]
fn item_selection_and_background_context_actions_match_the_frame_action_queue() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
    ]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let one = idle.items[0].rect.center();
    let two = idle.items[1].rect.center();
    let three = idle.items[2].rect.center();
    let _ = click(
        one,
        1,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let _ = click(
        two,
        1,
        Modifiers::new(false, true, false, false),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let selected_target =
        CollectionContextTarget::selection([id(1), id(2)]).expect("selection target");
    let selected_open = context_click(one, &model, cfg.clone(), &mut state, &mut memory);
    assert_eq!(
        selected_open.output.context_opened,
        Some(selected_target.clone())
    );
    let menu = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let delete = semantic_center(&menu.frame, "Delete");
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(delete, true, true, false, 1),
        false,
        false,
    );
    let mut invoked = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(delete, false, false, true, 1),
        false,
        false,
    );
    assert_context_parity(
        &mut invoked,
        &ActionId::new("asset.delete"),
        &selected_target,
    );

    let item_target = CollectionContextTarget::item(id(3));
    let item_open = context_click(three, &model, cfg.clone(), &mut state, &mut memory);
    assert_eq!(item_open.output.context_opened, Some(item_target.clone()));
    let menu = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let inspect = semantic_center(&menu.frame, "Inspect");
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(inspect, true, true, false, 1),
        false,
        false,
    );
    let mut invoked = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(inspect, false, false, true, 1),
        false,
        false,
    );
    assert_context_parity(&mut invoked, &ActionId::new("asset.inspect"), &item_target);

    let background_target = CollectionContextTarget::background();
    let background = Point::new(200.0, 110.0);
    let background_open = context_click(background, &model, cfg.clone(), &mut state, &mut memory);
    assert_eq!(
        background_open.output.context_opened,
        Some(background_target.clone())
    );
    let menu = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        false,
    );
    let create = semantic_center(&menu.frame, "Create");
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        primary_input(create, true, true, false, 1),
        false,
        false,
    );
    let mut invoked = run_frame(
        &model,
        cfg,
        &mut state,
        &mut memory,
        primary_input(create, false, false, true, 1),
        false,
        false,
    );
    assert_context_parity(
        &mut invoked,
        &ActionId::new("asset.create"),
        &background_target,
    );
}

fn assert_context_parity(run: &mut Run, action: &ActionId, target: &CollectionContextTarget) {
    let Some(AssetBrowserRequest::Context(request)) = run.output.requests.first() else {
        panic!("context menu must emit a typed request");
    };
    let invocation = run
        .frame
        .actions
        .pop_front()
        .expect("matching frame action");
    assert_eq!(&request.action_id, action);
    assert_eq!(&request.target, target);
    assert_eq!(request.target_ids, target.target_ids());
    assert_eq!(&invocation.action_id, action);
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, ActionContext::Widget(run.root));
    assert!(run.frame.actions.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn malformed_inputs_fail_closed_and_global_disable_clears_only_component_owned_state() {
    let theme = default_dark_theme();
    let mut malformed_memory = UiMemory::new();
    let duplicate = AssetBrowserModel::new(vec![
        asset(1, "First", "mesh"),
        asset(1, "Duplicate", "image"),
    ]);
    let ui = Ui::begin_frame(context(UiInput::default()), &mut malformed_memory, &theme);
    assert!(
        ui.prepare_asset_browser(
            "duplicate",
            config(AssetBrowserViewMode::List),
            &duplicate,
            &AssetBrowserState::new(),
        )
        .is_none()
    );
    assert!(
        ui.prepare_asset_browser(
            "bad-bounds",
            AssetBrowserConfig::new(
                Rect::new(f32::NAN, 0.0, 240.0, 120.0),
                layout(AssetBrowserViewMode::Grid),
            ),
            &assets([1]),
            &AssetBrowserState::new(),
        )
        .is_none()
    );
    assert!(
        ui.prepare_asset_browser(
            "bad-layout",
            AssetBrowserConfig::new(
                BOUNDS,
                AssetBrowserLayout::new(
                    AssetBrowserViewMode::List,
                    GridLayout {
                        columns: GridColumns::Fixed(1),
                        item_size: Size::new(72.0, 72.0),
                        gap: 4.0,
                    },
                    ListLayout::new(0.0),
                ),
            ),
            &assets([1]),
            &AssetBrowserState::new(),
        )
        .is_none()
    );
    drop(ui);

    let model = AssetBrowserModel::new(vec![asset(1, "One", "mesh"), asset(2, "Two", "mesh")]);
    let cfg = config(AssetBrowserViewMode::List);

    let mut edit_state = AssetBrowserState::new();
    let mut edit_memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut edit_state,
        &mut edit_memory,
        UiInput::default(),
        false,
        false,
    );
    let _ = click(
        idle.items[0].rect.center(),
        1,
        Modifiers::default(),
        &model,
        cfg.clone(),
        &mut edit_state,
        &mut edit_memory,
    );
    let edit = run_frame(
        &model,
        cfg.clone(),
        &mut edit_state,
        &mut edit_memory,
        key_input(Key::Function(2)),
        false,
        false,
    );
    edit_memory.focus(edit.outside);
    let disabled_edit = run_frame(
        &model,
        cfg.clone().disabled(true),
        &mut edit_state,
        &mut edit_memory,
        UiInput::default(),
        false,
        false,
    );
    assert!(matches!(
        disabled_edit.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(1) && cancel.reason == InlineEditCancelReason::Explicit
    ));
    assert_eq!(edit_state.rename_target(), None);
    assert_eq!(edit_memory.focused(), Some(edit.outside));

    let mut context_state = AssetBrowserState::new();
    let mut context_memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
        false,
        false,
    );
    let opened = context_click(
        idle.items[0].rect.center(),
        &model,
        cfg.clone(),
        &mut context_state,
        &mut context_memory,
    );
    assert!(opened.output.context_opened.is_some());
    context_memory.focus(opened.outside);
    let disabled_context = run_frame(
        &model,
        cfg.clone().disabled(true),
        &mut context_state,
        &mut context_memory,
        UiInput::default(),
        false,
        false,
    );
    assert_eq!(context_state.context_target(), None);
    assert!(disabled_context.output.requests.is_empty());
    assert!(disabled_context.frame.actions.is_empty());
    assert!(context_memory.is_focused(opened.outside));

    let mut drag_state = AssetBrowserState::new();
    let mut drag_memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
        false,
        false,
    );
    let source = idle.items[0].rect.center();
    let target = idle.items[1].rect.center();
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        primary_input(source, true, true, false, 1),
        false,
        false,
    );
    let dragging = run_frame(
        &model,
        cfg.clone(),
        &mut drag_state,
        &mut drag_memory,
        move_input(target, Vec2::new(target.x - source.x, target.y - source.y)),
        false,
        false,
    );
    assert!(dragging.output.drag_payload.is_some());
    drag_memory.focus(dragging.outside);
    let disabled_drag = run_frame(
        &model,
        cfg.disabled(true),
        &mut drag_state,
        &mut drag_memory,
        UiInput::default(),
        false,
        false,
    );
    assert_eq!(drag_state.drag_source(), None);
    assert_eq!(drag_memory.drag_source(), None);
    assert!(disabled_drag.output.requests.is_empty());
    assert!(drag_memory.is_focused(dragging.outside));
    assert_eq!(disabled_drag.frame.repaint, RepaintRequest::NextFrame);
}
