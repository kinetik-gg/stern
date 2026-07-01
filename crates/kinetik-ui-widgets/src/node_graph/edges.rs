#[allow(clippy::wildcard_imports)]
use super::*;

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

pub(crate) fn resolve_edge_route_points<'a>(
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

pub(crate) fn resolve_endpoint(
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

pub(crate) fn resolve_link_draft_endpoint(
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

pub(crate) fn link_draft_compatibility(
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

pub(crate) fn resolve_link_edit_request_endpoint(
    graph: &NodeGraphDescriptor,
    endpoint: PortEndpoint,
) -> Result<NodeGraphLinkDraftEndpoint, NodeGraphLinkEditRequestError> {
    resolve_link_draft_endpoint(graph, endpoint).map_err(NodeGraphLinkEditRequestError::Endpoint)
}

pub(crate) fn validate_link_edit_compatibility(
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

pub(crate) fn resolve_link_edit_edge(
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

pub(crate) fn port_anchor(node: &NodeDescriptor, port: &PortDescriptor) -> GraphPoint {
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

pub(crate) fn usize_to_graph_slot(value: usize) -> f32 {
    f32::from(u16::try_from(value).unwrap_or(u16::MAX))
}
