//! Windowless keyboard and typeahead conformance for menu-like overlays.

use kinetik_ui_core::{ActionContext, ActionDescriptor, ActionId, ActionSource, Rect, WidgetId};
use kinetik_ui_widgets::overlays::{
    DropdownItem, DropdownItemId, DropdownModel, DropdownNavigationIntent, DropdownOverlay, Menu,
    MenuItem, MenuNavigationIntent, MenuOverlay, OverlayEntry, OverlayId, OverlayKind,
    OverlayNavigationInput, TypeaheadBuffer,
};

fn action(id: &str, label: &str) -> ActionDescriptor {
    ActionDescriptor::new(id, label)
}

fn disabled_action(id: &str, label: &str) -> ActionDescriptor {
    let mut action = action(id, label);
    action.state.enabled = false;
    action
}

fn hidden_action(id: &str, label: &str) -> ActionDescriptor {
    let mut action = action(id, label);
    action.state.visible = false;
    action
}

fn dropdown_item(raw: u64, label: &str) -> DropdownItem {
    DropdownItem::new(DropdownItemId::from_raw(raw), label)
}

fn dropdown_overlay(model: DropdownModel) -> DropdownOverlay {
    DropdownOverlay::new(
        OverlayEntry::new(
            OverlayId::from_raw(20),
            OverlayKind::Dropdown,
            Rect::new(0.0, 0.0, 200.0, 160.0),
        ),
        WidgetId::from_key("dropdown-trigger"),
        model,
    )
}

#[test]
fn menu_navigation_skips_non_actionable_items_and_wraps() {
    let mut menu = Menu::new();
    menu.push(MenuItem::Label("File".to_owned()));
    menu.push(MenuItem::Action(disabled_action("disabled", "Disabled")));
    menu.push(MenuItem::Separator);
    menu.push(MenuItem::Action(action("open", "Open")));
    menu.push(MenuItem::Action(hidden_action("hidden", "Hidden")));
    menu.push(MenuItem::Action(action("save", "Save")));

    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Next),
        Some(ActionId::new("open"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Next),
        Some(ActionId::new("save"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Next),
        Some(ActionId::new("open"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Previous),
        Some(ActionId::new("save"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::First),
        Some(ActionId::new("open"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Last),
        Some(ActionId::new("save"))
    );
    assert_eq!(menu.highlighted_visible_index(), Some(4));

    assert_eq!(
        Menu::new().move_highlight(OverlayNavigationInput::Next),
        None
    );
    let mut disabled = Menu::from_actions([disabled_action("disabled", "Disabled")]);
    assert_eq!(disabled.move_highlight(OverlayNavigationInput::Next), None);
}

#[test]
fn menu_activation_emits_action_submenu_and_close_intents_without_execution() {
    let mut recent = Menu::from_actions([action("recent.one", "First Project")]);
    recent.push_submenu(
        action("recent.more", "More"),
        Menu::from_actions([action("recent.all", "All Projects")]),
    );
    let mut menu = Menu::from_actions([action("open", "Open")]);
    menu.push_submenu(action("recent", "Recent"), recent);
    let frame = WidgetId::from_key("editor-frame");
    let mut overlay = MenuOverlay::new(
        OverlayEntry::new(
            OverlayId::from_raw(31),
            OverlayKind::ContextMenu,
            Rect::new(0.0, 0.0, 200.0, 160.0),
        ),
        menu,
        ActionSource::Programmatic,
        ActionContext::Frame(frame),
    );

    overlay.navigate(OverlayNavigationInput::First);
    let MenuNavigationIntent::Invoke(invocation) = overlay
        .navigate(OverlayNavigationInput::Activate)
        .expect("action intent")
    else {
        panic!("expected invocation intent");
    };
    assert_eq!(invocation.action_id, ActionId::new("open"));
    assert_eq!(invocation.source, ActionSource::Programmatic);
    assert_eq!(invocation.context, ActionContext::Frame(frame));

    overlay.navigate(OverlayNavigationInput::Next);
    let MenuNavigationIntent::OpenSubmenu(intent) = overlay
        .navigate(OverlayNavigationInput::Activate)
        .expect("submenu intent")
    else {
        panic!("expected submenu intent");
    };
    assert_eq!(intent.parent_overlay, OverlayId::from_raw(31));
    assert_eq!(intent.trigger_action, ActionId::new("recent"));
    assert_eq!(intent.visible_index, 1);
    assert_eq!(intent.source, ActionSource::Programmatic);
    assert_eq!(intent.context, ActionContext::Frame(frame));
    let submenu = overlay
        .menu
        .submenu_for_action(&intent.trigger_action)
        .expect("nested menu data");
    assert_eq!(submenu.visible_items().len(), 2);
    assert!(
        submenu
            .submenu_for_action(&ActionId::new("recent.more"))
            .is_some()
    );
    assert_eq!(
        overlay.menu.invocation_for_visible(
            intent.visible_index,
            ActionSource::Menu,
            ActionContext::Global,
        ),
        None
    );

    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Escape),
        Some(MenuNavigationIntent::Close {
            overlay_id: OverlayId::from_raw(31)
        })
    );
}

#[test]
fn menu_replacement_preserves_valid_highlight_and_clears_stale_state() {
    let mut menu = Menu::from_actions([action("first", "First"), action("second", "Second")]);
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Last),
        Some(ActionId::new("second"))
    );

    menu.replace_items([
        MenuItem::Action(action("second", "Second renamed")),
        MenuItem::Action(action("first", "First")),
    ]);
    assert_eq!(menu.highlighted_action_id(), Some(&ActionId::new("second")));
    assert_eq!(menu.highlighted_visible_index(), Some(0));

    menu.replace_items([
        MenuItem::Action(hidden_action("second", "Second")),
        MenuItem::Action(action("first", "First")),
    ]);
    assert_eq!(menu.highlighted_action_id(), None);

    menu.move_highlight(OverlayNavigationInput::First);
    menu.replace_items([MenuItem::Action(disabled_action("first", "First"))]);
    assert_eq!(menu.highlighted_action_id(), None);
}

#[test]
fn menu_typeahead_cycles_resets_is_bounded_and_handles_unicode_no_match() {
    let mut menu = Menu::from_actions([
        action("save", "Save"),
        action("settings", "Settings"),
        action("eclair", "Éclair"),
        action("export", "Export"),
        disabled_action("secret", "Secret"),
    ]);
    let mut state = TypeaheadBuffer::new(1_000, 4);

    assert_eq!(
        menu.typeahead(&mut state, "s", 0),
        Some(ActionId::new("save"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "S", 100),
        Some(ActionId::new("settings"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "s", 200),
        Some(ActionId::new("save"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "É", 1_200),
        Some(ActionId::new("eclair"))
    );
    assert_eq!(menu.typeahead(&mut state, "x", 1_300), None);
    assert_eq!(menu.highlighted_action_id(), Some(&ActionId::new("eclair")));

    state.clear();
    assert_eq!(
        menu.typeahead(&mut state, "exported", 2_000),
        Some(ActionId::new("export"))
    );
    assert_eq!(state.query(), "expo");

    assert_eq!(
        menu.typeahead(&mut state, "s", 3_000),
        Some(ActionId::new("save"))
    );
    assert_eq!(state.query(), "s");
}

#[test]
fn menu_typeahead_starts_after_the_current_highlight() {
    let mut menu = Menu::from_actions([
        action("alpha.first", "Alpha One"),
        action("beta", "Beta"),
        action("alpha.second", "Alpha Two"),
    ]);
    let mut state = TypeaheadBuffer::default();

    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Last),
        Some(ActionId::new("alpha.second"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "a", 0),
        Some(ActionId::new("alpha.first"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "l", 100),
        Some(ActionId::new("alpha.first"))
    );
    assert_eq!(
        menu.move_highlight(OverlayNavigationInput::Last),
        Some(ActionId::new("alpha.second"))
    );
    assert_eq!(
        menu.typeahead(&mut state, "a", 1_100),
        Some(ActionId::new("alpha.first"))
    );
}

#[test]
fn dropdown_keyboard_navigation_wraps_and_emits_selection_and_close_intents() {
    let mut overlay = dropdown_overlay(DropdownModel::from_items([
        dropdown_item(1, "First"),
        dropdown_item(2, "Disabled").with_enabled(false),
        dropdown_item(3, "Third"),
    ]));

    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Next),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(1)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Next),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(3)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Next),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(1)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Previous),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(3)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::First),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(1)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Last),
        Some(DropdownNavigationIntent::Highlighted(
            DropdownItemId::from_raw(3)
        ))
    );
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Activate),
        Some(DropdownNavigationIntent::Select(DropdownItemId::from_raw(
            3
        )))
    );
    assert_eq!(overlay.model.selected_id(), None);
    assert_eq!(
        overlay.navigate(OverlayNavigationInput::Escape),
        Some(DropdownNavigationIntent::Close {
            overlay_id: OverlayId::from_raw(20),
            focus_return: WidgetId::from_key("dropdown-trigger")
        })
    );

    let mut empty = dropdown_overlay(DropdownModel::new());
    assert_eq!(empty.navigate(OverlayNavigationInput::Next), None);
    assert_eq!(empty.navigate(OverlayNavigationInput::Activate), None);
    let mut disabled = dropdown_overlay(DropdownModel::from_items([
        dropdown_item(1, "First").with_enabled(false),
        dropdown_item(2, "Second").with_enabled(false),
    ]));
    assert_eq!(disabled.navigate(OverlayNavigationInput::Last), None);
}

#[test]
fn dropdown_typeahead_matches_menu_behavior_and_reconciles_replacement() {
    let mut model = DropdownModel::from_items([
        dropdown_item(1, "Render"),
        dropdown_item(2, "Replace"),
        dropdown_item(3, "Édition"),
        dropdown_item(4, "Disabled").with_enabled(false),
    ]);
    let mut state = TypeaheadBuffer::new(500, 8);

    assert_eq!(
        model.typeahead(&mut state, "r", 0),
        Some(DropdownItemId::from_raw(1))
    );
    assert_eq!(
        model.typeahead(&mut state, "R", 100),
        Some(DropdownItemId::from_raw(2))
    );
    assert_eq!(
        model.typeahead(&mut state, "é", 600),
        Some(DropdownItemId::from_raw(3))
    );
    assert_eq!(model.typeahead(&mut state, "z", 700), None);
    assert_eq!(model.highlighted_id(), Some(DropdownItemId::from_raw(3)));

    model.replace_items([
        dropdown_item(3, "Édition").with_enabled(false),
        dropdown_item(2, "Replace"),
    ]);
    assert_eq!(model.highlighted_id(), None);
    assert_eq!(
        model.keyboard_move(OverlayNavigationInput::Next),
        Some(DropdownItemId::from_raw(2))
    );
}

#[test]
fn dropdown_typeahead_starts_after_the_current_highlight() {
    let mut model = DropdownModel::from_items([
        dropdown_item(1, "Alpha One"),
        dropdown_item(2, "Beta"),
        dropdown_item(3, "Alpha Two"),
    ]);
    let mut state = TypeaheadBuffer::default();

    assert_eq!(
        model.keyboard_move(OverlayNavigationInput::Last),
        Some(DropdownItemId::from_raw(3))
    );
    assert_eq!(
        model.typeahead(&mut state, "a", 0),
        Some(DropdownItemId::from_raw(1))
    );
    assert_eq!(
        model.typeahead(&mut state, "l", 100),
        Some(DropdownItemId::from_raw(1))
    );
    assert_eq!(
        model.keyboard_move(OverlayNavigationInput::Last),
        Some(DropdownItemId::from_raw(3))
    );
    assert_eq!(
        model.typeahead(&mut state, "a", 1_100),
        Some(DropdownItemId::from_raw(1))
    );
}

#[test]
fn legacy_dropdown_highlight_helpers_remain_clamped_for_compatibility() {
    let mut model =
        DropdownModel::from_items([dropdown_item(1, "First"), dropdown_item(2, "Second")]);

    assert_eq!(model.highlight_next(), Some(DropdownItemId::from_raw(1)));
    assert_eq!(model.highlight_next(), Some(DropdownItemId::from_raw(2)));
    assert_eq!(model.highlight_next(), Some(DropdownItemId::from_raw(2)));
    assert_eq!(
        model.keyboard_move(OverlayNavigationInput::Next),
        Some(DropdownItemId::from_raw(1))
    );
}
