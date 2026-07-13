use super::super::{
    ACTION_SAVE, EditorShowcase, item_id, workflow_asset_id, workflow_asset_model,
    workflow_outliner_model,
};
use super::{editor_test_context, pointer_input_at_with_delta};
use kinetik_ui::core::{
    FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point, PointerButtonState,
    PointerInput, Rect, SemanticRole, UiInput, UiMemory, Vec2, WidgetId, default_dark_theme,
};
use kinetik_ui::widgets::inline_edit::{
    InlineEditCommitReason, InlineEditCommitRequest, InlineEditRequest,
};
use kinetik_ui::widgets::outliner::OutlinerRequest;
use kinetik_ui::widgets::{Ui, viewport};

#[derive(Debug, Clone, Copy)]
struct WorkflowPoints {
    player: Point,
    asset_search: Point,
    terrain_asset: Point,
    viewport_move: Point,
}

fn render_frame(editor: &mut EditorShowcase, memory: &mut UiMemory, input: UiInput) -> FrameOutput {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(editor_test_context(input), memory, &theme);
    let _ = editor.render(&mut ui, 0);
    ui.finish_output()
}

fn primary_input(
    point: Point,
    down: bool,
    pressed: bool,
    released: bool,
    click_count: u8,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            click_count,
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key, modifiers: Modifiers) -> UiInput {
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

fn workflow_points(editor: &EditorShowcase) -> WorkflowPoints {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let ui = Ui::begin_frame(editor_test_context(UiInput::default()), &mut memory, &theme);
    let outliner_model = workflow_outliner_model(&editor.object_names);
    let asset_model = workflow_asset_model();
    let scenes = editor.prepare_workflow_scenes(
        &ui,
        Rect::new(0.0, 0.0, 1440.0, 900.0),
        &outliner_model,
        &asset_model,
    );
    let player = scenes
        .outliner
        .as_ref()
        .and_then(|scene| scene.rows().iter().find(|row| row.row.id == item_id(7)))
        .map(|row| row.label_rect.center())
        .expect("Player row is visible in the prepared public outliner");
    let terrain_asset = scenes
        .assets
        .as_ref()
        .and_then(|scene| {
            scene
                .layout()
                .items
                .iter()
                .find(|item| item.item.id == workflow_asset_id(1))
        })
        .map(|item| item.rect.center())
        .expect("filtered terrain asset is visible in the prepared public browser");
    let asset_search = scenes
        .asset_search_target
        .map(|(_, rect)| rect.center())
        .expect("public asset search field is prepared above the Dock frame");
    let viewport_move = scenes
        .viewport_tools
        .as_ref()
        .and_then(|scene| {
            scene
                .handles()
                .iter()
                .find(|handle| handle.kind == viewport::ViewportTransformHandleKind::Move)
        })
        .map(|handle| handle.handle_screen_rect.center())
        .expect("selected object exposes a public viewport move handle");

    WorkflowPoints {
        player,
        asset_search,
        terrain_asset,
        viewport_move,
    }
}

fn semantic_node<'a>(
    output: &'a FrameOutput,
    role: SemanticRole,
    label: &str,
) -> &'a kinetik_ui::core::SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == role && node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing {role:?} semantics for {label}"))
}

#[test]
fn rendered_public_editor_workflow_edits_drags_moves_and_saves_project_state() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let points = workflow_points(&editor);

    let idle = render_frame(&mut editor, &mut memory, UiInput::default());
    assert!(idle.warnings.is_empty());
    semantic_node(&idle, SemanticRole::List, "Scene outliner");
    semantic_node(&idle, SemanticRole::Grid, "Project assets");
    semantic_node(&idle, SemanticRole::Grid, "Property grid");
    semantic_node(&idle, SemanticRole::Viewport, "Project viewport");
    semantic_node(&idle, SemanticRole::ListItem, "terrain_forest");
    assert!(idle.semantics.nodes().iter().all(|node| {
        !matches!(
            node.label.as_deref(),
            Some("camp_scene" | "van_body" | "night_sky")
        )
    }));

    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.asset_search, true, true, false, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.asset_search, false, false, true, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        key_input(
            Key::Character("a".to_owned()),
            Modifiers::new(false, true, false, false),
        ),
    );
    let _ = render_frame(&mut editor, &mut memory, typed_input("terrain_forest"));
    assert_eq!(editor.asset_filter.text, "terrain_forest");
    let filtered = render_frame(&mut editor, &mut memory, UiInput::default());
    semantic_node(&filtered, SemanticRole::ListItem, "terrain_forest");
    assert_eq!(
        filtered
            .semantics
            .nodes()
            .iter()
            .filter(|node| node.role == SemanticRole::ListItem && node.description.is_some())
            .filter(|node| node.description.as_deref() == Some("mesh"))
            .count(),
        1
    );

    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.player, true, true, false, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.player, false, false, true, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        key_input(Key::Function(2), Modifiers::default()),
    );
    assert_eq!(editor.outliner_state.rename_target(), Some(item_id(7)));
    let _ = render_frame(
        &mut editor,
        &mut memory,
        key_input(
            Key::Character("a".to_owned()),
            Modifiers::new(false, true, false, false),
        ),
    );
    let _ = render_frame(&mut editor, &mut memory, typed_input("Hero"));
    let _ = render_frame(
        &mut editor,
        &mut memory,
        key_input(Key::Enter, Modifiers::default()),
    );
    assert_eq!(editor.object_names[6], "Hero");
    assert_eq!(editor.outliner_state.rename_target(), None);
    assert_eq!(editor.status, "Renamed object to Hero");
    let renamed = render_frame(&mut editor, &mut memory, UiInput::default());
    semantic_node(&renamed, SemanticRole::ListItem, "Hero");

    let roughness_row = semantic_node(&renamed, SemanticRole::Row, "Roughness");
    let roughness_slider = renamed
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.role == SemanticRole::Slider
                && roughness_row.bounds.contains_point(node.bounds.center())
        })
        .expect("Roughness row contains the shared slider");
    let roughness_point = Point::new(
        roughness_slider.bounds.x + roughness_slider.bounds.width * 0.8,
        roughness_slider.bounds.center().y,
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(roughness_point, true, true, false, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(roughness_point, false, false, true, 1),
    );
    assert!(
        editor.roughness > 0.75,
        "roughness was {}",
        editor.roughness
    );
    assert!(editor.status.starts_with("Roughness edited to "));

    let asset_drag_target = Point::new(points.terrain_asset.x + 18.0, points.terrain_asset.y + 8.0);
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.terrain_asset, true, true, false, 1),
    );
    let _ = render_frame(
        &mut editor,
        &mut memory,
        pointer_input_at_with_delta(
            asset_drag_target.x,
            asset_drag_target.y,
            true,
            false,
            false,
            Vec2::new(18.0, 8.0),
        ),
    );
    assert_eq!(editor.dragged_asset, Some(workflow_asset_id(1)));
    assert_eq!(editor.assigned_asset, Some(workflow_asset_id(1)));
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(asset_drag_target, false, false, true, 1),
    );

    let viewport_before = editor.viewport_selection_rect;
    let viewport_drag_target =
        Point::new(points.viewport_move.x + 24.0, points.viewport_move.y + 12.0);
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(points.viewport_move, true, true, false, 1),
    );
    let moved = render_frame(
        &mut editor,
        &mut memory,
        pointer_input_at_with_delta(
            viewport_drag_target.x,
            viewport_drag_target.y,
            true,
            false,
            false,
            Vec2::new(24.0, 12.0),
        ),
    );
    assert!(moved.warnings.is_empty());
    assert_ne!(editor.viewport_selection_rect, viewport_before);
    assert_eq!(editor.position[0], editor.viewport_selection_rect.x);
    assert_eq!(editor.position[1], editor.viewport_selection_rect.y);
    assert_eq!(editor.status, "Viewport object moved");
    let _ = render_frame(
        &mut editor,
        &mut memory,
        primary_input(viewport_drag_target, false, false, true, 1),
    );

    assert!(editor.apply_action(ACTION_SAVE));
    let saved = editor.saved_project.as_ref().expect("saved workflow state");
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.object_names[6], "Hero");
    assert_eq!(saved.roughness, editor.roughness);
    assert_eq!(saved.asset_query, "terrain_forest");
    assert_eq!(saved.dragged_asset, Some(workflow_asset_id(1)));
    assert_eq!(saved.assigned_asset, Some(workflow_asset_id(1)));
    assert_eq!(
        saved.viewport_selection_rect,
        editor.viewport_selection_rect
    );
}

#[test]
fn application_owned_rename_is_reflected_by_the_public_outliner_model() {
    let mut editor = EditorShowcase::new();
    editor.apply_outliner_requests(&[OutlinerRequest::Rename(InlineEditRequest::Commit(
        InlineEditCommitRequest {
            target: item_id(7),
            draft_text: "Hero".to_owned(),
            text_widget_id: WidgetId::from_raw(77),
            reason: InlineEditCommitReason::Enter,
        },
    ))]);

    assert_eq!(editor.object_names[6], "Hero");
    assert_eq!(editor.outliner_state.selection.active, Some(item_id(7)));
    let model = super::super::workflow_outliner_model(&editor.object_names);
    assert_eq!(
        model.item_by_id(item_id(7)).map(|item| item.label.as_str()),
        Some("Hero")
    );
}

#[test]
fn save_action_captures_the_resulting_project_state_in_memory() {
    let mut editor = EditorShowcase::new();
    editor.object_names[6] = "Hero".to_owned();
    editor.roughness = 0.81;
    editor.asset_filter.text = "terrain".to_owned();
    editor.dragged_asset = Some(workflow_asset_id(1));
    editor.assigned_asset = Some(workflow_asset_id(1));
    editor.viewport_selection_rect.x += 24.0;

    assert!(editor.apply_action(ACTION_SAVE));
    let saved = editor.saved_project.as_ref().expect("saved snapshot");
    assert_eq!(saved.revision, 1);
    assert_eq!(saved.object_names[6], "Hero");
    assert_eq!(saved.selected_object, Some(item_id(7)));
    assert_eq!(saved.roughness, 0.81);
    assert_eq!(saved.asset_query, "terrain");
    assert_eq!(saved.dragged_asset, Some(workflow_asset_id(1)));
    assert_eq!(saved.assigned_asset, Some(workflow_asset_id(1)));
    assert_eq!(saved.viewport_selection_rect.x, 744.0);
    assert!(
        saved
            .workspace
            .diagnostics(super::super::editor_panel_registry().descriptors())
            .is_valid()
    );
    assert_eq!(editor.status, "Project state saved in memory (revision 1)");
}
