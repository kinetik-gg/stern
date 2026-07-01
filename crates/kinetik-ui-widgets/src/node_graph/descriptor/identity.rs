pub(crate) const DEFAULT_ZOOM: f32 = 1.0;
pub(crate) const MIN_ZOOM: f32 = 0.01;
pub(crate) const NODE_GRAPH_EDGE_HIT_BOUNDARY_MARGIN: f32 = 0.001;
/// Default screen-space tolerance for node graph edge hit testing.
pub const DEFAULT_NODE_GRAPH_EDGE_HIT_TOLERANCE: f32 = 6.0;
/// Default screen-space square size for node graph port hit testing.
pub const DEFAULT_NODE_GRAPH_PORT_HIT_SIZE: f32 = 8.0;
/// Default screen-space square size for node graph reroute hit testing.
pub const DEFAULT_NODE_GRAPH_REROUTE_HIT_SIZE: f32 = 10.0;
/// Default graph-space height for node title hit testing.
pub const DEFAULT_NODE_GRAPH_TITLE_BAR_HEIGHT: f32 = 24.0;

macro_rules! node_graph_id {
    ($name:ident, $doc:literal) => {
        #[doc = $doc]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(u64);

        impl $name {
            /// Creates an ID from raw bits.
            #[must_use]
            pub const fn from_raw(raw: u64) -> Self {
                Self(raw)
            }

            /// Returns raw ID bits.
            #[must_use]
            pub const fn raw(self) -> u64 {
                self.0
            }
        }
    };
}

node_graph_id!(NodeId, "Stable node identity.");
node_graph_id!(PortId, "Stable node port identity.");
node_graph_id!(EdgeId, "Stable node graph edge identity.");
node_graph_id!(RerouteId, "Stable node graph reroute identity.");
node_graph_id!(NodeFrameId, "Stable identity for a node frame surface.");
node_graph_id!(NodeGroupId, "Stable identity for a node group.");
node_graph_id!(
    NodeGraphAddNodeDescriptorId,
    "Stable application-owned add-node descriptor identity."
);
node_graph_id!(
    PortTypeId,
    "Application-owned node port compatibility key identity."
);

/// Port flow direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortDirection {
    /// The port consumes values or connections.
    Input,
    /// The port produces values or connections.
    Output,
}

/// Stable address for one port scoped by its owning node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PortEndpoint {
    /// Owning node.
    pub node: NodeId,
    /// Port on the owning node.
    pub port: PortId,
}

impl PortEndpoint {
    /// Creates a port endpoint.
    #[must_use]
    pub const fn new(node: NodeId, port: PortId) -> Self {
        Self { node, port }
    }
}
