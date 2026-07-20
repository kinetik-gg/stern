//! Retained interaction state and output for the public Dock controller.

use stern_core::WidgetId;

use super::{
    DockDropTarget, DockInteractionPolicy, DockSplitPath, DockSplitTopologyFingerprint,
    DockSplitterContextAction, DockTabDrag, FrameId, PanelId, PanelInstanceLocation,
};

/// Caller-owned configuration for one Dock controller evaluation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockControllerConfig {
    /// Interaction policy applied to drop targets and splitters.
    pub policy: DockInteractionPolicy,
    /// Caller-owned unused frame ID used when an edge drop inserts a split.
    pub new_frame: FrameId,
}

impl DockControllerConfig {
    /// Creates controller configuration using the default interaction policy.
    #[must_use]
    pub fn new(new_frame: FrameId) -> Self {
        Self {
            policy: DockInteractionPolicy::default(),
            new_frame,
        }
    }

    /// Sets the interaction policy.
    #[must_use]
    pub const fn with_policy(mut self, policy: DockInteractionPolicy) -> Self {
        self.policy = policy;
        self
    }
}

/// Stable tab focus retained by a [`DockController`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockControllerFocus {
    /// Frame containing the focused panel tab.
    pub frame: FrameId,
    /// Focused panel tab.
    pub panel: PanelId,
    /// Stable widget ID used by UI memory and semantics.
    pub widget: WidgetId,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DockSplitterResizeTransaction {
    pub(crate) widget: WidgetId,
    pub(crate) path: DockSplitPath,
    pub(crate) starting_ratio: f32,
    pub(crate) topology: DockSplitTopologyFingerprint,
}

/// Caller-owned retained state for public Dock interactions.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DockController {
    pub(crate) drag: Option<DockTabDrag>,
    pub(crate) preview: Option<DockDropTarget>,
    pub(crate) focus: Option<DockControllerFocus>,
    pub(crate) splitter_resize: Option<DockSplitterResizeTransaction>,
}

impl DockController {
    /// Creates empty controller state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            drag: None,
            preview: None,
            focus: None,
            splitter_resize: None,
        }
    }

    /// Returns the active tab drag, when one has crossed the drag threshold.
    #[must_use]
    pub const fn tab_drag(&self) -> Option<DockTabDrag> {
        self.drag
    }

    /// Returns the currently resolved drop preview.
    #[must_use]
    pub const fn drop_preview(&self) -> Option<DockDropTarget> {
        self.preview
    }

    /// Returns the controller-owned focused tab.
    #[must_use]
    pub const fn focused_tab(&self) -> Option<DockControllerFocus> {
        self.focus
    }
}

/// Application-owned context-menu request emitted for one splitter.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSplitterContextRequest {
    /// Split-tree path addressed by the request.
    pub path: DockSplitPath,
    /// Pure join and swap metadata available to the application.
    pub actions: Vec<DockSplitterContextAction>,
}

/// Output from one public Dock controller evaluation.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DockControllerOutput {
    /// Whether controller evaluation mutated the Dock snapshot.
    pub changed: bool,
    /// Whether controller evaluation moved or repaired keyboard focus.
    pub focus_changed: bool,
    /// Exact panel locations whose visible close affordance was activated.
    pub close_requests: Vec<PanelInstanceLocation>,
    /// Splitter context requests for application-owned presentation/dispatch.
    pub splitter_context_requests: Vec<DockSplitterContextRequest>,
    /// Drop preview retained for the next prepared [`super::DockScene`].
    pub drop_preview: Option<DockDropTarget>,
}
