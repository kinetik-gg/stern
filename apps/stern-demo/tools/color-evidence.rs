use std::collections::BTreeSet;

use stern::core::{
    ActionSource, FrameOutput, Key, KeyEvent, KeyState, Modifiers, MouseButton, Point,
    PointerButtonState, PointerInput, SemanticRole, UiInput, UiInputEvent, Vec2, WidgetId,
};
use stern_demo::{DemoApp, DemoColorSaveState, DemoScenario, DemoWorkspace, demo_context};

use crate::json::{Json as Value, json};

pub(super) struct ColorLog {
    pub(super) passed: bool,
    pub(super) picker_log: Value,
    pub(super) gradient_log: Value,
    pub(super) serialization_log: Value,
    pub(super) focus_log: Value,
}

pub(super) struct RecoveryLog {
    pub(super) passed: bool,
    pub(super) failure_log: Value,
    pub(super) retry_log: Value,
    pub(super) focus_log: Value,
}

pub(super) struct OverlayRecoveryLog {
    pub(super) passed: bool,
    pub(super) route_log: Value,
    pub(super) owner_removal_log: Value,
}

#[allow(clippy::too_many_lines)]
pub(super) fn color_gradient_journey() -> Result<ColorLog, String> {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let trigger = semantic_node(&initial, &SemanticRole::Button, "Fill color")?.id;
    let original = app.tagged_color();

    let picker = click(&mut app, &initial, &SemanticRole::Button, "Fill color")?;
    let picker_projected = has_custom_role(&picker, "color-picker");
    let adjusted = click(&mut app, &picker, &SemanticRole::Button, "Increase Red")?;
    let draft_isolated = app.tagged_color() == original && app.color_revision() == 0;
    let _ = click(&mut app, &adjusted, &SemanticRole::Button, "Cancel")?;
    let cancelled = app.frame(demo_context(UiInput::default()));
    let cancel_preserved = app.tagged_color() == original
        && app.color_revision() == 0
        && app.focused() == Some(trigger)
        && !has_custom_role(&cancelled, "color-picker");

    let closed = app.frame(demo_context(UiInput::default()));
    let picker = click(&mut app, &closed, &SemanticRole::Button, "Fill color")?;
    let picker_value = custom_node(&picker, "color-picker", "Color picker")?
        .description
        .clone();
    let _ = click(&mut app, &picker, &SemanticRole::Button, "Increase Green")?;
    let adjusted = app.frame(demo_context(UiInput::default()));
    let adjusted_value = custom_node(&adjusted, "color-picker", "Color picker")?
        .description
        .clone();
    let draft_adjusted = adjusted_value != picker_value;
    let _ = click(&mut app, &adjusted, &SemanticRole::Button, "Apply")?;
    let applied = app.frame(demo_context(UiInput::default()));
    let apply_committed_once = app.tagged_color() != original
        && app.color_revision() == 1
        && app.focused() == Some(trigger)
        && !has_custom_role(&applied, "color-picker");
    let picker_passed = picker_projected
        && draft_isolated
        && cancel_preserved
        && draft_adjusted
        && apply_committed_once;

    let gradient = app.frame(demo_context(UiInput::default()));
    let gradient_root = custom_node(&gradient, "gradient-editor", "Gradient editor")?.id;
    let selected = app.selected_gradient_stop();
    let original_ids = gradient_ids(&app);
    let before_move = selected_position(&app, selected)?;
    let marker = custom_node(
        &gradient,
        "gradient-stop",
        &format!("Gradient stop {}", selected.raw()),
    )?
    .bounds
    .center();
    let moved = Point::new(marker.x + 20.0, marker.y);
    let _ = app.frame(demo_context(gradient_move(marker, moved)));
    let after_move = selected_position(&app, selected)?;
    let moved_stably = app.focused() == Some(gradient_root)
        && after_move.to_bits() != before_move.to_bits()
        && app.selected_gradient_stop() == selected
        && gradient_ids(&app) == original_ids;

    let before_reverse = app.gradient_stops().to_vec();
    let reverse = app.frame(demo_context(UiInput::default()));
    let _ = click(&mut app, &reverse, &SemanticRole::Button, "sRGB · Reverse")?;
    let reversed_stably = app.selected_gradient_stop() == selected
        && gradient_ids(&app) == original_ids
        && before_reverse.iter().all(|before| {
            app.gradient_stops()
                .iter()
                .find(|after| after.id == before.id)
                .is_some_and(|after| {
                    (after.position - (1.0 - before.position)).abs() < f32::EPSILON
                })
        });
    let gradient_passed = moved_stably && reversed_stably;

    let _ = invoke_workspace_action(&mut app, "Save Color Style")?;
    let failed_without_value = app.color_save_state() == DemoColorSaveState::Failed
        && app.serialized_color_style().is_none();
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let _ = app.frame(demo_context(UiInput::default()));
    let _ = invoke_workspace_action(&mut app, "Save Color Style")?;
    let serialized = app.serialized_color_style().unwrap_or_default();
    let explicit_srgb = failed_without_value
        && app.color_save_state() == DemoColorSaveState::Succeeded
        && serialized.starts_with("color=srgb(")
        && serialized.contains(";gradient=sRGB")
        && serialized.matches("=srgb(").count() == app.gradient_stops().len() + 1
        && app
            .gradient_stops()
            .iter()
            .all(|stop| serialized.contains(&format!(";{}@", stop.id.raw())));

    Ok(ColorLog {
        passed: picker_passed && gradient_passed && explicit_srgb,
        picker_log: json!({
            "id": "color-picker-cancel-apply", "input": "pointer", "draftIsolated": draft_isolated,
            "cancelPreserved": cancel_preserved, "singleCommit": apply_committed_once,
            "revision": app.color_revision(), "status": status(picker_passed),
        }),
        gradient_log: json!({
            "id": "gradient-stable-id-move-reverse", "input": "pointer", "selectedStop": selected.raw(),
            "stableIds": gradient_ids(&app) == original_ids, "moved": moved_stably,
            "reversed": reversed_stably, "status": status(gradient_passed),
        }),
        serialization_log: json!({
            "id": "color-style-explicit-srgb", "input": "action-retry", "explicitSrgb": explicit_srgb,
            "serializedStopCount": app.gradient_stops().len(), "status": status(explicit_srgb),
        }),
        focus_log: json!({
            "workspaceId": "edit-workspace", "overlay": "Color picker", "dismissal": "Cancel and Apply",
            "focusOwner": widget(Some(trigger)), "restored": cancel_preserved && apply_committed_once,
        }),
    })
}

pub(super) fn recovery_journey() -> Result<RecoveryLog, String> {
    let mut app = DemoApp::new();
    let initial = app.frame(demo_context(UiInput::default()));
    let focused = click(&mut app, &initial, &SemanticRole::ListItem, "Backdrop")?;
    let owner = app
        .focused()
        .ok_or("recovery focus owner was not established")?;

    let palette = app.frame(demo_context(key(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_opened = has_role(&palette, &SemanticRole::SearchField);
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let palette_closed = app.frame(demo_context(UiInput::default()));
    let palette_recovered =
        !has_role(&palette_closed, &SemanticRole::SearchField) && app.focused() == Some(owner);

    let original_color = app.tagged_color();
    let original_stops = app.gradient_stops().to_vec();
    let failed_action = invoke_workspace_action_from(&mut app, &focused, "Save Color Style")?;
    let action_owner_recovered =
        action_count(&failed_action, "color-style.save") == 1 && app.focused() == Some(owner);
    let failed = app.frame(demo_context(UiInput::default()));
    let failure_feedback =
        has_label(&failed, "Color style save failed") && has_label(&failed, "Color recovery hint");
    let failure_preserved = app.color_save_state() == DemoColorSaveState::Failed
        && app.serialized_color_style().is_none()
        && app.tagged_color() == original_color
        && app.gradient_stops() == original_stops;

    let outside = Point::new(8.0, 440.0);
    let _ = app.frame(demo_context(pointer(outside, true, true, false)));
    let _ = app.frame(demo_context(pointer(outside, false, false, true)));
    let passive_closed = app.frame(demo_context(UiInput::default()));
    let passive_recovered =
        !has_label(&passive_closed, "Color recovery hint") && app.focused() == Some(owner);

    let recovered_action = invoke_workspace_action(&mut app, "Save Color Style")?;
    let retry_owner_recovered =
        action_count(&recovered_action, "color-style.save") == 1 && app.focused() == Some(owner);
    let recovered = app.frame(demo_context(UiInput::default()));
    let modal_projected = has_label(&recovered, "Color style recovered")
        && has_label(&recovered, "Color style saved")
        && !has_label(&recovered, "Color style save failed");
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let modal_closed = app.frame(demo_context(UiInput::default()));
    let modal_recovered =
        !has_label(&modal_closed, "Color style recovered") && app.focused() == Some(owner);
    let failure_passed = failure_feedback && failure_preserved;
    let retry_passed = app.color_save_state() == DemoColorSaveState::Succeeded
        && app.serialized_color_style().is_some()
        && modal_projected;
    let focus_passed = palette_opened
        && palette_recovered
        && action_owner_recovered
        && passive_recovered
        && retry_owner_recovered
        && modal_recovered;

    Ok(RecoveryLog {
        passed: failure_passed && retry_passed && focus_passed,
        failure_log: json!({
            "id": "color-style-save-failure", "input": "action", "actionCount": action_count(&failed_action, "color-style.save"),
            "optimisticMutation": false, "applicationStatePreserved": failure_preserved,
            "semanticFeedback": failure_feedback, "status": status(failure_passed),
        }),
        retry_log: json!({
            "id": "color-style-save-retry", "input": "action", "actionCount": action_count(&recovered_action, "color-style.save"),
            "staleFailureCleared": modal_projected, "explicitValue": app.serialized_color_style().is_some(),
            "status": status(retry_passed),
        }),
        focus_log: json!({
            "workspaceId": "edit-workspace", "overlay": "Palette and recovery overlays",
            "dismissal": "Escape and outside pointer", "focusOwner": widget(Some(owner)),
            "restored": focus_passed,
        }),
    })
}

#[allow(clippy::too_many_lines)]
pub(super) fn overlay_recovery_journey() -> Result<OverlayRecoveryLog, String> {
    let mut app = DemoApp::for_scenario(DemoScenario::OverlayRecoveryJourney);
    let initial = app.frame(demo_context(UiInput::default()));
    let focused = click(&mut app, &initial, &SemanticRole::ListItem, "Backdrop")?;
    let owner = app
        .focused()
        .ok_or("overlay recovery focus owner was not established")?;
    let help = semantic_node(&focused, &SemanticRole::Button, "Overlay help")?
        .bounds
        .center();

    let tooltip = app.frame(demo_context(hover(help)));
    let tooltip_exclusive = overlay_state(&tooltip) == [true, false, false, false, false];
    let clear = app.frame(demo_context(hover(Point::new(8.0, 440.0))));
    let tooltip_closed = overlay_state(&clear) == [false; 5];

    let menu = open_workspace_menu(&mut app, &clear)?;
    let menu_exclusive = overlay_state(&menu) == [false, true, false, false, false];
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let clear = app.frame(demo_context(UiInput::default()));
    let menu_closed = overlay_state(&clear) == [false; 5];

    let palette = app.frame(demo_context(key(
        Key::Character("p".to_owned()),
        Modifiers::new(true, true, false, false),
    )));
    let palette_exclusive = overlay_state(&palette) == [false, false, true, false, false];
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let clear = app.frame(demo_context(UiInput::default()));

    let failed_action = invoke_workspace_action_from(&mut app, &clear, "Save Color Style")?;
    let failed_once = action_count(&failed_action, "color-style.save") == 1;
    let popover = app.frame(demo_context(UiInput::default()));
    let popover_exclusive = overlay_state(&popover) == [false, false, false, true, false];
    let outside = Point::new(8.0, 440.0);
    let _ = app.frame(demo_context(pointer(outside, true, true, false)));
    let _ = app.frame(demo_context(pointer(outside, false, false, true)));
    let clear = app.frame(demo_context(UiInput::default()));

    let recovered_action = invoke_workspace_action_from(&mut app, &clear, "Save Color Style")?;
    let recovered_once = action_count(&recovered_action, "color-style.save") == 1;
    let modal = app.frame(demo_context(UiInput::default()));
    let modal_exclusive = overlay_state(&modal) == [false, false, false, false, true];
    let _ = app.frame(demo_context(key(Key::Escape, Modifiers::default())));
    let clear = app.frame(demo_context(UiInput::default()));
    let modal_closed = overlay_state(&clear) == [false; 5];

    let menu = open_workspace_menu(&mut app, &clear)?;
    let transition = click(&mut app, &menu, &SemanticRole::MenuItem, "Graph Workspace")?;
    let action_count = action_count(&transition, "workspace.graph");
    let menu_action = transition.actions.clone().drain().any(|invocation| {
        invocation.action_id.as_str() == "workspace.graph"
            && invocation.source == ActionSource::Menu
    });
    let workspace_changed = app.workspace() == DemoWorkspace::Graph;
    let graph_focus = WidgetId::from_key("root").child("workspace.graph");
    let focus_selected = app.focused() == Some(graph_focus);
    let settled = app.frame(demo_context(UiInput::default()));
    let overlay_closed = overlay_state(&settled) == [false; 5];
    let old_owner_live = settled.semantics.get(owner).is_some();
    let restored_focus_live = settled
        .semantics
        .get(graph_focus)
        .is_some_and(|node| node.focusable && node.state.focused);
    let owner_removal_passed = action_count == 1
        && menu_action
        && workspace_changed
        && focus_selected
        && overlay_closed
        && !old_owner_live
        && restored_focus_live
        && app.focused() == Some(graph_focus);
    let route_passed = tooltip_exclusive
        && tooltip_closed
        && menu_exclusive
        && menu_closed
        && palette_exclusive
        && failed_once
        && popover_exclusive
        && recovered_once
        && modal_exclusive
        && modal_closed;

    Ok(OverlayRecoveryLog {
        passed: route_passed && owner_removal_passed,
        route_log: json!({
            "id": "overlay-route-exclusivity", "input": "pointer-keyboard-action",
            "sequence": ["tooltip", "menu", "palette", "popover", "modal"],
            "exclusive": route_passed, "sharedRoute": true, "status": status(route_passed),
        }),
        owner_removal_log: json!({
            "workspaceId": "edit-to-graph", "interaction": "focus-owner removal cleanup",
            "scope": "real DemoApp Edit-to-Graph shared-overlay transition",
            "focusOwner": widget(Some(owner)), "actionId": "workspace.graph",
            "actionSource": "Menu", "actionCount": action_count,
            "workspaceChanged": workspace_changed, "overlayClosed": overlay_closed,
            "oldOwnerLive": old_owner_live, "restoredFocus": widget(Some(graph_focus)),
            "restoredFocusLive": restored_focus_live, "restored": owner_removal_passed,
        }),
    })
}

fn invoke_workspace_action(app: &mut DemoApp, label: &str) -> Result<FrameOutput, String> {
    let current = app.frame(demo_context(UiInput::default()));
    invoke_workspace_action_from(app, &current, label)
}

fn invoke_workspace_action_from(
    app: &mut DemoApp,
    current: &FrameOutput,
    label: &str,
) -> Result<FrameOutput, String> {
    let menu = open_workspace_menu(app, current)?;
    click(app, &menu, &SemanticRole::MenuItem, label)
}

fn open_workspace_menu(app: &mut DemoApp, current: &FrameOutput) -> Result<FrameOutput, String> {
    let _ = click(app, current, &SemanticRole::MenuItem, "Workspace")?;
    Ok(app.frame(demo_context(UiInput::default())))
}

fn click(
    app: &mut DemoApp,
    output: &FrameOutput,
    role: &SemanticRole,
    label: &str,
) -> Result<FrameOutput, String> {
    let point = semantic_node(output, role, label)?.bounds.center();
    let _ = app.frame(demo_context(pointer(point, true, true, false)));
    Ok(app.frame(demo_context(pointer(point, false, false, true))))
}

fn semantic_node<'a>(
    output: &'a FrameOutput,
    role: &SemanticRole,
    label: &str,
) -> Result<&'a stern::core::SemanticNode, String> {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| &node.role == role && node.label.as_deref() == Some(label))
        .ok_or_else(|| format!("missing semantic {role:?} {label}"))
}

fn custom_node<'a>(
    output: &'a FrameOutput,
    role: &str,
    label: &str,
) -> Result<&'a stern::core::SemanticNode, String> {
    semantic_node(output, &SemanticRole::Custom(role.to_owned()), label)
}

fn has_label(output: &FrameOutput, label: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| node.label.as_deref() == Some(label))
}

fn has_role(output: &FrameOutput, role: &SemanticRole) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role)
}

fn has_custom_role(output: &FrameOutput, role: &str) -> bool {
    output
        .semantics
        .nodes()
        .iter()
        .any(|node| matches!(&node.role, SemanticRole::Custom(value) if value == role))
}

fn overlay_state(output: &FrameOutput) -> [bool; 5] {
    [
        has_label(output, "Overlay help tooltip"),
        has_label(output, "Workspace commands"),
        has_role(output, &SemanticRole::SearchField),
        has_label(output, "Color recovery hint"),
        has_label(output, "Color style recovered"),
    ]
}

fn action_count(output: &FrameOutput, id: &str) -> usize {
    let mut actions = output.actions.clone();
    actions
        .drain()
        .filter(|action| action.action_id.as_str() == id)
        .count()
}

fn gradient_ids(app: &DemoApp) -> BTreeSet<stern::widgets::gradient_editor::GradientEditorStopId> {
    app.gradient_stops().iter().map(|stop| stop.id).collect()
}

fn selected_position(
    app: &DemoApp,
    selected: stern::widgets::gradient_editor::GradientEditorStopId,
) -> Result<f32, String> {
    app.gradient_stops()
        .iter()
        .find(|stop| stop.id == selected)
        .map(|stop| stop.position)
        .ok_or_else(|| "selected gradient stop is missing".to_owned())
}

fn gradient_move(from: Point, to: Point) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(from),
    });
    input.push_event(UiInputEvent::PointerMoved {
        position: to,
        delta: Vec2::new(to.x - from.x, to.y - from.y),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(to),
    });
    input
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn hover(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key(key: Key, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: stern::core::KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, KeyState::Pressed, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn status(passed: bool) -> &'static str {
    if passed { "passed" } else { "failed" }
}

fn widget(id: Option<WidgetId>) -> Option<String> {
    id.map(|id| format!("{:016x}", id.raw()))
}
