#[allow(clippy::wildcard_imports)]
use super::*;

/// Resolved data-only metadata for a link draft endpoint.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkDraftEndpoint {
    /// Stable port endpoint.
    pub endpoint: PortEndpoint,
    /// Directed port flow.
    pub direction: PortDirection,
    /// Application-owned compatibility key.
    pub port_type: PortTypeId,
    /// Graph-space anchor for backend-independent draft drawing.
    pub anchor: GraphPoint,
}

/// Structured endpoint resolution failure for link draft metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkDraftEndpointError {
    /// Descriptor validation failed before endpoint resolution could run.
    Validation(NodeGraphValidationError),
    /// The endpoint references a missing node.
    MissingNode {
        /// Missing node ID.
        node: NodeId,
    },
    /// The endpoint references a missing port on an existing node.
    MissingPort {
        /// Existing node ID.
        node: NodeId,
        /// Missing port ID.
        port: PortId,
    },
    /// The owning node exists but is disabled.
    DisabledNode {
        /// Disabled node ID.
        node: NodeId,
    },
    /// The endpoint exists but its port is disabled.
    DisabledPort {
        /// Owning node ID.
        node: NodeId,
        /// Disabled port ID.
        port: PortId,
    },
}

impl From<NodeGraphValidationError> for NodeGraphLinkDraftEndpointError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

/// Hover target metadata for a link draft.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkDraftTarget {
    /// The current hover hit is not a port target.
    Hit(NodeGraphHitTarget),
    /// The current hover hit is a resolved port target.
    Port(NodeGraphLinkDraftPortTarget),
}

impl NodeGraphLinkDraftTarget {
    /// Returns the underlying hit target.
    #[must_use]
    pub const fn hit_target(&self) -> NodeGraphHitTarget {
        match self {
            Self::Hit(target) => *target,
            Self::Port(target) => NodeGraphHitTarget::Port(target.endpoint.endpoint),
        }
    }

    /// Returns true when the target is a compatible completion target.
    #[must_use]
    pub fn is_compatible(&self) -> bool {
        matches!(self, Self::Port(target) if target.compatibility.is_ok())
    }
}

/// Resolved hover port metadata for a link draft.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftPortTarget {
    /// Resolved endpoint under the current pointer.
    pub endpoint: NodeGraphLinkDraftEndpoint,
    /// Generic directed compatibility result against the draft start endpoint.
    pub compatibility: Result<(), PortCompatibilityError>,
}

/// Structured hover target resolution failure for link drafts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkDraftTargetError {
    /// Hit testing failed before a hover target could be resolved.
    HitTest(NodeGraphHitTestError),
    /// A hit port target could not be resolved to endpoint metadata.
    Endpoint(NodeGraphLinkDraftEndpointError),
}

impl From<NodeGraphHitTestError> for NodeGraphLinkDraftTargetError {
    fn from(error: NodeGraphHitTestError) -> Self {
        Self::HitTest(error)
    }
}

impl From<NodeGraphLinkDraftEndpointError> for NodeGraphLinkDraftTargetError {
    fn from(error: NodeGraphLinkDraftEndpointError) -> Self {
        Self::Endpoint(error)
    }
}

/// Data-only application-owned link draft state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraft {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, once resolved against a viewport.
    pub current_graph_point: Option<GraphPoint>,
    /// Current hover target metadata.
    pub target: NodeGraphLinkDraftTarget,
}

impl NodeGraphLinkDraft {
    /// Starts a link draft from an enabled endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured endpoint error when descriptors are invalid, the
    /// endpoint is stale, or the endpoint is disabled.
    pub fn start(
        graph: &NodeGraphDescriptor,
        start: PortEndpoint,
        current_pointer: Point,
    ) -> Result<Self, NodeGraphLinkDraftEndpointError> {
        Ok(Self {
            start: resolve_link_draft_endpoint(graph, start)?,
            current_pointer: sanitize_point(current_pointer),
            current_graph_point: None,
            target: NodeGraphLinkDraftTarget::Hit(NodeGraphHitTarget::Canvas),
        })
    }

    /// Resolves current hover target metadata using default node graph hit testing.
    ///
    /// # Errors
    ///
    /// Returns a structured target error when hit testing or endpoint
    /// resolution fails.
    pub fn resolve_hover_target(
        &self,
        graph: &NodeGraphDescriptor,
        viewport: NodeGraphViewport,
        current_pointer: Point,
    ) -> Result<Self, NodeGraphLinkDraftTargetError> {
        self.resolve_hover_target_with_config(
            graph,
            viewport,
            current_pointer,
            NodeGraphHitTestConfig::default(),
        )
    }

    /// Resolves current hover target metadata with explicit hit test geometry.
    ///
    /// # Errors
    ///
    /// Returns a structured target error when hit testing or endpoint
    /// resolution fails.
    pub fn resolve_hover_target_with_config(
        &self,
        graph: &NodeGraphDescriptor,
        viewport: NodeGraphViewport,
        current_pointer: Point,
        config: NodeGraphHitTestConfig,
    ) -> Result<Self, NodeGraphLinkDraftTargetError> {
        let current_pointer = sanitize_point(current_pointer);
        let hit = graph.hit_test_with_config(viewport, current_pointer, config)?;
        let target = match hit {
            NodeGraphHitTarget::Port(endpoint) => {
                let endpoint = resolve_link_draft_endpoint(graph, endpoint)?;
                NodeGraphLinkDraftTarget::Port(NodeGraphLinkDraftPortTarget {
                    endpoint,
                    compatibility: link_draft_compatibility(self.start, endpoint),
                })
            }
            target => NodeGraphLinkDraftTarget::Hit(target),
        };

        Ok(Self {
            start: self.start,
            current_pointer,
            current_graph_point: Some(viewport.screen_to_graph(current_pointer)),
            target,
        })
    }

    /// Returns a deterministic cancel outcome without mutating graph descriptors.
    #[must_use]
    pub fn cancel(&self) -> NodeGraphLinkDraftOutcome {
        NodeGraphLinkDraftOutcome::Cancelled(NodeGraphLinkDraftCancelled {
            start: self.start,
            current_pointer: self.current_pointer,
            current_graph_point: self.current_graph_point,
            target: self.target.clone(),
        })
    }

    /// Returns a deterministic completion or rejection outcome without mutating graph descriptors.
    #[must_use]
    pub fn complete(&self) -> NodeGraphLinkDraftOutcome {
        let NodeGraphLinkDraftTarget::Port(target) = &self.target else {
            return NodeGraphLinkDraftOutcome::Rejected(NodeGraphLinkDraftRejected {
                start: self.start,
                current_pointer: self.current_pointer,
                current_graph_point: self.current_graph_point,
                target: self.target.clone(),
                error: NodeGraphLinkDraftCompletionError::NoPortTarget {
                    target: self.target.hit_target(),
                },
            });
        };

        if let Err(error) = target.compatibility {
            return NodeGraphLinkDraftOutcome::Rejected(NodeGraphLinkDraftRejected {
                start: self.start,
                current_pointer: self.current_pointer,
                current_graph_point: self.current_graph_point,
                target: self.target.clone(),
                error: NodeGraphLinkDraftCompletionError::IncompatiblePort {
                    target: target.endpoint,
                    error,
                },
            });
        }

        let (from, to) = if self.start.direction == PortDirection::Output {
            (self.start, target.endpoint)
        } else {
            (target.endpoint, self.start)
        };

        NodeGraphLinkDraftOutcome::Completed(NodeGraphLinkDraftCompleted {
            from,
            to,
            current_pointer: self.current_pointer,
            current_graph_point: self.current_graph_point,
        })
    }
}

/// Deterministic link draft outcome.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkDraftOutcome {
    /// The draft was cancelled.
    Cancelled(NodeGraphLinkDraftCancelled),
    /// The draft completed with a compatible output-to-input endpoint pair.
    Completed(NodeGraphLinkDraftCompleted),
    /// The draft could not complete.
    Rejected(NodeGraphLinkDraftRejected),
}

/// Deterministic link draft cancel metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftCancelled {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
    /// Last resolved hover target.
    pub target: NodeGraphLinkDraftTarget,
}

/// Deterministic link draft completion metadata.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkDraftCompleted {
    /// Canonical output endpoint.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Canonical input endpoint.
    pub to: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
}

/// Deterministic link draft rejection metadata.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphLinkDraftRejected {
    /// Resolved start endpoint metadata.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Current UI logical screen-space pointer.
    pub current_pointer: Point,
    /// Current graph-space pointer, if hover resolution ran.
    pub current_graph_point: Option<GraphPoint>,
    /// Last resolved hover target.
    pub target: NodeGraphLinkDraftTarget,
    /// Reason completion was rejected.
    pub error: NodeGraphLinkDraftCompletionError,
}

/// Structured link draft completion failure.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum NodeGraphLinkDraftCompletionError {
    /// The current hover target is not a port.
    NoPortTarget {
        /// Current non-port hit target.
        target: NodeGraphHitTarget,
    },
    /// The current hover port is not compatible with the draft start endpoint.
    IncompatiblePort {
        /// Current hover endpoint metadata.
        target: NodeGraphLinkDraftEndpoint,
        /// Generic directed compatibility failure.
        error: PortCompatibilityError,
    },
}

/// Data-only application-owned link edit request.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphLinkEditRequest {
    /// Request to create a new link between two compatible endpoints.
    CreateLink(NodeGraphCreateLinkRequest),
    /// Request to reconnect an existing edge source endpoint.
    ReconnectSource(NodeGraphReconnectLinkSourceRequest),
    /// Request to reconnect an existing edge target endpoint.
    ReconnectTarget(NodeGraphReconnectLinkTargetRequest),
    /// Request to detach one endpoint from an existing edge.
    DetachEdge(NodeGraphDetachLinkRequest),
    /// Request to cut an existing edge.
    CutEdge(NodeGraphCutLinkRequest),
}

impl NodeGraphLinkEditRequest {
    /// Creates metadata for a new app-owned link creation request.
    ///
    /// # Errors
    ///
    /// Returns a structured error when either endpoint cannot be resolved or
    /// the endpoints are not a compatible output-to-input pair.
    pub fn create_link(
        graph: &NodeGraphDescriptor,
        from: PortEndpoint,
        to: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let from = resolve_link_edit_request_endpoint(graph, from)?;
        let to = resolve_link_edit_request_endpoint(graph, to)?;
        validate_link_edit_compatibility(from, to)?;

        Ok(Self::CreateLink(NodeGraphCreateLinkRequest { from, to }))
    }

    /// Creates metadata for reconnecting an edge source endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current target.
    pub fn reconnect_source(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        new_source: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let new_source = resolve_link_edit_request_endpoint(graph, new_source)?;
        validate_link_edit_compatibility(new_source, edge.to)?;

        Ok(Self::ReconnectSource(NodeGraphReconnectLinkSourceRequest {
            edge,
            old_source: edge.from,
            new_source,
            target: edge.to,
        }))
    }

    /// Creates metadata for reconnecting an edge target endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current source.
    pub fn reconnect_target(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        new_target: PortEndpoint,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let new_target = resolve_link_edit_request_endpoint(graph, new_target)?;
        validate_link_edit_compatibility(edge.from, new_target)?;

        Ok(Self::ReconnectTarget(NodeGraphReconnectLinkTargetRequest {
            edge,
            source: edge.from,
            old_target: edge.to,
            new_target,
        }))
    }

    /// Creates metadata for detaching one endpoint from an existing edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn detach_edge(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
        detached: EdgeEndpointRole,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        let edge = resolve_link_edit_edge(graph, edge)?;
        let endpoint = match detached {
            EdgeEndpointRole::Source => edge.from,
            EdgeEndpointRole::Target => edge.to,
        };

        Ok(Self::DetachEdge(NodeGraphDetachLinkRequest {
            edge,
            detached,
            endpoint,
        }))
    }

    /// Creates metadata for cutting an existing edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn cut_edge(
        graph: &NodeGraphDescriptor,
        edge: EdgeId,
    ) -> Result<Self, NodeGraphLinkEditRequestError> {
        Ok(Self::CutEdge(NodeGraphCutLinkRequest {
            edge: resolve_link_edit_edge(graph, edge)?,
        }))
    }
}

/// Resolved edge context captured by link edit requests.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphLinkEditEdgeContext {
    /// Stable edge identity.
    pub edge: EdgeId,
    /// Current source endpoint metadata.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Current target endpoint metadata.
    pub to: NodeGraphLinkDraftEndpoint,
    /// Whether the edge is currently enabled.
    pub enabled: bool,
}

/// Metadata for an app-owned create-link request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphCreateLinkRequest {
    /// Requested source endpoint.
    pub from: NodeGraphLinkDraftEndpoint,
    /// Requested target endpoint.
    pub to: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned reconnect-source request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphReconnectLinkSourceRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Current source endpoint before reconnect.
    pub old_source: NodeGraphLinkDraftEndpoint,
    /// Requested replacement source endpoint.
    pub new_source: NodeGraphLinkDraftEndpoint,
    /// Unchanged target endpoint.
    pub target: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned reconnect-target request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphReconnectLinkTargetRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Unchanged source endpoint.
    pub source: NodeGraphLinkDraftEndpoint,
    /// Current target endpoint before reconnect.
    pub old_target: NodeGraphLinkDraftEndpoint,
    /// Requested replacement target endpoint.
    pub new_target: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned detach-edge request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphDetachLinkRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
    /// Endpoint role to detach.
    pub detached: EdgeEndpointRole,
    /// Endpoint metadata for the detached side.
    pub endpoint: NodeGraphLinkDraftEndpoint,
}

/// Metadata for an app-owned cut-edge request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NodeGraphCutLinkRequest {
    /// Resolved existing edge context.
    pub edge: NodeGraphLinkEditEdgeContext,
}

/// Structured link edit request creation failure.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphLinkEditRequestError {
    /// Descriptor validation failed before request creation could run.
    Validation(NodeGraphValidationError),
    /// A requested edge does not exist.
    MissingEdge {
        /// Missing edge ID.
        edge: EdgeId,
    },
    /// Existing edge context could not be resolved.
    Edge(EdgeResolutionError),
    /// A requested replacement endpoint could not be resolved.
    Endpoint(NodeGraphLinkDraftEndpointError),
    /// The requested output-to-input pair is not generically compatible.
    IncompatiblePort {
        /// Requested source endpoint.
        from: PortEndpoint,
        /// Requested target endpoint.
        to: PortEndpoint,
        /// Generic directed compatibility failure.
        error: PortCompatibilityError,
    },
}

impl From<NodeGraphValidationError> for NodeGraphLinkEditRequestError {
    fn from(error: NodeGraphValidationError) -> Self {
        Self::Validation(error)
    }
}

impl From<EdgeResolutionError> for NodeGraphLinkEditRequestError {
    fn from(error: EdgeResolutionError) -> Self {
        Self::Edge(error)
    }
}

impl From<NodeGraphLinkDraftEndpointError> for NodeGraphLinkEditRequestError {
    fn from(error: NodeGraphLinkDraftEndpointError) -> Self {
        Self::Endpoint(error)
    }
}
