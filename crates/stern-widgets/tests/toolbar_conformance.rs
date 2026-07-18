//! Windowless toolbar conformance for reusable editor chrome contracts.

use stern_core::{ActionContext, ActionDescriptor, ActionId, ActionQueue, ActionSource, WidgetId};
use stern_widgets::{Toolbar, ToolbarGroup, ToolbarGroupId, ToolbarItem, ToolbarItemPresentation};

fn group_id(raw: u64) -> ToolbarGroupId {
    ToolbarGroupId::from_raw(raw)
}

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn hidden_action(id: &str) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(id, "Hidden");
    action.state.visible = false;
    action
}

fn disabled_action(id: &str, label: &str) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(id, label);
    action.state.enabled = false;
    action
}

#[test]
fn toolbar_visible_items_filter_hidden_actions_and_retain_disabled_actions() {
    let group = ToolbarGroup::from_actions(
        group_id(1),
        "Playback",
        [
            action("transport.play", "Play"),
            hidden_action("transport.hidden"),
            disabled_action("transport.stop", "Stop"),
        ],
    );

    let visible = group.visible_items();

    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].action_id(), &ActionId::new("transport.play"));
    assert!(visible[0].enabled());
    assert_eq!(visible[1].action_id(), &ActionId::new("transport.stop"));
    assert!(!visible[1].enabled());
}

#[test]
fn toolbar_disabled_items_cannot_invoke_and_unknown_indexes_are_ignored() {
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        group_id(1),
        "File",
        [
            action("file.save", "Save"),
            disabled_action("file.export", "Export"),
        ],
    )]);
    let mut queue = ActionQueue::new();

    assert!(toolbar.invoke_visible(0, 0, &mut queue, ActionContext::Global));
    assert!(!toolbar.invoke_visible(0, 1, &mut queue, ActionContext::Global));
    assert!(!toolbar.invoke_visible(0, 2, &mut queue, ActionContext::Global));
    assert!(!toolbar.invoke_visible(2, 0, &mut queue, ActionContext::Global));
    assert!(!toolbar.invoke_group_visible(group_id(99), 0, &mut queue, ActionContext::Global));

    let invocation = queue.pop_front().expect("toolbar invocation");
    assert_eq!(invocation.action_id, ActionId::new("file.save"));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Global);
    assert!(queue.is_empty());
}

#[test]
fn toolbar_checked_state_icon_and_presentation_metadata_are_preserved() {
    let mut move_tool = action("tool.move", "Move");
    move_tool.icon = Some(stern_icons_phosphor::regular::CURSOR_CLICK.into());
    move_tool.state.checked = Some(true);
    let item = ToolbarItem::new(move_tool).with_presentation(ToolbarItemPresentation::IconAndText);

    assert_eq!(item.label(), "Move");
    assert_eq!(
        item.icon(),
        Some(stern_icons_phosphor::regular::CURSOR_CLICK.icon())
    );
    assert_eq!(item.checked(), Some(true));
    assert!(item.selected());
    assert_eq!(item.presentation, ToolbarItemPresentation::IconAndText);
}

#[test]
fn toolbar_visible_groups_skip_groups_with_no_visible_items() {
    let tools = group_id(1);
    let hidden = group_id(2);
    let empty = group_id(3);
    let view = group_id(4);
    let toolbar = Toolbar::from_groups([
        ToolbarGroup::from_actions(tools, "Tools", [action("tool.select", "Select")]),
        ToolbarGroup::from_actions(hidden, "Hidden", [hidden_action("tool.hidden")]),
        ToolbarGroup::from_actions(empty, "Empty", Vec::<ActionDescriptor>::new()),
        ToolbarGroup::from_actions(view, "View", [disabled_action("view.grid", "Grid")]),
    ]);

    let visible = toolbar.visible_groups();

    assert_eq!(visible.len(), 2);
    assert_eq!(visible[0].id, tools);
    assert_eq!(visible[0].title, "Tools");
    assert_eq!(visible[1].id, view);
    assert_eq!(visible[1].visible_items().len(), 1);
}

#[test]
fn toolbar_invocation_preserves_action_id_button_source_and_explicit_context() {
    let group = group_id(7);
    let context = ActionContext::Widget(WidgetId::from_key("editor-toolbar"));
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        group,
        "Editing",
        [action("edit.undo", "Undo")],
    )]);
    let mut queue = ActionQueue::new();

    assert!(toolbar.invoke_group_visible(group, 0, &mut queue, context.clone()));
    let invocation = queue.pop_front().expect("toolbar invocation");

    assert_eq!(invocation.action_id, ActionId::new("edit.undo"));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, context);
    assert!(queue.is_empty());
}
