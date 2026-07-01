#[allow(clippy::wildcard_imports)]
use super::*;

/// Stable identity for a viewport tool declared by the application.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ViewportToolId(u64);

impl ViewportToolId {
    /// Creates a viewport tool ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns the raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Generic viewport command kind presented through application-owned actions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ViewportActionKind {
    /// Activate a declared viewport tool.
    ActivateTool,
    /// Focus the current selected target or selection set.
    FocusSelected,
    /// Fit the full content bounds in the viewport.
    FitContent,
    /// Fit the current selected target or selection set in the viewport.
    FitSelection,
    /// Show source content at its native/100% size.
    ActualSize,
    /// Increase viewport zoom.
    ZoomIn,
    /// Decrease viewport zoom.
    ZoomOut,
    /// Enter or toggle pan mode.
    PanMode,
    /// Toggle an application-owned viewport overlay.
    ToggleOverlay,
}

impl ViewportActionKind {
    /// Returns true when the action is usually presented as toggled/checkable.
    #[must_use]
    pub const fn is_toggle(self) -> bool {
        matches!(
            self,
            Self::ActivateTool | Self::PanMode | Self::ToggleOverlay
        )
    }
}

/// Stable viewport action target context captured with descriptors and requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportActionTarget {
    /// Stable viewport widget identity.
    pub viewport: WidgetId,
    /// Optional tool identity for tool activation or tool-scoped actions.
    pub tool: Option<ViewportToolId>,
    /// Optional selection target identity for focus/fit requests.
    pub selection: Option<ViewportSelectionTargetId>,
    /// Optional overlay identity for overlay toggle requests.
    pub overlay: Option<ViewportOverlayId>,
}

impl ViewportActionTarget {
    /// Creates viewport-scoped action target context.
    #[must_use]
    pub const fn new(viewport: WidgetId) -> Self {
        Self {
            viewport,
            tool: None,
            selection: None,
            overlay: None,
        }
    }

    /// Adds tool context.
    #[must_use]
    pub const fn with_tool(mut self, tool: ViewportToolId) -> Self {
        self.tool = Some(tool);
        self
    }

    /// Adds selection target context.
    #[must_use]
    pub const fn with_selection(mut self, selection: ViewportSelectionTargetId) -> Self {
        self.selection = Some(selection);
        self
    }

    /// Adds overlay target context.
    #[must_use]
    pub const fn with_overlay(mut self, overlay: ViewportOverlayId) -> Self {
        self.overlay = Some(overlay);
        self
    }
}

/// Data-only viewport action descriptor backed by app-owned action metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportActionDescriptor {
    /// Action metadata shared with menus, toolbars, shortcuts, and command palettes.
    pub action: ActionDescriptor,
    /// Generic viewport command kind used for routing and semantics.
    pub kind: ViewportActionKind,
    /// Stable viewport/tool/target context.
    pub target: ViewportActionTarget,
}

impl ViewportActionDescriptor {
    /// Creates a viewport action descriptor from app-owned action metadata.
    #[must_use]
    pub fn new(
        action: ActionDescriptor,
        kind: ViewportActionKind,
        target: ViewportActionTarget,
    ) -> Self {
        Self {
            action,
            kind,
            target,
        }
    }

    /// Returns the backing action ID.
    #[must_use]
    pub const fn action_id(&self) -> &ActionId {
        &self.action.id
    }

    /// Returns true when this descriptor should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when this descriptor can currently emit a request.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns checked/toggled state when available.
    #[must_use]
    pub const fn checked(&self) -> Option<bool> {
        self.action.state.checked
    }

    /// Returns true when this visible descriptor can emit a viewport request.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates viewport request metadata when the action is visible and enabled.
    #[must_use]
    pub fn request(
        &self,
        source: ActionSource,
        context: ActionContext,
    ) -> Option<ViewportActionRequest> {
        self.can_request().then(|| {
            ViewportActionRequest::new(
                self.action.id.clone(),
                self.kind,
                source,
                context,
                self.target.clone(),
                self.action.state.checked,
            )
        })
    }

    /// Builds semantic metadata for this viewport action when it is visible.
    #[must_use]
    pub fn semantics(&self, root: WidgetId, rect: Rect) -> Option<SemanticNode> {
        viewport_action_semantics(root, rect, self)
    }
}

/// Data-only viewport action request emitted for application execution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViewportActionRequest {
    /// Invoked application action identity.
    pub action_id: ActionId,
    /// Generic viewport command kind.
    pub kind: ViewportActionKind,
    /// Source surface that emitted the request.
    pub source: ActionSource,
    /// Shared action routing context.
    pub context: ActionContext,
    /// Stable viewport/tool/target context.
    pub target: ViewportActionTarget,
    /// Checked/toggled state captured when the request was emitted.
    pub checked: Option<bool>,
}

impl ViewportActionRequest {
    /// Creates viewport action request metadata.
    #[must_use]
    pub fn new(
        action_id: ActionId,
        kind: ViewportActionKind,
        source: ActionSource,
        context: ActionContext,
        target: ViewportActionTarget,
        checked: Option<bool>,
    ) -> Self {
        Self {
            action_id,
            kind,
            source,
            context,
            target,
            checked,
        }
    }

    /// Converts this request to the shared app-owned action invocation boundary.
    #[must_use]
    pub fn action_invocation(&self) -> ActionInvocation {
        ActionInvocation::new(self.action_id.clone(), self.source, self.context.clone())
    }
}

/// Returns request metadata for all visible and enabled viewport action descriptors.
#[must_use]
pub fn viewport_action_requests(
    actions: &[ViewportActionDescriptor],
    source: ActionSource,
    context: &ActionContext,
) -> Vec<ViewportActionRequest> {
    actions
        .iter()
        .filter_map(|action| action.request(source, context.clone()))
        .collect()
}

/// Returns stable semantic ID for an app-owned viewport action.
#[must_use]
pub fn viewport_action_widget_id(root: WidgetId, action: &ActionId) -> WidgetId {
    root.child(("viewport-action", action.as_str()))
}

/// Builds backend-neutral semantic metadata for a viewport action.
#[must_use]
pub fn viewport_action_semantics(
    root: WidgetId,
    rect: Rect,
    action: &ViewportActionDescriptor,
) -> Option<SemanticNode> {
    if !action.visible() {
        return None;
    }

    let enabled = action.enabled();
    let role = if action.kind.is_toggle() || action.checked().is_some() {
        SemanticRole::Toggle
    } else {
        SemanticRole::Button
    };
    let mut node = SemanticNode::new(
        viewport_action_widget_id(root, &action.action.id),
        role,
        sanitize_rect(rect),
    )
    .with_label(action.action.label.clone())
    .focusable(enabled);
    node.description.clone_from(&action.action.tooltip);
    node.state.disabled = !enabled;
    node.state.checked = action.checked();
    node.state.selected = action.action.state.is_checked();
    node.state.value = Some(SemanticValue::Text(format!(
        "{:?} viewport {}",
        action.kind,
        action.target.viewport.raw()
    )));
    if enabled {
        node.actions
            .push(SemanticAction::from_action_descriptor(&action.action));
    }
    Some(node)
}

/// Builds backend-neutral semantic metadata for visible viewport actions.
#[must_use]
pub fn viewport_actions_semantics(
    root: WidgetId,
    bounds: Rect,
    label: impl Into<String>,
    actions: &[ViewportActionDescriptor],
    rects: impl IntoIterator<Item = (ActionId, Rect)>,
) -> Vec<SemanticNode> {
    let rects = rects
        .into_iter()
        .collect::<std::collections::BTreeMap<_, _>>();
    let children = actions
        .iter()
        .filter(|action| action.visible() && rects.contains_key(&action.action.id))
        .map(|action| viewport_action_widget_id(root, &action.action.id))
        .collect::<Vec<_>>();
    let mut nodes = Vec::with_capacity(children.len() + 1);
    nodes.push(
        SemanticNode::new(
            root,
            SemanticRole::Custom("viewport-actions".to_owned()),
            sanitize_rect(bounds),
        )
        .with_label(label)
        .with_children(children),
    );
    nodes.extend(actions.iter().filter_map(|action| {
        let rect = *rects.get(&action.action.id)?;
        viewport_action_semantics(root, rect, action)
    }));
    nodes
}
