use stern::core::{Rect, WidgetId};
use stern::widgets::Ui;
use stern::widgets::node_graph::{
    EdgeDescriptor, EdgeId, GraphRect, NodeDescriptor, NodeGraphDescriptor, NodeGraphPanZoom,
    NodeGraphSelection, NodeGraphStaticView, NodeGraphViewport, NodeGraphWidgetConfig,
    NodeGraphWidgetIntent, NodeId, PortDescriptor, PortDirection, PortEndpoint, PortId, PortTypeId,
};

const GRAPH_ROOT: WidgetId = WidgetId::from_raw(0x0047_5241_5048);
const SOURCE_NODE: NodeId = NodeId::from_raw(1);
const OUTPUT_NODE: NodeId = NodeId::from_raw(2);
const IMAGE_OUTPUT: PortId = PortId::from_raw(1);
const IMAGE_INPUT: PortId = PortId::from_raw(1);
const IMAGE_TYPE: PortTypeId = PortTypeId::from_raw(1);

/// Application-owned deterministic fixture and selection for the Graph workspace.
#[derive(Debug, Clone, PartialEq)]
pub struct GraphWorkspaceState {
    graph: NodeGraphDescriptor,
    selection: NodeGraphSelection,
}

impl GraphWorkspaceState {
    /// Creates the deterministic two-node, one-edge graph fixture.
    #[must_use]
    pub fn new() -> Self {
        let source_output =
            PortDescriptor::new(IMAGE_OUTPUT, PortDirection::Output, "Image", IMAGE_TYPE);
        let output_input =
            PortDescriptor::new(IMAGE_INPUT, PortDirection::Input, "Image", IMAGE_TYPE);
        let source = NodeDescriptor::new(
            SOURCE_NODE,
            "Image Source",
            GraphRect::new(36.0, 28.0, 156.0, 96.0),
        )
        .with_ports(vec![source_output]);
        let output = NodeDescriptor::new(
            OUTPUT_NODE,
            "Viewer",
            GraphRect::new(360.0, 88.0, 156.0, 96.0),
        )
        .with_ports(vec![output_input]);
        let edge = EdgeDescriptor::new(
            EdgeId::from_raw(1),
            PortEndpoint::new(SOURCE_NODE, IMAGE_OUTPUT),
            PortEndpoint::new(OUTPUT_NODE, IMAGE_INPUT),
        );
        let mut graph = NodeGraphDescriptor::new();
        graph.nodes = vec![source, output];
        graph.edges = vec![edge];
        Self {
            graph,
            selection: NodeGraphSelection::new(),
        }
    }

    /// Returns the caller-owned graph selection.
    #[must_use]
    pub const fn selection(&self) -> &NodeGraphSelection {
        &self.selection
    }

    /// Returns the stable Graph workspace root identity.
    #[must_use]
    pub const fn root_id(&self) -> WidgetId {
        GRAPH_ROOT
    }

    pub(crate) fn compose(&mut self, ui: &mut Ui<'_>, bounds: Rect) {
        let viewport = NodeGraphViewport::new(bounds, NodeGraphPanZoom::default());
        let view = NodeGraphStaticView::new(GRAPH_ROOT, viewport, &self.graph)
            .with_selection(self.selection.clone());
        let widget = ui
            .prepare_node_graph_widget(NodeGraphWidgetConfig::new(view))
            .expect("deterministic demo graph is valid");
        let output = ui
            .node_graph_widget(&widget)
            .expect("deterministic graph hit testing is valid");
        for NodeGraphWidgetIntent::Selection(operation) in output.intents {
            self.selection = self.selection.apply(operation);
        }
    }
}

impl Default for GraphWorkspaceState {
    fn default() -> Self {
        Self::new()
    }
}
