use super::{
    ActionId, Axis, Dock, DockInteractionPolicy, DockNeighborDirection, DockPlacement,
    DockSplitPath, DockSplitter, Frame, FrameId, FrameLayout, FrameNeighbors, PanelClosePolicy,
    PanelDockHint, PanelDuplicatePolicy, PanelFloatPolicy, PanelId, PanelInstanceId,
    PanelInstancePolicy, PanelInstanceSnapshot, PanelRegistry, PanelTypeDescriptor, PanelTypeId,
    PanelWorkspaceContext, Point, Size, collect_frame_ids, frame_is_valid, frame_neighbor,
    resolve_frame_split_affordance_with_policy, split_children_at_path, splitter_adjacent_frame,
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

/// Finds an open panel instance in deterministic dock tree order.
#[must_use]
pub fn locate_panel_instance(
    dock: &Dock,
    panel_instance: PanelInstanceId,
) -> Option<PanelInstanceLocation> {
    let panel = PanelId::from_instance_id(panel_instance);
    dock.frames()
        .into_iter()
        .find(|frame| frame.panels.iter().any(|item| item.id == panel))
        .map(|frame| PanelInstanceLocation {
            panel_instance,
            panel,
            frame: frame.id,
        })
}

/// Resolves tab and panel affordances without mutating dock or app state.
#[must_use]
pub fn resolve_panel_affordances(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> PanelAffordances {
    let panel = PanelId::from_instance_id(panel_instance);
    let panel_in_frame = frame.panels.iter().any(|item| item.id == panel);
    PanelAffordances {
        panel_type: descriptor.id,
        panel_instance,
        close_visible: descriptor.close_policy == PanelClosePolicy::Closable
            && frame.panel_dismissible(panel),
        duplicate_available: panel_in_frame
            && descriptor.instance_policy == PanelInstancePolicy::MultiInstance
            && descriptor.duplicate_policy == PanelDuplicatePolicy::Allowed,
        float_available: panel_in_frame && descriptor.float_policy == PanelFloatPolicy::Allowed,
    }
}

/// Resolves whether opening a panel type should focus an existing singleton or
/// ask the application to create a new instance.
#[must_use]
pub fn resolve_panel_open_decision(
    descriptor: &PanelTypeDescriptor,
    panel_instances: &[PanelInstanceSnapshot],
    dock: &Dock,
    context: PanelWorkspaceContext,
) -> Option<PanelOpenDecision> {
    if !descriptor.allowed_contexts.contains(&context) {
        return None;
    }

    if descriptor.instance_policy == PanelInstancePolicy::Singleton
        && let Some(target) = locate_first_panel_type_instance(dock, panel_instances, descriptor.id)
    {
        return Some(PanelOpenDecision::FocusExisting(PanelFocusRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            target,
        }));
    }

    Some(PanelOpenDecision::OpenNew(PanelOpenRequest {
        metadata: PanelPolicyMetadata::from_descriptor(descriptor),
        context,
        dock_hint: descriptor.dock_hints.first().copied(),
        default_size: descriptor.default_size,
    }))
}

/// Produces an app-owned close request when descriptor and frame policy allow it.
#[must_use]
pub fn resolve_panel_close_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> Option<PanelCloseRequest> {
    resolve_panel_affordances(descriptor, panel_instance, frame)
        .close_visible
        .then(|| PanelCloseRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            target: PanelInstanceLocation::new(panel_instance, frame.id),
        })
}

/// Produces an app-owned duplicate request without creating a panel.
#[must_use]
pub fn resolve_panel_duplicate_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
    context: PanelWorkspaceContext,
) -> Option<PanelDuplicateRequest> {
    if !resolve_panel_affordances(descriptor, panel_instance, frame).duplicate_available
        || !descriptor.allowed_contexts.contains(&context)
    {
        return None;
    }

    Some(PanelDuplicateRequest {
        metadata: PanelPolicyMetadata::from_descriptor(descriptor),
        source: PanelInstanceLocation::new(panel_instance, frame.id),
        context,
        dock_hint: descriptor.dock_hints.first().copied(),
        default_size: descriptor.default_size,
    })
}

/// Produces an app-owned future float request without creating a native window.
#[must_use]
pub fn resolve_panel_float_request(
    descriptor: &PanelTypeDescriptor,
    panel_instance: PanelInstanceId,
    frame: &Frame,
) -> Option<PanelFloatRequest> {
    resolve_panel_affordances(descriptor, panel_instance, frame)
        .float_available
        .then(|| PanelFloatRequest {
            metadata: PanelPolicyMetadata::from_descriptor(descriptor),
            source: PanelInstanceLocation::new(panel_instance, frame.id),
        })
}

/// Resolves panel affordances and app-owned request metadata from registry,
/// instance, dock, frame, and workspace context.
///
/// The resolver is metadata-only. It composes the focused panel policy helpers
/// and does not mutate dock state, create panels, close panels, duplicate
/// panels, open native windows, or execute application commands.
#[must_use]
pub fn resolve_panel_policy_context(context: PanelPolicyContext<'_>) -> PanelPolicyResolution {
    let Some(instance) = context
        .panel_instances
        .iter()
        .find(|instance| instance.id == context.panel_instance)
    else {
        return unavailable_panel_policy_resolution(
            &context,
            None,
            None,
            None,
            PanelPolicyUnavailableReason::MissingPanelInstance,
        );
    };

    let panel_type = Some(instance.panel_type);
    let location = locate_panel_instance(context.dock, context.panel_instance);

    let Some(descriptor) = context.registry.descriptor(instance.panel_type) else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            location,
            None,
            PanelPolicyUnavailableReason::MissingDescriptor,
        );
    };

    let Some(location) = location else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            None,
            None,
            PanelPolicyUnavailableReason::MissingPanelLocation,
        );
    };

    let panel = PanelId::from_instance_id(context.panel_instance);
    let Some(frame) = context.dock.frame(context.frame) else {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            None,
            PanelPolicyUnavailableReason::MissingFrameMembership,
        );
    };

    if location.frame != context.frame || !frame.panels.iter().any(|item| item.id == panel) {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            None,
            PanelPolicyUnavailableReason::MissingFrameMembership,
        );
    }

    let affordances = resolve_panel_affordances(descriptor, context.panel_instance, frame);

    if !descriptor
        .allowed_contexts
        .contains(&context.workspace_context)
    {
        return unavailable_panel_policy_resolution(
            &context,
            panel_type,
            Some(location),
            Some(affordances),
            PanelPolicyUnavailableReason::DisallowedContext,
        );
    }

    PanelPolicyResolution {
        panel_instance: context.panel_instance,
        panel_type,
        frame: context.frame,
        location: Some(location),
        workspace_context: context.workspace_context,
        unavailable: None,
        affordances: Some(affordances),
        open_decision: resolve_panel_open_decision(
            descriptor,
            context.panel_instances,
            context.dock,
            context.workspace_context,
        ),
        close_request: resolve_panel_close_request(descriptor, context.panel_instance, frame),
        duplicate_request: resolve_panel_duplicate_request(
            descriptor,
            context.panel_instance,
            frame,
            context.workspace_context,
        ),
        float_request: resolve_panel_float_request(descriptor, context.panel_instance, frame),
    }
}

fn unavailable_panel_policy_resolution(
    context: &PanelPolicyContext<'_>,
    panel_type: Option<PanelTypeId>,
    location: Option<PanelInstanceLocation>,
    affordances: Option<PanelAffordances>,
    reason: PanelPolicyUnavailableReason,
) -> PanelPolicyResolution {
    PanelPolicyResolution {
        panel_instance: context.panel_instance,
        panel_type,
        frame: context.frame,
        location,
        workspace_context: context.workspace_context,
        unavailable: Some(reason),
        affordances,
        open_decision: None,
        close_request: None,
        duplicate_request: None,
        float_request: None,
    }
}

/// Resolves an app-owned frame edge split request from frame layouts.
///
/// Center/tab-merge zones and invalid geometry return `None`. The returned
/// request is metadata only; callers decide what panel content to create or
/// move before applying any future dock mutation.
#[must_use]
pub fn resolve_frame_split_affordance_request(
    dock: &Dock,
    frames: &[FrameLayout],
    source_frame: FrameId,
    point: Point,
    new_frame: FrameId,
) -> Option<FrameSplitAffordanceRequest> {
    resolve_frame_split_affordance_request_with_policy(
        dock,
        frames,
        source_frame,
        point,
        new_frame,
        DockInteractionPolicy::default(),
    )
}

/// Resolves a pure frame split affordance request using dock interaction policy.
#[must_use]
pub fn resolve_frame_split_affordance_request_with_policy(
    dock: &Dock,
    frames: &[FrameLayout],
    source_frame: FrameId,
    point: Point,
    new_frame: FrameId,
    policy: DockInteractionPolicy,
) -> Option<FrameSplitAffordanceRequest> {
    let source = dock.frame(source_frame)?;
    let active_panel = active_panel_location(source);
    let (target_frame, placement) =
        resolve_frame_split_affordance_with_policy(frames, point, policy)?;
    dock.frame(target_frame)?;

    Some(FrameSplitAffordanceRequest {
        source_frame,
        target_frame,
        placement,
        active_panel,
        new_frame,
    })
}

/// Resolves a neighbor join request from solved frame neighbor topology.
///
/// The source frame must have a distinct resolved target in the requested
/// direction, and that target must also appear in the supplied topology.
#[must_use]
pub fn resolve_dock_join_request(
    neighbors: &[FrameNeighbors],
    source_frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<DockJoinRequest> {
    let source_neighbors = neighbors
        .iter()
        .find(|neighbors| neighbors.frame == source_frame)?;
    let target_frame = source_neighbors.neighbor(direction)?;
    if target_frame == source_frame
        || !neighbors
            .iter()
            .any(|neighbors| neighbors.frame == target_frame)
    {
        return None;
    }

    Some(DockJoinRequest {
        source_frame,
        direction,
        target_frame,
    })
}

/// Resolves a neighbor swap request from solved frame neighbor topology.
///
/// The source frame must have a distinct resolved target in the requested
/// direction, and that target must also appear in the supplied topology.
#[must_use]
pub fn resolve_dock_swap_request(
    neighbors: &[FrameNeighbors],
    source_frame: FrameId,
    direction: DockNeighborDirection,
) -> Option<DockSwapRequest> {
    let source_neighbors = neighbors
        .iter()
        .find(|neighbors| neighbors.frame == source_frame)?;
    let target_frame = source_neighbors.neighbor(direction)?;
    if target_frame == source_frame
        || !neighbors
            .iter()
            .any(|neighbors| neighbors.frame == target_frame)
    {
        return None;
    }

    Some(DockSwapRequest {
        source_frame,
        direction,
        target_frame,
    })
}

/// Resolves pure context action metadata for a dock splitter.
///
/// The returned actions are stable and do not mutate dock state, enqueue
/// application actions, or execute commands. Invalid paths, stale splitters,
/// invalid geometry, or missing adjacent frames produce disabled actions with
/// the unresolved frame context preserved as `None`.
#[must_use]
pub fn resolve_dock_splitter_context_actions(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
) -> Vec<DockSplitterContextAction> {
    resolve_dock_splitter_context_actions_with_policy(
        dock,
        frames,
        splitter,
        DockInteractionPolicy::default(),
    )
}

/// Resolves pure context action metadata using dock interaction policy.
///
/// Disabled join or swap policy leaves action metadata present but disabled.
#[must_use]
pub fn resolve_dock_splitter_context_actions_with_policy(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
    policy: DockInteractionPolicy,
) -> Vec<DockSplitterContextAction> {
    let context = resolve_dock_splitter_action_context(dock, frames, splitter);
    let (first_to_second, second_to_first) = splitter_context_directions(splitter.axis);
    let policy = policy.sanitized();

    vec![
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Join,
                source_side: DockSplitterSide::First,
                target_side: DockSplitterSide::Second,
                direction: first_to_second,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Join,
                source_side: DockSplitterSide::Second,
                target_side: DockSplitterSide::First,
                direction: second_to_first,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Swap,
                source_side: DockSplitterSide::First,
                target_side: DockSplitterSide::Second,
                direction: first_to_second,
            },
            context.clone(),
        ),
        dock_splitter_context_action(
            dock,
            frames,
            policy,
            DockSplitterActionSpec {
                kind: DockSplitterContextActionKind::Swap,
                source_side: DockSplitterSide::Second,
                target_side: DockSplitterSide::First,
                direction: second_to_first,
            },
            context,
        ),
    ]
}

fn resolve_dock_splitter_action_context(
    dock: &Dock,
    frames: &[FrameLayout],
    splitter: &DockSplitter,
) -> DockSplitterActionContext {
    let Some((axis, first, second)) = split_children_at_path(&dock.root, splitter.path.elements())
    else {
        return DockSplitterActionContext {
            path: splitter.path.clone(),
            axis: splitter.axis,
            first_frame: None,
            second_frame: None,
        };
    };

    if axis != splitter.axis {
        return DockSplitterActionContext {
            path: splitter.path.clone(),
            axis: splitter.axis,
            first_frame: None,
            second_frame: None,
        };
    }

    let first_frames = collect_frame_ids(first);
    let second_frames = collect_frame_ids(second);

    DockSplitterActionContext {
        path: splitter.path.clone(),
        axis: splitter.axis,
        first_frame: splitter_adjacent_frame(
            frames,
            &first_frames,
            splitter,
            DockSplitterSide::First,
        ),
        second_frame: splitter_adjacent_frame(
            frames,
            &second_frames,
            splitter,
            DockSplitterSide::Second,
        ),
    }
}

#[derive(Debug, Clone, Copy)]
struct DockSplitterActionSpec {
    kind: DockSplitterContextActionKind,
    source_side: DockSplitterSide,
    target_side: DockSplitterSide,
    direction: DockNeighborDirection,
}

fn dock_splitter_context_action(
    dock: &Dock,
    frames: &[FrameLayout],
    policy: DockInteractionPolicy,
    spec: DockSplitterActionSpec,
    context: DockSplitterActionContext,
) -> DockSplitterContextAction {
    let source_frame = splitter_context_frame(&context, spec.source_side);
    let target_frame = splitter_context_frame(&context, spec.target_side);
    let enabled = policy.allows_splitter_action(spec.kind)
        && source_frame
            .zip(target_frame)
            .is_some_and(|(source_frame, target_frame)| {
                if source_frame == target_frame
                    || !dock.frame(source_frame).is_some_and(frame_is_valid)
                    || !dock.frame(target_frame).is_some_and(frame_is_valid)
                {
                    return false;
                }

                match spec.kind {
                    DockSplitterContextActionKind::Join => join_request_matches_layout(
                        frames,
                        DockJoinRequest {
                            source_frame,
                            direction: spec.direction,
                            target_frame,
                        },
                    ),
                    DockSplitterContextActionKind::Swap => swap_request_matches_layout(
                        frames,
                        DockSwapRequest {
                            source_frame,
                            direction: spec.direction,
                            target_frame,
                        },
                    ),
                }
            });

    DockSplitterContextAction {
        kind: spec.kind,
        context,
        source_side: spec.source_side,
        target_side: spec.target_side,
        source_frame,
        target_frame,
        direction: spec.direction,
        enabled,
    }
}

fn splitter_context_frame(
    context: &DockSplitterActionContext,
    side: DockSplitterSide,
) -> Option<FrameId> {
    match side {
        DockSplitterSide::First => context.first_frame,
        DockSplitterSide::Second => context.second_frame,
    }
}

fn splitter_context_directions(axis: Axis) -> (DockNeighborDirection, DockNeighborDirection) {
    match axis {
        Axis::Horizontal => (DockNeighborDirection::Right, DockNeighborDirection::Left),
        Axis::Vertical => (DockNeighborDirection::Down, DockNeighborDirection::Up),
    }
}

pub(crate) fn join_request_matches_layout(
    frames: &[FrameLayout],
    request: DockJoinRequest,
) -> bool {
    request.source_frame != request.target_frame
        && frame_neighbor(frames, request.source_frame, request.direction)
            == Some(request.target_frame)
}

pub(crate) fn swap_request_matches_layout(
    frames: &[FrameLayout],
    request: DockSwapRequest,
) -> bool {
    request.source_frame != request.target_frame
        && frame_neighbor(frames, request.source_frame, request.direction)
            == Some(request.target_frame)
}

fn locate_first_panel_type_instance(
    dock: &Dock,
    panel_instances: &[PanelInstanceSnapshot],
    panel_type: PanelTypeId,
) -> Option<PanelInstanceLocation> {
    dock.frames().into_iter().find_map(|frame| {
        frame.panels.iter().find_map(|panel| {
            let panel_instance = panel.instance_id();
            panel_instances
                .iter()
                .any(|instance| instance.id == panel_instance && instance.panel_type == panel_type)
                .then_some(PanelInstanceLocation {
                    panel_instance,
                    panel: panel.id,
                    frame: frame.id,
                })
        })
    })
}

fn active_panel_location(frame: &Frame) -> Option<PanelInstanceLocation> {
    frame.active_panel().map(|panel| PanelInstanceLocation {
        panel_instance: panel.instance_id(),
        panel: panel.id,
        frame: frame.id,
    })
}
