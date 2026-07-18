use stern_core::{Response, WidgetId};

use super::{
    NodeGraphCreateLinkRequest, NodeGraphEmissionError, NodeGraphHitTarget, NodeGraphHitTestConfig,
    NodeGraphLinkDraft, NodeGraphLinkDraftCancelled, NodeGraphLinkDraftEndpoint,
    NodeGraphLinkDraftEndpointError, NodeGraphLinkDraftRejected, NodeGraphLinkEditRequestError,
    NodeGraphSelection, NodeGraphSelectionOperation, NodeGraphStaticOutput, NodeGraphStaticView,
    NodeGraphViewport, PortEndpoint,
};

/// Caller-owned configuration for one retained node graph widget.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidgetConfig<'graph> {
    view: NodeGraphStaticView<'graph>,
    disabled: bool,
    read_only: bool,
    hit_test: NodeGraphHitTestConfig,
}

impl<'graph> NodeGraphWidgetConfig<'graph> {
    /// Creates an enabled widget from a caller-owned static graph snapshot.
    #[must_use]
    pub fn new(view: NodeGraphStaticView<'graph>) -> Self {
        Self {
            view,
            disabled: false,
            read_only: false,
            hit_test: NodeGraphHitTestConfig::new(),
        }
    }

    /// Sets whether graph interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets whether mutation intents are suppressed while selection stays available.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Replaces the screen-space hit testing recipe.
    #[must_use]
    pub const fn with_hit_test(mut self, hit_test: NodeGraphHitTestConfig) -> Self {
        self.hit_test = hit_test;
        self
    }
}

/// Immutable frame-local node graph widget.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidget<'graph> {
    view: NodeGraphStaticView<'graph>,
    output: NodeGraphStaticOutput,
    disabled: bool,
    read_only: bool,
    hit_test: NodeGraphHitTestConfig,
}

impl<'graph> NodeGraphWidget<'graph> {
    pub(crate) fn prepare(
        config: NodeGraphWidgetConfig<'graph>,
    ) -> Result<Self, NodeGraphEmissionError> {
        let output = config.view.emit()?;
        Ok(Self {
            view: config.view,
            output,
            disabled: config.disabled,
            read_only: config.read_only,
            hit_test: config.hit_test,
        })
    }

    /// Returns the stable graph root identity.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.view.id
    }

    /// Returns the frozen viewport shared by paint, hit testing, and semantics.
    #[must_use]
    pub const fn viewport(&self) -> NodeGraphViewport {
        self.view.viewport
    }

    /// Returns the caller-owned selection snapshot used for this frame.
    #[must_use]
    pub const fn selection(&self) -> &NodeGraphSelection {
        &self.view.selection
    }

    /// Returns whether graph interaction is disabled.
    #[must_use]
    pub const fn disabled(&self) -> bool {
        self.disabled
    }

    /// Returns whether graph mutation intents are suppressed.
    #[must_use]
    pub const fn read_only(&self) -> bool {
        self.read_only
    }

    pub(crate) const fn view(&self) -> &NodeGraphStaticView<'graph> {
        &self.view
    }

    pub(crate) const fn output(&self) -> &NodeGraphStaticOutput {
        &self.output
    }

    pub(crate) const fn hit_test(&self) -> NodeGraphHitTestConfig {
        self.hit_test
    }
}

/// Typed application intent emitted by the retained node graph widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphWidgetIntent {
    /// Apply a pure operation to caller-owned graph selection state.
    Selection(NodeGraphSelectionOperation),
}

/// Typed lifecycle intent for one retained connection edit.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphConnectionIntent {
    /// A port drag crossed the threshold and began a connection preview.
    Begin(NodeGraphConnectionBegin),
    /// Candidate geometry changed under the frozen graph transform.
    Preview(NodeGraphConnectionPreview),
    /// The candidate passed the canonical typed link policy.
    Accepted(NodeGraphCreateLinkRequest),
    /// The candidate failed target resolution or typed link policy.
    Rejected(NodeGraphConnectionRejection),
    /// The accepted connection should be committed by the application.
    Commit(NodeGraphCreateLinkRequest),
    /// The transaction ended without application mutation.
    Cancel(NodeGraphConnectionCancel),
}

/// Stable connection transaction metadata captured when dragging begins.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphConnectionBegin {
    /// Retained node graph that owns the gesture.
    pub graph: WidgetId,
    /// Stable node and port identity that began the draft.
    pub start: NodeGraphLinkDraftEndpoint,
    /// Pan/zoom transform frozen for targeting until commit or cancellation.
    pub viewport: NodeGraphViewport,
}

/// Candidate connection geometry resolved through the captured transform.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphConnectionPreview {
    /// Retained node graph that owns the gesture.
    pub graph: WidgetId,
    /// Current typed link draft and candidate target.
    pub draft: NodeGraphLinkDraft,
    /// Pan/zoom transform frozen when the gesture began.
    pub viewport: NodeGraphViewport,
}

/// Deterministic reason a connection candidate was rejected.
#[derive(Debug, Clone, PartialEq)]
pub enum NodeGraphConnectionRejection {
    /// Candidate targeting resolved, but draft completion failed.
    Draft(NodeGraphLinkDraftRejected),
    /// Candidate endpoints no longer form a canonical create-link request.
    Link(NodeGraphLinkEditRequestError),
    /// Candidate port metadata could not be resolved.
    Endpoint(NodeGraphLinkDraftEndpointError),
}

/// Why an active connection transaction was cancelled.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeGraphConnectionCancelReason {
    /// Escape was pressed while the graph owned the gesture.
    Escape,
    /// Pointer capture or window focus was lost.
    CaptureLost,
    /// The graph became disabled during the transaction.
    Disabled,
    /// The graph became read-only during the transaction.
    ReadOnly,
}

/// Cancellation metadata for restoring the last committed application state.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphConnectionCancel {
    /// Retained node graph that owned the gesture.
    pub graph: WidgetId,
    /// Deterministic cancellation cause.
    pub reason: NodeGraphConnectionCancelReason,
    /// Last typed candidate state before cancellation.
    pub draft: NodeGraphLinkDraftCancelled,
    /// Pan/zoom transform frozen when the gesture began.
    pub viewport: NodeGraphViewport,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NodeGraphConnectionCapture {
    pub(crate) owner: WidgetId,
    pub(crate) viewport: NodeGraphViewport,
    pub(crate) hit_test: NodeGraphHitTestConfig,
    pub(crate) draft: NodeGraphLinkDraft,
    pub(crate) started: bool,
}

/// Opaque caller-retained state for one node graph connection gesture.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct NodeGraphConnectionController {
    pub(crate) capture: Option<NodeGraphConnectionCapture>,
}

impl NodeGraphConnectionController {
    /// Returns true after a port drag crossed the interaction threshold.
    #[must_use]
    pub fn is_connecting(&self) -> bool {
        self.capture.as_ref().is_some_and(|capture| capture.started)
    }

    /// Returns the stable start endpoint for the active connection gesture.
    #[must_use]
    pub fn start_endpoint(&self) -> Option<PortEndpoint> {
        self.capture
            .as_ref()
            .map(|capture| capture.draft.start.endpoint)
    }

    /// Returns the transform frozen at pointer press.
    #[must_use]
    pub fn frozen_viewport(&self) -> Option<NodeGraphViewport> {
        self.capture.as_ref().map(|capture| capture.viewport)
    }
}

/// Output from one retained node graph widget evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct NodeGraphWidgetOutput {
    /// Common interaction response for the graph root.
    pub response: Response,
    /// Exact target under the accepted release, when one was emitted.
    pub hit: Option<NodeGraphHitTarget>,
    /// Ordered typed application intents.
    pub intents: Vec<NodeGraphWidgetIntent>,
    /// Ordered typed connection lifecycle intents.
    pub connection_intents: Vec<NodeGraphConnectionIntent>,
}
