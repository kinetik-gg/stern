#[allow(clippy::wildcard_imports)]
use super::*;

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
        let projection = NodeGraphVisibleProjection::for_viewport(
            self.viewport,
            self.graph,
            &resolved_edges,
            &self.style,
        );

        Ok(NodeGraphStaticOutput {
            primitives: self.primitives(&resolved_edges, &projection),
            semantics: self.semantics(&resolved_edges, &projection),
        })
    }

    fn primitives(
        &self,
        resolved_edges: &[ResolvedEdge<'_>],
        projection: &NodeGraphVisibleProjection,
    ) -> Vec<Primitive> {
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
            projection
                .edges
                .iter()
                .flat_map(|&edge_index| self.edge_primitives(&resolved_edges[edge_index])),
        );
        primitives.extend(
            projection
                .reroutes
                .iter()
                .map(|&reroute_index| self.reroute_primitive(&self.graph.reroutes[reroute_index])),
        );
        primitives.extend(
            projection
                .nodes
                .iter()
                .map(|&node_index| self.node_primitive(&self.graph.nodes[node_index])),
        );

        for &node_index in &projection.nodes {
            let node = &self.graph.nodes[node_index];
            for port in &node.ports {
                primitives.push(self.port_primitive(node, port));
            }
        }

        for &node_index in &projection.nodes {
            let node = &self.graph.nodes[node_index];
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

    fn semantics(
        &self,
        resolved_edges: &[ResolvedEdge<'_>],
        projection: &NodeGraphVisibleProjection,
    ) -> Vec<SemanticNode> {
        let edge_ids = projection
            .edges
            .iter()
            .map(|&edge_index| self.edge_semantic_id(resolved_edges[edge_index].edge.id));
        let reroute_ids = projection
            .reroutes
            .iter()
            .map(|&reroute_index| self.reroute_semantic_id(self.graph.reroutes[reroute_index].id));
        let node_ids = projection
            .nodes
            .iter()
            .map(|&node_index| self.node_semantic_id(self.graph.nodes[node_index].id));
        let mut semantics = vec![
            SemanticNode::new(
                self.id,
                SemanticRole::Custom("node-graph".to_owned()),
                self.viewport.effective_bounds(),
            )
            .with_label("Node graph")
            .with_children(edge_ids.chain(reroute_ids).chain(node_ids)),
        ];

        semantics.extend(
            projection
                .edges
                .iter()
                .map(|&edge_index| self.edge_semantics(&resolved_edges[edge_index])),
        );
        semantics.extend(
            projection
                .reroutes
                .iter()
                .map(|&reroute_index| self.reroute_semantics(&self.graph.reroutes[reroute_index])),
        );
        for &node_index in &projection.nodes {
            let node = &self.graph.nodes[node_index];
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

pub(crate) fn grid_primitives(
    viewport: NodeGraphViewport,
    grid: NodeGraphGridStyle,
) -> Vec<Primitive> {
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

pub(crate) fn edge_screen_points(
    viewport: NodeGraphViewport,
    edge: &ResolvedEdge<'_>,
) -> Vec<Point> {
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

pub(crate) fn edge_screen_bounds(
    viewport: NodeGraphViewport,
    edge: &ResolvedEdge<'_>,
    outset: f32,
) -> Rect {
    let first = viewport.graph_to_screen(edge.from.anchor);
    let mut min = first;
    let mut max = first;

    for point in edge
        .route_points
        .iter()
        .map(|route_point| viewport.graph_to_screen(route_point.position))
        .chain(std::iter::once(viewport.graph_to_screen(edge.to.anchor)))
    {
        let point = sanitize_point(point);
        min = Point::new(min.x.min(point.x), min.y.min(point.y));
        max = Point::new(max.x.max(point.x), max.y.max(point.y));
    }

    Rect::from_min_max(min, max)
        .outset(finite_non_negative(outset).max(1.0))
        .max_zero()
}

pub(crate) fn polyline_bounds(points: &[Point]) -> Rect {
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
