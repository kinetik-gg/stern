#[allow(clippy::wildcard_imports)]
use super::*;

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

pub(crate) fn resolve_node_graph_context_actions(
    graph: &NodeGraphDescriptor,
    target: NodeGraphContextTarget,
    selection: &NodeGraphSelection,
) -> Vec<NodeGraphContextAction> {
    DEFAULT_NODE_GRAPH_CONTEXT_ACTION_KINDS
        .into_iter()
        .map(|kind| node_graph_default_context_action(graph, kind, target, selection))
        .collect()
}

pub(crate) fn node_graph_default_context_action(
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

pub(crate) fn node_graph_context_action(
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

pub(crate) fn node_graph_context_action_request(
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

pub(crate) fn node_graph_context_selection_request(
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

pub(crate) fn node_graph_disconnect_context_request(
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

pub(crate) fn node_graph_edge_disconnect_request(
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

pub(crate) fn node_graph_endpoint_disconnect_request(
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

pub(crate) fn node_graph_detach_context_request(
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

pub(crate) fn node_graph_organization_context_request(
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

pub(crate) fn node_graph_selection_organization_request(
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

pub(crate) fn node_graph_ungroup_context_request(
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

pub(crate) fn node_graph_select_all_context_request(
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

pub(crate) fn node_graph_default_paste_context_action(
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

pub(crate) fn node_graph_paste_context_request(
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

pub(crate) fn context_selected_targets(
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

pub(crate) fn context_selectable_targets(
    graph: &NodeGraphDescriptor,
) -> Vec<NodeGraphSelectionTarget> {
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

pub(crate) fn connected_edge_ids_for_endpoint(
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

pub(crate) fn validate_context_target_available(
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

pub(crate) fn enabled_context_target(
    enabled: bool,
) -> Result<(), NodeGraphContextActionUnavailableReason> {
    enabled
        .then_some(())
        .ok_or(NodeGraphContextActionUnavailableReason::DisabledTarget)
}
