#[allow(clippy::wildcard_imports)]
use super::*;

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
