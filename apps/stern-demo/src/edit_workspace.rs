use std::collections::BTreeMap;

use stern::core::{
    ActionContext, ActionInvocation, ActionSource, Axis, MouseButton, PointerOrder, PointerTarget,
    PointerTargetPlan, Rect, Size, TextureId, UiInput, UiInputEvent, WidgetId,
};
use stern::render::{RenderImage, RenderImageSampling, RenderResources, TextureResource};
use stern::text::TextEditState;
use stern::widgets::asset_browser::{
    AssetBrowserConfig, AssetBrowserItem, AssetBrowserLayout, AssetBrowserModel,
    AssetBrowserRequest, AssetBrowserState, AssetBrowserViewMode,
};
use stern::widgets::dock::{DockScene, DockSceneConfig};
use stern::widgets::gradient_editor::GradientEditorConfig;
use stern::widgets::inspector::{
    InspectorPickerCommit, InspectorPickerState, PropertyGridConfig, PropertyGridIntent,
    property_grid_row_affordance_rects, property_grid_row_widget_id, property_grid_value_widget_id,
};
use stern::widgets::{
    ChromeScene, ChromeSceneConfig, ChromeSceneIntent, ChromeSceneItemKey, ColorFieldConfig, Dock,
    DockNode, DropdownItem, DropdownItemId, DropdownModel, Frame, FrameId, FrameTab, GridColumns,
    GridLayout, InlineEditDraftDisposition, InlineEditDraftPolicy, InlineEditFocusLossPolicy,
    InlineEditRequest, ItemId, ListLayout, MenuBar, MenuBarMenu, MenuBarMenuId, NumericInputDraft,
    NumericScrubInputConfig, OverlayId, OverlayScene, Panel, PanelId, PropertyGridRow,
    SelectFieldConfig, StatusBar, StatusItem, StatusItemId, StatusItemKind, TabStrip,
    TextFieldAccess, Toolbar, ToolbarGroup, ToolbarGroupId, Ui, ViewportSurface, ViewportWidget,
    ViewportWidgetConfig,
};

use crate::overlay_workspace::SharedOverlayRoute;
use crate::timeline_workspace::{
    TimelineWorkspace, apply_timeline_output, compose_tool_actions, compose_transport_actions,
    declare_tool_actions, declare_transport_actions, prepare_feedback, prepare_timeline,
    timeline_feedback_rects, timeline_transport_layout, viewport_actions, viewport_content_rect,
    viewport_tool_rects,
};
use crate::{DemoActionRegistry, DemoApplicationModel, DemoWorkspace};

const ASSETS_PANEL: PanelId = PanelId::from_raw(11);
const VIEWPORT_PANEL: PanelId = PanelId::from_raw(21);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(31);
const TIMELINE_PANEL: PanelId = PanelId::from_raw(41);
pub(crate) const VIEWPORT_TEXTURE: TextureId = TextureId::from_raw(1);
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);
const APPLICATION_MENU: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const KIND_PICKER_OVERLAY: OverlayId = OverlayId::from_raw(4);
const COLOR_PICKER_OVERLAY: OverlayId = OverlayId::from_raw(5);
const INSPECTOR_SECTION: ItemId = ItemId::from_raw(100);
const NAME_PROPERTY: ItemId = ItemId::from_raw(101);
const KIND_PROPERTY: ItemId = ItemId::from_raw(102);
const VISIBLE_PROPERTY: ItemId = ItemId::from_raw(103);
const OPACITY_PROPERTY: ItemId = ItemId::from_raw(104);
const COLOR_PROPERTY: ItemId = ItemId::from_raw(105);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AssetKind {
    Raster,
    Vector,
    Adjustment,
    Text,
}

impl AssetKind {
    const fn label(self) -> &'static str {
        match self {
            Self::Raster => "Raster layer",
            Self::Vector => "Vector layer",
            Self::Adjustment => "Adjustment layer",
            Self::Text => "Text layer",
        }
    }

    const fn dropdown_id(self) -> DropdownItemId {
        DropdownItemId::from_raw(match self {
            Self::Raster => 1,
            Self::Vector => 2,
            Self::Adjustment => 3,
            Self::Text => 4,
        })
    }

    const fn from_dropdown(id: DropdownItemId) -> Option<Self> {
        match id.raw() {
            1 => Some(Self::Raster),
            2 => Some(Self::Vector),
            3 => Some(Self::Adjustment),
            4 => Some(Self::Text),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct AssetRecord {
    id: ItemId,
    name: String,
    kind: AssetKind,
    visible: bool,
    opacity: f32,
    selected: bool,
    defaults: AssetDefaults,
}

#[derive(Debug, Clone)]
struct AssetDefaults {
    name: String,
    kind: AssetKind,
    visible: bool,
    opacity: f32,
}

impl AssetRecord {
    fn new(id: u64, name: &str, kind: AssetKind, visible: bool, opacity: f32) -> Self {
        Self {
            id: ItemId::from_raw(id),
            name: name.to_owned(),
            kind,
            visible,
            opacity,
            selected: id == 1,
            defaults: AssetDefaults {
                name: name.to_owned(),
                kind,
                visible,
                opacity,
            },
        }
    }
}

/// Read-only view over the selected canonical application asset record.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DemoSelectedAssetSnapshot<'a> {
    /// Stable collection item identity.
    pub item_id: ItemId,
    /// Current application-owned asset name.
    pub name: &'a str,
    /// Current application-owned kind label.
    pub kind: &'static str,
    /// Current application-owned visibility value.
    pub visible: bool,
    /// Current application-owned opacity value.
    pub opacity: f32,
}

impl<'a> From<&'a AssetRecord> for DemoSelectedAssetSnapshot<'a> {
    fn from(asset: &'a AssetRecord) -> Self {
        Self {
            item_id: asset.id,
            name: &asset.name,
            kind: asset.kind.label(),
            visible: asset.visible,
            opacity: asset.opacity,
        }
    }
}

/// Retained public Stern state for the deterministic Edit workspace fixture.
pub(crate) struct EditWorkspace {
    dock: Dock,
    assets: Vec<AssetRecord>,
    asset_browser: AssetBrowserState,
    opacity_drafts: BTreeMap<ItemId, TextEditState>,
    inspector_picker: InspectorPickerState,
    timeline: TimelineWorkspace,
    texture: TextureResource,
}

impl EditWorkspace {
    pub(crate) fn new(application: &DemoApplicationModel) -> Self {
        let assets = asset_records();
        let model = asset_browser_model(&assets);
        let mut asset_browser = AssetBrowserState::new();
        asset_browser.selection.replace(assets[0].id);
        let _ = asset_browser
            .cursor
            .activate(&model.projection(), assets[0].id);
        let opacity_drafts = assets
            .iter()
            .map(|asset| (asset.id, TextEditState::new(asset.opacity.to_string())))
            .collect();

        Self {
            dock: edit_dock(),
            assets,
            asset_browser,
            opacity_drafts,
            inspector_picker: InspectorPickerState::new(),
            timeline: TimelineWorkspace::new(application),
            texture: viewport_texture(),
        }
    }

    pub(crate) fn selected_asset(&self) -> Option<DemoSelectedAssetSnapshot<'_>> {
        self.assets
            .iter()
            .find(|asset| asset.selected)
            .map(DemoSelectedAssetSnapshot::from)
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn compose(
        &mut self,
        ui: &mut Ui<'_>,
        actions: &DemoActionRegistry,
        workspace: DemoWorkspace,
        model: &mut DemoApplicationModel,
        overlays: &mut SharedOverlayRoute,
        bounds: Size,
    ) -> Option<WidgetId> {
        self.timeline.project(model);
        let layout = WorkspaceLayout::new(bounds);
        let mut menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
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
            workspace_tab(101, "Edit Workspace", workspace == DemoWorkspace::Edit),
            workspace_tab(102, "Graph Workspace", workspace == DemoWorkspace::Graph),
        ]);
        let mut status_items = vec![workspace_status(
            model.applied_revision(),
            self.asset_browser
                .rename_conflict()
                .map(|conflict| conflict.message.as_str()),
        )];
        status_items.extend(TimelineWorkspace::status_items(model));
        let status_bar = StatusBar::from_items(status_items);
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
        overlays.open_color_notice(ui, model, bounds);

        let assets_bounds = panel_bounds(&dock_scene, ASSETS_PANEL).map(|rect| rect.inset(8.0));
        let viewport_bounds = panel_bounds(&dock_scene, VIEWPORT_PANEL).map(|rect| rect.inset(8.0));
        let inspector_bounds =
            panel_bounds(&dock_scene, INSPECTOR_PANEL).map(|rect| rect.inset(8.0));
        let inspector_components = inspector_bounds.map(inspector_component_bounds);
        let inspector_grid_bounds = inspector_components.map(|(grid, _)| grid);
        let gradient_bounds = inspector_components.map(|(_, gradient)| gradient);
        let gradient_id = ui.make_id("edit-workspace.gradient");
        let timeline_bounds = panel_bounds(&dock_scene, TIMELINE_PANEL).map(|rect| rect.inset(6.0));
        let inspector_root = panel_widget_id(&dock_scene, INSPECTOR_PANEL).map(|panel| {
            ui.make_id(("dock-panel-content", panel.raw()))
                .child("selected-asset")
        });
        let tool_root = panel_widget_id(&dock_scene, VIEWPORT_PANEL)
            .map(|panel| ui.make_id(("dock-panel-content", panel.raw())));
        let timeline_root = panel_widget_id(&dock_scene, TIMELINE_PANEL)
            .map(|panel| ui.make_id(("dock-panel-content", panel.raw())));
        let inspector_rows = self
            .assets
            .iter()
            .find(|asset| asset.selected)
            .map_or_else(Vec::new, inspector_rows);
        let asset_model = asset_browser_model(&self.assets);
        let asset_browser =
            prepare_asset_browser(ui, assets_bounds, &asset_model, &self.asset_browser);
        let picker_scene = self.inspector_picker.scene().cloned();
        let tool_rects = viewport_bounds.map(viewport_tool_rects);
        let viewport = viewport_bounds.map(|rect| {
            let id = WidgetId::from_key("edit-workspace.viewport");
            let config = ViewportWidgetConfig::new(
                id,
                ViewportSurface {
                    texture: VIEWPORT_TEXTURE,
                    source_size: Size::new(1280.0, 720.0),
                    bounds: viewport_content_rect(rect),
                    pan_zoom: self.timeline.pan_zoom,
                },
            )
            .with_actions(viewport_actions(actions, id));
            let config = if model.scenario().has_timeline_journey() {
                config.with_label(format!(
                    "Viewport · {}",
                    model.timeline().position().label()
                ))
            } else {
                config
            };
            ui.prepare_viewport_widget(config)
        });
        let viewport_scene = viewport
            .as_ref()
            .map(|viewport| TimelineWorkspace::viewport_scene(ui, viewport, model));
        let timeline_transport = timeline_bounds
            .filter(|_| model.scenario().has_timeline_journey())
            .map(timeline_transport_layout);
        let timeline_content_bounds =
            timeline_transport.map_or(timeline_bounds, |(_, content)| Some(content));
        let timeline_rects = timeline_content_bounds.map(timeline_feedback_rects);
        let timeline = timeline_rects.map(|(bounds, _)| {
            prepare_timeline(
                ui,
                bounds,
                &self.timeline.descriptor,
                &self.timeline.viewport_state,
                model.timeline().frame_rate(),
            )
        });
        let feedback = timeline_rects.map(|(_, bounds)| {
            prepare_feedback(
                ui,
                bounds,
                &self.timeline.jobs,
                &self.timeline.diagnostics,
                &self.timeline.feedback,
            )
        });

        overlays.open_palette_if_requested(ui, actions, bounds);
        let context_route = workspace_context_route(ui, viewport_bounds);

        declare_workspace_targets(
            ui,
            &dock_scene,
            asset_browser.as_ref(),
            &self.asset_browser,
            viewport.as_ref(),
            viewport_scene.as_ref(),
            tool_rects,
            tool_root,
            actions,
            timeline_transport.map(|(rects, _)| rects),
            timeline_root,
            timeline.as_ref(),
            feedback.as_ref(),
            inspector_grid_bounds,
            inspector_root,
            &inspector_rows,
            gradient_bounds,
            gradient_id,
            context_route,
            &chrome,
            overlays.scene(),
            picker_scene.as_ref(),
        );
        resolve_picker_outside_before_lower_ui(ui, &mut self.inspector_picker);

        compose_workspace_panels(
            ui,
            &dock_scene,
            asset_browser.as_ref(),
            viewport.as_ref(),
            viewport_scene.as_ref(),
            tool_rects,
            actions,
            timeline_transport.map(|(rects, _)| rects),
            timeline.as_ref(),
            feedback.as_ref(),
            inspector_grid_bounds,
            &inspector_rows,
            &mut self.asset_browser,
            &mut self.assets,
            &mut self.opacity_drafts,
            &mut self.inspector_picker,
            &mut self.timeline.pan_zoom,
            &mut self.timeline.viewport_tools,
            &mut self.timeline.scrub,
            &mut self.timeline.clip_edit,
            model,
        );
        if let Some(bounds) = gradient_bounds {
            compose_gradient_editor(ui, bounds, gradient_id, model);
        }
        let context_requested = shared_context_requested(ui, viewport_bounds);
        let chrome_output = ui.chrome_scene(&chrome);
        route_workspace_tabs(ui, actions, &chrome_output.intents);
        overlays.reconcile(
            ui,
            actions,
            &mut menu_bar,
            &chrome_output.intents,
            context_requested,
            bounds,
        )
    }

    pub(crate) fn register_resources(&self, resources: &mut RenderResources) {
        resources.register_texture(self.texture.clone());
    }
}

fn resolve_picker_outside_before_lower_ui(ui: &mut Ui<'_>, picker: &mut InspectorPickerState) {
    let Some(bounds) = picker
        .scene()
        .map(stern::widgets::inspector::InspectorPickerScene::bounds)
    else {
        return;
    };
    let input = ui.input();
    let release = if input.events.is_empty() {
        input
            .pointer
            .primary
            .released
            .then_some(input.pointer.position)
            .flatten()
    } else {
        input.events.iter().rev().find_map(|event| match event {
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                position,
                ..
            } => (*position).or(input.pointer.position),
            _ => None,
        })
    };
    if release.is_some_and(|point| !bounds.contains_point(point)) {
        let _ = ui.inspector_picker_scene(picker);
    }
}

fn workspace_context_route(
    ui: &Ui<'_>,
    viewport_bounds: Option<Rect>,
) -> Option<(WidgetId, Option<Rect>)> {
    secondary_route_active(ui.input()).then(|| {
        (
            ui.make_id("edit-workspace.shared-action-context"),
            viewport_bounds,
        )
    })
}

#[allow(clippy::too_many_arguments)]
fn declare_workspace_targets(
    ui: &mut Ui<'_>,
    dock_scene: &DockScene,
    asset_browser: Option<&stern::widgets::asset_browser::AssetBrowserScene<'_>>,
    asset_state: &AssetBrowserState,
    viewport: Option<&ViewportWidget>,
    viewport_scene: Option<&stern::widgets::ViewportToolScene>,
    tool_rects: Option<[Rect; 2]>,
    tool_root: Option<WidgetId>,
    actions: &DemoActionRegistry,
    transport_rects: Option<[Rect; 2]>,
    timeline_root: Option<WidgetId>,
    timeline: Option<&stern::widgets::TimelineWidget<'_>>,
    feedback: Option<&stern::widgets::chrome::SystemFeedbackScene<'_>>,
    inspector_bounds: Option<Rect>,
    inspector_root: Option<WidgetId>,
    inspector_rows: &[PropertyGridRow],
    gradient_bounds: Option<Rect>,
    gradient_id: WidgetId,
    context: Option<(WidgetId, Option<Rect>)>,
    chrome: &ChromeScene<'_>,
    overlay: Option<&OverlayScene>,
    picker: Option<&stern::widgets::inspector::InspectorPickerScene>,
) {
    let gradient_reverse_id = ui.make_id(("gradient-reverse", gradient_id.raw()));
    ui.resolve_pointer_targets(|plan| {
        let mut next = dock_scene.declare_pointer_targets_with_content(
            plan,
            PointerOrder::new(0),
            |plan, mut next| {
                if let Some(asset_browser) = asset_browser {
                    next = asset_browser.declare_pointer_targets(plan, next, asset_state);
                }
                if let Some(viewport) = viewport {
                    next = viewport.declare_pointer_targets(plan, next);
                }
                if let Some(viewport_scene) = viewport_scene {
                    next = viewport_scene.declare_pointer_targets(plan, next);
                }
                if let (Some(rects), Some(root)) = (tool_rects, tool_root) {
                    next = declare_tool_actions(plan, next, root, actions, rects);
                }
                if let (Some(rects), Some(root)) = (transport_rects, timeline_root) {
                    next = declare_transport_actions(plan, next, root, actions, rects);
                }
                if let Some(timeline) = timeline {
                    next = timeline.declare_pointer_targets(plan, next);
                }
                if let Some(feedback) = feedback {
                    next = feedback.declare_pointer_targets(plan, next);
                }
                if let (Some(bounds), Some(root)) = (inspector_bounds, inspector_root) {
                    next = declare_inspector_targets(plan, next, root, bounds, inspector_rows);
                }
                if let Some(bounds) = gradient_bounds {
                    plan.target(PointerTarget::new(gradient_id, bounds, next).domain_drag_source());
                    next = PointerOrder::new(next.raw() + 1);
                    plan.target(PointerTarget::new(
                        gradient_reverse_id,
                        Rect::new(bounds.max_x() - 120.0, bounds.y + 4.0, 112.0, 20.0),
                        next,
                    ));
                    next = PointerOrder::new(next.raw() + 1);
                }
                next
            },
        );
        if let Some((target, Some(rect))) = context {
            plan.target(PointerTarget::new(target, rect, next));
            next = PointerOrder::new(next.raw() + 1);
        }
        next = chrome.declare_pointer_targets(plan, next);
        if let Some(overlay) = overlay {
            next = overlay.declare_pointer_targets(plan, next);
        }
        if let Some(picker) = picker {
            picker.declare_pointer_targets(plan, next);
        }
    })
    .expect("Edit workspace pointer targets are valid");
}

#[allow(clippy::too_many_arguments)]
fn compose_workspace_panels(
    ui: &mut Ui<'_>,
    dock_scene: &DockScene,
    asset_browser: Option<&stern::widgets::asset_browser::AssetBrowserScene<'_>>,
    viewport: Option<&ViewportWidget>,
    viewport_scene: Option<&stern::widgets::ViewportToolScene>,
    tool_rects: Option<[Rect; 2]>,
    actions: &DemoActionRegistry,
    transport_rects: Option<[Rect; 2]>,
    timeline: Option<&stern::widgets::TimelineWidget<'_>>,
    feedback: Option<&stern::widgets::chrome::SystemFeedbackScene<'_>>,
    inspector_bounds: Option<Rect>,
    inspector_rows: &[PropertyGridRow],
    asset_state: &mut AssetBrowserState,
    assets: &mut [AssetRecord],
    opacity_drafts: &mut BTreeMap<ItemId, TextEditState>,
    inspector_picker: &mut InspectorPickerState,
    pan_zoom: &mut stern::widgets::PanZoom,
    viewport_tools: &mut stern::widgets::ViewportToolController,
    scrub: &mut stern::widgets::TimelineScrubController,
    clip_edit: &mut stern::widgets::TimelineClipEditController,
    model: &mut DemoApplicationModel,
) {
    let _ = ui.dock_scene(dock_scene, |ui, panel| match panel.panel {
        ASSETS_PANEL => {
            if let Some(asset_browser) = asset_browser {
                let output = ui.asset_browser(
                    asset_browser,
                    asset_state,
                    |target, draft| rename_conflict(assets, target, draft),
                    |_| Vec::new(),
                );
                apply_asset_browser_requests(assets, output.requests);
                project_selection(assets, asset_state);
            }
        }
        VIEWPORT_PANEL => {
            if let Some(rects) = tool_rects {
                compose_tool_actions(ui, actions, rects);
            }
            if let Some(viewport) = viewport {
                let output = ui.viewport_widget(viewport, pan_zoom, &[]);
                *pan_zoom = output.next_pan_zoom;
            }
            if let Some(viewport_scene) = viewport_scene {
                let _ = ui.viewport_tool_scene(viewport_scene, viewport_tools);
            }
        }
        INSPECTOR_PANEL => {
            let selected = assets.iter_mut().find(|asset| asset.selected);
            if let (Some(bounds), Some(selected)) = (inspector_bounds, selected)
                && let Some(opacity_draft) = opacity_drafts.get_mut(&selected.id)
            {
                inspector(
                    ui,
                    bounds,
                    inspector_rows,
                    selected,
                    opacity_draft,
                    inspector_picker,
                    model,
                );
            }
        }
        TIMELINE_PANEL => {
            if let Some(rects) = transport_rects {
                compose_transport_actions(ui, actions, rects);
            }
            if let Some(timeline) = timeline {
                let output = ui.timeline_widget_with_controllers(timeline, scrub, clip_edit);
                apply_timeline_output(
                    model,
                    output.timeline.intent,
                    &output.timeline.scrub_intents,
                    &output.clip_edit_intents,
                );
            }
            if let Some(feedback) = feedback {
                let _ = ui.system_feedback(feedback);
            }
        }
        _ => {}
    });
}

fn asset_records() -> Vec<AssetRecord> {
    [
        (1, "Backdrop", AssetKind::Raster, true, 1.0),
        (2, "Character", AssetKind::Vector, true, 0.9),
        (3, "Lighting", AssetKind::Adjustment, true, 0.75),
        (4, "Title", AssetKind::Text, true, 1.0),
        (5, "Clouds", AssetKind::Raster, true, 0.8),
        (6, "Foreground", AssetKind::Vector, true, 1.0),
        (7, "Grade", AssetKind::Adjustment, true, 0.65),
        (8, "Subtitle", AssetKind::Text, false, 1.0),
        (9, "Mountains", AssetKind::Raster, true, 0.95),
        (10, "Effects", AssetKind::Vector, true, 0.85),
        (11, "Bloom", AssetKind::Adjustment, true, 0.5),
        (12, "Credits", AssetKind::Text, false, 1.0),
        (13, "Sky", AssetKind::Raster, true, 1.0),
        (14, "Props", AssetKind::Vector, true, 1.0),
        (15, "Contrast", AssetKind::Adjustment, true, 0.4),
        (16, "Location", AssetKind::Text, true, 0.9),
        (17, "Ground", AssetKind::Raster, true, 1.0),
        (18, "Particles", AssetKind::Vector, true, 0.7),
        (19, "Vignette", AssetKind::Adjustment, true, 0.55),
        (20, "Watermark", AssetKind::Text, false, 0.6),
        (21, "Reflections", AssetKind::Raster, true, 0.8),
        (22, "Guides", AssetKind::Vector, false, 1.0),
        (23, "Exposure", AssetKind::Adjustment, true, 0.5),
        (24, "Notes", AssetKind::Text, true, 1.0),
    ]
    .into_iter()
    .map(|(id, name, kind, visible, opacity)| AssetRecord::new(id, name, kind, visible, opacity))
    .collect()
}

fn asset_browser_model(assets: &[AssetRecord]) -> AssetBrowserModel {
    AssetBrowserModel::new(
        assets
            .iter()
            .map(|asset| AssetBrowserItem::new(asset.id, &asset.name, asset.kind.label()))
            .collect::<Vec<_>>(),
    )
}

fn asset_browser_layout() -> AssetBrowserLayout {
    AssetBrowserLayout::new(
        AssetBrowserViewMode::List,
        GridLayout {
            columns: GridColumns::Fixed(2),
            item_size: Size::new(96.0, 72.0),
            gap: 4.0,
        },
        ListLayout::new(28.0),
    )
    .with_overscan(1)
}

fn prepare_asset_browser<'a>(
    ui: &mut Ui<'_>,
    bounds: Option<Rect>,
    model: &'a AssetBrowserModel,
    state: &AssetBrowserState,
) -> Option<stern::widgets::asset_browser::AssetBrowserScene<'a>> {
    bounds.and_then(|rect| {
        ui.prepare_asset_browser(
            "assets",
            AssetBrowserConfig::new(rect, asset_browser_layout())
                .label("Assets")
                .rename_policy(
                    InlineEditFocusLossPolicy::Commit,
                    InlineEditDraftPolicy::new(
                        InlineEditDraftDisposition::Commit,
                        InlineEditDraftDisposition::Cancel,
                    ),
                ),
            model,
            state,
        )
    })
}

fn rename_conflict(assets: &[AssetRecord], target: ItemId, draft: &str) -> Option<String> {
    let draft = draft.trim();
    if draft.is_empty() {
        return Some("Name is required".to_owned());
    }
    assets
        .iter()
        .any(|asset| asset.id != target && asset.name.eq_ignore_ascii_case(draft))
        .then(|| "Name already exists".to_owned())
}

fn apply_asset_browser_requests(assets: &mut [AssetRecord], requests: Vec<AssetBrowserRequest>) {
    for request in requests {
        let AssetBrowserRequest::Rename(InlineEditRequest::Commit(commit)) = request else {
            continue;
        };
        if let Some(asset) = assets.iter_mut().find(|asset| asset.id == commit.target) {
            commit.draft_text.trim().clone_into(&mut asset.name);
        }
    }
}

fn project_selection(assets: &mut [AssetRecord], state: &AssetBrowserState) {
    for asset in assets {
        asset.selected = state.selection.contains(asset.id);
    }
}

fn shared_context_requested(ui: &mut Ui<'_>, bounds: Option<Rect>) -> bool {
    bounds.is_some_and(|rect| {
        ui.context_menu_trigger("edit-workspace.shared-action-context", rect, false)
            .context_requested
    })
}

fn route_workspace_tabs(
    ui: &mut Ui<'_>,
    actions: &DemoActionRegistry,
    intents: &[ChromeSceneIntent],
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

fn secondary_route_active(input: &UiInput) -> bool {
    let secondary = input.pointer.secondary;
    secondary.down || secondary.pressed || secondary.released
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
        (ChromeSceneItemKey::Status(StatusItemId::from_raw(2)), 168.0),
        (ChromeSceneItemKey::Status(StatusItemId::from_raw(3)), 204.0),
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

fn workspace_status(revision: u32, rename_error: Option<&str>) -> StatusItem {
    rename_error.map_or_else(
        || {
            StatusItem::new(
                StatusItemId::from_raw(1),
                "Revision",
                format!("Applied revision {revision}"),
                StatusItemKind::Ready,
            )
        },
        |message| {
            StatusItem::new(
                StatusItemId::from_raw(1),
                "Rename validation",
                message,
                StatusItemKind::Error,
            )
        },
    )
}

fn edit_dock() -> Dock {
    let assets = dock_frame(1, ASSETS_PANEL, "Assets");
    let viewport = dock_frame(2, VIEWPORT_PANEL, "Viewport");
    let inspector = dock_frame(3, INSPECTOR_PANEL, "Inspector");
    let timeline = dock_frame(4, TIMELINE_PANEL, "Timeline");
    let upper = split(Axis::Horizontal, 0.60, viewport, inspector);
    let right = split(Axis::Vertical, 0.68, upper, timeline);
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

fn panel_widget_id(scene: &DockScene, panel: PanelId) -> Option<WidgetId> {
    scene
        .layout()
        .frames
        .iter()
        .find_map(|frame| frame.panel.as_ref().filter(|item| item.panel == panel))
        .map(|panel| panel.id)
}

fn inspector_rows(asset: &AssetRecord) -> Vec<PropertyGridRow> {
    vec![
        PropertyGridRow::section(INSPECTOR_SECTION, "Selection"),
        PropertyGridRow::property(NAME_PROPERTY, "Name", 0)
            .with_resettable(true, asset.name == asset.defaults.name),
        PropertyGridRow::property(KIND_PROPERTY, "Kind", 0)
            .with_resettable(true, asset.kind == asset.defaults.kind),
        PropertyGridRow::property(VISIBLE_PROPERTY, "Visible", 0)
            .with_resettable(true, asset.visible == asset.defaults.visible),
        PropertyGridRow::property(OPACITY_PROPERTY, "Opacity", 0).with_resettable(
            true,
            asset.opacity.to_bits() == asset.defaults.opacity.to_bits(),
        ),
        PropertyGridRow::property(COLOR_PROPERTY, "Color", 0),
    ]
}

fn inspector_component_bounds(bounds: Rect) -> (Rect, Rect) {
    let grid_height = 146.0_f32.min(bounds.height.max(0.0));
    let gap = 4.0_f32.min((bounds.height - grid_height).max(0.0));
    (
        Rect::new(bounds.x, bounds.y, bounds.width, grid_height),
        Rect::new(
            bounds.x,
            bounds.y + grid_height + gap,
            bounds.width,
            (bounds.height - grid_height - gap).max(0.0),
        ),
    )
}

fn declare_inspector_targets(
    plan: &mut PointerTargetPlan,
    mut next: PointerOrder,
    root: WidgetId,
    bounds: Rect,
    rows: &[PropertyGridRow],
) -> PointerOrder {
    let config = PropertyGridConfig::default();
    plan.target(PointerTarget::wheel_only(
        root.child("property-grid-scroll"),
        bounds,
        next,
    ));
    next = PointerOrder::new(next.raw() + 1);
    for geometry in config.layout.visible_row_rects(bounds, rows, 0.0, 0) {
        let row = &rows[geometry.index];
        if row.id == INSPECTOR_SECTION {
            continue;
        }
        let rects = property_grid_row_affordance_rects(
            row,
            geometry.value_rect.inset(2.0).max_zero(),
            config.affordances,
        );
        let value = property_grid_value_widget_id(root, row.id);
        let key = match row.id {
            NAME_PROPERTY => "name",
            KIND_PROPERTY => "kind",
            VISIBLE_PROPERTY => "visible",
            OPACITY_PROPERTY => "opacity",
            COLOR_PROPERTY => "color",
            _ => continue,
        };
        let target = PointerTarget::new(value.child(key), rects.value_rect, next);
        plan.target(if row.id == OPACITY_PROPERTY {
            target.domain_drag_source()
        } else {
            target
        });
        next = PointerOrder::new(next.raw() + 1);
        if let Some(reset) = rects.reset_rect {
            let reset_id = property_grid_row_widget_id(root, row.id)
                .child("affordances")
                .child("reset");
            plan.target(PointerTarget::new(reset_id, reset, next));
            next = PointerOrder::new(next.raw() + 1);
        }
    }
    next
}

fn inspector(
    ui: &mut Ui<'_>,
    bounds: Rect,
    rows: &[PropertyGridRow],
    asset: &mut AssetRecord,
    opacity_draft: &mut TextEditState,
    picker: &mut InspectorPickerState,
    model: &mut DemoApplicationModel,
) {
    let kind_model = kind_model(asset.kind);
    let picker_width = 260.0;
    let picker_bounds = Rect::new(
        (bounds.max_x() - picker_width).max(4.0),
        bounds.y + 24.0,
        picker_width,
        164.0,
    );
    let mut name = TextEditState::new(asset.name.clone());
    let output = ui
        .property_grid(
            "selected-asset",
            bounds,
            rows,
            PropertyGridConfig::default(),
            |ui, cell| match cell.row.id {
                NAME_PROPERTY => {
                    let _ = ui.text_field_with_access(
                        "name",
                        cell.value_rect,
                        &mut name,
                        TextFieldAccess::ReadOnly,
                    );
                }
                KIND_PROPERTY => {
                    let field = ui.select_field(
                        "kind",
                        cell.value_rect,
                        asset.kind.label(),
                        &kind_model,
                        SelectFieldConfig::new("Select kind").open(picker.kind().is_some()),
                    );
                    let _ = ui.select_picker(
                        picker,
                        &field,
                        KIND_PICKER_OVERLAY,
                        picker_bounds,
                        "Asset kind",
                        &kind_model,
                    );
                }
                VISIBLE_PROPERTY => {
                    let _ = ui.checkbox_value_with_label(
                        "visible",
                        cell.value_rect,
                        "Visible",
                        &mut asset.visible,
                        false,
                    );
                }
                OPACITY_PROPERTY => {
                    let output = ui.numeric_scrub_input(
                        "opacity",
                        cell.value_rect,
                        &mut asset.opacity,
                        opacity_draft,
                        NumericScrubInputConfig::new(0.01).with_range(0.0, 1.0),
                    );
                    if output.input.policy.commit_requested
                        && let NumericInputDraft::Valid(value) = output.input.policy.draft
                    {
                        asset.opacity = value.clamp(0.0, 1.0);
                        *opacity_draft = TextEditState::new(asset.opacity.to_string());
                    } else if output.input.policy.revert_requested {
                        *opacity_draft = TextEditState::new(asset.opacity.to_string());
                    }
                }
                COLOR_PROPERTY => {
                    let field = ui.color_field(
                        "color",
                        cell.value_rect,
                        "Fill color",
                        model.tagged_color().color(),
                        ColorFieldConfig::default(),
                    );
                    let _ = ui.color_picker(picker, &field, COLOR_PICKER_OVERLAY, picker_bounds);
                }
                _ => unreachable!("property-grid callback skips section rows"),
            },
        )
        .expect("deterministic inspector rows have unique identities");

    for intent in output.intents {
        if let PropertyGridIntent::Reset { row } = intent {
            reset_asset_property(asset, opacity_draft, row);
        }
    }
    if let Some(commit) = ui.inspector_picker_scene(picker).commit {
        match commit {
            InspectorPickerCommit::Select(kind) => {
                if let Some(kind) = AssetKind::from_dropdown(kind) {
                    asset.kind = kind;
                }
            }
            InspectorPickerCommit::Color(color) => model.commit_color(color),
            InspectorPickerCommit::Asset(_) | InspectorPickerCommit::Path(_) => {}
        }
    }
}

fn compose_gradient_editor(
    ui: &mut Ui<'_>,
    bounds: Rect,
    id: WidgetId,
    model: &mut DemoApplicationModel,
) {
    let stops = model.gradient_stops().to_vec();
    let config = GradientEditorConfig::new(id, bounds, model.gradient_interpolation(), &stops)
        .selected_stop(model.selected_gradient_stop());
    let widget = ui
        .prepare_gradient_editor(config)
        .expect("demo gradient uses valid sRGB stops");
    let output = ui.gradient_editor(&widget);
    model.apply_gradient_intents(&output.intents);
}

fn kind_model(selected: AssetKind) -> DropdownModel {
    let mut model = DropdownModel::from_items([
        DropdownItem::new(AssetKind::Raster.dropdown_id(), AssetKind::Raster.label()),
        DropdownItem::new(AssetKind::Vector.dropdown_id(), AssetKind::Vector.label()),
        DropdownItem::new(
            AssetKind::Adjustment.dropdown_id(),
            AssetKind::Adjustment.label(),
        ),
        DropdownItem::new(AssetKind::Text.dropdown_id(), AssetKind::Text.label()),
    ]);
    let _ = model.set_selected_id(selected.dropdown_id());
    model
}

fn reset_asset_property(asset: &mut AssetRecord, opacity_draft: &mut TextEditState, row: ItemId) {
    match row {
        NAME_PROPERTY => asset.defaults.name.clone_into(&mut asset.name),
        KIND_PROPERTY => asset.kind = asset.defaults.kind,
        VISIBLE_PROPERTY => asset.visible = asset.defaults.visible,
        OPACITY_PROPERTY => {
            asset.opacity = asset.defaults.opacity;
            *opacity_draft = TextEditState::new(asset.opacity.to_string());
        }
        _ => {}
    }
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
