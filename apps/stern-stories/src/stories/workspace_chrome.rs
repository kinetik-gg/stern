//! Workspace chrome-bands story: viewport-driven editor chrome plus dock.
//!
//! Full-window composition for issue #944: the menu bar, toolbar, tab
//! strip, dock content, and status bar are laid out with
//! [`ChromeBandLayout::from_viewport`], so band heights come from theme
//! size tokens and the dock band absorbs every remaining logical pixel.
//! Rendering this story across the workspace size matrix (960x640,
//! 1280x800, 3840x2160) proves the layout tracks the viewport instead of
//! leaving a fixed-size island with dead space.

use stern::core::{ActionContext, ActionDescriptor, Axis, PointerOrder, Rect, WidgetId};
use stern::widgets::chrome::{
    ChromeBandLayout, ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, MenuBar, MenuBarMenu,
    MenuBarMenuId, StatusBar, StatusItem, StatusItemId, StatusItemKind, TabStrip, Toolbar,
    ToolbarGroup, ToolbarGroupId,
};
use stern::widgets::dock::{DockScene, DockSceneConfig};
use stern::widgets::{Dock, DockNode, Frame, FrameId, FrameTab, Panel, PanelId, Ui};

use crate::story::{Story, StoryKind};

const ASSETS_PANEL: PanelId = PanelId::from_raw(1);
const VIEWPORT_PANEL: PanelId = PanelId::from_raw(2);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(3);
const APPLICATION_MENU: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);

/// Chrome-bands workspace: token-derived chrome ladder over a full-window
/// dock, recomputed from the story viewport every compose.
#[must_use]
pub fn with_bands() -> Story {
    Story {
        id: "workspace/chrome-bands",
        title: "Workspace chrome bands track the viewport",
        kind: StoryKind::Workspace,
        compose,
    }
}

fn story_dock() -> Dock {
    let assets = frame(1, ASSETS_PANEL, "Assets");
    let viewport = frame(2, VIEWPORT_PANEL, "Viewport");
    let inspector = frame(3, INSPECTOR_PANEL, "Inspector");
    let right = split(Axis::Horizontal, 0.7, viewport, inspector);
    let mut dock = Dock::new(split(Axis::Horizontal, 0.22, assets, right));
    let _ = dock.set_active_frame(FrameId::from_raw(2));
    dock
}

fn frame(id: u64, panel: PanelId, title: &str) -> DockNode {
    DockNode::Frame(Frame::new(
        FrameId::from_raw(id),
        vec![Panel::new(panel, title)],
    ))
}

fn split(axis: Axis, ratio: f32, first: DockNode, second: DockNode) -> DockNode {
    DockNode::Split {
        axis,
        ratio,
        min_first: 120.0,
        min_second: 120.0,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn workspace_tab(panel: u64, title: &str, active: bool) -> FrameTab {
    FrameTab {
        panel: PanelId::from_raw(panel),
        title: title.to_owned(),
        active,
        close_visible: false,
        draggable: false,
    }
}

#[allow(clippy::too_many_lines)]
fn compose(ui: &mut Ui<'_>, rect: Rect) {
    let bands = ChromeBandLayout::from_viewport(rect, ui.theme());
    let actions = [
        ActionDescriptor::new("story.apply", "Apply Changes"),
        ActionDescriptor::new("story.render", "Render Preview"),
    ];
    let menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
        APPLICATION_MENU,
        "Workspace",
        actions.iter().cloned(),
    )]);
    let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
        TOOLBAR_GROUP,
        "Workspace actions",
        actions.iter().cloned(),
    )]);
    let tab_strip = TabStrip::from_tabs([
        workspace_tab(101, "Edit Workspace", true),
        workspace_tab(102, "Graph Workspace", false),
    ]);
    let status_bar = StatusBar::from_items([StatusItem::new(
        StatusItemId::from_raw(1),
        "Revision",
        "Applied revision 3",
        StatusItemKind::Ready,
    )]);
    let mut widths = vec![
        (ChromeSceneItemKey::Menu(APPLICATION_MENU), 96.0),
        (ChromeSceneItemKey::Tab(PanelId::from_raw(101)), 132.0),
        (ChromeSceneItemKey::Tab(PanelId::from_raw(102)), 140.0),
        (ChromeSceneItemKey::Status(StatusItemId::from_raw(1)), 152.0),
    ];
    widths.extend(actions.iter().map(|action| {
        (
            ChromeSceneItemKey::Toolbar {
                group: TOOLBAR_GROUP,
                action: action.id.clone(),
            },
            144.0,
        )
    }));
    let chrome = ChromeScene::new(
        ChromeSceneConfig::new(
            WidgetId::from_key("story-chrome-bands.chrome"),
            bands.menu_bar,
            bands.toolbar,
            bands.tab_strip,
            bands.status_bar,
            ActionContext::Editor,
        )
        .with_widths(widths),
        &menu_bar,
        &toolbar,
        &tab_strip,
        &status_bar,
    );
    let dock = story_dock();
    let dock_scene = DockScene::new(
        DockSceneConfig::new(WidgetId::from_key("story-chrome-bands.dock"), bands.content),
        &dock,
    );

    ui.resolve_pointer_targets(|plan| {
        let next = dock_scene.declare_pointer_targets_with_content(
            plan,
            PointerOrder::new(0),
            |_, next| next,
        );
        let _ = chrome.declare_pointer_targets(plan, next);
    })
    .expect("story chrome and dock declare unique pointer targets");

    let _ = ui.dock_scene(&dock_scene, |ui, panel| {
        let body = panel.rect.inset(8.0);
        let label = match panel.panel {
            ASSETS_PANEL => "Assets fill the left band",
            INSPECTOR_PANEL => "Inspector fills the right band",
            _ => "Viewport fills the remaining pixels",
        };
        ui.label(Rect::new(body.x, body.y, body.width, 16.0), label);
    });
    let _ = ui.chrome_scene(&chrome);
}
