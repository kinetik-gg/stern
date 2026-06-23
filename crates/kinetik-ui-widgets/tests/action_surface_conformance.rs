//! Windowless action-surface conformance for existing widget helpers.

use kinetik_ui_core::{
    ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, MouseButton, Point, Rect,
    UiInput, UiMemory, WidgetId, default_dark_theme,
};
use kinetik_ui_widgets::{CommandPalette, Menu, MenuItem, Ui};

fn descriptor(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn pointer_input(position: Point, down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.pointer.position = Some(position);
    input
        .pointer
        .apply_button_transition(MouseButton::Primary, down);
    input
}

#[test]
fn action_surface_conformance_descriptor_invocation_preserves_source_context_and_order() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let frame = WidgetId::from_key("frame");
    let panel = WidgetId::from_key("panel");
    let action = descriptor("export", "Export");
    let mut hidden = descriptor("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = descriptor("disabled", "Disabled");
    disabled.state.enabled = false;
    let mut ui = Ui::new(&input, &mut memory, &theme);

    assert!(ui.invoke_action_descriptor(&action, ActionSource::Menu, ActionContext::Frame(frame),));
    assert!(ui.invoke_action_descriptor(
        &action,
        ActionSource::CommandPalette,
        ActionContext::Panel(panel),
    ));
    assert!(!ui.invoke_action_descriptor(
        &hidden,
        ActionSource::Programmatic,
        ActionContext::Global,
    ));
    assert!(!ui.invoke_action_descriptor(
        &disabled,
        ActionSource::Programmatic,
        ActionContext::Global,
    ));
    let mut output = ui.finish_output();

    assert_eq!(
        output
            .actions
            .drain()
            .map(|invocation| (invocation.action_id, invocation.source, invocation.context,))
            .collect::<Vec<_>>(),
        vec![
            (
                ActionId::new("export"),
                ActionSource::Menu,
                ActionContext::Frame(frame),
            ),
            (
                ActionId::new("export"),
                ActionSource::CommandPalette,
                ActionContext::Panel(panel),
            ),
        ]
    );
}

#[test]
fn action_surface_conformance_action_button_invokes_only_visible_enabled_actions() {
    let theme = default_dark_theme();
    let rect = Rect::new(0.0, 0.0, 80.0, 28.0);
    let context = ActionContext::Widget(WidgetId::from_key("toolbar"));
    let action = descriptor("run", "Run");
    let mut memory = UiMemory::new();

    let input = pointer_input(Point::new(4.0, 4.0), true);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let pressed = ui
        .action_button("run", rect, &action, context.clone())
        .expect("visible button");
    assert!(pressed.state.pressed);
    assert!(ui.finish_output().actions.is_empty());

    let input = pointer_input(Point::new(4.0, 4.0), false);
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let released = ui
        .action_button("run", rect, &action, context.clone())
        .expect("visible button");
    let mut output = ui.finish_output();
    let invocation = output.actions.pop_front().expect("button invocation");

    assert!(released.clicked);
    assert_eq!(invocation.action_id, ActionId::new("run"));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, context);

    let mut hidden = descriptor("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = descriptor("disabled", "Disabled");
    disabled.state.enabled = false;
    let input = pointer_input(Point::new(4.0, 4.0), true);
    let mut disabled_memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut disabled_memory, &theme);

    assert_eq!(
        ui.action_button("hidden", rect, &hidden, ActionContext::Global),
        None
    );
    let disabled_response = ui
        .action_button("disabled", rect, &disabled, ActionContext::Global)
        .expect("disabled visible button");
    let output = ui.finish_output();

    assert!(disabled_response.state.disabled);
    assert!(output.actions.is_empty());
}

#[test]
fn action_surface_conformance_menu_invokes_enabled_visible_items_only() {
    let context = ActionContext::Panel(WidgetId::from_key("inspector"));
    let mut checked = descriptor("view.grid", "Grid");
    checked.state.checked = Some(true);
    let mut hidden = descriptor("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = descriptor("disabled", "Disabled");
    disabled.state.enabled = false;
    let menu = Menu::from_actions([descriptor("open", "Open"), checked, hidden, disabled]);
    let mut queue = ActionQueue::new();

    let visible = menu.visible_items();
    assert_eq!(visible.len(), 3);
    assert!(matches!(
        visible[1],
        MenuItem::Action(action) if action.id == ActionId::new("view.grid")
            && action.state.checked == Some(true)
    ));
    assert!(menu.invoke_visible(0, &mut queue, context.clone()));
    assert!(menu.invoke_visible(1, &mut queue, context.clone()));
    assert!(!menu.invoke_visible(2, &mut queue, context.clone()));
    assert!(!menu.invoke_visible(3, &mut queue, context.clone()));

    let invocation = queue.pop_front().expect("menu invocation");
    assert_eq!(invocation.action_id, ActionId::new("open"));
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, context);

    let invocation = queue.pop_front().expect("checked menu invocation");
    assert_eq!(invocation.action_id, ActionId::new("view.grid"));
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(invocation.context, context);
    assert!(queue.is_empty());
}

#[test]
fn action_surface_conformance_command_palette_filters_and_invokes_selected_entry_only() {
    let context = ActionContext::Global;
    let mut save = descriptor("save", "Save Project");
    save.keywords = vec!["write".to_owned(), "persist".to_owned()];
    save.state.checked = Some(true);
    let mut hidden = descriptor("hidden", "Hidden");
    hidden.state.visible = false;
    let mut disabled = descriptor("disabled", "Disabled");
    disabled.state.enabled = false;
    let mut palette =
        CommandPalette::from_actions(&[save, hidden, disabled, descriptor("export", "Export")]);

    palette.query = "write".to_owned();
    let matches = palette.matches();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].action_id, ActionId::new("save"));
    assert_eq!(matches[0].checked, Some(true));
    assert!(matches[0].enabled);

    palette.query = "expo".to_owned();
    let matches = palette.matches();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].action_id, ActionId::new("export"));

    palette.query = "write".to_owned();
    let mut queue = ActionQueue::new();
    assert!(palette.invoke_selected(&mut queue, context.clone()));
    let invocation = queue.pop_front().expect("palette invocation");
    assert_eq!(invocation.action_id, ActionId::new("save"));
    assert_eq!(invocation.source, ActionSource::CommandPalette);
    assert_eq!(invocation.context, context);

    palette.query = "disabled".to_owned();
    palette.selected = 0;
    assert!(!palette.invoke_selected(&mut queue, ActionContext::Global));
    assert!(queue.is_empty());

    palette.query = "hidden".to_owned();
    assert!(palette.matches().is_empty());
}

#[test]
fn action_surface_conformance_command_palette_clamps_selection_and_empty_results() {
    let mut palette = CommandPalette::from_actions(&[
        descriptor("first", "First"),
        descriptor("second", "Second"),
        descriptor("third", "Third"),
    ]);
    let mut queue = ActionQueue::new();

    palette.move_selection(99);
    assert_eq!(palette.selected, 2);
    palette.move_selection(-99);
    assert_eq!(palette.selected, 0);

    palette.selected = 2;
    palette.query = "second".to_owned();
    palette.clamp_selection();
    assert_eq!(palette.selected, 0);
    assert!(palette.invoke_selected(&mut queue, ActionContext::Global));
    assert_eq!(
        queue.pop_front().expect("clamped invocation").action_id,
        ActionId::new("second")
    );

    palette.selected = 2;
    palette.query = "missing".to_owned();
    palette.clamp_selection();
    assert_eq!(palette.selected, 0);
    assert!(palette.matches().is_empty());
    assert!(!palette.invoke_selected(&mut queue, ActionContext::Global));
    palette.move_selection(1);
    assert_eq!(palette.selected, 0);
    assert!(queue.is_empty());
}
