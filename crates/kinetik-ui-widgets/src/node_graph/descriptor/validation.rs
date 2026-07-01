#[allow(clippy::wildcard_imports)]
use super::*;

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
