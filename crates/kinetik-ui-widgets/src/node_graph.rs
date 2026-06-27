//! Backend-independent node graph identity, descriptor, and coordinate contracts.

use std::collections::BTreeSet;

use kinetik_ui_core::{Point, Rect};

const DEFAULT_ZOOM: f32 = 1.0;
const MIN_ZOOM: f32 = 0.01;

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
node_graph_id!(NodeFrameId, "Stable identity for a node frame surface.");
node_graph_id!(NodeGroupId, "Stable identity for a node group.");
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

    /// Sets whether the node is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }
}

/// Data-only edge descriptor.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EdgeDescriptor {
    /// Stable edge identity.
    pub id: EdgeId,
    /// Output endpoint.
    pub from: PortEndpoint,
    /// Input endpoint.
    pub to: PortEndpoint,
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
            enabled: true,
        }
    }

    /// Sets whether the edge is currently available.
    #[must_use]
    pub const fn with_enabled(mut self, enabled: bool) -> Self {
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

/// Resolved edge with source and target descriptor references.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ResolvedEdge<'a> {
    /// Edge descriptor.
    pub edge: &'a EdgeDescriptor,
    /// Resolved output endpoint.
    pub from: ResolvedEndpoint<'a>,
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
            enabled: true,
        }
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
            enabled: true,
        }
    }

    /// Sets contained nodes.
    #[must_use]
    pub fn with_nodes(mut self, nodes: impl Into<Vec<NodeId>>) -> Self {
        self.nodes = nodes.into();
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
            frames: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Validates deterministic descriptor invariants.
    ///
    /// # Errors
    ///
    /// Returns a structured validation error when node IDs are duplicated or a
    /// node contains duplicate port IDs.
    pub fn validate(&self) -> Result<(), NodeGraphValidationError> {
        validate_node_graph_descriptors(&self.nodes)
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

        resolved.push(ResolvedEdge { edge, from, to });
    }

    Ok(resolved)
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

fn sanitize_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_or_zero(rect.x),
        finite_or_zero(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
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
