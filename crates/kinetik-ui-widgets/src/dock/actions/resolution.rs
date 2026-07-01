use super::super::{
    Axis, Dock, DockInteractionPolicy, DockNeighborDirection, DockSplitter, Frame, FrameId,
    FrameLayout, FrameNeighbors, PanelClosePolicy, PanelDuplicatePolicy, PanelFloatPolicy, PanelId,
    PanelInstanceId, PanelInstancePolicy, PanelInstanceSnapshot, PanelTypeDescriptor, PanelTypeId,
    PanelWorkspaceContext, Point, collect_frame_ids, frame_is_valid, frame_neighbor,
    resolve_frame_split_affordance_with_policy, split_children_at_path, splitter_adjacent_frame,
};
use super::{
    DockJoinRequest, DockSplitterActionContext, DockSplitterContextAction,
    DockSplitterContextActionKind, DockSplitterSide, DockSwapRequest, FrameSplitAffordanceRequest,
    PanelAffordances, PanelCloseRequest, PanelDuplicateRequest, PanelFloatRequest,
    PanelFocusRequest, PanelInstanceLocation, PanelOpenDecision, PanelOpenRequest,
    PanelPolicyContext, PanelPolicyMetadata, PanelPolicyResolution, PanelPolicyUnavailableReason,
};

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
