//! Public, windowless conformance for retained inspector picker flows.

use std::time::Duration;

use kinetik_ui_core::{
    Color, FrameContext, FrameOutput, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Rect,
    Response, ScaleFactor, SemanticRole, Size, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId,
    default_dark_theme,
};
use kinetik_ui_text::TextEditState;
use kinetik_ui_widgets::inspector::{
    AssetPickerItem, InspectorPickerCancelReason, InspectorPickerCommit, InspectorPickerKind,
    InspectorPickerOutput, InspectorPickerState, PathPickerKind, PathPickerOutcome,
    PathPickerResult,
};
use kinetik_ui_widgets::overlays::{DropdownItem, DropdownItemId, DropdownModel, OverlayId};
use kinetik_ui_widgets::{
    AssetSlotConfig, AssetSlotOutput, ColorFieldConfig, ColorFieldOutput, PathFieldConfig,
    PathFieldOutput, SelectFieldConfig, SelectFieldOutput, Ui, asset_slot_field, color_field,
    path_field, select_field,
};

const FIELD: Rect = Rect::new(8.0, 8.0, 200.0, 28.0);
const OVERLAY: Rect = Rect::new(8.0, 44.0, 220.0, 168.0);
const LOWER: Rect = Rect::new(0.0, 0.0, 640.0, 480.0);
const OVERLAY_ID: OverlayId = OverlayId::from_raw(632);

fn item(raw: u64) -> DropdownItemId {
    DropdownItemId::from_raw(raw)
}

fn choices() -> DropdownModel {
    let mut model = DropdownModel::from_items([
        DropdownItem::new(item(1), "Alpha"),
        DropdownItem::new(item(2), "Blocked").with_enabled(false),
        DropdownItem::new(item(3), "Gamma"),
    ]);
    assert!(model.set_selected_id(item(1)));
    model
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(640, 480),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn pressed_at(point: Point) -> UiInput {
    pointer_input(point, true, true, false)
}

fn released_at(point: Point) -> UiInput {
    pointer_input(point, false, false, true)
}

fn key_input(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn typed(character: &str) -> UiInput {
    let event = KeyEvent::new(
        Key::Character(character.to_owned()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(character);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

struct SceneRun {
    lower: Option<Response>,
    output: InspectorPickerOutput,
    frame: FrameOutput,
}

fn run_scene(
    state: &mut InspectorPickerState,
    memory: &mut UiMemory,
    input: UiInput,
    lower: bool,
) -> SceneRun {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let lower_id = ui.make_id("lower");
    let scene = state.scene().cloned();
    if let Some(scene) = scene.as_ref() {
        ui.resolve_pointer_targets(|plan| {
            if lower {
                plan.target(PointerTarget::new(lower_id, LOWER, PointerOrder::new(10)));
            }
            scene.declare_pointer_targets(plan, PointerOrder::new(100));
        })
        .expect("valid picker pointer plan");
        // The originating entry remains present beneath the picker in a real frame.
        ui.register_id(scene.trigger());
    }
    let lower = lower.then(|| ui.pressable("lower", LOWER, false));
    let output = ui.inspector_picker_scene(state);
    let frame = ui.finish_output();
    SceneRun {
        lower,
        output,
        frame,
    }
}

fn click_scene(
    state: &mut InspectorPickerState,
    memory: &mut UiMemory,
    point: Point,
    lower: bool,
) -> SceneRun {
    let _ = run_scene(state, memory, pressed_at(point), lower);
    run_scene(state, memory, released_at(point), lower)
}

fn requested_select(model: &DropdownModel, config: SelectFieldConfig) -> SelectFieldOutput {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("select-trigger");
    let mut memory = UiMemory::new();
    let _ = select_field(
        id,
        FIELD,
        "Mode",
        model,
        config.clone(),
        &pressed_at(FIELD.center()),
        &mut memory,
        &theme,
    );
    select_field(
        id,
        FIELD,
        "Mode",
        model,
        config,
        &released_at(FIELD.center()),
        &mut memory,
        &theme,
    )
}

fn requested_color(color: Color, config: ColorFieldConfig) -> ColorFieldOutput {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("color-trigger");
    let mut memory = UiMemory::new();
    let _ = color_field(
        id,
        FIELD,
        "Tint",
        color,
        config,
        &pressed_at(FIELD.center()),
        &mut memory,
        &theme,
    );
    color_field(
        id,
        FIELD,
        "Tint",
        color,
        config,
        &released_at(FIELD.center()),
        &mut memory,
        &theme,
    )
}

fn requested_asset(config: AssetSlotConfig) -> AssetSlotOutput {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("asset-trigger");
    let mut memory = UiMemory::new();
    let _ = asset_slot_field(
        id,
        FIELD,
        "Material",
        None,
        config.clone(),
        &pressed_at(FIELD.center()),
        &mut memory,
        &theme,
    );
    asset_slot_field(
        id,
        FIELD,
        "Material",
        None,
        config,
        &released_at(FIELD.center()),
        &mut memory,
        &theme,
    )
}

fn requested_path(config: PathFieldConfig, text: &mut TextEditState) -> PathFieldOutput {
    let theme = default_dark_theme();
    let id = WidgetId::from_key("path-trigger");
    let browse = Point::new(FIELD.max_x() - 14.0, FIELD.center().y);
    let mut memory = UiMemory::new();
    let _ = path_field(
        id,
        FIELD,
        "Source",
        text,
        config,
        &pressed_at(browse),
        &mut memory,
        &theme,
    );
    path_field(
        id,
        FIELD,
        "Source",
        text,
        config,
        &released_at(browse),
        &mut memory,
        &theme,
    )
}

fn open_select(
    state: &mut InspectorPickerState,
    field: &SelectFieldOutput,
    model: &DropdownModel,
    bounds: Rect,
) -> bool {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(UiInput::default()), &mut memory, &theme);
    let opened = ui.select_picker(state, field, OVERLAY_ID, bounds, "Modes", model);
    let _ = ui.finish_output();
    opened
}

fn open_color(state: &mut InspectorPickerState, field: &ColorFieldOutput, bounds: Rect) -> bool {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(UiInput::default()), &mut memory, &theme);
    let opened = ui.color_picker(state, field, OVERLAY_ID, bounds);
    let _ = ui.finish_output();
    opened
}

fn open_asset(
    state: &mut InspectorPickerState,
    field: &AssetSlotOutput,
    items: &[AssetPickerItem],
) -> bool {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(UiInput::default()), &mut memory, &theme);
    let opened = ui.asset_picker(state, field, OVERLAY_ID, OVERLAY, "Assets", items);
    let _ = ui.finish_output();
    opened
}

fn open_path(
    state: &mut InspectorPickerState,
    field: &PathFieldOutput,
    kind: PathPickerKind,
) -> bool {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let mut ui = Ui::begin_frame(context(UiInput::default()), &mut memory, &theme);
    let opened = ui.path_picker(state, field, kind);
    let _ = ui.finish_output();
    opened
}

fn semantic_bounds(frame: &FrameOutput, label: &str) -> Rect {
    frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some(label))
        .unwrap_or_else(|| panic!("missing semantic node {label}"))
        .bounds
}

#[test]
fn select_pointer_keyboard_typeahead_and_reconcile_commit_once() {
    let model = choices();
    let field = requested_select(&model, SelectFieldConfig::default());
    let trigger = field.response.id;
    let app_selection = item(1);

    let mut pointer_state = InspectorPickerState::new();
    assert!(open_select(&mut pointer_state, &field, &model, OVERLAY));
    let mut pointer_memory = UiMemory::new();
    let _ = run_scene(
        &mut pointer_state,
        &mut pointer_memory,
        UiInput::default(),
        false,
    );
    let blocked = click_scene(
        &mut pointer_state,
        &mut pointer_memory,
        Point::new(OVERLAY.x + 20.0, OVERLAY.y + 42.0),
        false,
    );
    assert_eq!(blocked.output.commit, None);
    assert_eq!(pointer_state.kind(), Some(InspectorPickerKind::Select));
    let selected = click_scene(
        &mut pointer_state,
        &mut pointer_memory,
        Point::new(OVERLAY.x + 20.0, OVERLAY.y + 70.0),
        false,
    );
    assert_eq!(
        selected.output.commit,
        Some(InspectorPickerCommit::Select(item(3)))
    );
    assert_eq!(selected.output.focus_return, Some(trigger));
    assert_eq!(pointer_memory.focused(), Some(trigger));
    assert_eq!(app_selection, item(1));
    assert_eq!(pointer_state.kind(), None);
    assert_eq!(
        run_scene(
            &mut pointer_state,
            &mut pointer_memory,
            UiInput::default(),
            false,
        )
        .output
        .commit,
        None
    );

    let mut keyboard_state = InspectorPickerState::new();
    assert!(open_select(&mut keyboard_state, &field, &model, OVERLAY));
    let mut keyboard_memory = UiMemory::new();
    let _ = run_scene(
        &mut keyboard_state,
        &mut keyboard_memory,
        UiInput::default(),
        false,
    );
    let _ = run_scene(
        &mut keyboard_state,
        &mut keyboard_memory,
        key_input(Key::ArrowDown),
        false,
    );
    let keyboard = run_scene(
        &mut keyboard_state,
        &mut keyboard_memory,
        key_input(Key::Enter),
        false,
    );
    assert_eq!(
        keyboard.output.commit,
        Some(InspectorPickerCommit::Select(item(3)))
    );

    let mut repaired_state = InspectorPickerState::new();
    assert!(open_select(&mut repaired_state, &field, &model, OVERLAY));
    let repaired = DropdownModel::from_items([
        DropdownItem::new(item(3), "Gamma renamed"),
        DropdownItem::new(item(2), "Blocked").with_enabled(false),
    ]);
    assert!(repaired_state.reconcile_select(&repaired));
    let mut repaired_memory = UiMemory::new();
    let _ = run_scene(
        &mut repaired_state,
        &mut repaired_memory,
        UiInput::default(),
        false,
    );
    let _ = run_scene(&mut repaired_state, &mut repaired_memory, typed("g"), false);
    let typeahead = run_scene(
        &mut repaired_state,
        &mut repaired_memory,
        key_input(Key::Enter),
        false,
    );
    assert_eq!(
        typeahead.output.commit,
        Some(InspectorPickerCommit::Select(item(3)))
    );
}

#[test]
fn select_escape_and_outside_cancel_restore_focus_and_block_lower_ui() {
    let model = choices();
    let field = requested_select(&model, SelectFieldConfig::default());
    let trigger = field.response.id;
    let app_selection = item(1);
    let mut state = InspectorPickerState::new();
    assert!(open_select(&mut state, &field, &model, OVERLAY));
    let mut memory = UiMemory::new();

    let idle = run_scene(&mut state, &mut memory, UiInput::default(), true);
    let root = idle
        .frame
        .semantics
        .get(WidgetId::from_raw(OVERLAY_ID.raw()))
        .expect("dropdown semantics");
    assert_eq!(root.role, SemanticRole::Menu);
    assert_eq!(root.label.as_deref(), Some("Modes"));
    assert_eq!(root.children.len(), 3);
    assert!(idle.frame.warnings.is_empty());

    let escape = run_scene(&mut state, &mut memory, key_input(Key::Escape), true);
    assert_eq!(
        escape.output.cancel,
        Some(InspectorPickerCancelReason::Escape)
    );
    assert_eq!(escape.output.commit, None);
    assert_eq!(escape.output.focus_return, Some(trigger));
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(app_selection, item(1));

    assert!(open_select(&mut state, &field, &model, OVERLAY));
    let _ = run_scene(&mut state, &mut memory, UiInput::default(), true);
    let outside_point = Point::new(400.0, 300.0);
    let outside_press = run_scene(&mut state, &mut memory, pressed_at(outside_point), true);
    assert!(!outside_press.lower.expect("lower press").state.hovered);
    let outside = run_scene(&mut state, &mut memory, released_at(outside_point), true);
    assert!(!outside.lower.expect("lower release").clicked);
    assert_eq!(
        outside.output.cancel,
        Some(InspectorPickerCancelReason::OutsideClick)
    );
    assert_eq!(outside.output.commit, None);
    assert_eq!(outside.output.focus_return, Some(trigger));
    assert_eq!(app_selection, item(1));
}

#[test]
fn color_draft_adjusts_then_apply_or_cancel_resolves_without_mutating_app_value() {
    let original = Color::rgba(0.25, 0.50, 0.75, 1.0);
    let app_color = original;
    let field = requested_color(original, ColorFieldConfig::default());
    let trigger = field.response.id;
    let mut state = InspectorPickerState::new();
    assert!(open_color(&mut state, &field, OVERLAY));
    let mut memory = UiMemory::new();

    let idle = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let color_root = idle
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Color picker"))
        .expect("color picker semantics");
    assert_eq!(
        color_root.role,
        SemanticRole::Custom("color-picker".to_owned())
    );
    assert_eq!(color_root.state.expanded, Some(true));
    let increase_red = semantic_bounds(&idle.frame, "Increase Red").center();
    let adjusted = click_scene(&mut state, &mut memory, increase_red, false);
    assert_eq!(adjusted.output.commit, None);
    let draft = state.color_draft().expect("retained draft");
    assert!((draft.r - 0.30).abs() < f32::EPSILON);
    assert_eq!(app_color, original);

    let refreshed = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let apply = semantic_bounds(&refreshed.frame, "Apply").center();
    let applied = click_scene(&mut state, &mut memory, apply, false);
    assert_eq!(
        applied.output.commit,
        Some(InspectorPickerCommit::Color(draft))
    );
    assert_eq!(applied.output.cancel, None);
    assert_eq!(applied.output.focus_return, Some(trigger));
    assert_eq!(app_color, original);
    assert_eq!(state.kind(), None);

    assert!(open_color(&mut state, &field, OVERLAY));
    let idle = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let increase_red = semantic_bounds(&idle.frame, "Increase Red").center();
    let _ = click_scene(&mut state, &mut memory, increase_red, false);
    let refreshed = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let cancel = semantic_bounds(&refreshed.frame, "Cancel").center();
    let cancelled = click_scene(&mut state, &mut memory, cancel, false);
    assert_eq!(cancelled.output.commit, None);
    assert_eq!(
        cancelled.output.cancel,
        Some(InspectorPickerCancelReason::Explicit)
    );
    assert_eq!(cancelled.output.focus_return, Some(trigger));
    assert_eq!(app_color, original);
}

#[test]
fn asset_picker_reconciles_stable_identity_and_commits_once() {
    let field = requested_asset(AssetSlotConfig::default());
    let trigger = field.response.id;
    let app_asset: Option<String> = None;
    let items = [
        AssetPickerItem::new(item(10), "asset-a", "Material A"),
        AssetPickerItem::new(item(11), "asset-b", "Material B"),
    ];
    let mut state = InspectorPickerState::new();
    assert!(open_asset(&mut state, &field, &items));
    let reordered = [
        AssetPickerItem::new(item(11), "asset-b", "Material B renamed"),
        AssetPickerItem::new(item(10), "asset-a", "Material A"),
    ];
    assert!(state.reconcile_assets(&reordered));
    let mut memory = UiMemory::new();
    let _ = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let selected = click_scene(
        &mut state,
        &mut memory,
        Point::new(OVERLAY.x + 20.0, OVERLAY.y + 14.0),
        false,
    );
    assert_eq!(
        selected.output.commit,
        Some(InspectorPickerCommit::Asset("asset-b".to_owned()))
    );
    assert_eq!(selected.output.focus_return, Some(trigger));
    assert_eq!(app_asset, None);
    assert_eq!(state.kind(), None);
    assert_eq!(
        run_scene(&mut state, &mut memory, UiInput::default(), false)
            .output
            .commit,
        None
    );
}

fn resolve_path(
    state: &mut InspectorPickerState,
    memory: &mut UiMemory,
    result: PathPickerResult,
) -> Option<InspectorPickerOutput> {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(UiInput::default()), memory, &theme);
    ui.register_id(result.trigger);
    let output = ui.resolve_path_picker_result(state, result);
    let _ = ui.finish_output();
    output
}

#[test]
fn path_service_request_is_redacted_one_shot_and_rejects_stale_results() {
    let original = "C:/private/current.scene";
    let mut text = TextEditState::new(original);
    let field = requested_path(PathFieldConfig::default(), &mut text);
    let trigger = field.browse_response.as_ref().expect("browse trigger").id;
    let mut state = InspectorPickerState::new();
    assert!(open_path(&mut state, &field, PathPickerKind::File));
    let mut memory = UiMemory::new();
    let request_frame = run_scene(&mut state, &mut memory, UiInput::default(), false);
    let request = request_frame
        .output
        .service_request
        .expect("service request");
    assert_eq!(request.trigger, trigger);
    assert_eq!(request.kind, PathPickerKind::File);
    assert!(request.generation > 0);
    assert!(!format!("{request:?}").contains(original));
    assert_eq!(
        run_scene(&mut state, &mut memory, UiInput::default(), false)
            .output
            .service_request,
        None
    );

    let old_result = PathPickerResult::new(
        request.generation.saturating_sub(1),
        request.trigger,
        PathPickerOutcome::Selected("stale.scene".to_owned()),
    );
    assert_eq!(resolve_path(&mut state, &mut memory, old_result), None);
    let wrong_target = PathPickerResult::new(
        request.generation,
        request.trigger.child("wrong"),
        PathPickerOutcome::Selected("wrong.scene".to_owned()),
    );
    assert_eq!(resolve_path(&mut state, &mut memory, wrong_target), None);
    assert_eq!(state.kind(), Some(InspectorPickerKind::Path));

    let selected_result = PathPickerResult::selected(request, "C:/chosen.scene");
    let selected = resolve_path(&mut state, &mut memory, selected_result.clone())
        .expect("matching selected result");
    assert_eq!(
        selected.commit,
        Some(InspectorPickerCommit::Path("C:/chosen.scene".to_owned()))
    );
    assert_eq!(selected.focus_return, Some(trigger));
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(resolve_path(&mut state, &mut memory, selected_result), None);
    assert_eq!(text.text, original);

    let field = requested_path(PathFieldConfig::default(), &mut text);
    assert!(open_path(&mut state, &field, PathPickerKind::Directory));
    let cancelled_request = run_scene(&mut state, &mut memory, UiInput::default(), false)
        .output
        .service_request
        .expect("cancel request");
    assert!(cancelled_request.generation > request.generation);
    let cancelled = resolve_path(
        &mut state,
        &mut memory,
        PathPickerResult::cancelled(cancelled_request),
    )
    .expect("matching cancellation");
    assert_eq!(
        cancelled.cancel,
        Some(InspectorPickerCancelReason::ServiceCancelled)
    );

    let field = requested_path(PathFieldConfig::default(), &mut text);
    assert!(open_path(&mut state, &field, PathPickerKind::File));
    let failed_request = run_scene(&mut state, &mut memory, UiInput::default(), false)
        .output
        .service_request
        .expect("failure request");
    let failed = resolve_path(
        &mut state,
        &mut memory,
        PathPickerResult::failed(failed_request),
    )
    .expect("matching failure");
    assert_eq!(
        failed.cancel,
        Some(InspectorPickerCancelReason::ServiceFailed)
    );
    assert_eq!(text.text, original);
}

#[test]
fn disabled_read_only_empty_and_invalid_requests_fail_closed() {
    let model = choices();
    for config in [
        SelectFieldConfig::default().disabled(true),
        SelectFieldConfig::default().read_only(true),
    ] {
        let field = requested_select(&model, config);
        let mut state = InspectorPickerState::new();
        assert!(!open_select(&mut state, &field, &model, OVERLAY));
        assert_eq!(state.kind(), None);
    }

    let select_request = requested_select(&model, SelectFieldConfig::default());
    let empty = DropdownModel::new();
    let mut state = InspectorPickerState::new();
    assert!(!open_select(&mut state, &select_request, &empty, OVERLAY));
    for bounds in [
        Rect::ZERO,
        Rect::new(0.0, 0.0, -1.0, 20.0),
        Rect::new(0.0, 0.0, f32::NAN, 20.0),
    ] {
        assert!(!open_select(&mut state, &select_request, &model, bounds));
    }

    let original = Color::rgb(0.2, 0.3, 0.4);
    for config in [
        ColorFieldConfig::default().disabled(true),
        ColorFieldConfig::default().read_only(true),
    ] {
        let field = requested_color(original, config);
        let mut state = InspectorPickerState::new();
        assert!(!open_color(&mut state, &field, OVERLAY));
    }
    let color_request = requested_color(original, ColorFieldConfig::default());
    assert!(!open_color(&mut state, &color_request, Rect::ZERO));

    for config in [
        AssetSlotConfig::default().disabled(true),
        AssetSlotConfig::default().read_only(true),
    ] {
        let field = requested_asset(config);
        let mut state = InspectorPickerState::new();
        assert!(!open_asset(
            &mut state,
            &field,
            &[AssetPickerItem::new(item(1), "asset", "Asset")],
        ));
    }
    let asset_request = requested_asset(AssetSlotConfig::default());
    assert!(!open_asset(&mut state, &asset_request, &[]));

    let mut text = TextEditState::new("keep.scene");
    for config in [
        PathFieldConfig::default().disabled(true),
        PathFieldConfig::default().read_only(true),
    ] {
        let field = requested_path(config, &mut text);
        let mut state = InspectorPickerState::new();
        assert!(!open_path(&mut state, &field, PathPickerKind::File));
        assert_eq!(
            run_scene(&mut state, &mut UiMemory::new(), UiInput::default(), false)
                .output
                .service_request,
            None
        );
    }
    assert_eq!(text.text, "keep.scene");
}
