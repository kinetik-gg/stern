//! Backend-independent node graph identity, descriptor, and coordinate contracts.

use std::collections::BTreeSet;

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, LinePrimitive, Point, Primitive, Rect, RectPrimitive,
    SemanticNode, SemanticRole, SemanticValue, Stroke, TextPrimitive, WidgetId,
};

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

        primitives.extend(resolved_edges.iter().map(|edge| self.edge_primitive(edge)));
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

    fn edge_primitive(&self, edge: &ResolvedEdge<'_>) -> Primitive {
        let color = if edge.edge.enabled {
            self.style.edge
        } else {
            self.style.disabled_edge
        };
        Primitive::Line(LinePrimitive {
            from: self.viewport.graph_to_screen(edge.from.anchor),
            to: self.viewport.graph_to_screen(edge.to.anchor),
            stroke: Stroke::new(self.style.edge_width, Brush::Solid(color)),
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
            .with_children(edge_ids.chain(node_ids)),
        ];

        semantics.extend(resolved_edges.iter().map(|edge| self.edge_semantics(edge)));
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
            line_bounds(
                self.viewport.graph_to_screen(edge.from.anchor),
                self.viewport.graph_to_screen(edge.to.anchor),
            ),
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

fn line_bounds(from: Point, to: Point) -> Rect {
    Rect::from_min_max(
        Point::new(from.x.min(to.x), from.y.min(to.y)),
        Point::new(from.x.max(to.x), from.y.max(to.y)),
    )
    .outset(1.0)
    .max_zero()
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
