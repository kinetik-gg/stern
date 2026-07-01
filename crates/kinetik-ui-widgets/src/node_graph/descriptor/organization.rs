#[allow(clippy::wildcard_imports)]
use super::*;

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
