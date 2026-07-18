use stern::core::{
    ActionContext, ActionInvocation, ActionSource, Axis, PointerOrder, Rect, Size, TextureId,
    WidgetId,
};
use stern::render::{RenderImage, RenderImageSampling, RenderResources, TextureResource};
use stern::widgets::dock::{DockScene, DockSceneConfig};
use stern::widgets::inspector::PropertyGridConfig;
use stern::widgets::{
    ChromeScene, ChromeSceneConfig, ChromeSceneIntent, ChromeSceneItemKey, CollectionCursor,
    CollectionProjection, Dock, DockNode, Frame, FrameId, FrameTab, ItemId, MenuBar, MenuBarMenu,
    MenuBarMenuId, PanZoom, Panel, PanelId, PropertyGridRow, Selection, StatusBar, StatusItem,
    StatusItemId, StatusItemKind, TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId, Ui,
    ViewportSurface, ViewportWidgetConfig, VirtualListConfig, VirtualListRow,
};

use crate::{DemoActionRegistry, DemoWorkspace};

const ASSETS_PANEL: PanelId = PanelId::from_raw(11);
const VIEWPORT_PANEL: PanelId = PanelId::from_raw(21);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(31);
const VIEWPORT_TEXTURE: TextureId = TextureId::from_raw(1);
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);

#[derive(Debug, Clone, Copy)]
struct AssetFixture {
    id: ItemId,
    name: &'static str,
    kind: &'static str,
}

const ASSETS: [AssetFixture; 4] = [
    AssetFixture {
        id: ItemId::from_raw(1),
        name: "Backdrop",
        kind: "Raster layer",
    },
    AssetFixture {
        id: ItemId::from_raw(2),
        name: "Character",
        kind: "Vector layer",
    },
    AssetFixture {
        id: ItemId::from_raw(3),
        name: "Lighting",
        kind: "Adjustment layer",
    },
    AssetFixture {
        id: ItemId::from_raw(4),
        name: "Title",
        kind: "Text layer",
    },
];

/// Retained public Stern state for the deterministic Edit workspace fixture.
pub(crate) struct EditWorkspace {
    dock: Dock,
    projection: CollectionProjection,
    cursor: CollectionCursor,
    selection: Selection,
    pan_zoom: PanZoom,
    texture: TextureResource,
}

impl EditWorkspace {
    pub(crate) fn new() -> Self {
        let mut selection = Selection::new();
        selection.replace(ASSETS[0].id);
        let mut cursor = CollectionCursor::new();
        let projection = CollectionProjection::from_source_ids(
            &ASSETS.iter().map(|asset| asset.id).collect::<Vec<_>>(),
        );
        let _ = cursor.activate(&projection, ASSETS[0].id);

        Self {
            dock: edit_dock(),
            projection,
            cursor,
            selection,
            pan_zoom: PanZoom::default(),
            texture: viewport_texture(),
        }
    }

    pub(crate) fn compose(
        &mut self,
        ui: &mut Ui<'_>,
        actions: &DemoActionRegistry,
        workspace: DemoWorkspace,
        revision: u32,
        bounds: Size,
    ) {
        let layout = WorkspaceLayout::new(bounds);
        let menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
            MenuBarMenuId::from_raw(1),
            "Workspace",
            actions.iter().cloned(),
        )]);
        let toolbar = Toolbar::from_groups([ToolbarGroup::from_actions(
            TOOLBAR_GROUP,
            "Workspace actions",
            actions.iter().cloned(),
        )]);
        let tab_strip = TabStrip::from_tabs([
            workspace_tab(101, "Edit Workspace", workspace == DemoWorkspace::Edit),
            workspace_tab(102, "Graph Workspace", workspace == DemoWorkspace::Graph),
        ]);
        let status_bar = StatusBar::from_items([StatusItem::new(
            StatusItemId::from_raw(1),
            "Revision",
            format!("Applied revision {revision}"),
            StatusItemKind::Ready,
        )]);
        let chrome = ChromeScene::new(
            chrome_config(layout, actions),
            &menu_bar,
            &toolbar,
            &tab_strip,
            &status_bar,
        );
        let dock_scene = DockScene::new(
            DockSceneConfig::new(WidgetId::from_key("edit-workspace.dock"), layout.dock),
            &self.dock,
        );

        let assets_bounds = panel_bounds(&dock_scene, ASSETS_PANEL).map(|rect| rect.inset(8.0));
        let viewport_bounds = panel_bounds(&dock_scene, VIEWPORT_PANEL).map(|rect| rect.inset(8.0));
        let list = assets_bounds.and_then(|rect| {
            ui.prepare_virtual_list(
                "assets",
                VirtualListConfig::new(rect, 28.0).label("Assets"),
                &self.projection,
            )
        });
        let viewport = viewport_bounds.map(|rect| {
            ui.prepare_viewport_widget(ViewportWidgetConfig::new(
                WidgetId::from_key("edit-workspace.viewport"),
                ViewportSurface {
                    texture: VIEWPORT_TEXTURE,
                    source_size: Size::new(1280.0, 720.0),
                    bounds: rect,
                    pan_zoom: self.pan_zoom,
                },
            ))
        });

        ui.resolve_pointer_targets(|plan| {
            let next = dock_scene.declare_pointer_targets_with_content(
                plan,
                PointerOrder::new(0),
                |plan, mut next| {
                    if let Some(list) = &list {
                        next = list.declare_pointer_targets(plan, next);
                    }
                    if let Some(viewport) = &viewport {
                        next = viewport.declare_pointer_targets(plan, next);
                    }
                    next
                },
            );
            chrome.declare_pointer_targets(plan, next);
        })
        .expect("Edit workspace pointer targets are valid");

        let projection = &self.projection;
        let cursor = &mut self.cursor;
        let selection = &mut self.selection;
        let pan_zoom = &mut self.pan_zoom;
        let _ = ui.dock_scene(&dock_scene, |ui, panel| match panel.panel {
            ASSETS_PANEL => {
                if let Some(list) = &list {
                    let _ = ui.virtual_list(list, cursor, selection, |item| {
                        VirtualListRow::new(ASSETS[item.source_index].name)
                    });
                }
            }
            VIEWPORT_PANEL => {
                if let Some(viewport) = &viewport {
                    let output = ui.viewport_widget(viewport, pan_zoom, &[]);
                    *pan_zoom = output.next_pan_zoom;
                }
            }
            INSPECTOR_PANEL => {
                let selected = selection
                    .active
                    .and_then(|id| projection.source_index(id))
                    .map(|index| ASSETS[index]);
                inspector(ui, panel.rect.inset(8.0), selected);
            }
            _ => {}
        });
        let chrome_output = ui.chrome_scene(&chrome);
        route_workspace_tabs(ui, actions, chrome_output.intents);
    }

    pub(crate) fn register_resources(&self, resources: &mut RenderResources) {
        resources.register_texture(self.texture.clone());
    }
}

fn route_workspace_tabs(
    ui: &mut Ui<'_>,
    actions: &DemoActionRegistry,
    intents: impl IntoIterator<Item = ChromeSceneIntent>,
) {
    for intent in intents {
        let ChromeSceneIntent::ActivateTab(target) = intent else {
            continue;
        };
        let action = if target.panel == PanelId::from_raw(101) {
            actions.edit_workspace()
        } else if target.panel == PanelId::from_raw(102) {
            actions.graph_workspace()
        } else {
            continue;
        };
        ui.push_action(ActionInvocation::new(
            action.id.clone(),
            ActionSource::Button,
            ActionContext::Editor,
        ));
    }
}

#[derive(Debug, Clone, Copy)]
struct WorkspaceLayout {
    menu: Rect,
    toolbar: Rect,
    tabs: Rect,
    dock: Rect,
    status: Rect,
}

impl WorkspaceLayout {
    fn new(size: Size) -> Self {
        let width = size.width.max(0.0);
        let height = size.height.max(0.0);
        let dock_y = 88.0_f32.min(height);
        let status_y = (height - 24.0).max(dock_y);
        Self {
            menu: Rect::new(0.0, 0.0, width, 28.0_f32.min(height)),
            toolbar: Rect::new(0.0, 28.0, width, 32.0_f32.min((height - 28.0).max(0.0))),
            tabs: Rect::new(0.0, 60.0, width, 28.0_f32.min((height - 60.0).max(0.0))),
            dock: Rect::new(0.0, dock_y, width, (status_y - dock_y).max(0.0)),
            status: Rect::new(0.0, status_y, width, (height - status_y).max(0.0)),
        }
    }
}

fn chrome_config(layout: WorkspaceLayout, actions: &DemoActionRegistry) -> ChromeSceneConfig {
    let mut widths = vec![
        (ChromeSceneItemKey::Menu(MenuBarMenuId::from_raw(1)), 96.0),
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
    ChromeSceneConfig::new(
        WidgetId::from_key("edit-workspace.chrome"),
        layout.menu,
        layout.toolbar,
        layout.tabs,
        layout.status,
        ActionContext::Editor,
    )
    .with_widths(widths)
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

fn edit_dock() -> Dock {
    let assets = dock_frame(1, ASSETS_PANEL, "Assets");
    let viewport = dock_frame(2, VIEWPORT_PANEL, "Viewport");
    let inspector = dock_frame(3, INSPECTOR_PANEL, "Inspector");
    let right = split(Axis::Horizontal, 0.72, viewport, inspector);
    let mut dock = Dock::new(split(Axis::Horizontal, 0.22, assets, right));
    let _ = dock.set_active_frame(FrameId::from_raw(2));
    dock
}

fn dock_frame(id: u64, panel_id: PanelId, title: &str) -> DockNode {
    let mut frame = Frame::new(FrameId::from_raw(id), vec![Panel::new(panel_id, title)]);
    let _ = frame.set_panel_dismissible(panel_id, false);
    DockNode::Frame(frame)
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

fn panel_bounds(scene: &DockScene, panel: PanelId) -> Option<Rect> {
    scene
        .layout()
        .frames
        .iter()
        .find_map(|frame| frame.panel.as_ref().filter(|item| item.panel == panel))
        .map(|panel| panel.rect)
}

fn inspector(ui: &mut Ui<'_>, bounds: Rect, selected: Option<AssetFixture>) {
    let rows = [
        PropertyGridRow::section(ItemId::from_raw(100), "Selection"),
        PropertyGridRow::property(ItemId::from_raw(101), "Name", 0).with_read_only(true),
        PropertyGridRow::property(ItemId::from_raw(102), "Kind", 0).with_read_only(true),
    ];
    let values = selected.map_or(("No selection", "Unavailable"), |asset| {
        (asset.name, asset.kind)
    });
    let _ = ui.property_grid(
        "selected-asset",
        bounds,
        &rows,
        PropertyGridConfig::default(),
        |ui, cell| {
            let value = if cell.row.id == ItemId::from_raw(101) {
                values.0
            } else {
                values.1
            };
            ui.label_keyed(("value", cell.row.id.raw()), cell.value_rect, value);
        },
    );
}

fn viewport_texture() -> TextureResource {
    let pixels = RenderImage::rgba8(
        1280,
        720,
        include_bytes!("../assets/viewport-1280x720.rgba").to_vec(),
    )
    .expect("bundled viewport RGBA dimensions are exact");
    TextureResource {
        id: VIEWPORT_TEXTURE,
        size: Size::new(1280.0, 720.0),
        sampling: RenderImageSampling::HighQuality,
        snapshot: Some(pixels),
    }
}
