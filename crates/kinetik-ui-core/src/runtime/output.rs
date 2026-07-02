use crate::debug::FrameDiagnostic;
use crate::render::Primitive;
use crate::{
    AccessibilitySnapshot, ActionContext, ActionId, ActionInvocation, ActionQueue, ActionSource,
    SemanticNode, SemanticTree, SemanticTreeError, WidgetId,
};

use super::types::{CursorShape, FrameWarning, PlatformRequest, RepaintRequest};

/// Output produced by a UI frame.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FrameOutput {
    /// Backend-independent render primitives.
    pub primitives: Vec<Primitive>,
    /// Accessibility semantic tree for the frame.
    pub semantics: SemanticTree,
    /// Repaint scheduling request.
    pub repaint: RepaintRequest,
    /// Action invocations emitted during the frame.
    pub actions: ActionQueue,
    /// Requests for platform/application adapters.
    pub platform_requests: Vec<PlatformRequest>,
    /// Diagnostics detected while building or finalizing the frame.
    pub warnings: Vec<FrameWarning>,
}

impl FrameOutput {
    /// Creates empty frame output.
    #[must_use]
    pub fn new() -> Self {
        Self {
            primitives: Vec::new(),
            semantics: SemanticTree::new(),
            repaint: RepaintRequest::None,
            actions: ActionQueue::new(),
            platform_requests: Vec::new(),
            warnings: Vec::new(),
        }
    }

    /// Appends one render primitive.
    pub fn push_primitive(&mut self, primitive: Primitive) {
        self.primitives.push(primitive);
    }

    /// Appends render primitives in order.
    pub fn extend_primitives(&mut self, primitives: impl IntoIterator<Item = Primitive>) {
        self.primitives.extend(primitives);
    }

    /// Sets the semantic root node.
    pub fn set_semantic_root(&mut self, root: WidgetId) {
        self.semantics.set_root(root);
    }

    /// Appends one semantic node in traversal order.
    pub fn push_semantic_node(&mut self, node: SemanticNode) {
        self.semantics.push(node);
    }

    /// Requests repaint scheduling.
    pub fn request_repaint(&mut self, request: RepaintRequest) {
        self.repaint = self.repaint.merge(request);
    }

    /// Adds an action invocation to the frame output.
    pub fn push_action(&mut self, invocation: ActionInvocation) {
        self.actions.push(invocation);
        self.request_repaint(RepaintRequest::NextFrame);
    }

    /// Adds an action invocation from simple parts.
    pub fn invoke_action(
        &mut self,
        action_id: ActionId,
        source: ActionSource,
        context: ActionContext,
    ) {
        self.actions.invoke(action_id, source, context);
        self.request_repaint(RepaintRequest::NextFrame);
    }

    /// Appends one platform request.
    pub fn push_platform_request(&mut self, request: PlatformRequest) {
        self.platform_requests.push(request);
    }

    /// Records the cursor request for this frame, replacing earlier cursor intent.
    pub fn request_cursor(&mut self, cursor: CursorShape) {
        self.platform_requests
            .retain(|request| !matches!(request, PlatformRequest::SetCursor(_)));
        self.platform_requests
            .push(PlatformRequest::SetCursor(cursor));
    }

    /// Appends one runtime warning.
    pub fn push_warning(&mut self, warning: FrameWarning) {
        self.warnings.push(warning);
    }

    /// Returns structured diagnostics derived from frame warnings in warning order.
    #[must_use]
    pub fn diagnostics(&self) -> Vec<FrameDiagnostic> {
        self.warnings.iter().map(FrameWarning::diagnostic).collect()
    }

    /// Exports a validated accessibility snapshot for platform adapters.
    ///
    /// # Errors
    ///
    /// Returns [`SemanticTreeError`] when the frame's semantic tree is
    /// structurally invalid.
    pub fn accessibility_snapshot(
        &self,
        focused: Option<WidgetId>,
    ) -> Result<AccessibilitySnapshot, SemanticTreeError> {
        self.semantics.accessibility_snapshot(focused)
    }
}
