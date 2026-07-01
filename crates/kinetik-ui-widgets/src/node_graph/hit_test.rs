#[allow(clippy::wildcard_imports)]
use super::*;

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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct NodeGraphVisibleProjection {
    pub(crate) nodes: Vec<usize>,
    pub(crate) reroutes: Vec<usize>,
    pub(crate) edges: Vec<usize>,
    pub(crate) frames: Vec<usize>,
    pub(crate) groups: Vec<usize>,
}

impl NodeGraphVisibleProjection {
    pub(crate) fn for_viewport(
        viewport: NodeGraphViewport,
        graph: &NodeGraphDescriptor,
        resolved_edges: &[ResolvedEdge<'_>],
        style: &NodeGraphStyle,
    ) -> Self {
        let bounds = viewport.effective_bounds();
        if bounds.is_empty() {
            return Self::default();
        }

        Self {
            nodes: graph
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    node_static_screen_bounds(viewport, node, style)
                        .intersection(bounds)
                        .is_some()
                        .then_some(index)
                })
                .collect(),
            reroutes: graph
                .reroutes
                .iter()
                .enumerate()
                .filter_map(|(index, reroute)| {
                    reroute_hit_rect(viewport, reroute, style.reroute_size)
                        .intersection(bounds)
                        .is_some()
                        .then_some(index)
                })
                .collect(),
            edges: resolved_edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    edge_screen_bounds(viewport, edge, style.edge_width * 0.5)
                        .intersection(bounds)
                        .is_some()
                        .then_some(index)
                })
                .collect(),
            frames: graph
                .frames
                .iter()
                .enumerate()
                .filter_map(|(index, frame)| {
                    viewport
                        .graph_rect_to_screen(frame.rect)
                        .intersection(bounds)
                        .is_some()
                        .then_some(index)
                })
                .collect(),
            groups: graph
                .groups
                .iter()
                .enumerate()
                .filter_map(|(index, group)| {
                    viewport
                        .graph_rect_to_screen(group.rect)
                        .intersection(bounds)
                        .is_some()
                        .then_some(index)
                })
                .collect(),
        }
    }

    fn for_hit(
        viewport: NodeGraphViewport,
        graph: &NodeGraphDescriptor,
        resolved_edges: &[ResolvedEdge<'_>],
        point: Point,
        config: NodeGraphHitTestConfig,
    ) -> Self {
        Self {
            nodes: graph
                .nodes
                .iter()
                .enumerate()
                .filter_map(|(index, node)| {
                    node_hit_candidate_contains_point(viewport, node, point, config)
                        .then_some(index)
                })
                .collect(),
            reroutes: graph
                .reroutes
                .iter()
                .enumerate()
                .filter_map(|(index, reroute)| {
                    reroute_hit_rect(viewport, reroute, config.reroute_size)
                        .contains_point(point)
                        .then_some(index)
                })
                .collect(),
            edges: resolved_edges
                .iter()
                .enumerate()
                .filter_map(|(index, edge)| {
                    edge_screen_bounds(
                        viewport,
                        edge,
                        finite_sum(config.edge_tolerance, NODE_GRAPH_EDGE_HIT_BOUNDARY_MARGIN),
                    )
                    .contains_point(point)
                    .then_some(index)
                })
                .collect(),
            frames: graph
                .frames
                .iter()
                .enumerate()
                .filter_map(|(index, frame)| {
                    viewport
                        .graph_rect_to_screen(frame.rect)
                        .contains_point(point)
                        .then_some(index)
                })
                .collect(),
            groups: graph
                .groups
                .iter()
                .enumerate()
                .filter_map(|(index, group)| {
                    viewport
                        .graph_rect_to_screen(group.rect)
                        .contains_point(point)
                        .then_some(index)
                })
                .collect(),
        }
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

    let projection =
        NodeGraphVisibleProjection::for_hit(viewport, graph, &resolved_edges, point, config);

    if let Some(target) = hit_test_ports(viewport, graph, &projection, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_nodes(viewport, graph, &projection, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_reroutes(viewport, graph, &projection, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_edges(viewport, &resolved_edges, &projection, point, config) {
        return Ok(target);
    }

    if let Some(target) = hit_test_frames(viewport, graph, &projection, point) {
        return Ok(target);
    }

    if let Some(target) = hit_test_groups(viewport, graph, &projection, point) {
        return Ok(target);
    }

    Ok(NodeGraphHitTarget::Canvas)
}

pub(crate) fn hit_test_ports(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    projection: &NodeGraphVisibleProjection,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    projection.nodes.iter().rev().find_map(|&node_index| {
        let node = &graph.nodes[node_index];
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

pub(crate) fn hit_test_nodes(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    projection: &NodeGraphVisibleProjection,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    projection.nodes.iter().rev().find_map(|&node_index| {
        let node = &graph.nodes[node_index];
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

pub(crate) fn hit_test_reroutes(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    projection: &NodeGraphVisibleProjection,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    projection.reroutes.iter().rev().find_map(|&reroute_index| {
        let reroute = &graph.reroutes[reroute_index];
        if !reroute.enabled {
            return None;
        }

        reroute_hit_rect(viewport, reroute, config.reroute_size)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Reroute(reroute.id))
    })
}

pub(crate) fn hit_test_edges(
    viewport: NodeGraphViewport,
    edges: &[ResolvedEdge<'_>],
    projection: &NodeGraphVisibleProjection,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> Option<NodeGraphHitTarget> {
    projection.edges.iter().rev().find_map(|&edge_index| {
        let edge = &edges[edge_index];
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

pub(crate) fn hit_test_frames(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    projection: &NodeGraphVisibleProjection,
    point: Point,
) -> Option<NodeGraphHitTarget> {
    projection.frames.iter().rev().find_map(|&frame_index| {
        let frame = &graph.frames[frame_index];
        if !frame.enabled {
            return None;
        }

        viewport
            .graph_rect_to_screen(frame.rect)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Frame(frame.id))
    })
}

pub(crate) fn hit_test_groups(
    viewport: NodeGraphViewport,
    graph: &NodeGraphDescriptor,
    projection: &NodeGraphVisibleProjection,
    point: Point,
) -> Option<NodeGraphHitTarget> {
    projection.groups.iter().rev().find_map(|&group_index| {
        let group = &graph.groups[group_index];
        if !group.enabled {
            return None;
        }

        viewport
            .graph_rect_to_screen(group.rect)
            .contains_point(point)
            .then_some(NodeGraphHitTarget::Group(group.id))
    })
}

pub(crate) fn port_hit_rect(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    port: &PortDescriptor,
    size: f32,
) -> Rect {
    let anchor = viewport.graph_to_screen(port_anchor(node, port));
    let size = finite_non_negative(size);
    Rect::new(anchor.x - size * 0.5, anchor.y - size * 0.5, size, size)
}

pub(crate) fn reroute_hit_rect(
    viewport: NodeGraphViewport,
    reroute: &RerouteDescriptor,
    size: f32,
) -> Rect {
    let center = viewport.graph_to_screen(reroute.position);
    let size = finite_non_negative(size);
    Rect::new(center.x - size * 0.5, center.y - size * 0.5, size, size)
}

pub(crate) fn node_title_rect(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    title_bar_height: f32,
) -> Rect {
    let rect = node.rect.sanitized();
    let title_height = finite_non_negative(title_bar_height).min(rect.height);
    viewport.graph_rect_to_screen(GraphRect::new(rect.x, rect.y, rect.width, title_height))
}

pub(crate) fn node_hit_candidate_contains_point(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    point: Point,
    config: NodeGraphHitTestConfig,
) -> bool {
    viewport
        .graph_rect_to_screen(node.rect)
        .contains_point(point)
        || node
            .ports
            .iter()
            .any(|port| port_hit_rect(viewport, node, port, config.port_size).contains_point(point))
}

pub(crate) fn node_static_screen_bounds(
    viewport: NodeGraphViewport,
    node: &NodeDescriptor,
    style: &NodeGraphStyle,
) -> Rect {
    let mut bounds = viewport.graph_rect_to_screen(node.rect);
    for port in &node.ports {
        let port_rect = port_hit_rect(viewport, node, port, style.port_size);
        bounds = bounds.union(port_rect);
        bounds = bounds.union(port_label_rect(port_rect, port, style));
    }
    bounds
}

pub(crate) fn port_label_rect(
    port_rect: Rect,
    port: &PortDescriptor,
    style: &NodeGraphStyle,
) -> Rect {
    let width = port_label_width(&port.label, style);
    let label_x = match port.direction {
        PortDirection::Input => port_rect.max_x() + 4.0,
        PortDirection::Output => port_rect.x - (width + 4.0),
    };
    Rect::new(
        label_x,
        port_rect.y,
        width,
        finite_non_negative(style.port_label_size + 3.0),
    )
}

pub(crate) fn point_to_segment_distance(point: Point, from: Point, to: Point) -> f32 {
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

pub(crate) fn point_distance(lhs: Point, rhs: Point) -> f32 {
    let x = lhs.x - rhs.x;
    let y = lhs.y - rhs.y;
    finite_sum(finite_product(x, x), finite_product(y, y)).sqrt()
}
