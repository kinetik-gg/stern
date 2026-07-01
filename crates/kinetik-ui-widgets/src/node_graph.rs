//! Backend-independent node graph identity, descriptor, and coordinate contracts.

use std::collections::BTreeSet;

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive,
    SemanticNode, SemanticRole, SemanticValue, Stroke, TextPrimitive, WidgetId,
};

const DEFAULT_ZOOM: f32 = 1.0;
const MIN_ZOOM: f32 = 0.01;
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

fn validate_node_graph_reroute_descriptors(
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

fn validate_node_graph_frame_descriptors(
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

fn validate_node_graph_group_descriptors(
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

fn validate_node_graph_memberships(
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

fn resolve_node_graph_node(
    graph: &NodeGraphDescriptor,
    node: NodeId,
) -> Result<&NodeDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .nodes
        .iter()
        .find(|descriptor| descriptor.id == node)
        .ok_or(NodeGraphOrganizationRequestError::MissingNode { node })
}

fn resolve_node_graph_frame(
    graph: &NodeGraphDescriptor,
    frame: NodeFrameId,
) -> Result<&NodeFrameDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .frames
        .iter()
        .find(|descriptor| descriptor.id == frame)
        .ok_or(NodeGraphOrganizationRequestError::MissingFrame { frame })
}

fn resolve_node_graph_group(
    graph: &NodeGraphDescriptor,
    group: NodeGroupId,
) -> Result<&NodeGroupDescriptor, NodeGraphOrganizationRequestError> {
    graph
        .groups
        .iter()
        .find(|descriptor| descriptor.id == group)
        .ok_or(NodeGraphOrganizationRequestError::MissingGroup { group })
}

fn frame_member_nodes(graph: &NodeGraphDescriptor, frame: NodeFrameId) -> Vec<NodeId> {
    graph
        .nodes
        .iter()
        .filter(|node| node.frame == Some(frame))
        .map(|node| node.id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn group_member_nodes(graph: &NodeGraphDescriptor, group: NodeGroupId) -> Vec<NodeId> {
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

fn resolve_annotation_target(
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

/// Structured directed port compatibility failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortCompatibilityError {
    /// Compatibility is only valid from an output port to an input port.
    DirectionMismatch {
        /// Source port direction.
        output: PortDirection,
        /// Target port direction.
        input: PortDirection,
    },
    /// One or both ports are disabled.
    DisabledPort {
        /// Whether the source port is enabled.
        output_enabled: bool,
        /// Whether the target port is enabled.
        input_enabled: bool,
    },
    /// The app-owned compatibility keys do not match.
    TypeMismatch {
        /// Source port compatibility key.
        output: PortTypeId,
        /// Target port compatibility key.
        input: PortTypeId,
    },
}

/// Validates directed output-to-input port compatibility.
///
/// # Errors
///
/// Returns a structured compatibility error when the pair is not directed from
/// output to input, one of the ports is disabled, or the app-owned
/// compatibility keys differ.
pub fn validate_port_compatibility(
    output: &PortDescriptor,
    input: &PortDescriptor,
) -> Result<(), PortCompatibilityError> {
    if output.direction != PortDirection::Output || input.direction != PortDirection::Input {
        return Err(PortCompatibilityError::DirectionMismatch {
            output: output.direction,
            input: input.direction,
        });
    }

    if !output.enabled || !input.enabled {
        return Err(PortCompatibilityError::DisabledPort {
            output_enabled: output.enabled,
            input_enabled: input.enabled,
        });
    }

    if output.port_type != input.port_type {
        return Err(PortCompatibilityError::TypeMismatch {
            output: output.port_type,
            input: input.port_type,
        });
    }

    Ok(())
}

/// Returns true when two ports form a valid output-to-input compatibility pair.
#[must_use]
pub fn ports_are_compatible(output: &PortDescriptor, input: &PortDescriptor) -> bool {
    validate_port_compatibility(output, input).is_ok()
}

/// Resolves all edge endpoints in descriptor order.
///
/// # Errors
///
/// Returns the first structured topology error encountered while walking edge
/// descriptors in order.
pub fn resolve_node_graph_edges(
    graph: &NodeGraphDescriptor,
) -> Result<Vec<ResolvedEdge<'_>>, EdgeResolutionError> {
    let mut seen_edges = BTreeSet::new();
    let mut resolved = Vec::with_capacity(graph.edges.len());

    for edge in &graph.edges {
        if !seen_edges.insert(edge.id) {
            return Err(EdgeResolutionError::DuplicateEdgeId { edge: edge.id });
        }

        let from = resolve_endpoint(
            &graph.nodes,
            edge.id,
            EdgeEndpointRole::Source,
            edge.from,
            PortDirection::Output,
        )?;
        let to = resolve_endpoint(
            &graph.nodes,
            edge.id,
            EdgeEndpointRole::Target,
            edge.to,
            PortDirection::Input,
        )?;

        if !from.port.enabled {
            return Err(EdgeResolutionError::DisabledPort {
                edge: edge.id,
                endpoint: EdgeEndpointRole::Source,
                node: from.endpoint.node,
                port: from.endpoint.port,
            });
        }

        if !to.port.enabled {
            return Err(EdgeResolutionError::DisabledPort {
                edge: edge.id,
                endpoint: EdgeEndpointRole::Target,
                node: to.endpoint.node,
                port: to.endpoint.port,
            });
        }

        if from.port.port_type != to.port.port_type {
            return Err(EdgeResolutionError::IncompatiblePortType {
                edge: edge.id,
                from: edge.from,
                to: edge.to,
                output: from.port.port_type,
                input: to.port.port_type,
            });
        }

        let route_points = resolve_edge_route_points(edge, &graph.reroutes)?;

        resolved.push(ResolvedEdge {
            edge,
            from,
            route_points,
            to,
        });
    }

    Ok(resolved)
}

fn resolve_edge_route_points<'a>(
    edge: &'a EdgeDescriptor,
    reroutes: &'a [RerouteDescriptor],
) -> Result<Vec<ResolvedEdgeRoutePoint<'a>>, EdgeResolutionError> {
    edge.route_points
        .iter()
        .map(|route_point| match *route_point {
            NodeGraphEdgeRoutePoint::Point(position) => Ok(ResolvedEdgeRoutePoint {
                route_point: *route_point,
                position: position.sanitized(),
                reroute: None,
            }),
            NodeGraphEdgeRoutePoint::Reroute(reroute_id) => {
                let reroute = reroutes
                    .iter()
                    .find(|reroute| reroute.id == reroute_id)
                    .ok_or(EdgeResolutionError::MissingReroute {
                        edge: edge.id,
                        reroute: reroute_id,
                    })?;

                Ok(ResolvedEdgeRoutePoint {
                    route_point: *route_point,
                    position: reroute.position.sanitized(),
                    reroute: Some(reroute),
                })
            }
        })
        .collect()
}

fn resolve_endpoint(
    nodes: &[NodeDescriptor],
    edge: EdgeId,
    role: EdgeEndpointRole,
    endpoint: PortEndpoint,
    expected_direction: PortDirection,
) -> Result<ResolvedEndpoint<'_>, EdgeResolutionError> {
    let node = nodes.iter().find(|node| node.id == endpoint.node).ok_or(
        EdgeResolutionError::MissingNode {
            edge,
            endpoint: role,
            node: endpoint.node,
        },
    )?;
    let port = node
        .ports
        .iter()
        .find(|port| port.id == endpoint.port)
        .ok_or(EdgeResolutionError::MissingPort {
            edge,
            endpoint: role,
            node: endpoint.node,
            port: endpoint.port,
        })?;

    if port.direction != expected_direction {
        return Err(EdgeResolutionError::WrongDirection {
            edge,
            endpoint: role,
            node: endpoint.node,
            port: endpoint.port,
            expected: expected_direction,
            actual: port.direction,
        });
    }

    Ok(ResolvedEndpoint {
        role,
        endpoint,
        node,
        port,
        anchor: port_anchor(node, port),
    })
}

fn resolve_link_draft_endpoint(
    graph: &NodeGraphDescriptor,
    endpoint: PortEndpoint,
) -> Result<NodeGraphLinkDraftEndpoint, NodeGraphLinkDraftEndpointError> {
    graph.validate()?;
    let node = graph
        .nodes
        .iter()
        .find(|node| node.id == endpoint.node)
        .ok_or(NodeGraphLinkDraftEndpointError::MissingNode {
            node: endpoint.node,
        })?;
    if !node.enabled {
        return Err(NodeGraphLinkDraftEndpointError::DisabledNode {
            node: endpoint.node,
        });
    }

    let port = node
        .ports
        .iter()
        .find(|port| port.id == endpoint.port)
        .ok_or(NodeGraphLinkDraftEndpointError::MissingPort {
            node: endpoint.node,
            port: endpoint.port,
        })?;
    if !port.enabled {
        return Err(NodeGraphLinkDraftEndpointError::DisabledPort {
            node: endpoint.node,
            port: endpoint.port,
        });
    }

    Ok(NodeGraphLinkDraftEndpoint {
        endpoint,
        direction: port.direction,
        port_type: port.port_type,
        anchor: port_anchor(node, port),
    })
}

fn link_draft_compatibility(
    start: NodeGraphLinkDraftEndpoint,
    target: NodeGraphLinkDraftEndpoint,
) -> Result<(), PortCompatibilityError> {
    let (output, input) = if start.direction == PortDirection::Output {
        (start, target)
    } else {
        (target, start)
    };

    if output.direction != PortDirection::Output || input.direction != PortDirection::Input {
        return Err(PortCompatibilityError::DirectionMismatch {
            output: output.direction,
            input: input.direction,
        });
    }

    if output.port_type != input.port_type {
        return Err(PortCompatibilityError::TypeMismatch {
            output: output.port_type,
            input: input.port_type,
        });
    }

    Ok(())
}

fn resolve_link_edit_request_endpoint(
    graph: &NodeGraphDescriptor,
    endpoint: PortEndpoint,
) -> Result<NodeGraphLinkDraftEndpoint, NodeGraphLinkEditRequestError> {
    resolve_link_draft_endpoint(graph, endpoint).map_err(NodeGraphLinkEditRequestError::Endpoint)
}

fn validate_link_edit_compatibility(
    from: NodeGraphLinkDraftEndpoint,
    to: NodeGraphLinkDraftEndpoint,
) -> Result<(), NodeGraphLinkEditRequestError> {
    let error = if from.direction != PortDirection::Output || to.direction != PortDirection::Input {
        Some(PortCompatibilityError::DirectionMismatch {
            output: from.direction,
            input: to.direction,
        })
    } else if from.port_type != to.port_type {
        Some(PortCompatibilityError::TypeMismatch {
            output: from.port_type,
            input: to.port_type,
        })
    } else {
        None
    };

    if let Some(error) = error {
        return Err(NodeGraphLinkEditRequestError::IncompatiblePort {
            from: from.endpoint,
            to: to.endpoint,
            error,
        });
    }

    Ok(())
}

fn resolve_link_edit_edge(
    graph: &NodeGraphDescriptor,
    edge: EdgeId,
) -> Result<NodeGraphLinkEditEdgeContext, NodeGraphLinkEditRequestError> {
    graph.validate()?;

    let mut seen_edges = BTreeSet::new();
    let mut resolved = None;
    for candidate in &graph.edges {
        if !seen_edges.insert(candidate.id) {
            return Err(EdgeResolutionError::DuplicateEdgeId { edge: candidate.id }.into());
        }

        if candidate.id == edge {
            resolved = Some(candidate);
        }
    }

    let edge = resolved.ok_or(NodeGraphLinkEditRequestError::MissingEdge { edge })?;
    let from = resolve_endpoint(
        &graph.nodes,
        edge.id,
        EdgeEndpointRole::Source,
        edge.from,
        PortDirection::Output,
    )?;
    let to = resolve_endpoint(
        &graph.nodes,
        edge.id,
        EdgeEndpointRole::Target,
        edge.to,
        PortDirection::Input,
    )?;

    if !from.port.enabled {
        return Err(EdgeResolutionError::DisabledPort {
            edge: edge.id,
            endpoint: EdgeEndpointRole::Source,
            node: from.endpoint.node,
            port: from.endpoint.port,
        }
        .into());
    }

    if !to.port.enabled {
        return Err(EdgeResolutionError::DisabledPort {
            edge: edge.id,
            endpoint: EdgeEndpointRole::Target,
            node: to.endpoint.node,
            port: to.endpoint.port,
        }
        .into());
    }

    if from.port.port_type != to.port.port_type {
        return Err(EdgeResolutionError::IncompatiblePortType {
            edge: edge.id,
            from: from.endpoint,
            to: to.endpoint,
            output: from.port.port_type,
            input: to.port.port_type,
        }
        .into());
    }

    Ok(NodeGraphLinkEditEdgeContext {
        edge: edge.id,
        from: NodeGraphLinkDraftEndpoint {
            endpoint: from.endpoint,
            direction: from.port.direction,
            port_type: from.port.port_type,
            anchor: from.anchor,
        },
        to: NodeGraphLinkDraftEndpoint {
            endpoint: to.endpoint,
            direction: to.port.direction,
            port_type: to.port.port_type,
            anchor: to.anchor,
        },
        enabled: edge.enabled,
    })
}

fn port_anchor(node: &NodeDescriptor, port: &PortDescriptor) -> GraphPoint {
    let rect = node.rect.sanitized();
    let same_direction_count = node
        .ports
        .iter()
        .filter(|candidate| candidate.direction == port.direction)
        .count();
    let same_direction_index = node
        .ports
        .iter()
        .filter(|candidate| candidate.direction == port.direction)
        .position(|candidate| candidate.id == port.id)
        .unwrap_or(0);
    let slot = usize_to_graph_slot(same_direction_index) + 1.0;
    let slot_count = usize_to_graph_slot(same_direction_count) + 1.0;
    let x = match port.direction {
        PortDirection::Input => rect.x,
        PortDirection::Output => finite_sum(rect.x, rect.width),
    };
    let y = finite_sum(rect.y, finite_product(rect.height, slot / slot_count));

    GraphPoint::new(x, y)
}

fn usize_to_graph_slot(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}

/// A point in node graph content space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphPoint {
    /// Horizontal graph coordinate.
    pub x: f32,
    /// Vertical graph coordinate.
    pub y: f32,
}

impl GraphPoint {
    /// The graph origin.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Creates a graph point.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns this point translated by a graph vector.
    #[must_use]
    pub const fn translate(self, offset: GraphVector) -> Self {
        Self::new(self.x + offset.x, self.y + offset.y)
    }

    /// Returns a copy with non-finite coordinates replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(finite_or_zero(self.x), finite_or_zero(self.y))
    }
}

/// A vector in node graph coordinate calculations.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphVector {
    /// Horizontal component.
    pub x: f32,
    /// Vertical component.
    pub y: f32,
}

impl GraphVector {
    /// The zero vector.
    pub const ZERO: Self = Self::new(0.0, 0.0);

    /// Creates a graph vector.
    #[must_use]
    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns a copy with non-finite components replaced by zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(finite_or_zero(self.x), finite_or_zero(self.y))
    }
}

/// An axis-aligned rectangle in node graph content space.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct GraphRect {
    /// Minimum x coordinate.
    pub x: f32,
    /// Minimum y coordinate.
    pub y: f32,
    /// Rectangle width in graph units.
    pub width: f32,
    /// Rectangle height in graph units.
    pub height: f32,
}

impl GraphRect {
    /// An empty graph-space rectangle at the origin.
    pub const ZERO: Self = Self::new(0.0, 0.0, 0.0, 0.0);

    /// Creates a graph-space rectangle.
    #[must_use]
    pub const fn new(x: f32, y: f32, width: f32, height: f32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    /// Creates a graph-space rectangle from an origin and size vector.
    #[must_use]
    pub const fn from_origin_size(origin: GraphPoint, size: GraphVector) -> Self {
        Self::new(origin.x, origin.y, size.x, size.y)
    }

    /// Returns the rectangle origin.
    #[must_use]
    pub const fn origin(self) -> GraphPoint {
        GraphPoint::new(self.x, self.y)
    }

    /// Returns the rectangle size as a graph vector.
    #[must_use]
    pub const fn size(self) -> GraphVector {
        GraphVector::new(self.width, self.height)
    }

    /// Returns the maximum x coordinate.
    #[must_use]
    pub const fn max_x(self) -> f32 {
        self.x + self.width
    }

    /// Returns the maximum y coordinate.
    #[must_use]
    pub const fn max_y(self) -> f32 {
        self.y + self.height
    }

    /// Returns true when either dimension is zero or negative.
    #[must_use]
    pub const fn is_empty(self) -> bool {
        self.width <= 0.0 || self.height <= 0.0
    }

    /// Returns a copy with non-finite coordinates replaced by zero and invalid
    /// dimensions clamped to zero.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self::new(
            finite_or_zero(self.x),
            finite_or_zero(self.y),
            finite_non_negative(self.width),
            finite_non_negative(self.height),
        )
    }

    /// Creates a graph-space rectangle from two corners.
    #[must_use]
    pub fn from_min_max(min: GraphPoint, max: GraphPoint) -> Self {
        let min = min.sanitized();
        let max = max.sanitized();
        let x = min.x.min(max.x);
        let y = min.y.min(max.y);
        Self::new(x, y, (max.x - min.x).abs(), (max.y - min.y).abs())
    }

    /// Returns true when this rectangle fully contains another rectangle.
    #[must_use]
    pub fn contains_rect(self, other: GraphRect) -> bool {
        let rect = self.sanitized();
        let other = other.sanitized();
        !rect.is_empty()
            && !other.is_empty()
            && other.x >= rect.x
            && other.y >= rect.y
            && other.max_x() <= rect.max_x()
            && other.max_y() <= rect.max_y()
    }

    /// Returns true when this rectangle overlaps another rectangle.
    #[must_use]
    pub fn intersects_rect(self, other: GraphRect) -> bool {
        let rect = self.sanitized();
        let other = other.sanitized();
        !rect.is_empty()
            && !other.is_empty()
            && rect.x < other.max_x()
            && rect.max_x() > other.x
            && rect.y < other.max_y()
            && rect.max_y() > other.y
    }
}

/// Pan and zoom state for a node graph viewport.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphPanZoom {
    /// Screen-space pan offset in viewport-local logical units.
    pub pan: GraphVector,
    /// Screen units per graph unit.
    pub zoom: f32,
}

impl Default for NodeGraphPanZoom {
    fn default() -> Self {
        Self {
            pan: GraphVector::ZERO,
            zoom: DEFAULT_ZOOM,
        }
    }
}

impl NodeGraphPanZoom {
    /// Creates pan/zoom state.
    #[must_use]
    pub const fn new(pan: GraphVector, zoom: f32) -> Self {
        Self { pan, zoom }
    }

    /// Returns a copy with deterministic, valid numeric values.
    #[must_use]
    pub fn sanitized(self) -> Self {
        Self {
            pan: self.pan.sanitized(),
            zoom: sanitize_zoom(self.zoom),
        }
    }

    /// Sets custom zoom, falling back to the default for invalid values.
    pub fn set_zoom(&mut self, zoom: f32) {
        self.zoom = sanitize_zoom(zoom);
    }

    /// Adds a screen-space pan delta.
    pub fn pan_by(&mut self, delta: GraphVector) {
        let pan = self.pan.sanitized();
        let delta = delta.sanitized();
        self.pan = GraphVector::new(finite_sum(pan.x, delta.x), finite_sum(pan.y, delta.y));
    }
}

/// Node graph viewport bounds plus pan/zoom conversion state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphViewport {
    /// Viewport bounds in UI logical screen coordinates.
    pub bounds: Rect,
    /// Pan and zoom state.
    pub pan_zoom: NodeGraphPanZoom,
}

impl NodeGraphViewport {
    /// Creates a node graph viewport.
    #[must_use]
    pub const fn new(bounds: Rect, pan_zoom: NodeGraphPanZoom) -> Self {
        Self { bounds, pan_zoom }
    }

    /// Returns sanitized viewport bounds.
    #[must_use]
    pub fn effective_bounds(self) -> Rect {
        sanitize_rect(self.bounds)
    }

    /// Returns sanitized pan/zoom state.
    #[must_use]
    pub fn effective_pan_zoom(self) -> NodeGraphPanZoom {
        self.pan_zoom.sanitized()
    }

    /// Converts a graph-space point to UI logical screen coordinates.
    #[must_use]
    pub fn graph_to_screen(self, point: GraphPoint) -> Point {
        let point = point.sanitized();
        let bounds = self.effective_bounds();
        let pan_zoom = self.effective_pan_zoom();
        Point::new(
            finite_sum(
                finite_sum(bounds.x, pan_zoom.pan.x),
                finite_product(point.x, pan_zoom.zoom),
            ),
            finite_sum(
                finite_sum(bounds.y, pan_zoom.pan.y),
                finite_product(point.y, pan_zoom.zoom),
            ),
        )
    }

    /// Converts a UI logical screen point to graph-space coordinates.
    #[must_use]
    pub fn screen_to_graph(self, point: Point) -> GraphPoint {
        let point = sanitize_point(point);
        let bounds = self.effective_bounds();
        let pan_zoom = self.effective_pan_zoom();
        GraphPoint::new(
            finite_div(
                finite_sum(finite_sum(point.x, -bounds.x), -pan_zoom.pan.x),
                pan_zoom.zoom,
            ),
            finite_div(
                finite_sum(finite_sum(point.y, -bounds.y), -pan_zoom.pan.y),
                pan_zoom.zoom,
            ),
        )
    }

    /// Converts a UI logical screen-space delta to graph-space units.
    #[must_use]
    pub fn screen_delta_to_graph(self, delta: GraphVector) -> GraphVector {
        let delta = delta.sanitized();
        let zoom = self.effective_pan_zoom().zoom;
        GraphVector::new(finite_div(delta.x, zoom), finite_div(delta.y, zoom))
    }

    /// Converts a graph-space rectangle to UI logical screen coordinates.
    #[must_use]
    pub fn graph_rect_to_screen(self, rect: GraphRect) -> Rect {
        let rect = rect.sanitized();
        let origin = self.graph_to_screen(rect.origin());
        let zoom = self.effective_pan_zoom().zoom;
        Rect::new(
            origin.x,
            origin.y,
            finite_product(rect.width, zoom).max(0.0),
            finite_product(rect.height, zoom).max(0.0),
        )
    }

    /// Converts a UI logical screen rectangle to graph-space coordinates.
    #[must_use]
    pub fn screen_rect_to_graph(self, rect: Rect) -> GraphRect {
        let rect = sanitize_rect(rect);
        let origin = self.screen_to_graph(rect.origin());
        let zoom = self.effective_pan_zoom().zoom;
        GraphRect::new(
            origin.x,
            origin.y,
            finite_div(rect.width, zoom).max(0.0),
            finite_div(rect.height, zoom).max(0.0),
        )
    }
}

/// Stable backend-independent node graph hit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphHitTarget {
    /// A hittable port on a node.
    Port(PortEndpoint),
    /// The node title bar.
    NodeTitle(NodeId),
    /// The node body below the title bar.
    NodeBody(NodeId),
    /// A reroute handle.
    Reroute(RerouteId),
    /// A resolved edge segment.
    Edge(EdgeId),
    /// A frame surface.
    Frame(NodeFrameId),
    /// A group surface.
    Group(NodeGroupId),
    /// The graph canvas or an out-of-viewport point.
    Canvas,
}

/// Stable selectable node graph target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphSelectionTarget {
    /// A node, independent from whether the title or body was hit.
    Node(NodeId),
    /// A graph edge.
    Edge(EdgeId),
    /// A reroute handle.
    Reroute(RerouteId),
    /// A node port endpoint.
    Port(PortEndpoint),
}

impl NodeGraphSelectionTarget {
    /// Converts a hit target into a selectable graph target.
    ///
    /// Canvas, frames, and groups are not selectable by this selection model.
    #[must_use]
    pub const fn from_hit_target(hit: NodeGraphHitTarget) -> Option<Self> {
        match hit {
            NodeGraphHitTarget::Port(endpoint) => Some(Self::Port(endpoint)),
            NodeGraphHitTarget::NodeTitle(node) | NodeGraphHitTarget::NodeBody(node) => {
                Some(Self::Node(node))
            }
            NodeGraphHitTarget::Reroute(reroute) => Some(Self::Reroute(reroute)),
            NodeGraphHitTarget::Edge(edge) => Some(Self::Edge(edge)),
            NodeGraphHitTarget::Frame(_)
            | NodeGraphHitTarget::Group(_)
            | NodeGraphHitTarget::Canvas => None,
        }
    }
}

/// Pure node graph selection operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphSelectionOperation {
    /// Replace the selection with one target.
    Replace(NodeGraphSelectionTarget),
    /// Toggle one target in or out of the selection.
    Toggle(NodeGraphSelectionTarget),
    /// Add one target to the selection.
    Extend(NodeGraphSelectionTarget),
    /// Remove one target from the selection.
    Remove(NodeGraphSelectionTarget),
    /// Clear all selected targets.
    Clear,
}

/// Data-only node graph selection metadata.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct NodeGraphSelection {
    selected: BTreeSet<NodeGraphSelectionTarget>,
    active: Option<NodeGraphSelectionTarget>,
}

impl NodeGraphSelection {
    /// Creates an empty selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates a selection from graph targets.
    #[must_use]
    pub fn from_targets(targets: impl IntoIterator<Item = NodeGraphSelectionTarget>) -> Self {
        let selected = targets.into_iter().collect::<BTreeSet<_>>();
        Self {
            active: selected.iter().next_back().copied(),
            selected,
        }
    }

    /// Returns true when no graph targets are selected.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.selected.is_empty()
    }

    /// Returns true when the target is selected.
    #[must_use]
    pub fn contains(&self, target: NodeGraphSelectionTarget) -> bool {
        self.selected.contains(&target)
    }

    /// Returns selected targets in deterministic sorted order.
    #[must_use]
    pub fn selected(&self) -> Vec<NodeGraphSelectionTarget> {
        self.selected.iter().copied().collect()
    }

    /// Returns selected node IDs in deterministic sorted order.
    #[must_use]
    pub fn selected_nodes(&self) -> Vec<NodeId> {
        self.selected
            .iter()
            .filter_map(|target| match target {
                NodeGraphSelectionTarget::Node(node) => Some(*node),
                NodeGraphSelectionTarget::Edge(_)
                | NodeGraphSelectionTarget::Reroute(_)
                | NodeGraphSelectionTarget::Port(_) => None,
            })
            .collect()
    }

    /// Returns the most recent operation target, when one is present.
    #[must_use]
    pub const fn active(&self) -> Option<NodeGraphSelectionTarget> {
        self.active
    }

    /// Applies a pure selection operation and returns the resulting selection.
    #[must_use]
    pub fn apply(&self, operation: NodeGraphSelectionOperation) -> Self {
        match operation {
            NodeGraphSelectionOperation::Replace(target) => self.replace(target),
            NodeGraphSelectionOperation::Toggle(target) => self.toggle(target),
            NodeGraphSelectionOperation::Extend(target) => self.extend(target),
            NodeGraphSelectionOperation::Remove(target) => self.remove(target),
            NodeGraphSelectionOperation::Clear => self.clear(),
        }
    }

    /// Returns a selection containing only one target.
    #[must_use]
    pub fn replace(&self, target: NodeGraphSelectionTarget) -> Self {
        Self {
            selected: BTreeSet::from([target]),
            active: Some(target),
        }
    }

    /// Returns a selection with one target toggled in or out.
    #[must_use]
    pub fn toggle(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        if !selected.remove(&target) {
            selected.insert(target);
        }
        Self {
            active: Some(target),
            selected,
        }
    }

    /// Returns a selection with one target added.
    #[must_use]
    pub fn extend(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        selected.insert(target);
        Self {
            active: Some(target),
            selected,
        }
    }

    /// Returns a selection with one target removed.
    #[must_use]
    pub fn remove(&self, target: NodeGraphSelectionTarget) -> Self {
        let mut selected = self.selected.clone();
        selected.remove(&target);
        let active = if selected.is_empty() {
            None
        } else if self.active == Some(target) {
            selected.iter().next_back().copied()
        } else {
            self.active
        };

        Self { selected, active }
    }

    /// Returns an empty selection.
    #[must_use]
    pub fn clear(&self) -> Self {
        Self::new()
    }

    /// Replaces selection from a hit target, clearing explicitly on canvas.
    ///
    /// Frame and group hits are ignored by this selection model.
    #[must_use]
    pub fn replace_from_hit(&self, hit: NodeGraphHitTarget) -> Self {
        if hit == NodeGraphHitTarget::Canvas {
            return self.clear();
        }

        NodeGraphSelectionTarget::from_hit_target(hit)
            .map_or_else(|| self.clone(), |target| self.replace(target))
    }
}

/// Stable node graph context-menu target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphContextTarget {
    /// A node, independent from whether its title or body was hit.
    Node(NodeId),
    /// A graph edge.
    Edge(EdgeId),
    /// A reroute handle.
    Reroute(RerouteId),
    /// A node port endpoint.
    Port(PortEndpoint),
    /// A frame surface.
    Frame(NodeFrameId),
    /// A group surface.
    Group(NodeGroupId),
    /// The graph canvas or an out-of-viewport point.
    Canvas,
}

impl NodeGraphContextTarget {
    /// Converts a hit target into a context-menu graph target.
    #[must_use]
    pub const fn from_hit_target(hit: NodeGraphHitTarget) -> Self {
        match hit {
            NodeGraphHitTarget::Port(endpoint) => Self::Port(endpoint),
            NodeGraphHitTarget::NodeTitle(node) | NodeGraphHitTarget::NodeBody(node) => {
                Self::Node(node)
            }
            NodeGraphHitTarget::Reroute(reroute) => Self::Reroute(reroute),
            NodeGraphHitTarget::Edge(edge) => Self::Edge(edge),
            NodeGraphHitTarget::Frame(frame) => Self::Frame(frame),
            NodeGraphHitTarget::Group(group) => Self::Group(group),
            NodeGraphHitTarget::Canvas => Self::Canvas,
        }
    }

    /// Converts this context target into a selectable graph target, when possible.
    #[must_use]
    pub const fn selection_target(self) -> Option<NodeGraphSelectionTarget> {
        match self {
            Self::Node(node) => Some(NodeGraphSelectionTarget::Node(node)),
            Self::Edge(edge) => Some(NodeGraphSelectionTarget::Edge(edge)),
            Self::Reroute(reroute) => Some(NodeGraphSelectionTarget::Reroute(reroute)),
            Self::Port(endpoint) => Some(NodeGraphSelectionTarget::Port(endpoint)),
            Self::Frame(_) | Self::Group(_) | Self::Canvas => None,
        }
    }
}

/// Operation represented by node graph context action metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum NodeGraphContextActionKind {
    /// Delete the context target or active graph selection.
    Delete,
    /// Duplicate the context target or active graph selection.
    Duplicate,
    /// Disconnect an edge or a port endpoint.
    Disconnect,
    /// Detach an edge source endpoint.
    DetachSource,
    /// Detach an edge target endpoint.
    DetachTarget,
    /// Place selected nodes into a frame.
    FrameSelection,
    /// Place selected nodes into a group.
    GroupSelection,
    /// Ungroup the addressed group target.
    Ungroup,
    /// Select all selectable graph targets.
    SelectAll,
    /// Paste through application-owned clipboard state.
    Paste,
}

/// Compatibility order for the built-in node graph context action catalog.
///
/// Applications may use this catalog directly, filter it, reorder it, or bypass
/// it and call the typed request builders on [`NodeGraphDescriptor`].
pub const DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS: [NodeGraphContextActionKind; 10] = [
    NodeGraphContextActionKind::Delete,
    NodeGraphContextActionKind::Duplicate,
    NodeGraphContextActionKind::Disconnect,
    NodeGraphContextActionKind::DetachSource,
    NodeGraphContextActionKind::DetachTarget,
    NodeGraphContextActionKind::FrameSelection,
    NodeGraphContextActionKind::GroupSelection,
    NodeGraphContextActionKind::Ungroup,
    NodeGraphContextActionKind::SelectAll,
    NodeGraphContextActionKind::Paste,
];

/// Deterministic reason a node graph context action is unavailable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphContextActionUnavailableReason {
    /// The action does not apply to the addressed context target.
    UnsupportedTarget,
    /// The action requires a non-empty context selection.
    EmptySelection,
    /// The action requires at least one node in the context selection.
    NoSelectedNodes,
    /// The addressed target is not present in the current graph descriptor.
    MissingTarget,
    /// The addressed target is present but disabled.
    DisabledTarget,
    /// The addressed port endpoint has no connected edges.
    NoConnectedEdges,
    /// Link endpoint metadata could not be resolved.
    LinkEndpoint(NodeGraphLinkDraftEndpointError),
    /// Link edit metadata could not be resolved.
    LinkEdit(NodeGraphLinkEditRequestError),
    /// The UI contract needs application state before enabling this action.
    RequiresApplicationState,
}

/// Pure app-dispatch metadata for a node graph context action.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphContextAction {
    /// Operation kind the application may present.
    pub kind: NodeGraphContextActionKind,
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Whether the action can be dispatched using the attached request metadata.
    pub enabled: bool,
    /// App-owned request metadata for enabled actions.
    pub request: Option<NodeGraphContextActionRequest>,
    /// Deterministic reason for disabled or unavailable actions.
    pub unavailable_reason: Option<NodeGraphContextActionUnavailableReason>,
}

impl NodeGraphContextAction {
    fn available(
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        request: NodeGraphContextActionRequest,
    ) -> Self {
        Self {
            kind,
            target,
            enabled: true,
            request: Some(request),
            unavailable_reason: None,
        }
    }

    fn unavailable(
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        reason: NodeGraphContextActionUnavailableReason,
    ) -> Self {
        Self {
            kind,
            target,
            enabled: false,
            request: None,
            unavailable_reason: Some(reason),
        }
    }
}

/// Data-only application-owned context action request.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphContextActionRequest {
    /// Request to delete selected targets or the addressed frame/group target.
    Delete(NodeGraphContextSelectionRequest),
    /// Request to duplicate selected targets or the addressed frame/group target.
    Duplicate(NodeGraphContextSelectionRequest),
    /// Request to disconnect an edge or port endpoint.
    Disconnect(NodeGraphContextDisconnectRequest),
    /// Request to detach one endpoint from an existing edge.
    DetachEndpoint(NodeGraphContextDetachEndpointRequest),
    /// Request to organize selected targets into frames or groups.
    Organization(NodeGraphContextOrganizationRequest),
    /// Request for a canvas-scoped context action.
    Canvas(NodeGraphContextCanvasRequest),
}

/// Selection-aware context request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphContextSelectionRequest {
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Selected graph targets captured in deterministic order.
    pub selected_targets: Vec<NodeGraphSelectionTarget>,
}

/// Link-specific context request metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphContextDisconnectRequest {
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Edge or endpoint identity to disconnect.
    pub disconnect: NodeGraphContextDisconnectTarget,
}

/// Edge or endpoint identity captured by a disconnect request.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphContextDisconnectTarget {
    /// A resolved edge context.
    Edge(NodeGraphLinkEditEdgeContext),
    /// A port endpoint and the edge IDs currently referencing it.
    Endpoint {
        /// Stable endpoint identity.
        endpoint: PortEndpoint,
        /// Connected edge IDs in deterministic order.
        connected_edges: Vec<EdgeId>,
    },
}

/// Edge endpoint detach request metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphContextDetachEndpointRequest {
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Existing detach-link request metadata.
    pub request: NodeGraphDetachLinkRequest,
}

/// Organization operation represented by node graph context metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeGraphContextOrganizationOperation {
    /// Place selected nodes into a frame.
    FrameSelection,
    /// Place selected nodes into a group.
    GroupSelection,
    /// Ungroup the addressed group target.
    Ungroup,
}

/// Frame/group operation request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphContextOrganizationRequest {
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Organization operation requested.
    pub operation: NodeGraphContextOrganizationOperation,
    /// Selected graph targets captured in deterministic order.
    pub selected_targets: Vec<NodeGraphSelectionTarget>,
}

/// Canvas operation represented by node graph context metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeGraphContextCanvasOperation {
    /// Select all selectable graph targets.
    SelectAll,
    /// Paste through application-owned clipboard state.
    Paste,
}

/// Canvas action request metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphContextCanvasRequest {
    /// Target addressed by the context menu source.
    pub target: NodeGraphContextTarget,
    /// Canvas operation requested.
    pub operation: NodeGraphContextCanvasOperation,
    /// Selection snapshot at the time the context action was requested.
    pub selection: NodeGraphSelection,
    /// Selectable graph targets captured in deterministic order.
    pub selectable_targets: Vec<NodeGraphSelectionTarget>,
}

fn resolve_node_graph_context_actions(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Vec<NodeGraphContextAction> {
    DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS
        .into_iter()
        .map(|kind| node_graph_default_context_action(graph, kind, target, selection))
        .collect()
}

fn node_graph_default_context_action(
    graph: &NodeGraphDescriptor,
    kind: NodeGraphContextActionKind,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> NodeGraphContextAction {
    if kind == NodeGraphContextActionKind::Paste {
        return node_graph_default_paste_context_action(target);
    }

    node_graph_context_action(graph, kind, target, selection)
}

fn node_graph_context_action(
    graph: &NodeGraphDescriptor,
    kind: NodeGraphContextActionKind,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> NodeGraphContextAction {
    match node_graph_context_action_request(graph, kind, target, selection) {
        Ok(request) => NodeGraphContextAction::available(kind, target, request),
        Err(reason) => NodeGraphContextAction::unavailable(kind, target, reason),
    }
}

fn node_graph_context_action_request(
    graph: &NodeGraphDescriptor,
    kind: NodeGraphContextActionKind,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Result<NodeGraphContextActionRequest, NodeGraphContextActionUnavailableReason> {
    match kind {
        NodeGraphContextActionKind::Delete => {
            node_graph_context_selection_request(graph, target, selection)
                .map(NodeGraphContextActionRequest::Delete)
        }
        NodeGraphContextActionKind::Duplicate => {
            node_graph_context_selection_request(graph, target, selection)
                .map(NodeGraphContextActionRequest::Duplicate)
        }
        NodeGraphContextActionKind::Disconnect => {
            node_graph_disconnect_context_request(graph, target)
                .map(NodeGraphContextActionRequest::Disconnect)
        }
        NodeGraphContextActionKind::DetachSource => {
            node_graph_detach_context_request(graph, target, EdgeEndpointRole::Source)
                .map(NodeGraphContextActionRequest::DetachEndpoint)
        }
        NodeGraphContextActionKind::DetachTarget => {
            node_graph_detach_context_request(graph, target, EdgeEndpointRole::Target)
                .map(NodeGraphContextActionRequest::DetachEndpoint)
        }
        NodeGraphContextActionKind::FrameSelection => node_graph_organization_context_request(
            graph,
            target,
            selection,
            NodeGraphContextOrganizationOperation::FrameSelection,
        )
        .map(NodeGraphContextActionRequest::Organization),
        NodeGraphContextActionKind::GroupSelection => node_graph_organization_context_request(
            graph,
            target,
            selection,
            NodeGraphContextOrganizationOperation::GroupSelection,
        )
        .map(NodeGraphContextActionRequest::Organization),
        NodeGraphContextActionKind::Ungroup => node_graph_organization_context_request(
            graph,
            target,
            selection,
            NodeGraphContextOrganizationOperation::Ungroup,
        )
        .map(NodeGraphContextActionRequest::Organization),
        NodeGraphContextActionKind::SelectAll => {
            node_graph_select_all_context_request(graph, target, selection)
                .map(NodeGraphContextActionRequest::Canvas)
        }
        NodeGraphContextActionKind::Paste => node_graph_paste_context_request(target, selection)
            .map(NodeGraphContextActionRequest::Canvas),
    }
}

fn node_graph_context_selection_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Result<NodeGraphContextSelectionRequest, NodeGraphContextActionUnavailableReason> {
    let selected_targets = context_selected_targets(target, selection);
    if selected_targets.is_empty()
        && !matches!(
            target,
            NodeGraphContextTarget::Frame(_) | NodeGraphContextTarget::Group(_)
        )
    {
        return Err(NodeGraphContextActionUnavailableReason::EmptySelection);
    }
    if target != NodeGraphContextTarget::Canvas {
        validate_context_target_available(graph, target)?;
    }

    Ok(NodeGraphContextSelectionRequest {
        target,
        selected_targets,
    })
}

fn node_graph_disconnect_context_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
) -> Result<NodeGraphContextDisconnectRequest, NodeGraphContextActionUnavailableReason> {
    match target {
        NodeGraphContextTarget::Edge(edge) => {
            node_graph_edge_disconnect_request(graph, target, edge)
        }
        NodeGraphContextTarget::Port(endpoint) => {
            node_graph_endpoint_disconnect_request(graph, target, endpoint)
        }
        NodeGraphContextTarget::Node(_)
        | NodeGraphContextTarget::Reroute(_)
        | NodeGraphContextTarget::Frame(_)
        | NodeGraphContextTarget::Group(_)
        | NodeGraphContextTarget::Canvas => {
            Err(NodeGraphContextActionUnavailableReason::UnsupportedTarget)
        }
    }
}

fn node_graph_edge_disconnect_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    edge: EdgeId,
) -> Result<NodeGraphContextDisconnectRequest, NodeGraphContextActionUnavailableReason> {
    let request = NodeGraphLinkEditRequest::cut_edge(graph, edge)
        .map_err(NodeGraphContextActionUnavailableReason::LinkEdit)?;
    let NodeGraphLinkEditRequest::CutEdge(request) = request else {
        unreachable!("cut_edge only returns cut-edge requests");
    };
    if !request.edge.enabled {
        return Err(NodeGraphContextActionUnavailableReason::DisabledTarget);
    }

    Ok(NodeGraphContextDisconnectRequest {
        target,
        disconnect: NodeGraphContextDisconnectTarget::Edge(request.edge),
    })
}

fn node_graph_endpoint_disconnect_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    endpoint: PortEndpoint,
) -> Result<NodeGraphContextDisconnectRequest, NodeGraphContextActionUnavailableReason> {
    resolve_link_draft_endpoint(graph, endpoint)
        .map_err(NodeGraphContextActionUnavailableReason::LinkEndpoint)?;
    let connected_edges = connected_edge_ids_for_endpoint(graph, endpoint);
    if connected_edges.is_empty() {
        return Err(NodeGraphContextActionUnavailableReason::NoConnectedEdges);
    }

    Ok(NodeGraphContextDisconnectRequest {
        target,
        disconnect: NodeGraphContextDisconnectTarget::Endpoint {
            endpoint,
            connected_edges,
        },
    })
}

fn node_graph_detach_context_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    endpoint: EdgeEndpointRole,
) -> Result<NodeGraphContextDetachEndpointRequest, NodeGraphContextActionUnavailableReason> {
    let NodeGraphContextTarget::Edge(edge) = target else {
        return Err(NodeGraphContextActionUnavailableReason::UnsupportedTarget);
    };

    NodeGraphLinkEditRequest::detach_edge(graph, edge, endpoint)
        .map_err(NodeGraphContextActionUnavailableReason::LinkEdit)
        .and_then(|request| {
            let NodeGraphLinkEditRequest::DetachEdge(request) = request else {
                unreachable!("detach_edge only returns detach-edge requests");
            };
            request
                .edge
                .enabled
                .then_some(request)
                .ok_or(NodeGraphContextActionUnavailableReason::DisabledTarget)
        })
        .map(|request| NodeGraphContextDetachEndpointRequest { target, request })
}

fn node_graph_organization_context_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
    operation: NodeGraphContextOrganizationOperation,
) -> Result<NodeGraphContextOrganizationRequest, NodeGraphContextActionUnavailableReason> {
    match operation {
        NodeGraphContextOrganizationOperation::FrameSelection
        | NodeGraphContextOrganizationOperation::GroupSelection => {
            node_graph_selection_organization_request(graph, target, selection, operation)
        }
        NodeGraphContextOrganizationOperation::Ungroup => {
            node_graph_ungroup_context_request(graph, target)
        }
    }
}

fn node_graph_selection_organization_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
    operation: NodeGraphContextOrganizationOperation,
) -> Result<NodeGraphContextOrganizationRequest, NodeGraphContextActionUnavailableReason> {
    let selected_targets = context_selected_targets(target, selection);
    if !selected_targets
        .iter()
        .any(|target| matches!(target, NodeGraphSelectionTarget::Node(_)))
    {
        return Err(NodeGraphContextActionUnavailableReason::NoSelectedNodes);
    }
    if target != NodeGraphContextTarget::Canvas {
        validate_context_target_available(graph, target)?;
    }

    Ok(NodeGraphContextOrganizationRequest {
        target,
        operation,
        selected_targets,
    })
}

fn node_graph_ungroup_context_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
) -> Result<NodeGraphContextOrganizationRequest, NodeGraphContextActionUnavailableReason> {
    let NodeGraphContextTarget::Group(_) = target else {
        return Err(NodeGraphContextActionUnavailableReason::UnsupportedTarget);
    };
    validate_context_target_available(graph, target)?;

    Ok(NodeGraphContextOrganizationRequest {
        target,
        operation: NodeGraphContextOrganizationOperation::Ungroup,
        selected_targets: Vec::new(),
    })
}

fn node_graph_select_all_context_request(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
    if target != NodeGraphContextTarget::Canvas {
        return Err(NodeGraphContextActionUnavailableReason::UnsupportedTarget);
    }

    let selectable_targets = context_selectable_targets(graph);
    if selectable_targets.is_empty() {
        return Err(NodeGraphContextActionUnavailableReason::EmptySelection);
    }

    Ok(NodeGraphContextCanvasRequest {
        target,
        operation: NodeGraphContextCanvasOperation::SelectAll,
        selection: selection.clone(),
        selectable_targets,
    })
}

fn node_graph_default_paste_context_action(
    target: NodeGraphContextTarget,
) -> NodeGraphContextAction {
    if target != NodeGraphContextTarget::Canvas {
        return NodeGraphContextAction::unavailable(
            NodeGraphContextActionKind::Paste,
            target,
            NodeGraphContextActionUnavailableReason::UnsupportedTarget,
        );
    }

    NodeGraphContextAction::unavailable(
        NodeGraphContextActionKind::Paste,
        target,
        NodeGraphContextActionUnavailableReason::RequiresApplicationState,
    )
}

fn node_graph_paste_context_request(
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
    if target != NodeGraphContextTarget::Canvas {
        return Err(NodeGraphContextActionUnavailableReason::UnsupportedTarget);
    }

    Ok(NodeGraphContextCanvasRequest {
        target,
        operation: NodeGraphContextCanvasOperation::Paste,
        selection: selection.clone(),
        selectable_targets: Vec::new(),
    })
}

fn context_selected_targets(
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Vec<NodeGraphSelectionTarget> {
    if target == NodeGraphContextTarget::Canvas {
        return selection.selected();
    }

    let Some(selection_target) = target.selection_target() else {
        return Vec::new();
    };

    if selection.contains(selection_target) {
        selection.selected()
    } else {
        vec![selection_target]
    }
}

fn context_selectable_targets(graph: &NodeGraphDescriptor) -> Vec<NodeGraphSelectionTarget> {
    let mut targets = BTreeSet::new();
    for node in &graph.nodes {
        if !node.enabled {
            continue;
        }
        targets.insert(NodeGraphSelectionTarget::Node(node.id));
        for port in &node.ports {
            if port.enabled {
                targets.insert(NodeGraphSelectionTarget::Port(PortEndpoint::new(
                    node.id, port.id,
                )));
            }
        }
    }
    for edge in &graph.edges {
        if edge.enabled {
            targets.insert(NodeGraphSelectionTarget::Edge(edge.id));
        }
    }
    for reroute in &graph.reroutes {
        if reroute.enabled {
            targets.insert(NodeGraphSelectionTarget::Reroute(reroute.id));
        }
    }

    targets.into_iter().collect()
}

fn connected_edge_ids_for_endpoint(
    graph: &NodeGraphDescriptor,
    endpoint: PortEndpoint,
) -> Vec<EdgeId> {
    graph
        .edges
        .iter()
        .filter(|edge| edge.from == endpoint || edge.to == endpoint)
        .map(|edge| edge.id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn validate_context_target_available(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
) -> Result<(), NodeGraphContextActionUnavailableReason> {
    match target {
        NodeGraphContextTarget::Node(id) => graph
            .nodes
            .iter()
            .find(|node| node.id == id)
            .map(|node| node.enabled)
            .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
            .and_then(enabled_context_target),
        NodeGraphContextTarget::Edge(id) => graph
            .edges
            .iter()
            .find(|edge| edge.id == id)
            .map(|edge| edge.enabled)
            .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
            .and_then(enabled_context_target),
        NodeGraphContextTarget::Reroute(id) => graph
            .reroutes
            .iter()
            .find(|reroute| reroute.id == id)
            .map(|reroute| reroute.enabled)
            .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
            .and_then(enabled_context_target),
        NodeGraphContextTarget::Port(endpoint) => {
            let node = graph
                .nodes
                .iter()
                .find(|node| node.id == endpoint.node)
                .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)?;
            if !node.enabled {
                return Err(NodeGraphContextActionUnavailableReason::DisabledTarget);
            }
            node.ports
                .iter()
                .find(|port| port.id == endpoint.port)
                .map(|port| port.enabled)
                .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
                .and_then(enabled_context_target)
        }
        NodeGraphContextTarget::Frame(id) => graph
            .frames
            .iter()
            .find(|frame| frame.id == id)
            .map(|frame| frame.enabled)
            .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
            .and_then(enabled_context_target),
        NodeGraphContextTarget::Group(id) => graph
            .groups
            .iter()
            .find(|group| group.id == id)
            .map(|group| group.enabled)
            .ok_or(NodeGraphContextActionUnavailableReason::MissingTarget)
            .and_then(enabled_context_target),
        NodeGraphContextTarget::Canvas => Ok(()),
    }
}

fn enabled_context_target(enabled: bool) -> Result<(), NodeGraphContextActionUnavailableReason> {
    enabled
        .then_some(())
        .ok_or(NodeGraphContextActionUnavailableReason::DisabledTarget)
}

/// Data-only descriptor for an add-node search entry.
///
/// This is intentionally application-owned metadata. It identifies a node kind
/// the application may create later, but it does not create or mutate graph
/// nodes by itself.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchEntry {
    /// Stable application-owned descriptor identity.
    pub id: NodeGraphAddNodeDescriptorId,
    /// User-facing entry label.
    pub label: String,
    /// Optional user-facing category used for deterministic filtering.
    pub category: Option<String>,
    /// Optional user-facing description used for deterministic filtering.
    pub description: Option<String>,
    /// Additional application-owned search keywords.
    pub keywords: Vec<String>,
    /// Whether this entry may currently be selected.
    pub enabled: bool,
}

impl NodeGraphAddNodeSearchEntry {
    /// Creates an enabled add-node search entry.
    #[must_use]
    pub fn new(id: NodeGraphAddNodeDescriptorId, label: impl Into<String>) -> Self {
        Self {
            id,
            label: label.into(),
            category: None,
            description: None,
            keywords: Vec::new(),
            enabled: true,
        }
    }

    /// Sets the entry category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Sets the entry description.
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets additional search keywords.
    #[must_use]
    pub fn with_keywords<I, S>(mut self, keywords: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.keywords = keywords.into_iter().map(Into::into).collect();
        self
    }

    /// Sets whether the entry may currently be selected.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    /// Returns true when this entry matches every non-empty query term.
    #[must_use]
    pub fn matches_query(&self, query: &str) -> bool {
        let terms = node_graph_add_node_query_terms(query);
        node_graph_add_node_entry_matches_terms(self, &terms)
    }

    /// Returns the first deterministic label highlight range for a query.
    ///
    /// Empty queries or matches found only in category, description, or
    /// keywords return `None`.
    #[must_use]
    pub fn label_highlight(&self, query: &str) -> Option<NodeGraphAddNodeSearchHighlight> {
        let terms = node_graph_add_node_query_terms(query);
        node_graph_add_node_label_highlight(&self.label, &terms)
    }
}

/// Byte range in an add-node search entry label that should be highlighted.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchHighlight {
    /// Inclusive byte offset where the highlight begins.
    pub start: usize,
    /// Exclusive byte offset where the highlight ends.
    pub end: usize,
}

impl NodeGraphAddNodeSearchHighlight {
    /// Creates a label highlight range.
    #[must_use]
    pub const fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }
}

/// One deterministic add-node search match.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchMatch<'a> {
    /// Matched descriptor.
    pub entry: &'a NodeGraphAddNodeSearchEntry,
    /// First matching label range, if the query matched the label.
    pub label_highlight: Option<NodeGraphAddNodeSearchHighlight>,
}

/// Filters add-node search descriptors in input order.
///
/// Empty or whitespace-only queries return every entry. Disabled entries remain
/// visible in results; selection helpers skip them.
#[must_use]
pub fn filter_node_graph_add_node_search_entries<'a>(
    entries: &'a [NodeGraphAddNodeSearchEntry],
    query: &str,
) -> Vec<NodeGraphAddNodeSearchMatch<'a>> {
    let terms = node_graph_add_node_query_terms(query);
    entries
        .iter()
        .filter(|entry| node_graph_add_node_entry_matches_terms(entry, &terms))
        .map(|entry| NodeGraphAddNodeSearchMatch {
            entry,
            label_highlight: node_graph_add_node_label_highlight(&entry.label, &terms),
        })
        .collect()
}

/// Selection metadata for add-node search results.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NodeGraphAddNodeSearchSelection {
    /// Selected application-owned add-node descriptor ID.
    pub selected: Option<NodeGraphAddNodeDescriptorId>,
}

impl NodeGraphAddNodeSearchSelection {
    /// Creates an empty add-node search selection.
    #[must_use]
    pub const fn new() -> Self {
        Self { selected: None }
    }

    /// Creates a selection from an existing descriptor ID.
    #[must_use]
    pub const fn from_selected(selected: NodeGraphAddNodeDescriptorId) -> Self {
        Self {
            selected: Some(selected),
        }
    }

    /// Selects the first enabled entry matching the query.
    #[must_use]
    pub fn select_first(entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        Self {
            selected: enabled_add_node_search_matches(entries, query)
                .first()
                .map(|result| result.entry.id),
        }
    }

    /// Selects the next enabled entry matching the query, wrapping at the end.
    #[must_use]
    pub fn select_next(&self, entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        let matches = enabled_add_node_search_matches(entries, query);
        let Some(selected) = next_add_node_search_selection(
            &matches,
            self.selected,
            AddNodeSearchSelectionDirection::Next,
        ) else {
            return Self::new();
        };

        Self {
            selected: Some(selected),
        }
    }

    /// Selects the previous enabled entry matching the query, wrapping at the start.
    #[must_use]
    pub fn select_previous(&self, entries: &[NodeGraphAddNodeSearchEntry], query: &str) -> Self {
        let matches = enabled_add_node_search_matches(entries, query);
        let Some(selected) = next_add_node_search_selection(
            &matches,
            self.selected,
            AddNodeSearchSelectionDirection::Previous,
        ) else {
            return Self::new();
        };

        Self {
            selected: Some(selected),
        }
    }

    /// Returns the selected enabled search match, if it is still present.
    #[must_use]
    pub fn selected_entry<'a>(
        &self,
        entries: &'a [NodeGraphAddNodeSearchEntry],
        query: &str,
    ) -> Option<NodeGraphAddNodeSearchMatch<'a>> {
        let selected = self.selected?;
        enabled_add_node_search_matches(entries, query)
            .into_iter()
            .find(|result| result.entry.id == selected)
    }

    /// Emits application-owned add-node request metadata for the selected entry.
    #[must_use]
    pub fn add_request(
        &self,
        entries: &[NodeGraphAddNodeSearchEntry],
        query: &str,
        insertion_point: GraphPoint,
    ) -> Option<NodeGraphAddNodeRequest> {
        let selected = self.selected_entry(entries, query)?.entry.id;
        Some(NodeGraphAddNodeRequest::new(selected, insertion_point))
    }
}

/// Application-owned request metadata for adding a node at a graph-space point.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphAddNodeRequest {
    /// Selected application-owned add-node descriptor ID.
    pub descriptor_id: NodeGraphAddNodeDescriptorId,
    /// Graph-space insertion point, sanitized to finite coordinates.
    pub insertion_point: GraphPoint,
}

impl NodeGraphAddNodeRequest {
    /// Creates add-node request metadata.
    #[must_use]
    pub fn new(descriptor_id: NodeGraphAddNodeDescriptorId, insertion_point: GraphPoint) -> Self {
        Self {
            descriptor_id,
            insertion_point: insertion_point.sanitized(),
        }
    }
}

fn node_graph_add_node_query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(str::to_ascii_lowercase)
        .filter(|term| !term.is_empty())
        .collect()
}

fn node_graph_add_node_entry_matches_terms(
    entry: &NodeGraphAddNodeSearchEntry,
    terms: &[String],
) -> bool {
    terms
        .iter()
        .all(|term| node_graph_add_node_entry_contains_term(entry, term))
}

fn node_graph_add_node_entry_contains_term(
    entry: &NodeGraphAddNodeSearchEntry,
    term: &str,
) -> bool {
    find_ascii_case_insensitive(&entry.label, term).is_some()
        || entry
            .category
            .as_deref()
            .is_some_and(|category| find_ascii_case_insensitive(category, term).is_some())
        || entry
            .description
            .as_deref()
            .is_some_and(|description| find_ascii_case_insensitive(description, term).is_some())
        || entry
            .keywords
            .iter()
            .any(|keyword| find_ascii_case_insensitive(keyword, term).is_some())
}

fn node_graph_add_node_label_highlight(
    label: &str,
    terms: &[String],
) -> Option<NodeGraphAddNodeSearchHighlight> {
    terms
        .iter()
        .filter_map(|term| {
            find_ascii_case_insensitive(label, term)
                .map(|(start, end)| NodeGraphAddNodeSearchHighlight::new(start, end))
        })
        .min_by_key(|highlight| (highlight.start, highlight.end))
}

fn enabled_add_node_search_matches<'a>(
    entries: &'a [NodeGraphAddNodeSearchEntry],
    query: &str,
) -> Vec<NodeGraphAddNodeSearchMatch<'a>> {
    filter_node_graph_add_node_search_entries(entries, query)
        .into_iter()
        .filter(|result| result.entry.enabled)
        .collect()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AddNodeSearchSelectionDirection {
    Next,
    Previous,
}

fn next_add_node_search_selection(
    matches: &[NodeGraphAddNodeSearchMatch<'_>],
    selected: Option<NodeGraphAddNodeDescriptorId>,
    direction: AddNodeSearchSelectionDirection,
) -> Option<NodeGraphAddNodeDescriptorId> {
    let len = matches.len();
    if len == 0 {
        return None;
    }

    let selected_index = selected.and_then(|selected| {
        matches
            .iter()
            .position(|result| result.entry.id == selected)
    });
    let next_index = match (selected_index, direction) {
        (Some(index), AddNodeSearchSelectionDirection::Next) => (index + 1) % len,
        (Some(0) | None, AddNodeSearchSelectionDirection::Previous) => len - 1,
        (Some(index), AddNodeSearchSelectionDirection::Previous) => index - 1,
        (None, AddNodeSearchSelectionDirection::Next) => 0,
    };

    Some(matches[next_index].entry.id)
}

fn find_ascii_case_insensitive(haystack: &str, needle: &str) -> Option<(usize, usize)> {
    if needle.is_empty() {
        return Some((0, 0));
    }

    let haystack = haystack.as_bytes();
    let needle = needle.as_bytes();
    haystack
        .windows(needle.len())
        .position(|window| window.eq_ignore_ascii_case(needle))
        .map(|start| (start, start + needle.len()))
}

/// Resolved data-only metadata for a link draft endpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkDraftEndpoint {
    /// Stable port endpoint.
    pub endpoint: PortEndpoint,
    /// Directed port flow.
    pub direction: PortDirection,
    /// Application-owned compatibility key.
    pub port_type: PortTypeId,
    /// Graph-space anchor for backend-independent draft drawing.
    pub anchor: GraphPoint,
}

/// Structured endpoint resolution failure for link draft metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkDraftEndpointError {
    /// Descriptor validation failed before endpoint resolution could run.
    Validation(NodeGraphValidationError),
    /// The endpoint references a missing node.
    MissingNode {
        /// Missing node ID.
        node: NodeId,
    },
    /// The endpoint references a missing port on an existing node.
    MissingPort {
        /// Existing node ID.
        node: NodeId,
        /// Missing port ID.
        port: PortId,
    },
    /// The owning node exists but is disabled.
    DisabledNode {
        /// Disabled node ID.
        node: NodeId,
    },
    /// The endpoint exists but its port is disabled.
    DisabledPort {
        /// Owning node ID.
        node: NodeId,
        /// Disabled port ID.
        port: PortId,
    },
}

impl From<NodeGraphValidationError> for NodeGraphLinkDraftEndpointError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Hover target metadata for a link draft.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkDraftTarget {
    /// The current hover hit is not a port target.
    Hit(NodeGraphHitTarget),
    /// The current hover hit is a resolved port target.
    Port(NodeGraphLinkDraftPortTarget),
}

impl NodeGraphLinkDraftTarget {
    /// Returns the underlying hit target.
    #[must_use]
    pub const fn hit_target(&self) -> NodeGraphHitTarget {
        match self {
            Self::Hit(target) => *target,
            Self::Port(target) => NodeGraphHitTarget::Port(target.endpoint.endpoint),
        }
    }

    /// Returns true when the target is a compatible completion target.
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::Port(target) if target.compatibility.is_ok())
    }
}

/// Resolved hover port metadata for a link draft.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftPortTarget {
    /// Resolved endpoint under the current pointer.
    pub endpoint: NodeGraphLinkDraftEndpoint,
    /// Generic directed compatibility result against the draft start endpoint.
    pub compatibility: Result<(), PortCompatibilityError>,
}

/// Structured hover target resolution failure for link drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkDraftTargetError {
    /// Hit testing failed before a hover target could be resolved.
    HitTest(NodeGraphHitTestError),
    /// A hit port target could not be resolved to endpoint metadata.
    Endpoint(NodeGraphLinkDraftEndpointError),
}

impl From<NodeGraphHitTestError> for NodeGraphLinkDraftTargetError {
    fn from(error: NodeGraphHitTestError) -> Self {
        Self::HitTest(error)
    }
}

impl From<NodeGraphLinkDraftEndpointError> for NodeGraphLinkDraftTargetError {
    fn from(error: NodeGraphLinkDraftEndpointError) -> Self {
        Self::Endpoint(error)
    }
}

/// Data-only application-owned link draft state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraft {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, once resolved against a viewport.
    pub current_graph_point: Option<GraphPoint>,
    /// Current hover target metadata.
    pub target: NodeGraphLinkDraftTarget,
}

impl NodeGraphLinkDraft {
    /// Starts a link draft from an enabled endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured endpoint error when descriptors are invalid, the
    /// endpoint is stale, or the endpoint is disabled.
    pub fn start(
        graph: &NodeGraphDescriptor,
        start: PortEndpoint,
        current_pointer: Point,
    ) -> Result<Self, NodeGraphLinkDraftEndpointError> {
        Ok(Self {
            start: resolve_link_draft_endpoint(graph, start)?,
            current_pointer: sanitize_point(current_pointer),
            current_graph_point: None,
            target: NodeGraphLinkDraftTarget::Hit(NodeGraphHitTarget::Canvas),
        })
    }

    /// Resolves current hover target metadata using default node graph hit testing.
    ///
    /// # Errors
    ///
    /// Returns a structured target error when hit testing or endpoint
    /// resolution fails.
    pub fn resolve_hover_target(
        &self,
        graph: &NodeGraphDescriptor,
        viewport: NodeGraphViewport,
        current_pointer: Point,
    ) -> Result<Self, NodeGraphLinkDraftTargetError> {
        self.resolve_hover_target_with_config(
            graph,
            viewport,
            current_pointer,
            NodeGraphHitTestConfig::default(),
        )
    }

    /// Resolves current hover target metadata with explicit hit test geometry.
    ///
    /// # Errors
    ///
    /// Returns a structured target error when hit testing or endpoint
    /// resolution fails.
    pub fn resolve_hover_target_with_config(
        &self,
        graph: &NodeGraphDescriptor,
        viewport: NodeGraphViewport,
        current_pointer: Point,
        config: NodeGraphHitTestConfig,
    ) -> Result<Self, NodeGraphLinkDraftTargetError> {
        let current_pointer = sanitize_point(current_pointer);
        let hit = graph.hit_test_with_config(viewport, current_pointer, config)?;
        let target = match hit {
            NodeGraphHitTarget::Port(endpoint) => {
                let endpoint = resolve_link_draft_endpoint(graph, endpoint)?;
                NodeGraphLinkDraftTarget::Port(NodeGraphLinkDraftPortTarget {
                    endpoint,
                    compatibility: link_draft_compatibility(self.start, endpoint),
                })
            }
            target => NodeGraphLinkDraftTarget::Hit(target),
        };

        Ok(Self {
            start: self.start,
            current_pointer,
            current_graph_point: Some(viewport.screen_to_graph(current_pointer)),
            target,
        })
    }

    /// Returns a deterministic cancel outcome without mutating graph descriptors.
    #[must_use]
    pub fn cancel(&self) -> NodeGraphLinkDraftOutcome {
        NodeGraphLinkDraftOutcome::Cancelled(NodeGraphLinkDraftCancelled {
            start: self.start,
            current_pointer: self.current_pointer,
            current_graph_point: self.current_graph_point,
            target: self.target.clone(),
        })
    }

    /// Returns a deterministic completion or rejection outcome without mutating graph descriptors.
    #[must_use]
    pub fn complete(&self) -> NodeGraphLinkDraftOutcome {
        let NodeGraphLinkDraftTarget::Port(target) = &self.target else {
            return NodeGraphLinkDraftOutcome::Rejected(NodeGraphLinkDraftRejected {
                start: self.start,
                current_pointer: self.current_pointer,
                current_graph_point: self.current_graph_point,
                target: self.target.clone(),
                error: NodeGraphLinkDraftCompletionError::NoPortTarget {
                    target: self.target.hit_target(),
                },
            });
        };

        if let Err(error) = target.compatibility {
            return NodeGraphLinkDraftOutcome::Rejected(NodeGraphLinkDraftRejected {
                start: self.start,
                current_pointer: self.current_pointer,
                current_graph_point: self.current_graph_point,
                target: self.target.clone(),
                error: NodeGraphLinkDraftCompletionError::IncompatiblePort {
                    target: target.endpoint,
                    error,
                },
            });
        }

        let (from, to) = if self.start.direction == PortDirection::Output {
            (self.start, target.endpoint)
        } else {
            (target.endpoint, self.start)
        };

        NodeGraphLinkDraftOutcome::Completed(NodeGraphLinkDraftCompleted {
            from,
            to,
            current_pointer: self.current_pointer,
            current_graph_point: self.current_graph_point,
        })
    }
}

/// Deterministic link draft outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkDraftOutcome {
    /// The draft was cancelled.
    Cancelled(NodeGraphLinkDraftCancelled),
    /// The draft completed with a compatible output-to-input endpoint pair.
    Completed(NodeGraphLinkDraftCompleted),
    /// The draft could not complete.
    Rejected(NodeGraphLinkDraftRejected),
}

/// Deterministic link draft cancel metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftCancelled {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
    /// Last resolved hover target.
    pub target: NodeGraphLinkDraftTarget,
}

/// Deterministic link draft completion metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkDraftCompleted {
    /// Canonical output endpoint.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Canonical input endpoint.
    pub to: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
}

/// Deterministic link draft rejection metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftRejected {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
    /// Last resolved hover target.
    pub target: NodeGraphLinkDraftTarget,
    /// Reason completion was rejected.
    pub error: NodeGraphLinkDraftCompletionError,
}

/// Structured link draft completion failure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeGraphLinkDraftCompletionError {
    /// The current hover target is not a port.
    NoPortTarget {
        /// Current non-port hit target.
        target: NodeGraphHitTarget,
    },
    /// The current hover port is not compatible with the draft start endpoint.
    IncompatiblePort {
        /// Current hover endpoint metadata.
        target: NodeGraphLinkDraftEndpoint,
        /// Generic directed compatibility failure.
        error: PortCompatibilityError,
    },
}

/// Data-only application-owned link edit request.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkEditRequest {
    /// Request to create a new link between two compatible endpoints.
    CreateLink(NodeGraphCreateLinkRequest),
    /// Request to reconnect an existing edge source endpoint.
    ReconnectSource(NodeGraphReconnectLinkSourceRequest),
    /// Request to reconnect an existing edge target endpoint.
    ReconnectTarget(NodeGraphReconnectLinkTargetRequest),
    /// Request to detach one endpoint from an existing edge.
    DetachEdge(NodeGraphDetachLinkRequest),
    /// Request to cut an existing edge.
    CutEdge(NodeGraphCutLinkRequest),
}

impl NodeGraphLinkEditRequest {
    /// Creates metadata for a new app-owned link creation request.
    ///
    /// # Errors
    ///
    /// Returns a structured error when either endpoint cannot be resolved or
    /// the endpoints are not a compatible output-to-input pair.
    pub fn create_link(
        graph: &NodeGraphDescriptor,
        from: PortEndpoint,
        to: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let from = resolve_link_edit_request_endpoint(graph, from)?;
        let to = resolve_link_edit_request_endpoint(graph, to)?;
        validate_link_edit_compatibility(from, to)?;

        Ok(Self::CreateLink(NodeGraphCreateLinkRequest { from, to }))
    }

    /// Creates metadata for reconnecting an edge source endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current target.
    pub fn reconnect_source(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        new_source: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let new_source = resolve_link_edit_request_endpoint(graph, new_source)?;
        validate_link_edit_compatibility(new_source, edge.to)?;

        Ok(Self::ReconnectSource(NodeGraphReconnectLinkSourceRequest {
            edge,
            old_source: edge.from,
            new_source,
            target: edge.to,
        }))
    }

    /// Creates metadata for reconnecting an edge target endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current source.
    pub fn reconnect_target(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        new_target: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let new_target = resolve_link_edit_request_endpoint(graph, new_target)?;
        validate_link_edit_compatibility(edge.from, new_target)?;

        Ok(Self::ReconnectTarget(NodeGraphReconnectLinkTargetRequest {
            edge,
            source: edge.from,
            old_target: edge.to,
            new_target,
        }))
    }

    /// Creates metadata for detaching one endpoint from an existing edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn detach_edge(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        detached: EdgeEndpointRole,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let endpoint = match detached {
            EdgeEndpointRole::Source => edge.from,
            EdgeEndpointRole::Target => edge.to,
        };

        Ok(Self::DetachEdge(NodeGraphDetachLinkRequest {
            edge,
            detached,
            endpoint,
        }))
    }

    /// Creates metadata for cutting an existing edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn cut_edge(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        Ok(Self::CutEdge(NodeGraphCutLinkRequest {
            edge: resolve_link_edit_edge(graph, edge)?,
        }))
    }
}

/// Resolved edge context captured by link edit requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkEditEdgeContext {
    /// Stable edge identity.
    pub edge: EdgeId,
    /// Current source endpoint metadata.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Current target endpoint metadata.
    pub to: NodeGraphLinkDraftEndpoint,
    /// Whether the edge is currently enabled.
    pub enabled: bool,
}

/// Metadata for an app-owned create-link request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphCreateLinkRequest {
    /// Requested source endpoint.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Requested target endpoint.
    pub to: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned reconnect-source request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphReconnectLinkSourceRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Current source endpoint before reconnect.
    pub old_source: NodeGraphLinkDraftEndpoint,
    /// Requested replacement source endpoint.
    pub new_source: NodeGraphLinkDraftEndpoint,
    /// Unchanged target endpoint.
    pub target: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned reconnect-target request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphReconnectLinkTargetRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Unchanged source endpoint.
    pub source: NodeGraphLinkDraftEndpoint,
    /// Current target endpoint before reconnect.
    pub old_target: NodeGraphLinkDraftEndpoint,
    /// Requested replacement target endpoint.
    pub new_target: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned detach-edge request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphDetachLinkRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Endpoint role to detach.
    pub detached: EdgeEndpointRole,
    /// Endpoint metadata for the detached side.
    pub endpoint: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned cut-edge request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphCutLinkRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
}

/// Structured link edit request creation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkEditRequestError {
    /// Descriptor validation failed before request creation could run.
    Validation(NodeGraphValidationError),
    /// A requested edge does not exist.
    MissingEdge {
        /// Missing edge ID.
        edge: EdgeId,
    },
    /// Existing edge context could not be resolved.
    Edge(EdgeResolutionError),
    /// A requested replacement endpoint could not be resolved.
    Endpoint(NodeGraphLinkDraftEndpointError),
    /// The requested output-to-input pair is not generically compatible.
    IncompatiblePort {
        /// Requested source endpoint.
        from: PortEndpoint,
        /// Requested target endpoint.
        to: PortEndpoint,
        /// Generic directed compatibility failure.
        error: PortCompatibilityError,
    },
}

impl From<NodeGraphValidationError> for NodeGraphLinkEditRequestError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<EdgeResolutionError> for NodeGraphLinkEditRequestError {
    fn from(error: EdgeResolutionError) -> Self {
        Self::Edge(error)
    }
}

impl From<NodeGraphLinkDraftEndpointError> for NodeGraphLinkEditRequestError {
    fn from(error: NodeGraphLinkDraftEndpointError) -> Self {
        Self::Endpoint(error)
    }
}

/// Geometry mode used for node graph box selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphBoxSelectionMode {
    /// Select only nodes fully contained by the box.
    Contains,
    /// Select nodes that overlap the box at all.
    Intersects,
}

/// Selection change intent for a node graph box selection request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphSelectionIntent {
    /// Replace the current selection with the box selection.
    Replace,
    /// Add the box selection to the current selection.
    Add,
    /// Remove the box selection from the current selection.
    Subtract,
}

/// Data-only metadata for one node graph box selection request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphBoxSelectionRequest {
    /// Sanitized UI logical screen-space rectangle.
    pub screen_rect: Rect,
    /// Box rectangle converted to graph space.
    pub graph_rect: GraphRect,
    /// Node inclusion mode.
    pub mode: NodeGraphBoxSelectionMode,
    /// Selection change intent.
    pub intent: NodeGraphSelectionIntent,
}

impl NodeGraphBoxSelectionRequest {
    /// Creates box selection metadata from a screen-space rectangle.
    #[must_use]
    pub fn new(
        viewport: NodeGraphViewport,
        screen_rect: Rect,
        mode: NodeGraphBoxSelectionMode,
        intent: NodeGraphSelectionIntent,
    ) -> Self {
        let screen_rect = normalize_screen_rect(screen_rect);
        let graph_min = viewport.screen_to_graph(screen_rect.origin());
        let graph_max =
            viewport.screen_to_graph(Point::new(screen_rect.max_x(), screen_rect.max_y()));
        Self {
            screen_rect,
            graph_rect: GraphRect::from_min_max(graph_min, graph_max),
            mode,
            intent,
        }
    }

    /// Creates box selection metadata from an already graph-space rectangle.
    #[must_use]
    pub fn from_graph_rect(
        graph_rect: GraphRect,
        mode: NodeGraphBoxSelectionMode,
        intent: NodeGraphSelectionIntent,
    ) -> Self {
        let graph_rect = if graph_rect.x.is_finite()
            && graph_rect.y.is_finite()
            && graph_rect.width.is_finite()
            && graph_rect.height.is_finite()
        {
            graph_rect.sanitized()
        } else {
            GraphRect::ZERO
        };

        Self {
            screen_rect: Rect::ZERO,
            graph_rect,
            mode,
            intent,
        }
    }

    /// Returns true when the request contains no selectable area.
    #[must_use]
    pub fn is_empty(self) -> bool {
        self.graph_rect.sanitized().is_empty()
    }

    /// Returns true when a node rectangle matches this request's geometry mode.
    #[must_use]
    pub fn matches_node_rect(self, rect: GraphRect) -> bool {
        match self.mode {
            NodeGraphBoxSelectionMode::Contains => self.graph_rect.contains_rect(rect),
            NodeGraphBoxSelectionMode::Intersects => self.graph_rect.intersects_rect(rect),
        }
    }

    /// Resolves this request against graph nodes without mutating graph state.
    #[must_use]
    pub fn select(self, graph: &NodeGraphDescriptor) -> NodeGraphBoxSelection {
        let targets = graph
            .nodes
            .iter()
            .filter(|node| node.enabled && self.matches_node_rect(node.rect))
            .map(|node| NodeGraphSelectionTarget::Node(node.id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let operations = box_selection_operations(self.intent, &targets);

        NodeGraphBoxSelection {
            request: self,
            targets,
            operations,
        }
    }
}

/// Data-only output metadata for one node graph box selection request.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphBoxSelection {
    /// Request metadata used to derive this output.
    pub request: NodeGraphBoxSelectionRequest,
    /// Matching selectable targets in deterministic order.
    pub targets: Vec<NodeGraphSelectionTarget>,
    /// Selection operations that represent the requested change.
    pub operations: Vec<NodeGraphSelectionOperation>,
}

impl NodeGraphBoxSelection {
    /// Returns true when the request would not alter selection through operations.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.operations.is_empty()
    }
}

/// Metadata for one selected node move candidate.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphNodeMove {
    /// Node to move.
    pub node: NodeId,
    /// Graph-space movement delta for this node.
    pub delta: GraphVector,
}

/// Data-only request metadata for moving the currently selected nodes.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphSelectedNodeMoveRequest {
    /// Selection snapshot used to derive this request.
    pub selection: NodeGraphSelection,
    /// Sanitized UI logical screen-space drag delta.
    pub screen_delta: GraphVector,
    /// Sanitized graph-space drag delta.
    pub graph_delta: GraphVector,
    /// Per-node move candidates in deterministic selected-node order.
    pub nodes: Vec<NodeGraphNodeMove>,
}

impl NodeGraphSelectedNodeMoveRequest {
    /// Creates selected-node move request metadata from a viewport and selection.
    #[must_use]
    pub fn new(
        viewport: NodeGraphViewport,
        selection: NodeGraphSelection,
        screen_delta: GraphVector,
    ) -> Self {
        let screen_delta = screen_delta.sanitized();
        let graph_delta = node_graph_drag_delta(viewport, screen_delta);
        let nodes = selection
            .selected_nodes()
            .into_iter()
            .map(|node| NodeGraphNodeMove {
                node,
                delta: graph_delta,
            })
            .collect();

        Self {
            selection,
            screen_delta,
            graph_delta,
            nodes,
        }
    }

    /// Returns true when the request has no node movement to apply.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.nodes.is_empty() || self.graph_delta == GraphVector::ZERO
    }
}

/// Data-only request metadata for panning the graph canvas.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphCanvasPanRequest {
    /// Selection snapshot preserved while panning.
    pub selection: NodeGraphSelection,
    /// Sanitized UI logical screen-space drag delta.
    pub screen_delta: GraphVector,
    /// Screen-space pan delta to apply to the viewport pan offset.
    pub pan_delta: GraphVector,
}

impl NodeGraphCanvasPanRequest {
    /// Creates canvas pan request metadata.
    #[must_use]
    pub fn new(selection: NodeGraphSelection, screen_delta: GraphVector) -> Self {
        let screen_delta = screen_delta.sanitized();
        Self {
            selection,
            screen_delta,
            pan_delta: screen_delta,
        }
    }

    /// Returns a new pan/zoom state with this request's pan delta applied.
    #[must_use]
    pub fn next_pan_zoom(&self, pan_zoom: NodeGraphPanZoom) -> NodeGraphPanZoom {
        let mut pan_zoom = pan_zoom.sanitized();
        pan_zoom.pan_by(self.pan_delta);
        pan_zoom
    }

    /// Returns true when the request has no viewport pan to apply.
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.pan_delta == GraphVector::ZERO
    }
}

/// Converts a node drag delta from UI logical screen space to graph space.
#[must_use]
pub fn node_graph_drag_delta(
    viewport: NodeGraphViewport,
    screen_delta: GraphVector,
) -> GraphVector {
    viewport.screen_delta_to_graph(screen_delta)
}

/// Snaps a graph-space point to the nearest grid intersection.
#[must_use]
pub fn node_graph_snap_point(point: GraphPoint, grid_size: f32) -> GraphPoint {
    let point = point.sanitized();
    let Some(grid_size) = effective_snap_grid_size(grid_size) else {
        return point;
    };

    GraphPoint::new(
        snap_graph_component(point.x, grid_size),
        snap_graph_component(point.y, grid_size),
    )
}

/// Snaps a graph-space rectangle's origin and size to the nearest grid units.
#[must_use]
pub fn node_graph_snap_rect(rect: GraphRect, grid_size: f32) -> GraphRect {
    let rect = rect.sanitized();
    let Some(grid_size) = effective_snap_grid_size(grid_size) else {
        return rect;
    };

    GraphRect::new(
        snap_graph_component(rect.x, grid_size),
        snap_graph_component(rect.y, grid_size),
        snap_graph_component(rect.width, grid_size).max(0.0),
        snap_graph_component(rect.height, grid_size).max(0.0),
    )
}

/// Snaps a graph-space movement delta to the nearest grid units.
#[must_use]
pub fn node_graph_snap_delta(delta: GraphVector, grid_size: f32) -> GraphVector {
    let delta = delta.sanitized();
    let Some(grid_size) = effective_snap_grid_size(grid_size) else {
        return delta;
    };

    GraphVector::new(
        snap_graph_component(delta.x, grid_size),
        snap_graph_component(delta.y, grid_size),
    )
}

/// Deterministic node graph hit test geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphHitTestConfig {
    /// Screen-space tolerance for edge segment hits.
    pub edge_tolerance: f32,
    /// Screen-space square size for port hits.
    pub port_size: f32,
    /// Screen-space square size for reroute hits.
    pub reroute_size: f32,
    /// Graph-space height of the title target within each node.
    pub title_bar_height: f32,
}

impl Default for NodeGraphHitTestConfig {
    fn default() -> Self {
        Self {
            edge_tolerance: DEFAULT_NODE_GRAPH_EDGE_HIT_TOLERANCE,
            port_size: DEFAULT_NODE_GRAPH_PORT_HIT_SIZE,
            reroute_size: DEFAULT_NODE_GRAPH_REROUTE_HIT_SIZE,
            title_bar_height: DEFAULT_NODE_GRAPH_TITLE_BAR_HEIGHT,
        }
    }
}

impl NodeGraphHitTestConfig {
    /// Creates a hit test configuration using deterministic defaults.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            edge_tolerance: DEFAULT_NODE_GRAPH_EDGE_HIT_TOLERANCE,
            port_size: DEFAULT_NODE_GRAPH_PORT_HIT_SIZE,
            reroute_size: DEFAULT_NODE_GRAPH_REROUTE_HIT_SIZE,
            title_bar_height: DEFAULT_NODE_GRAPH_TITLE_BAR_HEIGHT,
        }
    }

    /// Sets the edge hit tolerance in screen logical units.
    #[must_use]
    pub const fn with_edge_tolerance(mut self, edge_tolerance: f32) -> Self {
        self.edge_tolerance = edge_tolerance;
        self
    }

    /// Sets the port hit square size in screen logical units.
    #[must_use]
    pub const fn with_port_size(mut self, port_size: f32) -> Self {
        self.port_size = port_size;
        self
    }

    /// Sets the reroute hit square size in screen logical units.
    #[must_use]
    pub const fn with_reroute_size(mut self, reroute_size: f32) -> Self {
        self.reroute_size = reroute_size;
        self
    }

    /// Sets the node title hit height in graph units.
    #[must_use]
    pub const fn with_title_bar_height(mut self, title_bar_height: f32) -> Self {
        self.title_bar_height = title_bar_height;
        self
    }

    fn sanitized(self) -> Self {
        Self {
            edge_tolerance: finite_non_negative(self.edge_tolerance),
            port_size: finite_non_negative(self.port_size),
            reroute_size: finite_non_negative(self.reroute_size),
            title_bar_height: finite_non_negative(self.title_bar_height),
        }
    }
}

/// Structured node graph hit test failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphHitTestError {
    /// Descriptor validation failed before hit testing could run.
    Validation(NodeGraphValidationError),
    /// Edge endpoint resolution failed before edge hit testing could run.
    Edge(EdgeResolutionError),
}

impl From<NodeGraphValidationError> for NodeGraphHitTestError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<EdgeResolutionError> for NodeGraphHitTestError {
    fn from(error: EdgeResolutionError) -> Self {
        Self::Edge(error)
    }
}

/// Resolves one UI logical screen-space point to a stable typed node graph target.
///
/// Hit priority is deterministic: enabled ports, enabled node title/body,
/// enabled reroutes, enabled edges with enabled endpoint nodes, enabled frames,
/// enabled groups, then canvas. Within one priority tier, later descriptors
/// win so hit testing follows the same topmost-last ordering used by static
/// primitive emission.
/// Disabled targets are skipped instead of returned.
///
/// # Errors
///
/// Returns a structured error when descriptor validation or edge endpoint
/// resolution fails.
pub fn hit_test_node_graph(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
    hit_test_node_graph_with_config(viewport, graph, point, NodeGraphHitTestConfig::default())
}

/// Resolves one UI logical screen-space point with explicit hit geometry.
///
/// # Errors
///
/// Returns a structured error when descriptor validation or edge endpoint
/// resolution fails.
pub fn hit_test_node_graph_with_config(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
    graph.validate()?;
    let resolved_edges = graph.resolve_edges()?;
    let point = sanitize_point(point);
    let bounds = viewport.effective_bounds();
    let config = config.sanitized();

    if !bounds.contains_point(point) {
        return Ok(NodeGraphHitTarget::Canvas);
    }

    if let Some(target) = hit_test_ports(viewport, graph, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_nodes(viewport, graph, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_reroutes(viewport, graph, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_edges(viewport, &resolved_edges, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_frames(viewport, graph, point) {
        return Ok(target);
    }

    if let Some(target) = hit_test_groups(viewport, graph, point) {
        return Ok(target);
    }

    Ok(NodeGraphHitTarget::Canvas)
}

fn hit_test_ports(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    graph.nodes.iter().rev().find_map(|node| {
        if !node.enabled {
            return None;
        }

        node.ports.iter().rev().find_map(|port| {
            if !port.enabled {
                return None;
            }

            port_hit_rect(viewport, node, port, config.port_size)
                .contains_point(point)
                .then_some(NodeGraphHitTarget::Port(PortEndpoint::new(
                    node.id, port.id,
                )))
        })
    })
}

fn hit_test_nodes(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    graph.nodes.iter().rev().find_map(|node| {
        if !node.enabled {
            return None;
        }

        let node_rect = viewport.graph_rect_to_screen(node.rect);
        if !node_rect.contains_point(point) {
            return None;
        }

        if node_title_rect(viewport, node, config.title_bar_height).contains_point(point) {
            Some(NodeGraphHitTarget::NodeTitle(node.id))
        } else {
            Some(NodeGraphHitTarget::NodeBody(node.id))
        }
    })
}

fn hit_test_reroutes(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    graph.reroutes.iter().rev().find_map(|reroute| {
        if !reroute.enabled {
            return None;
        }

        reroute_hit_rect(viewport, reroute, config.reroute_size)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Reroute(reroute.id))
    })
}

fn hit_test_edges(
    viewport: NodeGraphViewport,
    edges: &[ResolvedEdge<'_>],
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    edges.iter().rev().find_map(|edge| {
        if !edge.edge.enabled || !edge.from.node.enabled || !edge.to.node.enabled {
            return None;
        }

        edge_screen_points(viewport, edge)
            .windows(2)
            .any(|segment| {
                point_to_segment_distance(point, segment[0], segment[1]) <= config.edge_tolerance
            })
            .then_some(NodeGraphHitTarget::Edge(edge.edge.id))
    })
}

fn hit_test_frames(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
) -> Option<NodeGraphHitTarget> {
    graph.frames.iter().rev().find_map(|frame| {
        if !frame.enabled {
            return None;
        }

        viewport
            .graph_rect_to_screen(frame.rect)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Frame(frame.id))
    })
}

fn hit_test_groups(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    point: Point,
) -> Option<NodeGraphHitTarget> {
    graph.groups.iter().rev().find_map(|group| {
        if !group.enabled {
            return None;
        }

        viewport
            .graph_rect_to_screen(group.rect)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Group(group.id))
    })
}

fn port_hit_rect(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    port: &PortDescriptor,
    size: f32,
) -> Rect {
    let anchor = viewport.graph_to_screen(port_anchor(node, port));
    let size = finite_non_negative(size);
    Rect::new(anchor.x - size * 0.5, anchor.y - size * 0.5, size, size)
}

fn reroute_hit_rect(viewport: NodeGraphViewport, reroute: &RerouteDescriptor, size: f32) -> Rect {
    let center = viewport.graph_to_screen(reroute.position);
    let size = finite_non_negative(size);
    Rect::new(center.x - size * 0.5, center.y - size * 0.5, size, size)
}

fn node_title_rect(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    title_bar_height: f32,
) -> Rect {
    let rect = node.rect.sanitized();
    let title_height = finite_non_negative(title_bar_height).min(rect.height);
    viewport.graph_rect_to_screen(GraphRect::new(rect.x, rect.y, rect.width, title_height))
}

fn point_to_segment_distance(point: Point, from: Point, to: Point) -> f32 {
    let segment_x = to.x - from.x;
    let segment_y = to.y - from.y;
    let length_squared = finite_sum(
        finite_product(segment_x, segment_x),
        finite_product(segment_y, segment_y),
    );

    if length_squared <= f32::EPSILON {
        return point_distance(point, from);
    }

    let point_x = point.x - from.x;
    let point_y = point.y - from.y;
    let projection = finite_div(
        finite_sum(
            finite_product(point_x, segment_x),
            finite_product(point_y, segment_y),
        ),
        length_squared,
    )
    .clamp(0.0, 1.0);
    let closest = Point::new(
        finite_sum(from.x, finite_product(segment_x, projection)),
        finite_sum(from.y, finite_product(segment_y, projection)),
    );

    point_distance(point, closest)
}

fn point_distance(lhs: Point, rhs: Point) -> f32 {
    let x = lhs.x - rhs.x;
    let y = lhs.y - rhs.y;
    finite_sum(finite_product(x, x), finite_product(y, y)).sqrt()
}

/// Static node graph primitive and semantic emission failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphEmissionError {
    /// Descriptor validation failed before static output could be emitted.
    Validation(NodeGraphValidationError),
    /// Edge endpoint resolution failed before static output could be emitted.
    Edge(EdgeResolutionError),
}

impl From<NodeGraphValidationError> for NodeGraphEmissionError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<EdgeResolutionError> for NodeGraphEmissionError {
    fn from(error: EdgeResolutionError) -> Self {
        Self::Edge(error)
    }
}

/// Static visual state for a node graph port.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphPortState {
    /// The port is enabled and has no static incompatibility marker.
    Normal,
    /// The port descriptor is disabled.
    Disabled,
    /// The port is enabled but caller-supplied compatibility context marks it incompatible.
    Incompatible,
}

impl NodeGraphPortState {
    /// Resolves static port state from descriptor availability and optional compatibility context.
    #[must_use]
    pub const fn from_port(port: &PortDescriptor, incompatible: bool) -> Self {
        if !port.enabled {
            Self::Disabled
        } else if incompatible {
            Self::Incompatible
        } else {
            Self::Normal
        }
    }
}

/// Deterministic visual metadata for one node graph port state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphPortStyle {
    /// Port body fill.
    pub fill: Color,
    /// Port outline.
    pub stroke: Color,
    /// Port label color.
    pub label: Color,
    /// Port outline width.
    pub stroke_width: f32,
}

/// Optional node graph grid styling.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphGridStyle {
    /// Grid spacing in graph-space units.
    pub spacing: f32,
    /// Grid line color.
    pub color: Color,
    /// Grid line width in screen logical units.
    pub stroke_width: f32,
}

impl NodeGraphGridStyle {
    /// Creates grid styling.
    #[must_use]
    pub const fn new(spacing: f32, color: Color, stroke_width: f32) -> Self {
        Self {
            spacing,
            color,
            stroke_width,
        }
    }

    fn effective_spacing(self) -> Option<f32> {
        self.spacing
            .is_finite()
            .then_some(self.spacing)
            .filter(|spacing| *spacing > 0.0)
    }
}

/// Static node graph visual recipe for backend-independent primitive emission.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphStyle {
    /// Optional viewport background fill.
    pub background: Option<Color>,
    /// Optional graph-space grid.
    pub grid: Option<NodeGraphGridStyle>,
    /// Node body fill.
    pub node_fill: Color,
    /// Disabled node body fill.
    pub disabled_node_fill: Color,
    /// Node outline color.
    pub node_stroke: Color,
    /// Node outline width.
    pub node_stroke_width: f32,
    /// Edge color.
    pub edge: Color,
    /// Disabled edge color.
    pub disabled_edge: Color,
    /// Edge stroke width.
    pub edge_width: f32,
    /// Reroute body fill.
    pub reroute_fill: Color,
    /// Disabled reroute body fill.
    pub disabled_reroute_fill: Color,
    /// Reroute outline color.
    pub reroute_stroke: Color,
    /// Reroute outline width.
    pub reroute_stroke_width: f32,
    /// Reroute square size in screen logical units.
    pub reroute_size: f32,
    /// Normal port style.
    pub port: NodeGraphPortStyle,
    /// Disabled port style.
    pub disabled_port: NodeGraphPortStyle,
    /// Incompatible port style.
    pub incompatible_port: NodeGraphPortStyle,
    /// Port square size in screen logical units.
    pub port_size: f32,
    /// Node and port label font family.
    pub font_family: &'static str,
    /// Node title font size.
    pub node_label_size: f32,
    /// Port label font size.
    pub port_label_size: f32,
    /// Label color.
    pub label: Color,
    /// Disabled label color.
    pub disabled_label: Color,
}

impl Default for NodeGraphStyle {
    fn default() -> Self {
        Self {
            background: Some(Color::rgba(0.07, 0.075, 0.085, 1.0)),
            grid: None,
            node_fill: Color::rgba(0.16, 0.17, 0.19, 1.0),
            disabled_node_fill: Color::rgba(0.11, 0.115, 0.125, 1.0),
            node_stroke: Color::rgba(0.43, 0.46, 0.50, 1.0),
            node_stroke_width: 1.0,
            edge: Color::rgba(0.70, 0.78, 0.90, 1.0),
            disabled_edge: Color::rgba(0.36, 0.38, 0.42, 1.0),
            edge_width: 2.0,
            reroute_fill: Color::rgba(0.22, 0.28, 0.34, 1.0),
            disabled_reroute_fill: Color::rgba(0.16, 0.17, 0.19, 1.0),
            reroute_stroke: Color::rgba(0.76, 0.82, 0.90, 1.0),
            reroute_stroke_width: 1.0,
            reroute_size: DEFAULT_NODE_GRAPH_REROUTE_HIT_SIZE,
            port: NodeGraphPortStyle {
                fill: Color::rgba(0.25, 0.55, 0.90, 1.0),
                stroke: Color::rgba(0.78, 0.86, 0.96, 1.0),
                label: Color::rgba(0.88, 0.91, 0.95, 1.0),
                stroke_width: 1.0,
            },
            disabled_port: NodeGraphPortStyle {
                fill: Color::rgba(0.20, 0.21, 0.23, 1.0),
                stroke: Color::rgba(0.38, 0.39, 0.42, 1.0),
                label: Color::rgba(0.55, 0.57, 0.60, 1.0),
                stroke_width: 1.0,
            },
            incompatible_port: NodeGraphPortStyle {
                fill: Color::rgba(0.75, 0.42, 0.18, 1.0),
                stroke: Color::rgba(0.96, 0.72, 0.42, 1.0),
                label: Color::rgba(0.95, 0.82, 0.65, 1.0),
                stroke_width: 1.0,
            },
            port_size: 8.0,
            font_family: "sans-serif",
            node_label_size: 12.0,
            port_label_size: 10.0,
            label: Color::rgba(0.92, 0.94, 0.97, 1.0),
            disabled_label: Color::rgba(0.55, 0.57, 0.60, 1.0),
        }
    }
}

impl NodeGraphStyle {
    /// Returns deterministic visual metadata for a static port state.
    #[must_use]
    pub const fn port_style(self, state: NodeGraphPortState) -> NodeGraphPortStyle {
        match state {
            NodeGraphPortState::Normal => self.port,
            NodeGraphPortState::Disabled => self.disabled_port,
            NodeGraphPortState::Incompatible => self.incompatible_port,
        }
    }
}

/// Output from static node graph composition.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphStaticOutput {
    /// Backend-independent draw primitives.
    pub primitives: Vec<Primitive>,
    /// Backend-independent semantic nodes.
    pub semantics: Vec<SemanticNode>,
}

/// Static node graph composition request.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphStaticView<'a> {
    /// Stable semantic root identity for the graph view.
    pub id: WidgetId,
    /// Clip identity used for the viewport clip commands.
    pub clip: ClipId,
    /// Graph viewport transform and clipping bounds.
    pub viewport: NodeGraphViewport,
    /// Data-only graph descriptor.
    pub graph: &'a NodeGraphDescriptor,
    /// Static visual recipe.
    pub style: NodeGraphStyle,
    /// Caller-supplied static selection metadata.
    pub selection: NodeGraphSelection,
    /// Caller-supplied set of ports to style as statically incompatible.
    pub incompatible_ports: BTreeSet<PortEndpoint>,
}

impl<'a> NodeGraphStaticView<'a> {
    /// Creates a static node graph composition request.
    #[must_use]
    pub fn new(id: WidgetId, viewport: NodeGraphViewport, graph: &'a NodeGraphDescriptor) -> Self {
        Self {
            id,
            clip: ClipId::from_raw(id.raw()),
            viewport,
            graph,
            style: NodeGraphStyle::default(),
            selection: NodeGraphSelection::new(),
            incompatible_ports: BTreeSet::new(),
        }
    }

    /// Sets the clip identity.
    #[must_use]
    pub const fn with_clip(mut self, clip: ClipId) -> Self {
        self.clip = clip;
        self
    }

    /// Sets the static visual recipe.
    #[must_use]
    pub const fn with_style(mut self, style: NodeGraphStyle) -> Self {
        self.style = style;
        self
    }

    /// Sets caller-supplied static selection metadata.
    #[must_use]
    pub fn with_selection(mut self, selection: NodeGraphSelection) -> Self {
        self.selection = selection;
        self
    }

    /// Sets caller-supplied static incompatible ports.
    #[must_use]
    pub fn with_incompatible_ports(
        mut self,
        ports: impl IntoIterator<Item = PortEndpoint>,
    ) -> Self {
        self.incompatible_ports = ports.into_iter().collect();
        self
    }

    /// Emits primitives and semantics after validating descriptors and resolving edges.
    ///
    /// # Errors
    ///
    /// Returns a structured error when graph descriptors are invalid or when
    /// any edge endpoint cannot be resolved honestly.
    pub fn emit(&self) -> Result<NodeGraphStaticOutput, NodeGraphEmissionError> {
        self.graph.validate()?;
        let resolved_edges = self.graph.resolve_edges()?;

        Ok(NodeGraphStaticOutput {
            primitives: self.primitives(&resolved_edges),
            semantics: self.semantics(&resolved_edges),
        })
    }

    fn primitives(&self, resolved_edges: &[ResolvedEdge<'_>]) -> Vec<Primitive> {
        let bounds = self.viewport.effective_bounds();
        let mut primitives = vec![Primitive::ClipBegin {
            id: self.clip,
            rect: bounds,
        }];

        if let Some(background) = self.style.background {
            primitives.push(Primitive::Rect(RectPrimitive {
                rect: bounds,
                fill: Some(Brush::Solid(background)),
                stroke: None,
                radius: CornerRadius::all(0.0),
            }));
        }

        if let Some(grid) = self.style.grid {
            primitives.extend(grid_primitives(self.viewport, grid));
        }

        primitives.extend(
            resolved_edges
                .iter()
                .flat_map(|edge| self.edge_primitives(edge)),
        );
        primitives.extend(
            self.graph
                .reroutes
                .iter()
                .map(|reroute| self.reroute_primitive(reroute)),
        );
        primitives.extend(
            self.graph
                .nodes
                .iter()
                .map(|node| self.node_primitive(node)),
        );

        for node in &self.graph.nodes {
            for port in &node.ports {
                primitives.push(self.port_primitive(node, port));
            }
        }

        for node in &self.graph.nodes {
            primitives.push(self.node_label_primitive(node));
            for port in &node.ports {
                primitives.push(self.port_label_primitive(node, port));
            }
        }

        primitives.push(Primitive::ClipEnd { id: self.clip });
        primitives
    }

    fn edge_primitives(&self, edge: &ResolvedEdge<'_>) -> Vec<Primitive> {
        let color = if edge.edge.enabled {
            self.style.edge
        } else {
            self.style.disabled_edge
        };
        let stroke = Stroke::new(self.style.edge_width, Brush::Solid(color));
        edge_screen_points(self.viewport, edge)
            .windows(2)
            .map(|segment| {
                Primitive::Line(LinePrimitive {
                    from: segment[0],
                    to: segment[1],
                    stroke,
                })
            })
            .collect()
    }

    fn reroute_primitive(&self, reroute: &RerouteDescriptor) -> Primitive {
        Primitive::Rect(RectPrimitive {
            rect: reroute_hit_rect(self.viewport, reroute, self.style.reroute_size),
            fill: Some(Brush::Solid(if reroute.enabled {
                self.style.reroute_fill
            } else {
                self.style.disabled_reroute_fill
            })),
            stroke: Some(Stroke::new(
                self.style.reroute_stroke_width,
                Brush::Solid(self.style.reroute_stroke),
            )),
            radius: CornerRadius::all(2.0),
        })
    }

    fn node_primitive(&self, node: &NodeDescriptor) -> Primitive {
        Primitive::Rect(RectPrimitive {
            rect: self.viewport.graph_rect_to_screen(node.rect),
            fill: Some(Brush::Solid(if node.enabled {
                self.style.node_fill
            } else {
                self.style.disabled_node_fill
            })),
            stroke: Some(Stroke::new(
                self.style.node_stroke_width,
                Brush::Solid(self.style.node_stroke),
            )),
            radius: CornerRadius::all(4.0),
        })
    }

    fn port_primitive(&self, node: &NodeDescriptor, port: &PortDescriptor) -> Primitive {
        let state = self.port_state(node.id, port);
        let style = self.style.port_style(state);
        Primitive::Rect(RectPrimitive {
            rect: self.port_rect(node, port),
            fill: Some(Brush::Solid(style.fill)),
            stroke: Some(Stroke::new(style.stroke_width, Brush::Solid(style.stroke))),
            radius: CornerRadius::all(2.0),
        })
    }

    fn node_label_primitive(&self, node: &NodeDescriptor) -> Primitive {
        let rect = self.viewport.graph_rect_to_screen(node.rect);
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(rect.x + 8.0, rect.y + self.style.node_label_size + 6.0),
            text: node.title.clone(),
            family: self.style.font_family.to_owned(),
            size: self.style.node_label_size,
            line_height: self.style.node_label_size + 4.0,
            brush: Brush::Solid(if node.enabled {
                self.style.label
            } else {
                self.style.disabled_label
            }),
        })
    }

    fn port_label_primitive(&self, node: &NodeDescriptor, port: &PortDescriptor) -> Primitive {
        let rect = self.port_rect(node, port);
        let label_x = match port.direction {
            PortDirection::Input => rect.max_x() + 4.0,
            PortDirection::Output => rect.x - (port_label_width(&port.label, &self.style) + 4.0),
        };
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(label_x, rect.y + self.style.port_label_size),
            text: port.label.clone(),
            family: self.style.font_family.to_owned(),
            size: self.style.port_label_size,
            line_height: self.style.port_label_size + 3.0,
            brush: Brush::Solid(self.style.port_style(self.port_state(node.id, port)).label),
        })
    }

    fn semantics(&self, resolved_edges: &[ResolvedEdge<'_>]) -> Vec<SemanticNode> {
        let edge_ids = resolved_edges
            .iter()
            .map(|edge| self.edge_semantic_id(edge.edge.id));
        let reroute_ids = self
            .graph
            .reroutes
            .iter()
            .map(|reroute| self.reroute_semantic_id(reroute.id));
        let node_ids = self
            .graph
            .nodes
            .iter()
            .map(|node| self.node_semantic_id(node.id));
        let mut semantics = vec![
            SemanticNode::new(
                self.id,
                SemanticRole::Custom("node-graph".to_owned()),
                self.viewport.effective_bounds(),
            )
            .with_label("Node graph")
            .with_children(edge_ids.chain(reroute_ids).chain(node_ids)),
        ];

        semantics.extend(resolved_edges.iter().map(|edge| self.edge_semantics(edge)));
        semantics.extend(
            self.graph
                .reroutes
                .iter()
                .map(|reroute| self.reroute_semantics(reroute)),
        );
        for node in &self.graph.nodes {
            semantics.push(self.node_semantics(node));
            semantics.extend(
                node.ports
                    .iter()
                    .map(|port| self.port_semantics(node, port)),
            );
        }
        semantics
    }

    fn edge_semantics(&self, edge: &ResolvedEdge<'_>) -> SemanticNode {
        let mut node = SemanticNode::new(
            self.edge_semantic_id(edge.edge.id),
            SemanticRole::Custom("edge".to_owned()),
            polyline_bounds(&edge_screen_points(self.viewport, edge)),
        )
        .with_label(format!(
            "Edge {}: {} {} to {} {}",
            edge.edge.id.raw(),
            edge.from.node.title,
            edge.from.port.label,
            edge.to.node.title,
            edge.to.port.label
        ));
        node.state.disabled = !edge.edge.enabled;
        node.state.selected = self
            .selection
            .contains(NodeGraphSelectionTarget::Edge(edge.edge.id));
        node
    }

    fn reroute_semantics(&self, reroute: &RerouteDescriptor) -> SemanticNode {
        let mut node = SemanticNode::new(
            self.reroute_semantic_id(reroute.id),
            SemanticRole::Custom("reroute".to_owned()),
            reroute_hit_rect(self.viewport, reroute, self.style.reroute_size),
        )
        .with_label(reroute.label.clone());
        node.state.disabled = !reroute.enabled;
        node.state.selected = self
            .selection
            .contains(NodeGraphSelectionTarget::Reroute(reroute.id));
        node.state.value = Some(SemanticValue::Text(reroute.label.clone()));
        node
    }

    fn node_semantics(&self, graph_node: &NodeDescriptor) -> SemanticNode {
        let mut node = SemanticNode::new(
            self.node_semantic_id(graph_node.id),
            SemanticRole::Custom("node".to_owned()),
            self.viewport.graph_rect_to_screen(graph_node.rect),
        )
        .with_label(graph_node.title.clone())
        .with_children(
            graph_node
                .ports
                .iter()
                .map(|port| self.port_semantic_id(graph_node.id, port.id)),
        );
        node.state.disabled = !graph_node.enabled;
        node.state.selected = self
            .selection
            .contains(NodeGraphSelectionTarget::Node(graph_node.id));
        node.state.value = Some(SemanticValue::Text(graph_node.title.clone()));
        node
    }

    fn port_semantics(&self, node: &NodeDescriptor, port: &PortDescriptor) -> SemanticNode {
        let state = self.port_state(node.id, port);
        let mut semantic = SemanticNode::new(
            self.port_semantic_id(node.id, port.id),
            SemanticRole::Custom("port".to_owned()),
            self.port_rect(node, port),
        )
        .with_label(format!("{} {}", port.direction.as_str(), port.label));
        semantic.state.disabled = state == NodeGraphPortState::Disabled;
        semantic.state.selected =
            self.selection
                .contains(NodeGraphSelectionTarget::Port(PortEndpoint::new(
                    node.id, port.id,
                )));
        semantic.state.value = Some(SemanticValue::Text(port.label.clone()));
        semantic.description = match state {
            NodeGraphPortState::Normal => None,
            NodeGraphPortState::Disabled => Some("Disabled port".to_owned()),
            NodeGraphPortState::Incompatible => Some("Incompatible port".to_owned()),
        };
        semantic
    }

    fn port_state(&self, node: NodeId, port: &PortDescriptor) -> NodeGraphPortState {
        NodeGraphPortState::from_port(
            port,
            self.incompatible_ports
                .contains(&PortEndpoint::new(node, port.id)),
        )
    }

    fn port_rect(&self, node: &NodeDescriptor, port: &PortDescriptor) -> Rect {
        let anchor = self.viewport.graph_to_screen(port_anchor(node, port));
        let size = finite_non_negative(self.style.port_size);
        Rect::new(anchor.x - size * 0.5, anchor.y - size * 0.5, size, size)
    }

    fn edge_semantic_id(&self, edge: EdgeId) -> WidgetId {
        self.id.child(("edge", edge.raw()))
    }

    fn reroute_semantic_id(&self, reroute: RerouteId) -> WidgetId {
        self.id.child(("reroute", reroute.raw()))
    }

    fn node_semantic_id(&self, node: NodeId) -> WidgetId {
        self.id.child(("node", node.raw()))
    }

    fn port_semantic_id(&self, node: NodeId, port: PortId) -> WidgetId {
        self.id.child(("port", node.raw(), port.raw()))
    }
}

impl PortDirection {
    /// Returns a stable display string for semantic labels.
    #[must_use]
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Input => "Input",
            Self::Output => "Output",
        }
    }
}

fn grid_primitives(viewport: NodeGraphViewport, grid: NodeGraphGridStyle) -> Vec<Primitive> {
    let Some(spacing) = grid.effective_spacing() else {
        return Vec::new();
    };
    let bounds = viewport.effective_bounds();
    let graph_bounds = viewport.screen_rect_to_graph(bounds);
    let mut primitives = Vec::new();
    let stroke = Stroke::new(grid.stroke_width, Brush::Solid(grid.color));

    let mut x = (graph_bounds.x / spacing).floor() * spacing;
    let max_x = graph_bounds.max_x();
    let mut count = 0_u16;
    while x <= max_x && count < 512 {
        let from = viewport.graph_to_screen(GraphPoint::new(x, graph_bounds.y));
        let to = viewport.graph_to_screen(GraphPoint::new(x, graph_bounds.max_y()));
        primitives.push(Primitive::Line(LinePrimitive { from, to, stroke }));
        x += spacing;
        count += 1;
    }

    let mut y = (graph_bounds.y / spacing).floor() * spacing;
    let max_y = graph_bounds.max_y();
    let mut count = 0_u16;
    while y <= max_y && count < 512 {
        let from = viewport.graph_to_screen(GraphPoint::new(graph_bounds.x, y));
        let to = viewport.graph_to_screen(GraphPoint::new(graph_bounds.max_x(), y));
        primitives.push(Primitive::Line(LinePrimitive { from, to, stroke }));
        y += spacing;
        count += 1;
    }

    primitives
}

fn edge_screen_points(viewport: NodeGraphViewport, edge: &ResolvedEdge<'_>) -> Vec<Point> {
    let mut points = Vec::with_capacity(edge.route_points.len() + 2);
    points.push(viewport.graph_to_screen(edge.from.anchor));
    points.extend(
        edge.route_points
            .iter()
            .map(|route_point| viewport.graph_to_screen(route_point.position)),
    );
    points.push(viewport.graph_to_screen(edge.to.anchor));
    points
}

fn polyline_bounds(points: &[Point]) -> Rect {
    let Some((first, rest)) = points.split_first() else {
        return Rect::ZERO;
    };
    let first = sanitize_point(*first);
    let mut min = first;
    let mut max = first;
    for point in rest {
        let point = sanitize_point(*point);
        min = Point::new(min.x.min(point.x), min.y.min(point.y));
        max = Point::new(max.x.max(point.x), max.y.max(point.y));
    }

    Rect::from_min_max(min, max).outset(1.0).max_zero()
}

#[allow(clippy::cast_precision_loss)]
fn port_label_width(label: &str, style: &NodeGraphStyle) -> f32 {
    label.chars().count() as f32 * style.port_label_size * 0.55
}

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
    )
}

fn normalize_screen_rect(rect: Rect) -> Rect {
    if !rect.x.is_finite()
        || !rect.y.is_finite()
        || !rect.width.is_finite()
        || !rect.height.is_finite()
    {
        return Rect::ZERO;
    }

    let min = sanitize_point(rect.origin());
    let max = sanitize_point(Point::new(
        finite_rect_extent(rect.x, rect.width),
        finite_rect_extent(rect.y, rect.height),
    ));
    Rect::from_min_max(
        Point::new(min.x.min(max.x), min.y.min(max.y)),
        Point::new(min.x.max(max.x), min.y.max(max.y)),
    )
}

fn sanitize_point(point: Point) -> Point {
    Point::new(finite_or_zero(point.x), finite_or_zero(point.y))
}

fn sanitize_zoom(zoom: f32) -> f32 {
    if zoom.is_finite() && zoom > 0.0 {
        zoom.max(MIN_ZOOM)
    } else {
        DEFAULT_ZOOM
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_sum(lhs: f32, rhs: f32) -> f32 {
    let sum = lhs + rhs;
    if sum.is_finite() {
        sum
    } else if sum.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

fn finite_product(lhs: f32, rhs: f32) -> f32 {
    let product = lhs * rhs;
    if product.is_finite() {
        product
    } else if product.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

fn finite_div(lhs: f32, rhs: f32) -> f32 {
    let quotient = lhs / rhs;
    finite_or_zero(quotient)
}

fn finite_rect_extent(origin: f32, size: f32) -> f32 {
    if origin.is_finite() && size.is_finite() {
        finite_or_zero(origin + size)
    } else {
        0.0
    }
}

fn box_selection_operations(
    intent: NodeGraphSelectionIntent,
    targets: &[NodeGraphSelectionTarget],
) -> Vec<NodeGraphSelectionOperation> {
    match intent {
        NodeGraphSelectionIntent::Replace => {
            let Some((first, rest)) = targets.split_first() else {
                return vec![NodeGraphSelectionOperation::Clear];
            };

            let mut operations = Vec::with_capacity(targets.len());
            operations.push(NodeGraphSelectionOperation::Replace(*first));
            operations.extend(
                rest.iter()
                    .copied()
                    .map(NodeGraphSelectionOperation::Extend),
            );
            operations
        }
        NodeGraphSelectionIntent::Add => targets
            .iter()
            .copied()
            .map(NodeGraphSelectionOperation::Extend)
            .collect(),
        NodeGraphSelectionIntent::Subtract => targets
            .iter()
            .copied()
            .map(NodeGraphSelectionOperation::Remove)
            .collect(),
    }
}

fn effective_snap_grid_size(grid_size: f32) -> Option<f32> {
    (grid_size.is_finite() && grid_size > 0.0).then_some(grid_size)
}

fn snap_graph_component(value: f32, grid_size: f32) -> f32 {
    finite_product((finite_div(value, grid_size)).round(), grid_size)
}
