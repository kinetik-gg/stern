use stern::core::{
    ActionContext, ActionDescriptor, ActionInvocation, Axis, PointerOrder, PointerTarget, Rect,
    WidgetId,
};
use stern::text::TextEditState;
use stern::widgets::chrome::{
    ChromeScene, ChromeSceneConfig, ChromeSceneItemKey, MenuBar, StatusBar, StatusItem,
    StatusItemId, StatusItemKind, TabStrip, Toolbar, ToolbarGroup, ToolbarGroupId,
};
use stern::widgets::dock::{
    Dock, DockNode, DockScene, DockSceneConfig, Frame, FrameId, FrameTab, Panel, PanelId,
};
use stern::widgets::inspector::{PropertyGridConfig, PropertyGridRow};
use stern::widgets::node_graph::{
    EdgeDescriptor, EdgeId, GraphRect, NodeDescriptor, NodeGraphConnectionCancelReason,
    NodeGraphConnectionController, NodeGraphConnectionIntent, NodeGraphCreateLinkRequest,
    NodeGraphDescriptor, NodeGraphPanZoom, NodeGraphSelection, NodeGraphSelectionTarget,
    NodeGraphStaticView, NodeGraphViewport, NodeGraphWidgetConfig, NodeGraphWidgetIntent, NodeId,
    PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
};
use stern::widgets::{ItemId, TextFieldAccess, Ui};

const GRAPH_ROOT: WidgetId = WidgetId::from_raw(0x0047_5241_5048);
const CHROME_ROOT: WidgetId = WidgetId::from_raw(0x4348_524f_4d45);
const CLEAR_SELECTION_ACTION: &str = "graph.clear-selection";
const TOOLBAR_GROUP: ToolbarGroupId = ToolbarGroupId::from_raw(1);
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
const INSPECTOR_FRAME: FrameId = FrameId::from_raw(2);
const GRAPH_PANEL: PanelId = PanelId::from_raw(1);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(2);
const INSPECTOR_SECTION: ItemId = ItemId::from_raw(1);
const INSPECTOR_TITLE: ItemId = ItemId::from_raw(2);
const INSPECTOR_X: ItemId = ItemId::from_raw(3);
const INSPECTOR_Y: ItemId = ItemId::from_raw(4);
const INSPECTOR_PORTS: ItemId = ItemId::from_raw(5);

/// Application-visible outcome of the latest public graph connection lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphConnectionFeedback {
    /// No connection edit has run yet.
    Ready,
    /// Stern resolved a typed preview candidate.
    Previewing,
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
    connection: NodeGraphConnectionController,
    connection_feedback: GraphConnectionFeedback,
    menu_bar: MenuBar,
    toolbar: Toolbar,
    tab_strip: TabStrip,
    status_bar: StatusBar,
}

impl GraphWorkspaceState {
    /// Creates the deterministic app-owned graph fixture.
    #[must_use]
    pub fn new() -> Self {
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
        let dock = Dock::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 2.0 / 3.0,
            min_first: 260.0,
            min_second: 180.0,
            first: Box::new(DockNode::Frame(Frame::new(
                GRAPH_FRAME,
                vec![Panel::new(GRAPH_PANEL, "Graph")],
            ))),
            second: Box::new(DockNode::Frame(Frame::new(
                INSPECTOR_FRAME,
                vec![Panel::new(INSPECTOR_PANEL, "Inspector")],
            ))),
        });
        let mut clear_selection = ActionDescriptor::new(CLEAR_SELECTION_ACTION, "Clear selection");
        clear_selection.state.enabled = false;
        Self {
            dock,
            graph,
            selection: NodeGraphSelection::new(),
            connection: NodeGraphConnectionController::default(),
            connection_feedback: GraphConnectionFeedback::Ready,
            menu_bar: MenuBar::new(),
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

    /// Handles the one application-owned action exposed by the Graph workspace.
    pub fn handle_action(&mut self, invocation: &ActionInvocation) -> bool {
        if invocation.action_id.as_str() != CLEAR_SELECTION_ACTION || self.selection.is_empty() {
            return false;
        }
        self.selection = NodeGraphSelection::new();
        true
    }

    pub(crate) fn compose(
        &mut self,
        ui: &mut Ui<'_>,
        bounds: Rect,
        app_targets: &[(WidgetId, Rect)],
    ) {
        self.sync_chrome_models();
        let [toolbar_rect, tab_strip_rect, dock_rect, status_bar_rect] = chrome_layout(bounds);
        let dock = self.dock.clone();
        let menu_bar = self.menu_bar.clone();
        let toolbar = self.toolbar.clone();
        let tab_strip = self.tab_strip.clone();
        let status_bar = self.status_bar.clone();
        let dock_scene = DockScene::new(DockSceneConfig::new(DOCK_ROOT, dock_rect), &dock);
        let chrome_scene = ChromeScene::new(
            ChromeSceneConfig::new(
                CHROME_ROOT,
                Rect::ZERO,
                toolbar_rect,
                tab_strip_rect,
                status_bar_rect,
                ActionContext::Editor,
            )
            .with_widths([
                (
                    ChromeSceneItemKey::Toolbar {
                        group: TOOLBAR_GROUP,
                        action: stern::core::ActionId::new(CLEAR_SELECTION_ACTION),
                    },
                    132.0,
                ),
                (ChromeSceneItemKey::Tab(GRAPH_PANEL), 120.0),
                (ChromeSceneItemKey::Status(SELECTION_STATUS), 160.0),
            ]),
            &menu_bar,
            &toolbar,
            &tab_strip,
            &status_bar,
        );
        ui.resolve_pointer_targets(|plan| {
            for (index, &(id, rect)) in app_targets.iter().enumerate() {
                plan.target(PointerTarget::new(
                    id,
                    rect,
                    PointerOrder::new(index as u64 + 1),
                ));
            }
            let next = dock_scene.declare_pointer_targets_with_content(
                plan,
                PointerOrder::new(10),
                |plan, order| {
                    let Some(panel) = dock_scene
                        .layout()
                        .frames
                        .iter()
                        .filter_map(|frame| frame.panel.as_ref())
                        .find(|panel| panel.panel == GRAPH_PANEL)
                    else {
                        return order;
                    };
                    plan.target(PointerTarget::new(GRAPH_ROOT, panel.rect, order));
                    PointerOrder::new(order.raw() + 1)
                },
            );
            chrome_scene.declare_pointer_targets(plan, next);
        })
        .expect("Graph Dock and chrome have unique pointer targets");
        let _ = ui.dock_scene(&dock_scene, |ui, panel| match panel.panel {
            GRAPH_PANEL => self.compose_graph(ui, panel.rect),
            INSPECTOR_PANEL => self.compose_inspector(ui, panel.rect),
            _ => unreachable!("demo Dock contains only Graph and Inspector panels"),
        });
        let _ = ui.chrome_scene(&chrome_scene);
    }

    fn sync_chrome_models(&mut self) {
        let selected = u32::try_from(self.selection.selected().len()).unwrap_or(u32::MAX);
        let mut clear_selection = ActionDescriptor::new(CLEAR_SELECTION_ACTION, "Clear selection");
        clear_selection.state.enabled = selected != 0;
        self.toolbar.replace_groups([ToolbarGroup::from_actions(
            TOOLBAR_GROUP,
            "Graph selection",
            [clear_selection],
        )]);
        self.status_bar
            .replace_items([connection_status(self.connection_feedback, selected)]);
    }

    fn compose_graph(&mut self, ui: &mut Ui<'_>, bounds: Rect) {
        let viewport = NodeGraphViewport::new(bounds, NodeGraphPanZoom::default());
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
            NodeGraphConnectionIntent::Accepted(_) => {}
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

fn chrome_layout(bounds: Rect) -> [Rect; 4] {
    let toolbar_height = 28.0_f32.min(bounds.height);
    let remaining = (bounds.height - toolbar_height).max(0.0);
    let tab_height = 28.0_f32.min(remaining);
    let remaining = (remaining - tab_height).max(0.0);
    let status_height = 28.0_f32.min(remaining);
    let dock_height = (remaining - status_height).max(0.0);
    [
        Rect::new(bounds.x, bounds.y, bounds.width, toolbar_height),
        Rect::new(
            bounds.x,
            bounds.y + toolbar_height,
            bounds.width,
            tab_height,
        ),
        Rect::new(
            bounds.x,
            bounds.y + toolbar_height + tab_height,
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
