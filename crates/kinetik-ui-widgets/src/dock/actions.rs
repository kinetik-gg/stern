mod resolution;

pub(crate) use resolution::{join_request_matches_layout, swap_request_matches_layout};
pub use resolution::{
    locate_panel_instance, resolve_dock_join_request, resolve_dock_splitter_context_actions,
    resolve_dock_splitter_context_actions_with_policy, resolve_dock_swap_request,
    resolve_frame_split_affordance_request, resolve_frame_split_affordance_request_with_policy,
    resolve_panel_affordances, resolve_panel_close_request, resolve_panel_duplicate_request,
    resolve_panel_float_request, resolve_panel_open_decision, resolve_panel_policy_context,
};

use super::{
    ActionId, Axis, Dock, DockNeighborDirection, DockPlacement, DockSplitPath, FrameId,
    PanelDockHint, PanelId, PanelInstanceId, PanelInstanceSnapshot, PanelRegistry,
    PanelTypeDescriptor, PanelTypeId, PanelWorkspaceContext, Size,
};

/// Application-owned metadata carried by panel policy requests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelPolicyMetadata {
    /// Developer-declared panel type identity.
    pub panel_type: PanelTypeId,
    /// Descriptor title used by default app surfaces.
    pub title: String,
    /// Optional application-owned default open action from the descriptor.
    pub default_open_action: Option<ActionId>,
}

impl PanelPolicyMetadata {
    /// Creates request metadata from a panel type descriptor.
    #[must_use]
    pub fn from_descriptor(descriptor: &PanelTypeDescriptor) -> Self {
        Self {
            panel_type: descriptor.id,
            title: descriptor.title.clone(),
            default_open_action: descriptor.default_open_action.clone(),
        }
    }
}

/// Location of an open panel instance in the current dock tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelInstanceLocation {
    /// Stable open panel instance identity.
    pub panel_instance: PanelInstanceId,
    /// Compatibility panel identity used by current dock callers.
    pub panel: PanelId,
    /// Frame currently containing the panel.
    pub frame: FrameId,
}

impl PanelInstanceLocation {
    /// Creates a location from panel instance vocabulary.
    #[must_use]
    pub const fn new(panel_instance: PanelInstanceId, frame: FrameId) -> Self {
        Self {
            panel_instance,
            panel: PanelId::from_instance_id(panel_instance),
            frame,
        }
    }
}

/// Resolved tab and panel affordances for a specific panel instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelAffordances {
    /// Panel type the affordances were resolved from.
    pub panel_type: PanelTypeId,
    /// Open panel instance identity.
    pub panel_instance: PanelInstanceId,
    /// Whether close chrome should be visible.
    pub close_visible: bool,
    /// Whether duplicate/open-another should be available.
    pub duplicate_available: bool,
    /// Whether future floating-surface affordances should be available.
    pub float_available: bool,
}

/// Request for the application to open a new panel instance.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelOpenRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Workspace context requested by the caller.
    pub context: PanelWorkspaceContext,
    /// Preferred dock placement hint, when the descriptor provides one.
    pub dock_hint: Option<PanelDockHint>,
    /// Preferred logical size from the descriptor.
    pub default_size: Size,
}

/// Request for the application to focus an already-open singleton instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFocusRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Existing panel instance to focus.
    pub target: PanelInstanceLocation,
}

/// Decision produced when the user asks to open a panel type.
#[derive(Debug, Clone, PartialEq)]
pub enum PanelOpenDecision {
    /// Focus an existing singleton instance instead of opening another one.
    FocusExisting(PanelFocusRequest),
    /// Ask the application to open a new panel instance.
    OpenNew(PanelOpenRequest),
}

/// Request for the application to close a panel instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelCloseRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Panel instance the application may close.
    pub target: PanelInstanceLocation,
}

/// Request for the application to duplicate a panel instance.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelDuplicateRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Source panel instance to duplicate.
    pub source: PanelInstanceLocation,
    /// Workspace context requested by the caller.
    pub context: PanelWorkspaceContext,
    /// Preferred dock placement hint, when the descriptor provides one.
    pub dock_hint: Option<PanelDockHint>,
    /// Preferred logical size from the descriptor.
    pub default_size: Size,
}

/// Request for a future floating surface without creating a native window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PanelFloatRequest {
    /// Application-owned metadata for the request.
    pub metadata: PanelPolicyMetadata,
    /// Panel instance that may be floated by the application/platform layer.
    pub source: PanelInstanceLocation,
}

/// Inputs used to resolve policy metadata for one panel instance.
///
/// This context borrows the current application-owned registry and panel
/// instance records, plus the dock/frame state needed to resolve frame-owned
/// affordances. It is intentionally read-only.
#[derive(Debug, Clone, Copy)]
pub struct PanelPolicyContext<'a> {
    /// Registry containing developer-declared panel descriptors.
    pub registry: &'a PanelRegistry,
    /// Current application-owned open panel instance records.
    pub panel_instances: &'a [PanelInstanceSnapshot],
    /// Current dock tree used for location and singleton focus lookup.
    pub dock: &'a Dock,
    /// Open panel instance to resolve.
    pub panel_instance: PanelInstanceId,
    /// Frame expected to currently contain the panel instance.
    pub frame: FrameId,
    /// Workspace context requested by the caller.
    pub workspace_context: PanelWorkspaceContext,
}

impl<'a> PanelPolicyContext<'a> {
    /// Creates a read-only panel policy context.
    #[must_use]
    pub const fn new(
        registry: &'a PanelRegistry,
        panel_instances: &'a [PanelInstanceSnapshot],
        dock: &'a Dock,
        panel_instance: PanelInstanceId,
        frame: FrameId,
        workspace_context: PanelWorkspaceContext,
    ) -> Self {
        Self {
            registry,
            panel_instances,
            dock,
            panel_instance,
            frame,
            workspace_context,
        }
    }
}

/// Deterministic reason a panel policy context could not produce requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PanelPolicyUnavailableReason {
    /// No application-owned instance record exists for the requested panel.
    MissingPanelInstance,
    /// The instance record references a panel type missing from the registry.
    MissingDescriptor,
    /// The instance record exists, but the panel is not present in the dock.
    MissingPanelLocation,
    /// The requested frame does not currently own the panel instance.
    MissingFrameMembership,
    /// The requested workspace context is not allowed by the descriptor.
    DisallowedContext,
}

/// Pure result for one resolved panel policy context.
#[derive(Debug, Clone, PartialEq)]
pub struct PanelPolicyResolution {
    /// Requested panel instance.
    pub panel_instance: PanelInstanceId,
    /// Resolved panel type from the instance record, when available.
    pub panel_type: Option<PanelTypeId>,
    /// Frame requested by the caller.
    pub frame: FrameId,
    /// Dock location found for the panel instance, when available.
    pub location: Option<PanelInstanceLocation>,
    /// Workspace context requested by the caller.
    pub workspace_context: PanelWorkspaceContext,
    /// Deterministic unavailable reason. `None` means all requests were
    /// resolved against a valid descriptor, instance, frame, and context.
    pub unavailable: Option<PanelPolicyUnavailableReason>,
    /// Descriptor/frame-derived affordances, when enough context exists.
    pub affordances: Option<PanelAffordances>,
    /// Optional open or focus metadata for the requested context.
    pub open_decision: Option<PanelOpenDecision>,
    /// Optional close metadata for the current panel instance.
    pub close_request: Option<PanelCloseRequest>,
    /// Optional duplicate metadata for the current panel instance.
    pub duplicate_request: Option<PanelDuplicateRequest>,
    /// Optional future floating-surface metadata for the current panel instance.
    pub float_request: Option<PanelFloatRequest>,
}

impl PanelPolicyResolution {
    /// Returns true when the resolver produced requests for an available
    /// descriptor, panel instance, frame membership, and workspace context.
    #[must_use]
    pub const fn is_available(&self) -> bool {
        self.unavailable.is_none()
    }
}

/// Request for an application-owned frame edge split affordance.
///
/// This is separate from tab drag/drop: it describes split intent only and does
/// not mutate the dock tree, create panels, or execute application commands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameSplitAffordanceRequest {
    /// Frame that owns the active panel or command source.
    pub source_frame: FrameId,
    /// Frame whose edge/corner affordance was targeted.
    pub target_frame: FrameId,
    /// Placement of the new frame relative to the target frame.
    pub placement: DockPlacement,
    /// Active source panel identity when the source frame has one.
    pub active_panel: Option<PanelInstanceLocation>,
    /// Application-supplied identity for the frame to be created.
    pub new_frame: FrameId,
}

/// Topology-validated request to join one frame into an adjacent neighbor.
///
/// The request is resolved from frame neighbor topology and is applied by
/// moving the source frame's tabs into the target frame.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockJoinRequest {
    pub(crate) source_frame: FrameId,
    pub(crate) direction: DockNeighborDirection,
    pub(crate) target_frame: FrameId,
}

impl DockJoinRequest {
    /// Returns the frame whose tabs will move into the target frame.
    #[must_use]
    pub const fn source_frame(self) -> FrameId {
        self.source_frame
    }

    /// Returns the requested neighbor direction from the source frame.
    #[must_use]
    pub const fn direction(self) -> DockNeighborDirection {
        self.direction
    }

    /// Returns the resolved neighboring frame that will survive the join.
    #[must_use]
    pub const fn target_frame(self) -> FrameId {
        self.target_frame
    }
}

/// Topology-validated request to swap one frame with an adjacent neighbor.
///
/// The request is resolved from frame neighbor topology and is applied by
/// swapping whole frame leaves in the dock tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockSwapRequest {
    pub(crate) source_frame: FrameId,
    pub(crate) direction: DockNeighborDirection,
    pub(crate) target_frame: FrameId,
}

impl DockSwapRequest {
    /// Returns the source frame that will trade dock-tree positions.
    #[must_use]
    pub const fn source_frame(self) -> FrameId {
        self.source_frame
    }

    /// Returns the requested neighbor direction from the source frame.
    #[must_use]
    pub const fn direction(self) -> DockNeighborDirection {
        self.direction
    }

    /// Returns the resolved neighboring frame that will trade positions.
    #[must_use]
    pub const fn target_frame(self) -> FrameId {
        self.target_frame
    }
}

/// Operation represented by splitter context action metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSplitterContextActionKind {
    /// Join one side of the splitter into the opposite side.
    Join,
    /// Swap the two resolved frame leaves adjacent to the splitter.
    Swap,
}

/// Logical side of a splitter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockSplitterSide {
    /// First split child: left for horizontal splits, top for vertical splits.
    First,
    /// Second split child: right for horizontal splits, bottom for vertical splits.
    Second,
}

/// Target context shared by splitter context actions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSplitterActionContext {
    /// Split path addressed by the context menu source.
    pub path: DockSplitPath,
    /// Split axis addressed by the context menu source.
    pub axis: Axis,
    /// Resolved frame leaf on the first side of the splitter.
    pub first_frame: Option<FrameId>,
    /// Resolved frame leaf on the second side of the splitter.
    pub second_frame: Option<FrameId>,
}

/// Pure app-dispatch metadata for a splitter context action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DockSplitterContextAction {
    /// Operation kind the application may present.
    pub kind: DockSplitterContextActionKind,
    /// Splitter target context.
    pub context: DockSplitterActionContext,
    /// Side that supplies the source frame for the operation.
    pub source_side: DockSplitterSide,
    /// Side that supplies the target frame for the operation.
    pub target_side: DockSplitterSide,
    /// Resolved source frame when available.
    pub source_frame: Option<FrameId>,
    /// Resolved target frame when available.
    pub target_frame: Option<FrameId>,
    /// Direction from the source side toward the target side.
    pub direction: DockNeighborDirection,
    /// Whether the action can be safely dispatched against the current layout.
    pub enabled: bool,
}

impl DockSplitterContextAction {
    /// Returns a validated join request when this enabled action is a join.
    #[must_use]
    pub fn join_request(&self) -> Option<DockJoinRequest> {
        if !self.enabled || !matches!(self.kind, DockSplitterContextActionKind::Join) {
            return None;
        }

        let source_frame = self.source_frame?;
        let target_frame = self.target_frame?;

        Some(DockJoinRequest {
            source_frame,
            direction: self.direction,
            target_frame,
        })
    }

    /// Returns a validated swap request when this enabled action is a swap.
    #[must_use]
    pub fn swap_request(&self) -> Option<DockSwapRequest> {
        if !self.enabled || !matches!(self.kind, DockSplitterContextActionKind::Swap) {
            return None;
        }

        let source_frame = self.source_frame?;
        let target_frame = self.target_frame?;

        Some(DockSwapRequest {
            source_frame,
            direction: self.direction,
            target_frame,
        })
    }
}
