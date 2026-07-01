#[allow(clippy::wildcard_imports)]
use super::*;

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

/// Data-only port descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortDescriptor {
    /// Stable port identity, scoped by the owning node.
    pub id: PortId,
    /// Directed port flow.
    pub direction: PortDirection,
    /// User-facing port label.
    pub label: String,
    /// Application-owned compatibility key.
    pub port_type: PortTypeId,
    /// Whether the port is currently available.
    pub enabled: bool,
}

impl PortDescriptor {
    /// Creates an enabled port descriptor.
    #[must_use]
    pub fn new(
        id: PortId,
        direction: PortDirection,
        label: impl Into<String>,
        port_type: PortTypeId,
    ) -> Self {
        Self {
            id,
            direction,
            label: label.into(),
            port_type,
            enabled: true,
        }
    }

    /// Sets whether the port is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only node descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeDescriptor {
    /// Stable node identity.
    pub id: NodeId,
    /// User-facing node title.
    pub title: String,
    /// Node bounds in graph space.
    pub rect: GraphRect,
    /// Ports exposed by this node.
    pub ports: Vec<PortDescriptor>,
    /// Optional frame containing this node.
    pub frame: Option<NodeFrameId>,
    /// Optional group containing this node.
    pub group: Option<NodeGroupId>,
    /// Whether the node is presented as muted by the application.
    pub muted: bool,
    /// Whether the node is presented as bypassed by the application.
    pub bypassed: bool,
    /// Optional user-facing secondary label metadata.
    pub label: Option<String>,
    /// Optional user-facing comment metadata.
    pub comment: Option<String>,
    /// Whether the node is currently available.
    pub enabled: bool,
}

impl NodeDescriptor {
    /// Creates an enabled node descriptor with no frame, group, or ports.
    #[must_use]
    pub fn new(id: NodeId, title: impl Into<String>, rect: GraphRect) -> Self {
        Self {
            id,
            title: title.into(),
            rect,
            ports: Vec::new(),
            frame: None,
            group: None,
            muted: false,
            bypassed: false,
            label: None,
            comment: None,
            enabled: true,
        }
    }

    /// Sets ports.
    #[must_use]
    pub fn with_ports(mut self, ports: impl Into<Vec<PortDescriptor>>) -> Self {
        self.ports = ports.into();
        self
    }

    /// Sets the containing frame.
    #[must_use]
    pub const fn with_frame(mut self, frame: NodeFrameId) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Sets the containing group.
    #[must_use]
    pub const fn with_group(mut self, group: NodeGroupId) -> Self {
        self.group = Some(group);
        self
    }

    /// Sets muted presentation metadata.
    #[must_use]
    pub const fn with_muted(mut self, muted: bool) -> Self {
        self.muted = muted;
        self
    }

    /// Sets bypassed presentation metadata.
    #[must_use]
    pub const fn with_bypassed(mut self, bypassed: bool) -> Self {
        self.bypassed = bypassed;
        self
    }

    /// Sets optional user-facing secondary label metadata.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets optional user-facing comment metadata.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Sets whether the node is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only edge route point descriptor.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeGraphEdgeRoutePoint {
    /// Literal graph-space route point.
    Point(GraphPoint),
    /// Route point resolved from a reroute descriptor.
    Reroute(RerouteId),
}

impl NodeGraphEdgeRoutePoint {
    /// Creates a literal graph-space route point.
    #[must_use]
    pub const fn point(position: GraphPoint) -> Self {
        Self::Point(position)
    }

    /// Creates a route point that follows a reroute descriptor.
    #[must_use]
    pub const fn reroute(reroute: RerouteId) -> Self {
        Self::Reroute(reroute)
    }
}

/// Data-only edge descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct EdgeDescriptor {
    /// Stable edge identity.
    pub id: EdgeId,
    /// Output endpoint.
    pub from: PortEndpoint,
    /// Input endpoint.
    pub to: PortEndpoint,
    /// Ordered intermediate graph-space route points.
    pub route_points: Vec<NodeGraphEdgeRoutePoint>,
    /// Whether the edge is currently available.
    pub enabled: bool,
}

impl EdgeDescriptor {
    /// Creates an enabled edge descriptor.
    #[must_use]
    pub const fn new(id: EdgeId, from: PortEndpoint, to: PortEndpoint) -> Self {
        Self {
            id,
            from,
            to,
            route_points: Vec::new(),
            enabled: true,
        }
    }

    /// Sets ordered intermediate route points.
    #[must_use]
    pub fn with_route_points(
        mut self,
        route_points: impl Into<Vec<NodeGraphEdgeRoutePoint>>,
    ) -> Self {
        self.route_points = route_points.into();
        self
    }

    /// Sets whether the edge is currently available.
    #[must_use]
    pub fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Source or target side of an edge descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeEndpointRole {
    /// The edge source endpoint.
    Source,
    /// The edge target endpoint.
    Target,
}

/// Resolved node graph endpoint with descriptor references and anchor geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedEndpoint<'a> {
    /// Source or target role for this resolved endpoint.
    pub role: EdgeEndpointRole,
    /// Stable endpoint address from the edge descriptor.
    pub endpoint: PortEndpoint,
    /// Owning node descriptor.
    pub node: &'a NodeDescriptor,
    /// Port descriptor.
    pub port: &'a PortDescriptor,
    /// Graph-space anchor for later backend-independent edge drawing.
    pub anchor: GraphPoint,
}

/// Resolved edge route point metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedEdgeRoutePoint<'a> {
    /// Descriptor route point that produced this resolved point.
    pub route_point: NodeGraphEdgeRoutePoint,
    /// Resolved graph-space position.
    pub position: GraphPoint,
    /// Reroute descriptor, when this route point follows one.
    pub reroute: Option<&'a RerouteDescriptor>,
}

/// Resolved edge with source, target, and route descriptor references.
#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedEdge<'a> {
    /// Edge descriptor.
    pub edge: &'a EdgeDescriptor,
    /// Resolved output endpoint.
    pub from: ResolvedEndpoint<'a>,
    /// Ordered resolved intermediate route points.
    pub route_points: Vec<ResolvedEdgeRoutePoint<'a>>,
    /// Resolved input endpoint.
    pub to: ResolvedEndpoint<'a>,
}

/// Structured edge endpoint resolution failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EdgeResolutionError {
    /// The graph contains a duplicate edge ID.
    DuplicateEdgeId {
        /// Duplicated edge ID.
        edge: EdgeId,
    },
    /// An edge endpoint references a missing node.
    MissingNode {
        /// Edge being resolved.
        edge: EdgeId,
        /// Source or target endpoint role.
        endpoint: EdgeEndpointRole,
        /// Missing node ID.
        node: NodeId,
    },
    /// An edge endpoint references a missing port on an existing node.
    MissingPort {
        /// Edge being resolved.
        edge: EdgeId,
        /// Source or target endpoint role.
        endpoint: EdgeEndpointRole,
        /// Existing node ID.
        node: NodeId,
        /// Missing port ID.
        port: PortId,
    },
    /// An endpoint exists but has the wrong directed flow for its edge side.
    WrongDirection {
        /// Edge being resolved.
        edge: EdgeId,
        /// Source or target endpoint role.
        endpoint: EdgeEndpointRole,
        /// Owning node ID.
        node: NodeId,
        /// Port ID.
        port: PortId,
        /// Required port direction.
        expected: PortDirection,
        /// Actual port direction.
        actual: PortDirection,
    },
    /// An endpoint exists but its port is disabled.
    DisabledPort {
        /// Edge being resolved.
        edge: EdgeId,
        /// Source or target endpoint role.
        endpoint: EdgeEndpointRole,
        /// Owning node ID.
        node: NodeId,
        /// Port ID.
        port: PortId,
    },
    /// Resolved output and input ports use different compatibility keys.
    IncompatiblePortType {
        /// Edge being resolved.
        edge: EdgeId,
        /// Source endpoint address.
        from: PortEndpoint,
        /// Target endpoint address.
        to: PortEndpoint,
        /// Source port compatibility key.
        output: PortTypeId,
        /// Target port compatibility key.
        input: PortTypeId,
    },
    /// An edge route references a missing reroute descriptor.
    MissingReroute {
        /// Edge being resolved.
        edge: EdgeId,
        /// Missing reroute ID.
        reroute: RerouteId,
    },
}

/// Data-only reroute descriptor.
#[derive(Debug, Clone, PartialEq)]
pub struct RerouteDescriptor {
    /// Stable reroute identity.
    pub id: RerouteId,
    /// User-facing reroute label.
    pub label: String,
    /// Reroute position in graph space.
    pub position: GraphPoint,
    /// Whether the reroute is currently available.
    pub enabled: bool,
}

impl RerouteDescriptor {
    /// Creates an enabled reroute descriptor.
    #[must_use]
    pub fn new(id: RerouteId, label: impl Into<String>, position: GraphPoint) -> Self {
        Self {
            id,
            label: label.into(),
            position,
            enabled: true,
        }
    }

    /// Sets the reroute graph-space position.
    #[must_use]
    pub const fn with_position(mut self, position: GraphPoint) -> Self {
        self.position = position;
        self
    }

    /// Sets whether the reroute is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only frame descriptor for node graph surfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeFrameDescriptor {
    /// Stable frame identity.
    pub id: NodeFrameId,
    /// User-facing frame title.
    pub title: String,
    /// Frame bounds in graph space.
    pub rect: GraphRect,
    /// Whether the frame is presented as collapsed by the application.
    pub collapsed: bool,
    /// Optional user-facing secondary label metadata.
    pub label: Option<String>,
    /// Optional user-facing comment metadata.
    pub comment: Option<String>,
    /// Whether the frame is currently available.
    pub enabled: bool,
}

impl NodeFrameDescriptor {
    /// Creates an enabled frame descriptor.
    #[must_use]
    pub fn new(id: NodeFrameId, title: impl Into<String>, rect: GraphRect) -> Self {
        Self {
            id,
            title: title.into(),
            rect,
            collapsed: false,
            label: None,
            comment: None,
            enabled: true,
        }
    }

    /// Sets collapsed presentation metadata.
    #[must_use]
    pub const fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Sets optional user-facing secondary label metadata.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets optional user-facing comment metadata.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Sets whether the frame is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only group descriptor for node graph surfaces.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGroupDescriptor {
    /// Stable group identity.
    pub id: NodeGroupId,
    /// User-facing group title.
    pub title: String,
    /// Group bounds in graph space.
    pub rect: GraphRect,
    /// Nodes contained by this group.
    pub nodes: Vec<NodeId>,
    /// Whether the group is presented as collapsed by the application.
    pub collapsed: bool,
    /// Optional user-facing secondary label metadata.
    pub label: Option<String>,
    /// Optional user-facing comment metadata.
    pub comment: Option<String>,
    /// Whether the group is currently available.
    pub enabled: bool,
}

impl NodeGroupDescriptor {
    /// Creates an enabled group descriptor with no contained nodes.
    #[must_use]
    pub fn new(id: NodeGroupId, title: impl Into<String>, rect: GraphRect) -> Self {
        Self {
            id,
            title: title.into(),
            rect,
            nodes: Vec::new(),
            collapsed: false,
            label: None,
            comment: None,
            enabled: true,
        }
    }

    /// Sets contained nodes.
    #[must_use]
    pub fn with_nodes(mut self, nodes: impl Into<Vec<NodeId>>) -> Self {
        self.nodes = nodes.into();
        self
    }

    /// Sets collapsed presentation metadata.
    #[must_use]
    pub const fn with_collapsed(mut self, collapsed: bool) -> Self {
        self.collapsed = collapsed;
        self
    }

    /// Sets optional user-facing secondary label metadata.
    #[must_use]
    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    /// Sets optional user-facing comment metadata.
    #[must_use]
    pub fn with_comment(mut self, comment: impl Into<String>) -> Self {
        self.comment = Some(comment.into());
        self
    }

    /// Sets whether the group is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only node graph descriptor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeGraphDescriptor {
    /// Nodes.
    pub nodes: Vec<NodeDescriptor>,
    /// Edges.
    pub edges: Vec<EdgeDescriptor>,
    /// Reroutes.
    pub reroutes: Vec<RerouteDescriptor>,
    /// Frames.
    pub frames: Vec<NodeFrameDescriptor>,
    /// Groups.
    pub groups: Vec<NodeGroupDescriptor>,
}

impl NodeGraphDescriptor {
    /// Creates an empty graph descriptor.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            nodes: Vec::new(),
            edges: Vec::new(),
            reroutes: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Validates deterministic descriptor invariants.
    ///
    /// # Errors
    ///
    /// Returns a structured validation error when node, frame, or group IDs are
    /// duplicated, or a node contains duplicate port IDs.
    pub fn validate(&self) -> Result<(), NodeGraphValidationError> {
        validate_node_graph_descriptors(&self.nodes)?;
        validate_node_graph_reroute_descriptors(&self.reroutes)?;
        validate_node_graph_frame_descriptors(&self.frames)?;
        validate_node_graph_group_descriptors(&self.groups)?;
        validate_node_graph_memberships(self)
    }

    /// Resolves edge endpoints against node and port descriptors.
    ///
    /// # Errors
    ///
    /// Returns a structured resolution error for duplicate edge IDs, missing
    /// nodes or ports, wrong endpoint directions, disabled ports, or
    /// incompatible port types.
    pub fn resolve_edges(&self) -> Result<Vec<ResolvedEdge<'_>>, EdgeResolutionError> {
        resolve_node_graph_edges(self)
    }

    /// Resolves one UI logical screen-space point to a stable typed hit target.
    ///
    /// Disabled targets are intentionally skipped. Invalid descriptors return a
    /// structured error before any target is guessed.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test error when descriptor validation or edge
    /// endpoint resolution fails.
    pub fn hit_test(
        &self,
        viewport: NodeGraphViewport,
        point: Point,
    ) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
        hit_test_node_graph(viewport, self, point)
    }

    /// Resolves one UI logical screen-space point with explicit hit geometry.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test error when descriptor validation or edge
    /// endpoint resolution fails.
    pub fn hit_test_with_config(
        &self,
        viewport: NodeGraphViewport,
        point: Point,
        config: NodeGraphHitTestConfig,
    ) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
        hit_test_node_graph_with_config(viewport, self, point, config)
    }

    /// Starts application-owned link draft metadata from an enabled port endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured endpoint error when descriptors are invalid, the
    /// endpoint is stale, or the endpoint is disabled.
    pub fn start_link_draft(
        &self,
        start: PortEndpoint,
        current_pointer: Point,
    ) -> Result<NodeGraphLinkDraft, NodeGraphLinkDraftEndpointError> {
        NodeGraphLinkDraft::start(self, start, current_pointer)
    }

    /// Creates application-owned metadata for a new link request.
    ///
    /// # Errors
    ///
    /// Returns a structured error when either endpoint cannot be resolved or
    /// the endpoints are not a compatible output-to-input pair.
    pub fn create_link_request(
        &self,
        from: PortEndpoint,
        to: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::create_link(self, from, to)
    }

    /// Creates application-owned metadata for reconnecting an edge source.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current target.
    pub fn reconnect_link_source_request(
        &self,
        edge: EdgeId,
        new_source: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::reconnect_source(self, edge, new_source)
    }

    /// Creates application-owned metadata for reconnecting an edge target.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current source.
    pub fn reconnect_link_target_request(
        &self,
        edge: EdgeId,
        new_target: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::reconnect_target(self, edge, new_target)
    }

    /// Creates application-owned metadata for detaching one edge endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn detach_link_endpoint_request(
        &self,
        edge: EdgeId,
        endpoint: EdgeEndpointRole,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::detach_edge(self, edge, endpoint)
    }

    /// Creates application-owned metadata for cutting an edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn cut_link_request(
        &self,
        edge: EdgeId,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::cut_edge(self, edge)
    }

    /// Resolves context action metadata from a raw hit-test target.
    #[must_use]
    pub fn context_actions_from_hit(
        &self,
        hit: NodeGraphHitTarget,
        selection: &NodeGraphSelection,
    ) -> Vec<NodeGraphContextAction> {
        self.context_actions(NodeGraphContextTarget::from_hit_target(hit), selection)
    }

    /// Resolves deterministic app-owned context action metadata.
    #[must_use]
    pub fn context_actions(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Vec<NodeGraphContextAction> {
        resolve_node_graph_context_actions(self, target, selection)
    }

    /// Resolves one context action by kind without materializing the default catalog.
    ///
    /// This path is intended for applications that present a subset or custom
    /// ordering of node graph context actions while still reusing Kinetik's
    /// typed request metadata.
    #[must_use]
    pub fn context_action(
        &self,
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> NodeGraphContextAction {
        node_graph_context_action(self, kind, target, selection)
    }

    /// Resolves one context action from a raw hit-test target.
    #[must_use]
    pub fn context_action_from_hit(
        &self,
        kind: NodeGraphContextActionKind,
        hit: NodeGraphHitTarget,
        selection: &NodeGraphSelection,
    ) -> NodeGraphContextAction {
        self.context_action(
            kind,
            NodeGraphContextTarget::from_hit_target(hit),
            selection,
        )
    }

    /// Creates delete request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target or selection
    /// cannot produce delete request metadata.
    pub fn delete_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextSelectionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_selection_request(self, target, selection)
    }

    /// Creates duplicate request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target or selection
    /// cannot produce duplicate request metadata.
    pub fn duplicate_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextSelectionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_selection_request(self, target, selection)
    }

    /// Creates disconnect request metadata for an edge or port context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the context target is
    /// unsupported, missing, disabled, or has no connected edges.
    pub fn disconnect_context_request(
        &self,
        target: NodeGraphContextTarget,
    ) -> Result<NodeGraphContextDisconnectRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_disconnect_context_request(self, target)
    }

    /// Creates detach-endpoint request metadata for an edge context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the context target is
    /// unsupported, missing, or disabled.
    pub fn detach_context_request(
        &self,
        target: NodeGraphContextTarget,
        endpoint: EdgeEndpointRole,
    ) -> Result<NodeGraphContextDetachEndpointRequest, NodeGraphContextActionUnavailableReason>
    {
        node_graph_detach_context_request(self, target, endpoint)
    }

    /// Creates organization request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the operation is not
    /// valid for the current target or selection.
    pub fn organization_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
        operation: NodeGraphContextOrganizationOperation,
    ) -> Result<NodeGraphContextOrganizationRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_organization_context_request(self, target, selection, operation)
    }

    /// Creates select-all request metadata for the canvas context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target is not the
    /// canvas or the graph has no selectable targets.
    pub fn select_all_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_select_all_context_request(self, target, selection)
    }

    /// Creates paste request metadata for the canvas context target.
    ///
    /// The default compatibility catalog keeps paste disabled until the
    /// application provides clipboard state. Applications with that state can
    /// use this builder to present a custom enabled paste action without
    /// duplicating target and selection metadata.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target is not the
    /// canvas.
    pub fn paste_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_paste_context_request(target, selection)
    }

    /// Creates application-owned request metadata for a context action kind.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the requested action
    /// kind does not apply to the current target or selection.
    pub fn context_action_request(
        &self,
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextActionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_action_request(self, kind, target, selection)
    }

    /// Returns frame member node IDs in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing.
    pub fn frame_member_nodes(
        &self,
        frame: NodeFrameId,
    ) -> Result<Vec<NodeId>, NodeGraphOrganizationRequestError> {
        self.validate()?;
        resolve_node_graph_frame(self, frame)?;
        Ok(frame_member_nodes(self, frame))
    }

    /// Returns group member node IDs in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the group target is missing.
    pub fn group_member_nodes(
        &self,
        group: NodeGroupId,
    ) -> Result<Vec<NodeId>, NodeGraphOrganizationRequestError> {
        self.validate()?;
        resolve_node_graph_group(self, group)?;
        Ok(group_member_nodes(self, group))
    }

    /// Creates application-owned metadata for moving a parent frame and its children.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn move_frame_request(
        &self,
        viewport: NodeGraphViewport,
        frame: NodeFrameId,
        screen_delta: GraphVector,
    ) -> Result<NodeGraphFrameMoveRequest, NodeGraphOrganizationRequestError> {
        NodeGraphFrameMoveRequest::new(self, viewport, frame, screen_delta)
    }

    /// Creates application-owned collapse metadata for a frame.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn collapse_frame_request(
        &self,
        frame: NodeFrameId,
        collapsed: bool,
    ) -> Result<NodeGraphCollapseRequest, NodeGraphOrganizationRequestError> {
        NodeGraphCollapseRequest::frame(self, frame, collapsed)
    }

    /// Creates application-owned collapse metadata for a group.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the group target is missing or disabled.
    pub fn collapse_group_request(
        &self,
        group: NodeGroupId,
        collapsed: bool,
    ) -> Result<NodeGraphCollapseRequest, NodeGraphOrganizationRequestError> {
        NodeGraphCollapseRequest::group(self, group, collapsed)
    }

    /// Creates application-owned node mute request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the node target is missing or disabled.
    pub fn mute_node_request(
        &self,
        node: NodeId,
        muted: bool,
    ) -> Result<NodeGraphNodeStateRequest, NodeGraphOrganizationRequestError> {
        NodeGraphNodeStateRequest::new(self, node, NodeGraphNodeStateAction::Mute, muted)
    }

    /// Creates application-owned node bypass request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the node target is missing or disabled.
    pub fn bypass_node_request(
        &self,
        node: NodeId,
        bypassed: bool,
    ) -> Result<NodeGraphNodeStateRequest, NodeGraphOrganizationRequestError> {
        NodeGraphNodeStateRequest::new(self, node, NodeGraphNodeStateAction::Bypass, bypassed)
    }

    /// Creates application-owned label request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the target is missing or disabled.
    pub fn label_request(
        &self,
        target: NodeGraphOrganizationTarget,
        label: impl Into<String>,
    ) -> Result<NodeGraphAnnotationRequest, NodeGraphOrganizationRequestError> {
        NodeGraphAnnotationRequest::new(
            self,
            target,
            NodeGraphAnnotationField::Label,
            Some(label.into()),
        )
    }

    /// Creates application-owned comment request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the target is missing or disabled.
    pub fn comment_request(
        &self,
        target: NodeGraphOrganizationTarget,
        comment: impl Into<String>,
    ) -> Result<NodeGraphAnnotationRequest, NodeGraphOrganizationRequestError> {
        NodeGraphAnnotationRequest::new(
            self,
            target,
            NodeGraphAnnotationField::Comment,
            Some(comment.into()),
        )
    }
}

/// Structured validation error for node graph descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphValidationError {
    /// The graph contains a duplicate node ID.
    DuplicateNodeId {
        /// Duplicated node ID.
        id: NodeId,
    },
    /// One node contains a duplicate port ID.
    DuplicatePortId {
        /// Node containing the duplicate port.
        node: NodeId,
        /// Duplicated port ID.
        port: PortId,
    },
    /// The graph contains a duplicate reroute ID.
    DuplicateRerouteId {
        /// Duplicated reroute ID.
        id: RerouteId,
    },
    /// The graph contains a duplicate frame ID.
    DuplicateFrameId {
        /// Duplicated frame ID.
        id: NodeFrameId,
    },
    /// The graph contains a duplicate group ID.
    DuplicateGroupId {
        /// Duplicated group ID.
        id: NodeGroupId,
    },
    /// A node references a missing frame.
    MissingFrameId {
        /// Node carrying the stale frame reference.
        node: NodeId,
        /// Missing frame ID.
        frame: NodeFrameId,
    },
    /// A node references a missing group.
    MissingGroupId {
        /// Node carrying the stale group reference.
        node: NodeId,
        /// Missing group ID.
        group: NodeGroupId,
    },
    /// A group lists the same node more than once.
    DuplicateGroupMember {
        /// Group containing the duplicate member.
        group: NodeGroupId,
        /// Duplicated member node ID.
        node: NodeId,
    },
    /// A group lists a missing node.
    MissingGroupMember {
        /// Group containing the stale member reference.
        group: NodeGroupId,
        /// Missing member node ID.
        node: NodeId,
    },
    /// A node is claimed by more than one group.
    DuplicateGroupMembership {
        /// Node with conflicting group membership.
        node: NodeId,
        /// First group discovered for the node.
        first: NodeGroupId,
        /// Later group discovered for the node.
        second: NodeGroupId,
    },
}

/// Validates deterministic descriptor invariants for nodes.
///
/// This intentionally does not resolve edge endpoints or validate application
/// domain semantics.
///
/// # Errors
///
/// Returns a structured validation error when node IDs are duplicated or a node
/// contains duplicate port IDs.
pub fn validate_node_graph_descriptors(
    nodes: &[NodeDescriptor],
) -> Result<(), NodeGraphValidationError> {
    let mut seen_nodes = BTreeSet::new();
    for node in nodes {
        if !seen_nodes.insert(node.id) {
            return Err(NodeGraphValidationError::DuplicateNodeId { id: node.id });
        }
    }

    for node in nodes {
        let mut seen_ports = BTreeSet::new();
        for port in &node.ports {
            if !seen_ports.insert(port.id) {
                return Err(NodeGraphValidationError::DuplicatePortId {
                    node: node.id,
                    port: port.id,
                });
            }
        }
    }

    Ok(())
}

pub(crate) fn validate_node_graph_reroute_descriptors(
    reroutes: &[RerouteDescriptor],
) -> Result<(), NodeGraphValidationError> {
    let mut seen_reroutes = BTreeSet::new();
    for reroute in reroutes {
        if !seen_reroutes.insert(reroute.id) {
            return Err(NodeGraphValidationError::DuplicateRerouteId { id: reroute.id });
        }
    }

    Ok(())
}

pub(crate) fn validate_node_graph_frame_descriptors(
    frames: &[NodeFrameDescriptor],
) -> Result<(), NodeGraphValidationError> {
    let mut seen_frames = BTreeSet::new();
    for frame in frames {
        if !seen_frames.insert(frame.id) {
            return Err(NodeGraphValidationError::DuplicateFrameId { id: frame.id });
        }
    }

    Ok(())
}

pub(crate) fn validate_node_graph_group_descriptors(
    groups: &[NodeGroupDescriptor],
) -> Result<(), NodeGraphValidationError> {
    let mut seen_groups = BTreeSet::new();
    for group in groups {
        if !seen_groups.insert(group.id) {
            return Err(NodeGraphValidationError::DuplicateGroupId { id: group.id });
        }
    }

    Ok(())
}

pub(crate) fn validate_node_graph_memberships(
    graph: &NodeGraphDescriptor,
) -> Result<(), NodeGraphValidationError> {
    let node_ids = graph
        .nodes
        .iter()
        .map(|node| node.id)
        .collect::<BTreeSet<_>>();
    let frame_ids = graph
        .frames
        .iter()
        .map(|frame| frame.id)
        .collect::<BTreeSet<_>>();
    let group_ids = graph
        .groups
        .iter()
        .map(|group| group.id)
        .collect::<BTreeSet<_>>();
    let mut group_memberships = BTreeSet::new();

    for node in &graph.nodes {
        if let Some(frame) = node.frame
            && !frame_ids.contains(&frame)
        {
            return Err(NodeGraphValidationError::MissingFrameId {
                node: node.id,
                frame,
            });
        }

        if let Some(group) = node.group {
            if !group_ids.contains(&group) {
                return Err(NodeGraphValidationError::MissingGroupId {
                    node: node.id,
                    group,
                });
            }
            group_memberships.insert((node.id, group));
        }
    }

    for group in &graph.groups {
        let mut group_nodes = BTreeSet::new();
        for node in &group.nodes {
            if !group_nodes.insert(*node) {
                return Err(NodeGraphValidationError::DuplicateGroupMember {
                    group: group.id,
                    node: *node,
                });
            }
            if !node_ids.contains(node) {
                return Err(NodeGraphValidationError::MissingGroupMember {
                    group: group.id,
                    node: *node,
                });
            }
            group_memberships.insert((*node, group.id));
        }
    }

    let mut by_node = BTreeSet::new();
    for (node, group) in group_memberships {
        if let Some((_, first)) = by_node.iter().find(|(candidate, _)| *candidate == node) {
            return Err(NodeGraphValidationError::DuplicateGroupMembership {
                node,
                first: *first,
                second: group,
            });
        }
        by_node.insert((node, group));
    }

    Ok(())
}

/// Structured organization request failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphOrganizationRequestError {
    /// Descriptor validation failed before request metadata could be created.
    Validation(NodeGraphValidationError),
    /// The addressed node is not present.
    MissingNode {
        /// Missing node ID.
        node: NodeId,
    },
    /// The addressed frame is not present.
    MissingFrame {
        /// Missing frame ID.
        frame: NodeFrameId,
    },
    /// The addressed group is not present.
    MissingGroup {
        /// Missing group ID.
        group: NodeGroupId,
    },
    /// The addressed node is disabled.
    DisabledNode {
        /// Disabled node ID.
        node: NodeId,
    },
    /// The addressed frame is disabled.
    DisabledFrame {
        /// Disabled frame ID.
        frame: NodeFrameId,
    },
    /// The addressed group is disabled.
    DisabledGroup {
        /// Disabled group ID.
        group: NodeGroupId,
    },
}

impl From<NodeGraphValidationError> for NodeGraphOrganizationRequestError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Organization target that can carry label/comment metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphOrganizationTarget {
    /// A node target.
    Node(NodeId),
    /// A frame target.
    Frame(NodeFrameId),
    /// A group target.
    Group(NodeGroupId),
}

/// Metadata for moving one parent frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphFrameMove {
    /// Frame to move.
    pub frame: NodeFrameId,
    /// Graph-space movement delta for the frame.
    pub delta: GraphVector,
}

/// Data-only request metadata for moving a parent frame and its member nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphFrameMoveRequest {
    /// Frame being moved.
    pub frame: NodeGraphFrameMove,
    /// Sanitized UI logical screen-space drag delta.
    pub screen_delta: GraphVector,
    /// Sanitized graph-space drag delta shared by the frame and children.
    pub graph_delta: GraphVector,
    /// Per-child move candidates in deterministic node order.
    pub children: Vec<NodeGraphNodeMove>,
}

impl NodeGraphFrameMoveRequest {
    /// Creates frame move request metadata from a viewport and frame target.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn new(
        graph: &NodeGraphDescriptor,
        viewport: NodeGraphViewport,
        frame: NodeFrameId,
        screen_delta: GraphVector,
    ) -> Result<Self, NodeGraphOrganizationRequestError> {
        graph.validate()?;
        let descriptor = resolve_node_graph_frame(graph, frame)?;
        if !descriptor.enabled {
            return Err(NodeGraphOrganizationRequestError::DisabledFrame { frame });
        }

        let screen_delta = screen_delta.sanitized();
        let graph_delta = node_graph_drag_delta(viewport, screen_delta);
        let children = frame_member_nodes(graph, frame)
            .into_iter()
            .map(|node| NodeGraphNodeMove {
                node,
                delta: graph_delta,
            })
            .collect();

        Ok(Self {
            frame: NodeGraphFrameMove {
                frame,
                delta: graph_delta,
            },
            screen_delta,
            graph_delta,
            children,
        })
    }

    /// Returns true when the request has no frame or child movement to apply.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.graph_delta == GraphVector::ZERO
    }
}

/// Collapsible organization target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphCollapseTarget {
    /// A frame target.
    Frame(NodeFrameId),
    /// A group target.
    Group(NodeGroupId),
}

/// Link identity metadata preserved while a frame or group is collapsed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NodeGraphCollapseLinkMetadata {
    /// Stable edge identity.
    pub edge: EdgeId,
    /// Source endpoint preserved from the edge descriptor.
    pub from: PortEndpoint,
    /// Target endpoint preserved from the edge descriptor.
    pub to: PortEndpoint,
}

/// Data-only request metadata for changing collapsed presentation state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphCollapseRequest {
    /// Collapsible target.
    pub target: NodeGraphCollapseTarget,
    /// Previously-presented collapsed state.
    pub previous_collapsed: bool,
    /// Requested collapsed state.
    pub collapsed: bool,
    /// Member nodes captured in deterministic order.
    pub nodes: Vec<NodeId>,
    /// Member ports preserved in deterministic endpoint order.
    pub ports: Vec<PortEndpoint>,
    /// Links touching member nodes, preserving edge endpoint identity metadata.
    pub links: Vec<NodeGraphCollapseLinkMetadata>,
}

impl NodeGraphCollapseRequest {
    /// Creates collapse request metadata for a frame.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn frame(
        graph: &NodeGraphDescriptor,
        frame: NodeFrameId,
        collapsed: bool,
    ) -> Result<Self, NodeGraphOrganizationRequestError> {
        graph.validate()?;
        let descriptor = resolve_node_graph_frame(graph, frame)?;
        if !descriptor.enabled {
            return Err(NodeGraphOrganizationRequestError::DisabledFrame { frame });
        }
        let nodes = frame_member_nodes(graph, frame);
        Ok(Self::from_members(
            graph,
            NodeGraphCollapseTarget::Frame(frame),
            descriptor.collapsed,
            collapsed,
            nodes,
        ))
    }

    /// Creates collapse request metadata for a group.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the group target is missing or disabled.
    pub fn group(
        graph: &NodeGraphDescriptor,
        group: NodeGroupId,
        collapsed: bool,
    ) -> Result<Self, NodeGraphOrganizationRequestError> {
        graph.validate()?;
        let descriptor = resolve_node_graph_group(graph, group)?;
        if !descriptor.enabled {
            return Err(NodeGraphOrganizationRequestError::DisabledGroup { group });
        }
        let nodes = group_member_nodes(graph, group);
        Ok(Self::from_members(
            graph,
            NodeGraphCollapseTarget::Group(group),
            descriptor.collapsed,
            collapsed,
            nodes,
        ))
    }

    fn from_members(
        graph: &NodeGraphDescriptor,
        target: NodeGraphCollapseTarget,
        previous_collapsed: bool,
        collapsed: bool,
        nodes: Vec<NodeId>,
    ) -> Self {
        let node_set = nodes.iter().copied().collect::<BTreeSet<_>>();
        let ports = graph
            .nodes
            .iter()
            .filter(|node| node_set.contains(&node.id))
            .flat_map(|node| {
                node.ports
                    .iter()
                    .map(|port| PortEndpoint::new(node.id, port.id))
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let links = graph
            .edges
            .iter()
            .filter(|edge| node_set.contains(&edge.from.node) || node_set.contains(&edge.to.node))
            .map(|edge| NodeGraphCollapseLinkMetadata {
                edge: edge.id,
                from: edge.from,
                to: edge.to,
            })
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();

        Self {
            target,
            previous_collapsed,
            collapsed,
            nodes,
            ports,
            links,
        }
    }

    /// Returns true when the collapsed state would not change.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        self.previous_collapsed == self.collapsed
    }
}

/// Node state operation represented by request metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphNodeStateAction {
    /// Set muted presentation state.
    Mute,
    /// Set bypassed presentation state.
    Bypass,
}

/// Data-only request metadata for node muted/bypassed state changes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeGraphNodeStateRequest {
    /// Node target.
    pub node: NodeId,
    /// Requested state action.
    pub action: NodeGraphNodeStateAction,
    /// Previously-presented state for this action.
    pub previous: bool,
    /// Requested state value.
    pub requested: bool,
}

impl NodeGraphNodeStateRequest {
    /// Creates node state request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the node target is missing or disabled.
    pub fn new(
        graph: &NodeGraphDescriptor,
        node: NodeId,
        action: NodeGraphNodeStateAction,
        requested: bool,
    ) -> Result<Self, NodeGraphOrganizationRequestError> {
        graph.validate()?;
        let descriptor = resolve_node_graph_node(graph, node)?;
        if !descriptor.enabled {
            return Err(NodeGraphOrganizationRequestError::DisabledNode { node });
        }
        let previous = match action {
            NodeGraphNodeStateAction::Mute => descriptor.muted,
            NodeGraphNodeStateAction::Bypass => descriptor.bypassed,
        };

        Ok(Self {
            node,
            action,
            previous,
            requested,
        })
    }

    /// Returns true when the requested state matches the current metadata.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        self.previous == self.requested
    }
}

/// Annotation field represented by request metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphAnnotationField {
    /// Secondary user-facing label metadata.
    Label,
    /// User-facing comment metadata.
    Comment,
}

/// Data-only request metadata for label/comment changes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphAnnotationRequest {
    /// Annotation target.
    pub target: NodeGraphOrganizationTarget,
    /// Requested annotation field.
    pub field: NodeGraphAnnotationField,
    /// Previously-presented annotation value.
    pub previous: Option<String>,
    /// Requested annotation value.
    pub requested: Option<String>,
}

impl NodeGraphAnnotationRequest {
    /// Creates annotation request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the target is missing or disabled.
    pub fn new(
        graph: &NodeGraphDescriptor,
        target: NodeGraphOrganizationTarget,
        field: NodeGraphAnnotationField,
        requested: Option<String>,
    ) -> Result<Self, NodeGraphOrganizationRequestError> {
        graph.validate()?;
        let previous = resolve_annotation_target(graph, target, field)?;

        Ok(Self {
            target,
            field,
            previous,
            requested,
        })
    }

    /// Returns true when the requested annotation matches current metadata.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.previous == self.requested
    }
}

pub(crate) fn resolve_node_graph_node(
    graph: &NodeGraphDescriptor,
    node: NodeId,
) -> Result<&NodeDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .nodes
        .iter()
        .find(|descriptor| descriptor.id == node)
        .ok_or(NodeGraphOrganizationRequestError::MissingNode { node })
}

pub(crate) fn resolve_node_graph_frame(
    graph: &NodeGraphDescriptor,
    frame: NodeFrameId,
) -> Result<&NodeFrameDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .frames
        .iter()
        .find(|descriptor| descriptor.id == frame)
        .ok_or(NodeGraphOrganizationRequestError::MissingFrame { frame })
}

pub(crate) fn resolve_node_graph_group(
    graph: &NodeGraphDescriptor,
    group: NodeGroupId,
) -> Result<&NodeGroupDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .groups
        .iter()
        .find(|descriptor| descriptor.id == group)
        .ok_or(NodeGraphOrganizationRequestError::MissingGroup { group })
}

pub(crate) fn frame_member_nodes(graph: &NodeGraphDescriptor, frame: NodeFrameId) -> Vec<NodeId> {
    graph
        .nodes
        .iter()
        .filter(|node| node.frame == Some(frame))
        .map(|node| node.id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn group_member_nodes(graph: &NodeGraphDescriptor, group: NodeGroupId) -> Vec<NodeId> {
    let mut members = graph
        .groups
        .iter()
        .find(|descriptor| descriptor.id == group)
        .map(|descriptor| descriptor.nodes.iter().copied().collect::<BTreeSet<_>>())
        .unwrap_or_default();
    members.extend(
        graph
            .nodes
            .iter()
            .filter(|node| node.group == Some(group))
            .map(|node| node.id),
    );
    members.into_iter().collect()
}

pub(crate) fn resolve_annotation_target(
    graph: &NodeGraphDescriptor,
    target: NodeGraphOrganizationTarget,
    field: NodeGraphAnnotationField,
) -> Result<Option<String>, NodeGraphOrganizationRequestError> {
    match target {
        NodeGraphOrganizationTarget::Node(node) => {
            let descriptor = resolve_node_graph_node(graph, node)?;
            if !descriptor.enabled {
                return Err(NodeGraphOrganizationRequestError::DisabledNode { node });
            }
            Ok(match field {
                NodeGraphAnnotationField::Label => descriptor.label.clone(),
                NodeGraphAnnotationField::Comment => descriptor.comment.clone(),
            })
        }
        NodeGraphOrganizationTarget::Frame(frame) => {
            let descriptor = resolve_node_graph_frame(graph, frame)?;
            if !descriptor.enabled {
                return Err(NodeGraphOrganizationRequestError::DisabledFrame { frame });
            }
            Ok(match field {
                NodeGraphAnnotationField::Label => descriptor.label.clone(),
                NodeGraphAnnotationField::Comment => descriptor.comment.clone(),
            })
        }
        NodeGraphOrganizationTarget::Group(group) => {
            let descriptor = resolve_node_graph_group(graph, group)?;
            if !descriptor.enabled {
                return Err(NodeGraphOrganizationRequestError::DisabledGroup { group });
            }
            Ok(match field {
                NodeGraphAnnotationField::Label => descriptor.label.clone(),
                NodeGraphAnnotationField::Comment => descriptor.comment.clone(),
            })
        }
    }
}
