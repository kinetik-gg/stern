use stern::core::{
    ActionContext, ActionDescriptor, ActionInvocation, Axis, PointerOrder, PointerTarget, Rect,
    Size, WidgetId,
};
use stern::text::TextEditState;
use stern::widgets::chrome::{
    ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, MenuBar, MenuBarMenu, MenuBarMenuId,
    StatusBar, StatusItem, StatusItemId, StatusItemKind, TabStrip, Toolbar, ToolbarGroup,
    ToolbarGroupId,
};
use stern::widgets::dock::{
    Dock, DockNode, DockScene, DockSceneConfig, Frame, FrameId, FrameTab, Panel, PanelId,
};
use stern::widgets::inspector::{PropertyGridConfig, PropertyGridRow};
use stern::widgets::node_graph::{
    EdgeDescriptor, EdgeId, GraphRect, GraphVector, NodeDescriptor,
    NodeGraphConnectionCancelReason, NodeGraphConnectionController, NodeGraphConnectionIntent,
    NodeGraphCreateLinkRequest, NodeGraphDescriptor, NodeGraphPanZoom, NodeGraphSelection,
    NodeGraphSelectionTarget, NodeGraphStaticView, NodeGraphViewport, NodeGraphWidgetConfig,
    NodeGraphWidgetIntent, NodeId, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
};
use stern::widgets::{
    ItemId, PanZoom, TextFieldAccess, Ui, ViewportCursorMetadata, ViewportCursorShape,
    ViewportSelectionTargetDescriptor, ViewportSelectionTargetId, ViewportSurface,
    ViewportToolController, ViewportToolDescriptor, ViewportToolId, ViewportToolScene,
    ViewportToolSceneConfig, ViewportTransformHandleSet, ViewportWidget, ViewportWidgetConfig,
};

use crate::edit_workspace::VIEWPORT_TEXTURE;
use crate::overlay_workspace::SharedOverlayRoute;
use crate::timeline_workspace::{
    compose_tool_actions, declare_tool_actions, viewport_actions, viewport_content_rect,
    viewport_tool_rects,
};
use crate::{DemoActionRegistry, DemoApplicationModel, DemoScenario, DemoViewportTool};

const GRAPH_ROOT: WidgetId = WidgetId::from_raw(0x0047_5241_5048);
const CHROME_ROOT: WidgetId = WidgetId::from_raw(0x4348_524f_4d45);
const CLEAR_SELECTION_ACTION: &str = "graph.clear-selection";
const REVERSE_NODE_ORDER_ACTION: &str = "graph.reverse-node-order";
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);
const APPLICATION_MENU: MenuBarMenuId = MenuBarMenuId::from_raw(1);
const SELECTION_STATUS: StatusItemId = StatusItemId::from_raw(1);
const SOURCE_NODE: NodeId = NodeId::from_raw(1);
const OUTPUT_NODE: NodeId = NodeId::from_raw(2);
const IMAGE_OUTPUT: PortId = PortId::from_raw(1);
const IMAGE_INPUT: PortId = PortId::from_raw(1);
const IMAGE_PREVIEW_INPUT: PortId = PortId::from_raw(2);
const VECTOR_INPUT: PortId = PortId::from_raw(3);
const IMAGE_TYPE: PortTypeId = PortTypeId::from_raw(1);
const VECTOR_TYPE: PortTypeId = PortTypeId::from_raw(2);
const EXISTING_EDGE: EdgeId = EdgeId::from_raw(1);
const COMMITTED_EDGE: EdgeId = EdgeId::from_raw(2);
const DOCK_ROOT: WidgetId = WidgetId::from_raw(0x0044_4f43_4b00);
const GRAPH_FRAME: FrameId = FrameId::from_raw(1);
const VIEWPORT_FRAME: FrameId = FrameId::from_raw(2);
const INSPECTOR_FRAME: FrameId = FrameId::from_raw(3);
const GRAPH_PANEL: PanelId = PanelId::from_raw(1);
const VIEWPORT_PANEL: PanelId = PanelId::from_raw(2);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(3);
const INSPECTOR_SECTION: ItemId = ItemId::from_raw(1);
const INSPECTOR_TITLE: ItemId = ItemId::from_raw(2);
const INSPECTOR_X: ItemId = ItemId::from_raw(3);
const INSPECTOR_Y: ItemId = ItemId::from_raw(4);
const INSPECTOR_PORTS: ItemId = ItemId::from_raw(5);
const VIEWPORT_TARGET: ViewportSelectionTargetId = ViewportSelectionTargetId::from_raw(2);
const SELECT_TOOL: ViewportToolId = ViewportToolId::from_raw(1);
const TRANSFORM_TOOL: ViewportToolId = ViewportToolId::from_raw(2);

/// Application-visible outcome of the latest public graph connection lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphConnectionFeedback {
    /// No connection edit has run yet.
    Ready,
    /// Stern resolved a typed preview candidate.
    Previewing,
    /// Stern accepted stable source and target endpoints before release.
    Accepted {
        /// Stable output endpoint accepted by the canonical typed-link policy.
        from: PortEndpoint,
        /// Stable input endpoint accepted by the canonical typed-link policy.
        to: PortEndpoint,
    },
    /// Stern rejected an incompatible candidate without application mutation.
    Rejected,
    /// The application committed the accepted request as its stable final edge.
    Committed(EdgeId),
    /// Stern cancelled the retained gesture and released its ownership.
    Cancelled(NodeGraphConnectionCancelReason),
}

/// Application-owned deterministic fixture and selection for the Graph workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphWorkspaceState {
    dock: Dock,
    graph: NodeGraphDescriptor,
    selection: NodeGraphSelection,
    graph_journey: bool,
    node_order_revision: u32,
    connection: NodeGraphConnectionController,
    connection_feedback: GraphConnectionFeedback,
    graph_pan_zoom: NodeGraphPanZoom,
    viewport_pan_zoom: PanZoom,
    viewport_tools: ViewportToolController,
    toolbar: Toolbar,
    tab_strip: TabStrip,
    status_bar: StatusBar,
}

impl GraphWorkspaceState {
    /// Creates the deterministic app-owned graph fixture.
    #[must_use]
    pub fn new() -> Self {
        Self::for_scenario(DemoScenario::Default)
    }

    /// Creates the app-owned graph fixture for an explicit journey scenario.
    #[must_use]
    pub fn for_scenario(scenario: DemoScenario) -> Self {
        let source_output =
            PortDescriptor::new(IMAGE_OUTPUT, PortDirection::Output, "Image", IMAGE_TYPE);
        let output_input =
            PortDescriptor::new(IMAGE_INPUT, PortDirection::Input, "Image", IMAGE_TYPE);
        let preview_input = PortDescriptor::new(
            IMAGE_PREVIEW_INPUT,
            PortDirection::Input,
            "Preview Image",
            IMAGE_TYPE,
        );
        let vector_input = PortDescriptor::new(
            VECTOR_INPUT,
            PortDirection::Input,
            "Vector Mask",
            VECTOR_TYPE,
        );
        let source = NodeDescriptor::new(
            SOURCE_NODE,
            "Image Source",
            GraphRect::new(36.0, 28.0, 156.0, 96.0),
        )
        .with_ports(vec![source_output]);
        let output = NodeDescriptor::new(
            OUTPUT_NODE,
            "Viewer",
            GraphRect::new(360.0, 28.0, 156.0, 96.0),
        )
        .with_ports(vec![output_input, preview_input, vector_input]);
        let edge = EdgeDescriptor::new(
            EXISTING_EDGE,
            PortEndpoint::new(SOURCE_NODE, IMAGE_OUTPUT),
            PortEndpoint::new(OUTPUT_NODE, IMAGE_INPUT),
        );
        let mut graph = NodeGraphDescriptor::new();
        graph.nodes = vec![source, output];
        graph.edges = vec![edge];
        let auxiliary = DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 72.0,
            min_second: 72.0,
            first: Box::new(DockNode::Frame(Frame::new(
                VIEWPORT_FRAME,
                vec![Panel::new(VIEWPORT_PANEL, "Viewport")],
            ))),
            second: Box::new(DockNode::Frame(Frame::new(
                INSPECTOR_FRAME,
                vec![Panel::new(INSPECTOR_PANEL, "Inspector")],
            ))),
        };
        let dock = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 2.0 / 3.0,
            min_first: 260.0,
            min_second: 180.0,
            first: Box::new(DockNode::Frame(Frame::new(
                GRAPH_FRAME,
                vec![Panel::new(GRAPH_PANEL, "Graph")],
            ))),
            second: Box::new(auxiliary),
        });
        let mut clear_selection = ActionDescriptor::new(CLEAR_SELECTION_ACTION, "Clear selection");
        clear_selection.state.enabled = false;
        Self {
            dock,
            graph,
            selection: NodeGraphSelection::new(),
            graph_journey: scenario.has_graph_journey(),
            node_order_revision: 0,
            connection: NodeGraphConnectionController::default(),
            connection_feedback: GraphConnectionFeedback::Ready,
            graph_pan_zoom: NodeGraphPanZoom::new(GraphVector::new(2.0, 2.0), 1.0),
            viewport_pan_zoom: PanZoom::default(),
            viewport_tools: ViewportToolController::default(),
            toolbar: Toolbar::from_groups([ToolbarGroup::from_actions(
                TOOLBAR_GROUP,
                "Graph selection",
                [clear_selection],
            )]),
            tab_strip: TabStrip::from_tabs([FrameTab {
                panel: GRAPH_PANEL,
                title: "Graph".to_owned(),
                active: true,
                close_visible: false,
                draggable: false,
            }]),
            status_bar: StatusBar::from_items([connection_status(
                GraphConnectionFeedback::Ready,
                0,
            )]),
        }
    }

    /// Returns the caller-owned graph selection.
    #[must_use]
    pub const fn selection(&self) -> &NodeGraphSelection {
        &self.selection
    }

    /// Returns the app-owned edge records in stable descriptor order.
    #[must_use]
    pub fn edges(&self) -> &[EdgeDescriptor] {
        &self.graph.edges
    }

    /// Returns graph nodes in application presentation order.
    #[must_use]
    pub fn nodes(&self) -> &[NodeDescriptor] {
        &self.graph.nodes
    }

    /// Returns the application-owned node presentation-order revision.
    #[must_use]
    pub const fn node_order_revision(&self) -> u32 {
        self.node_order_revision
    }

    /// Returns the latest public connection lifecycle outcome.
    #[must_use]
    pub const fn connection_feedback(&self) -> GraphConnectionFeedback {
        self.connection_feedback
    }

    /// Returns whether Stern currently owns a retained connection gesture.
    #[must_use]
    pub fn connection_active(&self) -> bool {
        self.connection.is_connecting()
    }

    /// Returns Stern's stable source endpoint while it retains a connection gesture.
    #[must_use]
    pub fn connection_start_endpoint(&self) -> Option<PortEndpoint> {
        self.connection.start_endpoint()
    }

    /// Returns the stable Graph workspace root identity.
    #[must_use]
    pub const fn root_id(&self) -> WidgetId {
        GRAPH_ROOT
    }

    /// Returns the retained non-default transform used by Graph presentation and targeting.
    #[must_use]
    pub const fn pan_zoom(&self) -> NodeGraphPanZoom {
        self.graph_pan_zoom
    }

    /// Handles application-owned actions exposed by the Graph workspace.
    pub fn handle_action(&mut self, invocation: &ActionInvocation) -> bool {
        if invocation.action_id.as_str() == CLEAR_SELECTION_ACTION && !self.selection.is_empty() {
            self.selection = NodeGraphSelection::new();
            return true;
        }
        if invocation.action_id.as_str() == REVERSE_NODE_ORDER_ACTION && self.graph_journey {
            self.graph.nodes.reverse();
            self.node_order_revision += 1;
            return true;
        }
        false
    }

    #[allow(clippy::too_many_arguments, clippy::too_many_lines)]
    pub(crate) fn compose(
        &mut self,
        ui: &mut Ui<'_>,
        bounds: Rect,
        viewport_size: Size,
        app_targets: &[(WidgetId, Rect)],
        actions: &DemoActionRegistry,
        model: &mut DemoApplicationModel,
        overlays: &mut SharedOverlayRoute,
    ) -> Option<WidgetId> {
        self.sync_chrome_models();
        let [
            menu_rect,
            toolbar_rect,
            tab_strip_rect,
            dock_rect,
            status_bar_rect,
        ] = chrome_layout(bounds);
        let dock = self.dock.clone();
        let mut menu_bar = MenuBar::from_menus([MenuBarMenu::from_actions(
            APPLICATION_MENU,
            "Workspace",
            actions.iter().cloned(),
        )]);
        let toolbar = self.toolbar.clone();
        let tab_strip = self.tab_strip.clone();
        let status_bar = self.status_bar.clone();
        let dock_scene = DockScene::new(DockSceneConfig::new(DOCK_ROOT, dock_rect), &dock);
        let viewport_bounds = panel_bounds(&dock_scene, VIEWPORT_PANEL).map(|rect| rect.inset(4.0));
        let viewport_root = panel_widget_id(&dock_scene, VIEWPORT_PANEL)
            .map(|panel| ui.make_id(("dock-panel-content", panel.raw())));
        let tool_rects = viewport_bounds.map(viewport_tool_rects);
        let viewport = viewport_bounds.map(|rect| {
            let id = WidgetId::from_key("graph-workspace.viewport");
            ui.prepare_viewport_widget(
                ViewportWidgetConfig::new(
                    id,
                    ViewportSurface {
                        texture: VIEWPORT_TEXTURE,
                        source_size: Size::new(1280.0, 720.0),
                        bounds: viewport_content_rect(rect),
                        pan_zoom: self.viewport_pan_zoom,
                    },
                )
                .with_label("Graph preview viewport")
                .with_actions(viewport_actions(actions, id)),
            )
        });
        let viewport_scene = viewport
            .as_ref()
            .map(|viewport| graph_viewport_scene(ui, viewport, model.viewport_tool()));
        overlays.open_palette_if_requested(ui, actions, viewport_size);
        let mut chrome_config = ChromeSceneConfig::new(
            CHROME_ROOT,
            menu_rect,
            toolbar_rect,
            tab_strip_rect,
            status_bar_rect,
            ActionContext::Editor,
        )
        .with_widths([
            (ChromeSceneItemKey::Menu(APPLICATION_MENU), 96.0),
            (
                ChromeSceneItemKey::Toolbar {
                    group: TOOLBAR_GROUP,
                    action: stern::core::ActionId::new(CLEAR_SELECTION_ACTION),
                },
                132.0,
            ),
            (ChromeSceneItemKey::Tab(GRAPH_PANEL), 120.0),
            (ChromeSceneItemKey::Status(SELECTION_STATUS), 160.0),
        ]);
        if self.graph_journey {
            chrome_config = chrome_config.with_width(
                ChromeSceneItemKey::Toolbar {
                    group: TOOLBAR_GROUP,
                    action: stern::core::ActionId::new(REVERSE_NODE_ORDER_ACTION),
                },
                168.0,
            );
        }
        let chrome_scene =
            ChromeScene::new(chrome_config, &menu_bar, &toolbar, &tab_strip, &status_bar);
        ui.resolve_pointer_targets(|plan| {
            for (index, &(id, rect)) in app_targets.iter().enumerate() {
                plan.target(PointerTarget::new(
                    id,
                    rect,
                    PointerOrder::new(index as u64 + 1),
                ));
            }
            let mut next = dock_scene.declare_pointer_targets_with_content(
                plan,
                PointerOrder::new(10),
                |plan, mut order| {
                    if let Some(panel) = dock_scene
                        .layout()
                        .frames
                        .iter()
                        .filter_map(|frame| frame.panel.as_ref())
                        .find(|panel| panel.panel == GRAPH_PANEL)
                    {
                        plan.target(PointerTarget::new(GRAPH_ROOT, panel.rect, order));
                        order = PointerOrder::new(order.raw() + 1);
                    }
                    if let Some(viewport) = viewport.as_ref() {
                        order = viewport.declare_pointer_targets(plan, order);
                    }
                    if let Some(viewport_scene) = viewport_scene.as_ref() {
                        order = viewport_scene.declare_pointer_targets(plan, order);
                    }
                    if let (Some(rects), Some(root)) = (tool_rects, viewport_root) {
                        order = declare_tool_actions(plan, order, root, actions, rects);
                    }
                    order
                },
            );
            next = chrome_scene.declare_pointer_targets(plan, next);
            if let Some(overlay) = overlays.scene() {
                overlay.declare_pointer_targets(plan, next);
            }
        })
        .expect("Graph Dock and chrome have unique pointer targets");
        let _ = ui.dock_scene(&dock_scene, |ui, panel| match panel.panel {
            GRAPH_PANEL => self.compose_graph(ui, panel.rect),
            VIEWPORT_PANEL => {
                if let Some(rects) = tool_rects {
                    compose_tool_actions(ui, actions, rects);
                }
                if let Some(viewport) = viewport.as_ref() {
                    let output = ui.viewport_widget(viewport, &mut self.viewport_pan_zoom, &[]);
                    self.viewport_pan_zoom = output.next_pan_zoom;
                }
                if let Some(viewport_scene) = viewport_scene.as_ref() {
                    let _ = ui.viewport_tool_scene(viewport_scene, &mut self.viewport_tools);
                }
            }
            INSPECTOR_PANEL => self.compose_inspector(ui, panel.rect),
            _ => unreachable!("demo Dock contains only Graph, Viewport, and Inspector panels"),
        });
        let chrome_output = ui.chrome_scene(&chrome_scene);
        overlays.reconcile(
            ui,
            actions,
            &mut menu_bar,
            &chrome_output.intents,
            false,
            viewport_size,
        )
    }

    fn sync_chrome_models(&mut self) {
        let selected = u32::try_from(self.selection.selected().len()).unwrap_or(u32::MAX);
        let mut clear_selection = ActionDescriptor::new(CLEAR_SELECTION_ACTION, "Clear selection");
        clear_selection.state.enabled = selected != 0;
        let mut toolbar_actions = vec![clear_selection];
        if self.graph_journey {
            toolbar_actions.push(ActionDescriptor::new(
                REVERSE_NODE_ORDER_ACTION,
                "Reverse node order",
            ));
        }
        self.toolbar.replace_groups([ToolbarGroup::from_actions(
            TOOLBAR_GROUP,
            "Graph selection",
            toolbar_actions,
        )]);
        self.status_bar
            .replace_items([connection_status(self.connection_feedback, selected)]);
    }

    fn compose_graph(&mut self, ui: &mut Ui<'_>, bounds: Rect) {
        let viewport = NodeGraphViewport::new(bounds, self.graph_pan_zoom);
        let view = NodeGraphStaticView::new(GRAPH_ROOT, viewport, &self.graph)
            .with_selection(self.selection.clone());
        let widget = ui
            .prepare_node_graph_widget(NodeGraphWidgetConfig::new(view))
            .expect("deterministic demo graph is valid");
        let output = ui
            .node_graph_widget_with_connections(&widget, &mut self.connection)
            .expect("deterministic graph hit testing is valid");
        for NodeGraphWidgetIntent::Selection(operation) in output.intents {
            self.selection = self.selection.apply(operation);
        }
        for intent in output.connection_intents {
            self.apply_connection_intent(intent);
        }
    }

    fn apply_connection_intent(&mut self, intent: NodeGraphConnectionIntent) {
        match intent {
            NodeGraphConnectionIntent::Begin(_) | NodeGraphConnectionIntent::Preview(_) => {
                self.connection_feedback = GraphConnectionFeedback::Previewing;
            }
            NodeGraphConnectionIntent::Accepted(request) => {
                self.connection_feedback = GraphConnectionFeedback::Accepted {
                    from: request.from.endpoint,
                    to: request.to.endpoint,
                };
            }
            NodeGraphConnectionIntent::Rejected(_) => {
                self.connection_feedback = GraphConnectionFeedback::Rejected;
            }
            NodeGraphConnectionIntent::Commit(request) => self.commit_connection(request),
            NodeGraphConnectionIntent::Cancel(cancel) => {
                self.connection_feedback = GraphConnectionFeedback::Cancelled(cancel.reason);
            }
        }
    }

    fn commit_connection(&mut self, request: NodeGraphCreateLinkRequest) {
        if self
            .graph
            .edges
            .iter()
            .any(|edge| edge.id == COMMITTED_EDGE)
        {
            return;
        }
        self.graph.edges.push(EdgeDescriptor::new(
            COMMITTED_EDGE,
            request.from.endpoint,
            request.to.endpoint,
        ));
        self.connection_feedback = GraphConnectionFeedback::Committed(COMMITTED_EDGE);
    }

    fn compose_inspector(&self, ui: &mut Ui<'_>, bounds: Rect) {
        let selected = match self.selection.active() {
            Some(NodeGraphSelectionTarget::Node(id))
                if self.selection.contains(NodeGraphSelectionTarget::Node(id)) =>
            {
                self.graph.nodes.iter().find(|node| node.id == id)
            }
            _ => None,
        };
        let rows = selected.map_or_else(Vec::new, |_| {
            vec![
                PropertyGridRow::section(INSPECTOR_SECTION, "Selected node"),
                PropertyGridRow::property(INSPECTOR_TITLE, "Title", 0).with_read_only(true),
                PropertyGridRow::property(INSPECTOR_X, "Position X", 0).with_read_only(true),
                PropertyGridRow::property(INSPECTOR_Y, "Position Y", 0).with_read_only(true),
                PropertyGridRow::property(INSPECTOR_PORTS, "Ports", 0).with_read_only(true),
            ]
        });
        let mut values = selected.map_or_else(Vec::new, |node| {
            vec![
                TextEditState::new(node.title.clone()),
                TextEditState::new(node.rect.x.to_string()),
                TextEditState::new(node.rect.y.to_string()),
                TextEditState::new(node.ports.len().to_string()),
            ]
        });
        ui.property_grid(
            "graph.inspector",
            bounds,
            &rows,
            PropertyGridConfig::default(),
            |ui, cell| {
                let index = match cell.row.id {
                    INSPECTOR_TITLE => 0,
                    INSPECTOR_X => 1,
                    INSPECTOR_Y => 2,
                    INSPECTOR_PORTS => 3,
                    _ => unreachable!("property-grid callback skips section rows"),
                };
                ui.text_field_with_access(
                    "value",
                    cell.value_rect,
                    &mut values[index],
                    TextFieldAccess::ReadOnly,
                )
            },
        )
        .expect("deterministic inspector rows have unique identities");
    }
}

fn graph_viewport_scene(
    ui: &Ui<'_>,
    viewport: &ViewportWidget,
    tool: DemoViewportTool,
) -> ViewportToolScene {
    let active = match tool {
        DemoViewportTool::Select => ViewportToolDescriptor::new(SELECT_TOOL, "Select Tool")
            .active(true)
            .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Pointer)),
        DemoViewportTool::Transform => {
            ViewportToolDescriptor::new(TRANSFORM_TOOL, "Transform Tool")
                .active(true)
                .with_cursor(ViewportCursorMetadata::new(ViewportCursorShape::Move))
        }
    };
    let bounds = viewport.surface().bounds;
    let target = ViewportSelectionTargetDescriptor::new(
        VIEWPORT_TARGET,
        Rect::new(
            bounds.x + bounds.width * 0.2,
            bounds.y + bounds.height * 0.2,
            bounds.width * 0.6,
            bounds.height * 0.6,
        ),
    )
    .with_label("Graph preview selection")
    .with_handles(ViewportTransformHandleSet::move_only());
    ui.prepare_viewport_tool_scene(
        viewport,
        ViewportToolSceneConfig::new([target])
            .with_active_tool(active)
            .disabled(tool == DemoViewportTool::Select),
    )
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

fn connection_status(feedback: GraphConnectionFeedback, selected: u32) -> StatusItem {
    let (label, message, kind) = match feedback {
        GraphConnectionFeedback::Ready => (
            "Connection",
            format!("{selected} selected"),
            StatusItemKind::Ready,
        ),
        GraphConnectionFeedback::Previewing => (
            "Connection",
            "Previewing typed connection".to_owned(),
            StatusItemKind::Progress,
        ),
        GraphConnectionFeedback::Accepted { from, to } => (
            "Connection",
            format!(
                "Accepted {}:{} -> {}:{}",
                from.node.raw(),
                from.port.raw(),
                to.node.raw(),
                to.port.raw()
            ),
            StatusItemKind::Progress,
        ),
        GraphConnectionFeedback::Rejected => (
            "Connection",
            "Incompatible connection rejected".to_owned(),
            StatusItemKind::Error,
        ),
        GraphConnectionFeedback::Committed(edge) => (
            "Connection",
            format!("Connection committed as edge {}", edge.raw()),
            StatusItemKind::Ready,
        ),
        GraphConnectionFeedback::Cancelled(reason) => (
            "Connection",
            format!("Connection cancelled: {reason:?}"),
            StatusItemKind::Message,
        ),
    };
    StatusItem::new(SELECTION_STATUS, label, message, kind).with_count(selected)
}

fn chrome_layout(bounds: Rect) -> [Rect; 5] {
    let menu_height = 28.0_f32.min(bounds.height);
    let remaining = (bounds.height - menu_height).max(0.0);
    let toolbar_height = 28.0_f32.min(bounds.height);
    let remaining = (remaining - toolbar_height).max(0.0);
    let tab_height = 28.0_f32.min(remaining);
    let remaining = (remaining - tab_height).max(0.0);
    let status_height = 28.0_f32.min(remaining);
    let dock_height = (remaining - status_height).max(0.0);
    [
        Rect::new(bounds.x, bounds.y, bounds.width, menu_height),
        Rect::new(
            bounds.x,
            bounds.y + menu_height,
            bounds.width,
            toolbar_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + menu_height + toolbar_height,
            bounds.width,
            tab_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + menu_height + toolbar_height + tab_height,
            bounds.width,
            dock_height,
        ),
        Rect::new(
            bounds.x,
            bounds.max_y() - status_height,
            bounds.width,
            status_height,
        ),
    ]
}

impl Default for GraphWorkspaceState {
    fn default() -> Self {
        Self::new()
    }
}
