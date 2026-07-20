//! Facade-only application-bar compile coverage.
use stern::{
    core::{Rect, WidgetId, default_dark_theme},
    widgets::{
        ApplicationBar, ApplicationBarConfig, ApplicationMenuBar, MenuBarMenu, MenuBarMenuId,
        WorkspaceTab, WorkspaceTabId,
    },
};
#[test]
fn application_bar_is_constructible_through_the_public_facade() {
    let menu = MenuBarMenu::from_actions(
        MenuBarMenuId::from_raw(1),
        "File",
        [stern::core::ActionDescriptor::new("open", "Open")],
    );
    let bar = ApplicationBar::new(
        ApplicationBarConfig::new(WidgetId::from_key("bar"), Rect::new(0.0, 0.0, 320.0, 40.0)),
        ApplicationMenuBar::from_menus([menu]),
        [WorkspaceTab::new(
            WorkspaceTabId::from_raw(1),
            "Editing",
            true,
        )],
    );
    let prepared = bar.prepare(&default_dark_theme()).unwrap();
    assert_eq!(prepared.drag_safe_regions().len(), 1);
}
