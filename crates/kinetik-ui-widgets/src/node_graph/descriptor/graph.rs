#[allow(clippy::wildcard_imports)]
use super::*;

/// Data-only node graph descriptor.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeGraphDescriptor {
    /// Nodes.
    pub nodes: Vec<NodeDescriptor>,
    /// Edges.
    pub edges: Vec<EdgeDescriptor>,
    /// Reroutes.
    pub reroutes: Vec<RerouteDescriptor>,
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
            reroutes: Vec::new(),
            frames: Vec::new(),
            groups: Vec::new(),
        }
    }

    /// Validates deterministic descriptor invariants.
    ///
    /// # Errors
    ///
    /// Returns a structured validation error when node, frame, or group IDs are
    /// duplicated, or a node contains duplicate port IDs.
    pub fn validate(&self) -> Result<(), NodeGraphValidationError> {
        validate_node_graph_descriptors(&self.nodes)?;
        validate_node_graph_reroute_descriptors(&self.reroutes)?;
        validate_node_graph_frame_descriptors(&self.frames)?;
        validate_node_graph_group_descriptors(&self.groups)?;
        validate_node_graph_memberships(self)
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

    /// Resolves one UI logical screen-space point to a stable typed hit target.
    ///
    /// Disabled targets are intentionally skipped. Invalid descriptors return a
    /// structured error before any target is guessed.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test error when descriptor validation or edge
    /// endpoint resolution fails.
    pub fn hit_test(
        &self,
        viewport: NodeGraphViewport,
        point: Point,
    ) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
        hit_test_node_graph(viewport, self, point)
    }

    /// Resolves one UI logical screen-space point with explicit hit geometry.
    ///
    /// # Errors
    ///
    /// Returns a structured hit test error when descriptor validation or edge
    /// endpoint resolution fails.
    pub fn hit_test_with_config(
        &self,
        viewport: NodeGraphViewport,
        point: Point,
        config: NodeGraphHitTestConfig,
    ) -> Result<NodeGraphHitTarget, NodeGraphHitTestError> {
        hit_test_node_graph_with_config(viewport, self, point, config)
    }

    /// Starts application-owned link draft metadata from an enabled port endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured endpoint error when descriptors are invalid, the
    /// endpoint is stale, or the endpoint is disabled.
    pub fn start_link_draft(
        &self,
        start: PortEndpoint,
        current_pointer: Point,
    ) -> Result<NodeGraphLinkDraft, NodeGraphLinkDraftEndpointError> {
        NodeGraphLinkDraft::start(self, start, current_pointer)
    }

    /// Creates application-owned metadata for a new link request.
    ///
    /// # Errors
    ///
    /// Returns a structured error when either endpoint cannot be resolved or
    /// the endpoints are not a compatible output-to-input pair.
    pub fn create_link_request(
        &self,
        from: PortEndpoint,
        to: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::create_link(self, from, to)
    }

    /// Creates application-owned metadata for reconnecting an edge source.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current target.
    pub fn reconnect_link_source_request(
        &self,
        edge: EdgeId,
        new_source: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::reconnect_source(self, edge, new_source)
    }

    /// Creates application-owned metadata for reconnecting an edge target.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge or replacement endpoint cannot
    /// be resolved, or when the replacement is not compatible with the current source.
    pub fn reconnect_link_target_request(
        &self,
        edge: EdgeId,
        new_target: PortEndpoint,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::reconnect_target(self, edge, new_target)
    }

    /// Creates application-owned metadata for detaching one edge endpoint.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn detach_link_endpoint_request(
        &self,
        edge: EdgeId,
        endpoint: EdgeEndpointRole,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::detach_edge(self, edge, endpoint)
    }

    /// Creates application-owned metadata for cutting an edge.
    ///
    /// # Errors
    ///
    /// Returns a structured error when the edge cannot be resolved.
    pub fn cut_link_request(
        &self,
        edge: EdgeId,
    ) -> Result<NodeGraphLinkEditRequest, NodeGraphLinkEditRequestError> {
        NodeGraphLinkEditRequest::cut_edge(self, edge)
    }

    /// Resolves context action metadata from a raw hit-test target.
    #[must_use]
    pub fn context_actions_from_hit(
        &self,
        hit: NodeGraphHitTarget,
        selection: &NodeGraphSelection,
    ) -> Vec<NodeGraphContextAction> {
        self.context_actions(NodeGraphContextTarget::from_hit_target(hit), selection)
    }

    /// Resolves deterministic app-owned context action metadata.
    #[must_use]
    pub fn context_actions(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Vec<NodeGraphContextAction> {
        resolve_node_graph_context_actions(self, target, selection)
    }

    /// Resolves one context action by kind without materializing the default catalog.
    ///
    /// This path is intended for applications that present a subset or custom
    /// ordering of node graph context actions while still reusing Kinetik's
    /// typed request metadata.
    #[must_use]
    pub fn context_action(
        &self,
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> NodeGraphContextAction {
        node_graph_context_action(self, kind, target, selection)
    }

    /// Resolves one context action from a raw hit-test target.
    #[must_use]
    pub fn context_action_from_hit(
        &self,
        kind: NodeGraphContextActionKind,
        hit: NodeGraphHitTarget,
        selection: &NodeGraphSelection,
    ) -> NodeGraphContextAction {
        self.context_action(
            kind,
            NodeGraphContextTarget::from_hit_target(hit),
            selection,
        )
    }

    /// Creates delete request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target or selection
    /// cannot produce delete request metadata.
    pub fn delete_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextSelectionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_selection_request(self, target, selection)
    }

    /// Creates duplicate request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target or selection
    /// cannot produce duplicate request metadata.
    pub fn duplicate_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextSelectionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_selection_request(self, target, selection)
    }

    /// Creates disconnect request metadata for an edge or port context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the context target is
    /// unsupported, missing, disabled, or has no connected edges.
    pub fn disconnect_context_request(
        &self,
        target: NodeGraphContextTarget,
    ) -> Result<NodeGraphContextDisconnectRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_disconnect_context_request(self, target)
    }

    /// Creates detach-endpoint request metadata for an edge context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the context target is
    /// unsupported, missing, or disabled.
    pub fn detach_context_request(
        &self,
        target: NodeGraphContextTarget,
        endpoint: EdgeEndpointRole,
    ) -> Result<NodeGraphContextDetachEndpointRequest, NodeGraphContextActionUnavailableReason>
    {
        node_graph_detach_context_request(self, target, endpoint)
    }

    /// Creates organization request metadata for a context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the operation is not
    /// valid for the current target or selection.
    pub fn organization_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
        operation: NodeGraphContextOrganizationOperation,
    ) -> Result<NodeGraphContextOrganizationRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_organization_context_request(self, target, selection, operation)
    }

    /// Creates select-all request metadata for the canvas context target.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target is not the
    /// canvas or the graph has no selectable targets.
    pub fn select_all_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_select_all_context_request(self, target, selection)
    }

    /// Creates paste request metadata for the canvas context target.
    ///
    /// The default compatibility catalog keeps paste disabled until the
    /// application provides clipboard state. Applications with that state can
    /// use this builder to present a custom enabled paste action without
    /// duplicating target and selection metadata.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the target is not the
    /// canvas.
    pub fn paste_context_request(
        &self,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextCanvasRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_paste_context_request(target, selection)
    }

    /// Creates application-owned request metadata for a context action kind.
    ///
    /// # Errors
    ///
    /// Returns a deterministic unavailable reason when the requested action
    /// kind does not apply to the current target or selection.
    pub fn context_action_request(
        &self,
        kind: NodeGraphContextActionKind,
        target: NodeGraphContextTarget,
        selection: &NodeGraphSelection,
    ) -> Result<NodeGraphContextActionRequest, NodeGraphContextActionUnavailableReason> {
        node_graph_context_action_request(self, kind, target, selection)
    }

    /// Returns frame member node IDs in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing.
    pub fn frame_member_nodes(
        &self,
        frame: NodeFrameId,
    ) -> Result<Vec<NodeId>, NodeGraphOrganizationRequestError> {
        self.validate()?;
        resolve_node_graph_frame(self, frame)?;
        Ok(frame_member_nodes(self, frame))
    }

    /// Returns group member node IDs in deterministic order.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the group target is missing.
    pub fn group_member_nodes(
        &self,
        group: NodeGroupId,
    ) -> Result<Vec<NodeId>, NodeGraphOrganizationRequestError> {
        self.validate()?;
        resolve_node_graph_group(self, group)?;
        Ok(group_member_nodes(self, group))
    }

    /// Creates application-owned metadata for moving a parent frame and its children.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn move_frame_request(
        &self,
        viewport: NodeGraphViewport,
        frame: NodeFrameId,
        screen_delta: GraphVector,
    ) -> Result<NodeGraphFrameMoveRequest, NodeGraphOrganizationRequestError> {
        NodeGraphFrameMoveRequest::new(self, viewport, frame, screen_delta)
    }

    /// Creates application-owned collapse metadata for a frame.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the frame target is missing or disabled.
    pub fn collapse_frame_request(
        &self,
        frame: NodeFrameId,
        collapsed: bool,
    ) -> Result<NodeGraphCollapseRequest, NodeGraphOrganizationRequestError> {
        NodeGraphCollapseRequest::frame(self, frame, collapsed)
    }

    /// Creates application-owned collapse metadata for a group.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the group target is missing or disabled.
    pub fn collapse_group_request(
        &self,
        group: NodeGroupId,
        collapsed: bool,
    ) -> Result<NodeGraphCollapseRequest, NodeGraphOrganizationRequestError> {
        NodeGraphCollapseRequest::group(self, group, collapsed)
    }

    /// Creates application-owned node mute request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the node target is missing or disabled.
    pub fn mute_node_request(
        &self,
        node: NodeId,
        muted: bool,
    ) -> Result<NodeGraphNodeStateRequest, NodeGraphOrganizationRequestError> {
        NodeGraphNodeStateRequest::new(self, node, NodeGraphNodeStateAction::Mute, muted)
    }

    /// Creates application-owned node bypass request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the node target is missing or disabled.
    pub fn bypass_node_request(
        &self,
        node: NodeId,
        bypassed: bool,
    ) -> Result<NodeGraphNodeStateRequest, NodeGraphOrganizationRequestError> {
        NodeGraphNodeStateRequest::new(self, node, NodeGraphNodeStateAction::Bypass, bypassed)
    }

    /// Creates application-owned label request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the target is missing or disabled.
    pub fn label_request(
        &self,
        target: NodeGraphOrganizationTarget,
        label: impl Into<String>,
    ) -> Result<NodeGraphAnnotationRequest, NodeGraphOrganizationRequestError> {
        NodeGraphAnnotationRequest::new(
            self,
            target,
            NodeGraphAnnotationField::Label,
            Some(label.into()),
        )
    }

    /// Creates application-owned comment request metadata.
    ///
    /// # Errors
    ///
    /// Returns a structured organization error when descriptors are invalid or
    /// the target is missing or disabled.
    pub fn comment_request(
        &self,
        target: NodeGraphOrganizationTarget,
        comment: impl Into<String>,
    ) -> Result<NodeGraphAnnotationRequest, NodeGraphOrganizationRequestError> {
        NodeGraphAnnotationRequest::new(
            self,
            target,
            NodeGraphAnnotationField::Comment,
            Some(comment.into()),
        )
    }
}
