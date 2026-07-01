#[allow(clippy::wildcard_imports)]
use super::*;

/// Generic transport intent metadata for action-backed playback controls.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransportControlIntent {
    /// Toggle between play and pause.
    PlayPause,
    /// Stop playback or preview.
    Stop,
    /// Step one unit backward.
    StepBackward,
    /// Step one unit forward.
    StepForward,
    /// Jump to the start of the available range.
    JumpToStart,
    /// Jump to the end of the available range.
    JumpToEnd,
    /// Jump to the previous marker.
    PreviousMarker,
    /// Jump to the next marker.
    NextMarker,
    /// Toggle loop playback.
    LoopToggle,
    /// Toggle playback constrained to the selected or marked range.
    RangePlaybackToggle,
}

impl TransportControlIntent {
    /// Returns a stable generic action ID for this transport intent.
    #[must_use]
    pub const fn default_action_id(self) -> &'static str {
        match self {
            Self::PlayPause => "transport.play-pause",
            Self::Stop => "transport.stop",
            Self::StepBackward => "transport.step-backward",
            Self::StepForward => "transport.step-forward",
            Self::JumpToStart => "transport.jump-to-start",
            Self::JumpToEnd => "transport.jump-to-end",
            Self::PreviousMarker => "transport.previous-marker",
            Self::NextMarker => "transport.next-marker",
            Self::LoopToggle => "transport.loop",
            Self::RangePlaybackToggle => "transport.range-playback",
        }
    }

    /// Returns a human-readable default label for this transport intent.
    #[must_use]
    pub const fn default_label(self) -> &'static str {
        match self {
            Self::PlayPause => "Play/Pause",
            Self::Stop => "Stop",
            Self::StepBackward => "Step Backward",
            Self::StepForward => "Step Forward",
            Self::JumpToStart => "Jump to Start",
            Self::JumpToEnd => "Jump to End",
            Self::PreviousMarker => "Previous Marker",
            Self::NextMarker => "Next Marker",
            Self::LoopToggle => "Loop",
            Self::RangePlaybackToggle => "Range Playback",
        }
    }

    /// Returns the default control kind for this transport intent.
    #[must_use]
    pub const fn default_control_kind(self) -> TransportControlKind {
        match self {
            Self::LoopToggle | Self::RangePlaybackToggle => TransportControlKind::Toggle,
            Self::PlayPause
            | Self::Stop
            | Self::StepBackward
            | Self::StepForward
            | Self::JumpToStart
            | Self::JumpToEnd
            | Self::PreviousMarker
            | Self::NextMarker => TransportControlKind::Button,
        }
    }

    /// Creates a generic action descriptor for this transport intent.
    #[must_use]
    pub fn default_action_descriptor(self) -> ActionDescriptor {
        let mut action = ActionDescriptor::new(self.default_action_id(), self.default_label());
        if self.default_control_kind() == TransportControlKind::Toggle {
            action.state.checked = Some(false);
        }
        action
    }
}

/// Presentation kind for an action-backed transport control.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum TransportControlKind {
    /// Momentary push-button style control.
    Button,
    /// Toggle/checkable style control.
    Toggle,
}

/// Optional timeline context captured with a transport action request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimelineTransportContext {
    /// Timeline surface identity when the transport is associated with a timeline.
    pub timeline: TimelineId,
    /// Current playhead time at request construction, if known.
    pub playhead_time: Option<TimelineTime>,
    /// Current selected or marked playback range, if known.
    pub selection_range: Option<TimelineRange>,
}

impl TimelineTransportContext {
    /// Creates timeline transport context for a timeline surface.
    #[must_use]
    pub const fn new(timeline: TimelineId) -> Self {
        Self {
            timeline,
            playhead_time: None,
            selection_range: None,
        }
    }

    /// Captures current playhead time metadata.
    #[must_use]
    pub const fn with_playhead_time(mut self, playhead_time: TimelineTime) -> Self {
        self.playhead_time = Some(playhead_time);
        self
    }

    /// Captures current range metadata.
    #[must_use]
    pub const fn with_selection_range(mut self, selection_range: TimelineRange) -> Self {
        self.selection_range = Some(selection_range);
        self
    }

    fn sanitized(self) -> Self {
        Self {
            timeline: self.timeline,
            playhead_time: self.playhead_time.map(TimelineTime::sanitized),
            selection_range: self.selection_range.map(TimelineRange::sanitized),
        }
    }
}

/// Data-only request emitted by transport controls for application execution.
#[derive(Debug, Clone, PartialEq)]
pub struct TransportActionRequest {
    /// Invoked action identity.
    pub action_id: ActionId,
    /// Generic transport intent used for presentation.
    pub intent: TransportControlIntent,
    /// Source surface that emitted the action request.
    pub source: ActionSource,
    /// Transport control presentation kind that emitted the request.
    pub control_kind: TransportControlKind,
    /// Optional timeline context captured with the request.
    pub timeline_context: Option<TimelineTransportContext>,
}

impl TransportActionRequest {
    /// Creates transport action request metadata.
    #[must_use]
    pub fn new(
        action_id: ActionId,
        intent: TransportControlIntent,
        source: ActionSource,
        control_kind: TransportControlKind,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Self {
        Self {
            action_id,
            intent,
            source,
            control_kind,
            timeline_context: timeline_context.map(TimelineTransportContext::sanitized),
        }
    }

    /// Converts this request to the shared action invocation boundary.
    #[must_use]
    pub fn action_invocation(&self, context: ActionContext) -> ActionInvocation {
        ActionInvocation::new(self.action_id.clone(), self.source, context)
    }
}

/// Data-only transport control descriptor backed by an app-owned action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransportControlDescriptor {
    /// Stable transport control identity.
    pub id: TransportControlId,
    /// Generic transport intent used for presentation and semantics.
    pub intent: TransportControlIntent,
    /// Action metadata shared with menus, toolbars, shortcuts, and command palettes.
    pub action: ActionDescriptor,
    /// Preferred transport control presentation kind.
    pub control_kind: TransportControlKind,
}

impl TransportControlDescriptor {
    /// Creates a transport control from app-owned action metadata.
    #[must_use]
    pub fn new(
        id: TransportControlId,
        intent: TransportControlIntent,
        action: ActionDescriptor,
    ) -> Self {
        Self {
            id,
            intent,
            action,
            control_kind: intent.default_control_kind(),
        }
    }

    /// Creates a transport control with generic default action metadata.
    #[must_use]
    pub fn from_intent(id: TransportControlId, intent: TransportControlIntent) -> Self {
        Self::new(id, intent, intent.default_action_descriptor())
    }

    /// Sets the transport control presentation kind.
    #[must_use]
    pub const fn with_control_kind(mut self, control_kind: TransportControlKind) -> Self {
        self.control_kind = control_kind;
        self
    }

    /// Returns the backing action ID.
    #[must_use]
    pub const fn action_id(&self) -> &ActionId {
        &self.action.id
    }

    /// Returns true when the control should be presented.
    #[must_use]
    pub const fn visible(&self) -> bool {
        self.action.state.visible
    }

    /// Returns true when the control can currently emit a request.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.action.state.enabled
    }

    /// Returns checked/toggled action state when available.
    #[must_use]
    pub const fn checked(&self) -> Option<bool> {
        self.action.state.checked
    }

    /// Returns true when this control is visible and enabled.
    #[must_use]
    pub const fn can_request(&self) -> bool {
        self.action.can_invoke()
    }

    /// Creates transport action request metadata when the backing action can invoke.
    #[must_use]
    pub fn request(
        &self,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.can_request().then(|| {
            TransportActionRequest::new(
                self.action.id.clone(),
                self.intent,
                source,
                self.control_kind,
                timeline_context,
            )
        })
    }

    /// Creates a shared action invocation when the backing action can invoke.
    #[must_use]
    pub fn action_invocation(&self, context: ActionContext) -> Option<ActionInvocation> {
        self.request(ActionSource::Button, None)
            .map(|request| request.action_invocation(context))
    }
}

/// Data-only transport control model preserving app-supplied presentation order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TransportControls {
    controls: Vec<TransportControlDescriptor>,
}

impl TransportControls {
    /// Creates an empty transport control model.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates transport controls from ordered descriptors.
    #[must_use]
    pub fn from_controls(controls: impl IntoIterator<Item = TransportControlDescriptor>) -> Self {
        Self {
            controls: controls.into_iter().collect(),
        }
    }

    /// Creates generic transport controls from ordered intents.
    #[must_use]
    pub fn from_intents(intents: impl IntoIterator<Item = TransportControlIntent>) -> Self {
        Self::from_controls(intents.into_iter().enumerate().map(|(index, intent)| {
            TransportControlDescriptor::from_intent(
                TransportControlId::from_raw(usize_to_u64_saturating(index)),
                intent,
            )
        }))
    }

    /// Returns all transport controls in presentation order.
    #[must_use]
    pub fn controls(&self) -> &[TransportControlDescriptor] {
        &self.controls
    }

    /// Replaces transport controls.
    pub fn replace_controls(
        &mut self,
        controls: impl IntoIterator<Item = TransportControlDescriptor>,
    ) {
        self.controls = controls.into_iter().collect();
    }

    /// Returns a control by stable identity.
    #[must_use]
    pub fn control(&self, id: TransportControlId) -> Option<&TransportControlDescriptor> {
        self.controls.iter().find(|control| control.id == id)
    }

    /// Returns visible transport controls in presentation order.
    #[must_use]
    pub fn visible_controls(&self) -> Vec<&TransportControlDescriptor> {
        self.controls
            .iter()
            .filter(|control| control.visible())
            .collect()
    }

    /// Creates request metadata for a visible control index.
    #[must_use]
    pub fn request_for_visible(
        &self,
        visible_index: usize,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.visible_controls()
            .get(visible_index)
            .and_then(|control| control.request(source, timeline_context))
    }

    /// Creates request metadata for a stable transport control ID.
    #[must_use]
    pub fn request_for_control(
        &self,
        control_id: TransportControlId,
        source: ActionSource,
        timeline_context: Option<TimelineTransportContext>,
    ) -> Option<TransportActionRequest> {
        self.control(control_id)
            .and_then(|control| control.request(source, timeline_context))
    }
}

/// Rect metadata used by transport semantic helper generation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TransportControlSemanticRect {
    /// Stable transport control identity.
    pub id: TransportControlId,
    /// Control bounds.
    pub rect: Rect,
}

impl TransportControlSemanticRect {
    /// Creates semantic rect metadata for a transport control.
    #[must_use]
    pub const fn new(id: TransportControlId, rect: Rect) -> Self {
        Self { id, rect }
    }
}

/// Builds backend-neutral semantic nodes for transport controls.
#[must_use]
pub fn transport_controls_semantics(
    root: WidgetId,
    bounds: Rect,
    label: impl Into<String>,
    controls: &TransportControls,
    rects: impl IntoIterator<Item = TransportControlSemanticRect>,
) -> Vec<SemanticNode> {
    let rects = rects
        .into_iter()
        .map(|rect| (rect.id, rect.rect))
        .collect::<BTreeMap<_, _>>();
    let children = controls
        .visible_controls()
        .into_iter()
        .filter(|control| rects.contains_key(&control.id))
        .map(|control| transport_control_widget_id(root, control.id))
        .collect::<Vec<_>>();
    let mut nodes = Vec::with_capacity(children.len() + 1);
    nodes.push(
        SemanticNode::new(
            root,
            SemanticRole::Custom("transport-controls".to_owned()),
            finite_rect(bounds),
        )
        .with_label(label)
        .with_children(children),
    );
    nodes.extend(
        controls
            .visible_controls()
            .into_iter()
            .filter_map(|control| {
                let rect = *rects.get(&control.id)?;
                transport_control_semantics(root, rect, control)
            }),
    );
    nodes
}

/// Builds a backend-neutral semantic node for one visible transport control.
#[must_use]
pub fn transport_control_semantics(
    root: WidgetId,
    rect: Rect,
    control: &TransportControlDescriptor,
) -> Option<SemanticNode> {
    if !control.visible() {
        return None;
    }

    let enabled = control.enabled();
    let role = match control.control_kind {
        TransportControlKind::Button => SemanticRole::Button,
        TransportControlKind::Toggle => SemanticRole::Toggle,
    };
    let mut node = SemanticNode::new(
        transport_control_widget_id(root, control.id),
        role,
        finite_rect(rect),
    )
    .with_label(control.action.label.clone())
    .focusable(enabled);
    node.description.clone_from(&control.action.tooltip);
    node.state.disabled = !enabled;
    node.state.checked = control.checked();
    node.state.selected = control.action.state.is_checked();
    node.state.value = Some(SemanticValue::Text(
        control.intent.default_label().to_owned(),
    ));
    if enabled {
        node.actions
            .push(SemanticAction::from_action_descriptor(&control.action));
    }
    Some(node)
}

/// Derives a stable semantic widget ID for a transport control.
#[must_use]
pub fn transport_control_widget_id(root: WidgetId, control: TransportControlId) -> WidgetId {
    root.child(("transport-control", control.raw()))
}
