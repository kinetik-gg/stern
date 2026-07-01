#[allow(clippy::wildcard_imports)]
use super::*;

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
