//! Retained UI memory.

use std::collections::{HashMap, HashSet};

use crate::{
    InputStreamConflict, LivenessRegistry, LivenessRemovalStatus, LivenessTargetId, LivenessToken,
    LivenessUpdateStatus, Modifiers, ObserverDelivery, ObserverDrain, ObserverNotification,
    ObserverNotificationId, ObserverPublishStatus, ObserverRegistry, ObserverSubscriptionHandle,
    ObserverSubscriptionId, Point, Response, UiInput, UiInputEvent, Vec2, WidgetId,
};

/// Frame-local routing decision for one pointer event class.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum PointerRoute {
    /// No explicit plan was installed, preserving low-level compatibility behavior.
    #[default]
    Unplanned,
    /// An explicit plan blocks this event class at the current pointer position.
    Blocked,
    /// One exact widget owns this event class.
    Target(WidgetId),
}

impl PointerRoute {
    pub(crate) fn allows(self, id: WidgetId) -> bool {
        match self {
            Self::Unplanned => true,
            Self::Blocked => false,
            Self::Target(owner) => owner == id,
        }
    }
}

/// Ordinary, drop, and wheel routes resolved from one frame-local target plan.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PointerRoutes {
    /// Exact owner for primary/secondary hover, press, focus, and drag behavior.
    pub ordinary: PointerRoute,
    /// Exact destination owner available alongside a captured drag source.
    pub drop: PointerRoute,
    /// Exact scroll viewport allowed to consume wheel input.
    pub wheel: PointerRoute,
}

/// Logical access mode for the widget that owns ordered text input.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextInputOwnerMode {
    /// The owner may mutate text and activate platform IME.
    Editable,
    /// The owner may navigate, select, and copy without activating platform IME.
    ReadOnly,
}

impl PointerRoutes {
    pub(crate) const BLOCKED: Self = Self {
        ordinary: PointerRoute::Blocked,
        drop: PointerRoute::Blocked,
        wheel: PointerRoute::Blocked,
    };
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum RootInputValidation {
    #[default]
    Unvalidated,
    Valid,
    Conflict(InputStreamConflict),
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum PlatformTextInputState {
    #[default]
    Inactive,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct PointerGesture {
    owner: WidgetId,
    press_origin: Point,
    threshold_crossed: bool,
    kind: PointerGestureKind,
    click_count: u8,
    selection_anchor: Option<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum PointerGestureKind {
    Press,
    DomainDrag,
    Selection,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CancelledPointerGesture {
    owner: WidgetId,
    kind: Option<PointerGestureKind>,
    click_count: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PlannedDragRelease {
    pub source: WidgetId,
    pub ordinal: usize,
}

/// Retained interaction and widget state owned by the UI runtime.
///
/// Memory is deliberately non-cloneable because it contains authority-scoped
/// async liveness and observer registries.
///
/// ```compile_fail
/// use kinetik_ui_core::UiMemory;
///
/// let memory = UiMemory::new();
/// let _authority_copy = memory.clone();
/// ```
#[derive(Debug, Default, PartialEq)]
pub struct UiMemory {
    /// Widget hovered during the current frame.
    hovered: Option<WidgetId>,
    /// Widget with keyboard focus.
    focused: Option<WidgetId>,
    /// Widget currently active for modal-like interaction.
    active: Option<WidgetId>,
    /// Widget currently pressed.
    pressed: Option<WidgetId>,
    /// Widget that owns the current secondary-button press.
    secondary_pressed: Option<WidgetId>,
    /// Widget currently holding pointer capture.
    pointer_capture: Option<WidgetId>,
    /// Widget that released pointer capture during this frame.
    pointer_capture_released: Option<WidgetId>,
    /// Greatest root event ordinal that released primary capture this frame.
    pointer_capture_released_ordinal: Option<usize>,
    /// Pointer interaction was cancelled at this frame's runtime boundary.
    pointer_interaction_cancelled: bool,
    /// Owner whose retained primary gesture was cancelled during this frame.
    cancelled_pointer_gesture: Option<CancelledPointerGesture>,
    pointer_routes: PointerRoutes,
    planned_drag_release: Option<PlannedDragRelease>,
    planned_drag_source: Option<WidgetId>,
    pointer_cursor_equivalents: HashSet<WidgetId>,
    scoped_pointer_cleanup_events: HashSet<usize>,
    scoped_pointer_event_ordinals: Vec<usize>,
    scoped_primary_transaction_open: Vec<bool>,
    selection_gesture_claims: HashSet<WidgetId>,
    domain_drag_frame_open: bool,
    domain_drag_responses: HashMap<WidgetId, Response>,
    /// Retained primary press origin and threshold latch.
    pointer_gesture: Option<PointerGesture>,
    /// Widget currently acting as an active drag source.
    drag_source: Option<WidgetId>,
    /// Drag source released during this frame.
    released_drag_source: Option<WidgetId>,
    /// Canonical root event ordinal that released the drag source.
    released_drag_ordinal: Option<usize>,
    /// Text-editing widget currently owning the ordered editing domain.
    text_input_owner: Option<WidgetId>,
    /// Logical access mode for the ordered text-input owner.
    text_input_owner_mode: Option<TextInputOwnerMode>,
    /// Whether platform text input is active for the logical Editable owner.
    platform_text_input_state: PlatformTextInputState,
    /// Text-input owner that claimed this frame's ordered editing stream.
    text_input_event_claim: Option<WidgetId>,
    /// Root stream validation recorded once by the frame runtime.
    root_input_validation: RootInputValidation,
    /// Modifier state retained before the next canonical root event.
    ordered_modifiers: Modifiers,
    /// Whether modifier/key changes are ignored until focus returns.
    ordered_modifiers_suspended: bool,
    /// Text-editing widget whose platform text input should be stopped.
    pending_text_input_stop: Option<WidgetId>,
    scroll_offsets: HashMap<WidgetId, Vec2>,
    pending_scroll_offsets: HashMap<WidgetId, Vec2>,
    open_popovers: HashSet<WidgetId>,
    liveness: LivenessRegistry,
    observers: ObserverRegistry,
}

impl UiMemory {
    /// Creates empty UI memory.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Clears transient frame state while preserving retained widget state.
    pub fn begin_frame(&mut self) {
        self.hovered = None;
        self.pointer_capture_released = None;
        self.pointer_capture_released_ordinal = None;
        self.released_drag_source = None;
        self.released_drag_ordinal = None;
        self.pointer_interaction_cancelled = false;
        self.cancelled_pointer_gesture = None;
        self.pointer_routes = PointerRoutes::default();
        self.planned_drag_release = None;
        self.planned_drag_source = None;
        self.pointer_cursor_equivalents.clear();
        self.scoped_pointer_cleanup_events.clear();
        self.scoped_pointer_event_ordinals.clear();
        self.scoped_primary_transaction_open.clear();
        self.selection_gesture_claims.clear();
        self.domain_drag_frame_open = true;
        self.domain_drag_responses.clear();
        self.text_input_event_claim = None;
        self.root_input_validation = RootInputValidation::Unvalidated;
        self.liveness.begin_frame();
        self.observers.prune_inactive_subscriptions();
    }

    /// Removes liveness targets not seen during the current frame.
    pub(crate) fn end_frame(&mut self) {
        self.domain_drag_frame_open = false;
        self.domain_drag_responses.clear();
        self.scroll_offsets
            .extend(std::mem::take(&mut self.pending_scroll_offsets));
        self.liveness.end_frame();
    }

    /// Returns the widget hovered during the current frame.
    #[must_use]
    pub const fn hovered(&self) -> Option<WidgetId> {
        self.hovered
    }

    /// Returns the widget with keyboard focus.
    #[must_use]
    pub const fn focused(&self) -> Option<WidgetId> {
        self.focused
    }

    /// Returns the widget currently active for pointer or modal interaction.
    #[must_use]
    pub const fn active(&self) -> Option<WidgetId> {
        self.active
    }

    /// Returns the widget currently pressed.
    #[must_use]
    pub const fn pressed(&self) -> Option<WidgetId> {
        self.pressed
    }

    /// Returns the widget that owns the current secondary-button press.
    #[must_use]
    pub const fn secondary_pressed(&self) -> Option<WidgetId> {
        self.secondary_pressed
    }

    /// Returns the widget currently holding pointer capture.
    #[must_use]
    pub const fn pointer_capture(&self) -> Option<WidgetId> {
        self.pointer_capture
    }

    /// Returns the widget that owns pointer routing during this frame.
    #[must_use]
    pub(crate) const fn pointer_routing_owner(&self) -> Option<WidgetId> {
        match self.pointer_capture {
            Some(owner) => Some(owner),
            None => self.pointer_capture_released,
        }
    }

    /// Returns the ordinary route installed for the current frame.
    #[must_use]
    pub const fn pointer_route(&self) -> PointerRoute {
        self.pointer_routes.ordinary
    }

    /// Returns the drop route installed for the current frame.
    #[must_use]
    pub const fn pointer_drop_route(&self) -> PointerRoute {
        self.pointer_routes.drop
    }

    /// Returns the wheel route installed for the current frame.
    #[must_use]
    pub const fn pointer_wheel_route(&self) -> PointerRoute {
        self.pointer_routes.wheel
    }

    pub(crate) fn install_pointer_routes(
        &mut self,
        routes: PointerRoutes,
        cursor_equivalents: impl IntoIterator<Item = WidgetId>,
        planned_drag_release: Option<PlannedDragRelease>,
        planned_drag_source: Option<WidgetId>,
    ) {
        self.pointer_routes = routes;
        self.planned_drag_release = planned_drag_release;
        self.planned_drag_source = planned_drag_source;
        self.pointer_cursor_equivalents.clear();
        self.pointer_cursor_equivalents.extend(cursor_equivalents);
    }

    pub(crate) fn pointer_route_allows(&self, id: WidgetId) -> bool {
        if self.pointer_interaction_cancelled {
            return false;
        }
        self.pointer_routing_owner().map_or_else(
            || self.pointer_routes.ordinary.allows(id),
            |owner| owner == id,
        )
    }

    pub(crate) fn canonical_primary_route_allows(
        &self,
        id: WidgetId,
        event_ordinal: usize,
    ) -> bool {
        if self.pointer_interaction_cancelled {
            return false;
        }
        if let Some(owner) = self.pointer_capture {
            return owner == id;
        }
        match (
            self.pointer_capture_released,
            self.pointer_capture_released_ordinal,
        ) {
            (Some(owner), Some(release_ordinal)) if event_ordinal <= release_ordinal => owner == id,
            (Some(owner), None) => owner == id,
            (Some(_), Some(_)) | (None, None) => self.pointer_routes.ordinary.allows(id),
            (None, Some(_)) => {
                debug_assert!(false, "released pointer owner and ordinal must be paired");
                false
            }
        }
    }

    pub(crate) fn pointer_drop_route_allows(&self, id: WidgetId) -> bool {
        !self.pointer_interaction_cancelled && self.pointer_routes.drop.allows(id)
    }

    pub(crate) fn pointer_drop_route_is_planned_for(&self, id: WidgetId) -> bool {
        self.pointer_routes.drop == PointerRoute::Target(id)
    }

    pub(crate) const fn planned_drag_release(&self) -> Option<PlannedDragRelease> {
        self.planned_drag_release
    }

    pub(crate) const fn planned_drag_source(&self) -> Option<WidgetId> {
        self.planned_drag_source
    }

    pub(crate) fn pointer_wheel_route_allows(&self, id: WidgetId) -> bool {
        !self.pointer_interaction_cancelled && self.pointer_routes.wheel.allows(id)
    }

    pub(crate) fn pointer_wheel_route_matches(&self, id: WidgetId) -> bool {
        self.pointer_routes.wheel.allows(id)
    }

    pub(crate) fn cursor_owner_matches_capture(
        &self,
        captured: WidgetId,
        requested: WidgetId,
    ) -> bool {
        captured == requested
            || (self.pointer_cursor_equivalents.contains(&captured)
                && self.pointer_cursor_equivalents.contains(&requested))
    }

    /// Returns true when pointer interaction was cancelled before primitive evaluation.
    #[must_use]
    pub const fn pointer_interaction_cancelled(&self) -> bool {
        self.pointer_interaction_cancelled
    }

    /// Returns the widget currently acting as an active drag source.
    #[must_use]
    pub const fn drag_source(&self) -> Option<WidgetId> {
        self.drag_source
    }

    /// Returns the drag source released during this frame.
    #[must_use]
    pub const fn released_drag_source(&self) -> Option<WidgetId> {
        self.released_drag_source
    }

    pub(crate) const fn released_drag_ordinal(&self) -> Option<usize> {
        self.released_drag_ordinal
    }

    /// Returns the widget currently owning the ordered text-input domain.
    #[must_use]
    pub const fn text_input_owner(&self) -> Option<WidgetId> {
        self.text_input_owner
    }

    /// Returns the logical access mode of the ordered text-input owner.
    #[must_use]
    pub const fn text_input_owner_mode(&self) -> Option<TextInputOwnerMode> {
        self.text_input_owner_mode
    }

    /// Returns true when the widget is hovered during the current frame.
    #[must_use]
    pub fn is_hovered(&self, id: WidgetId) -> bool {
        self.hovered == Some(id)
    }

    /// Returns true when the widget owns keyboard focus.
    #[must_use]
    pub fn is_focused(&self, id: WidgetId) -> bool {
        self.focused == Some(id)
    }

    /// Returns true when the widget owns active interaction capture.
    #[must_use]
    pub fn is_active(&self, id: WidgetId) -> bool {
        self.active == Some(id)
    }

    /// Returns true when the widget is the current pressed widget.
    #[must_use]
    pub fn is_pressed(&self, id: WidgetId) -> bool {
        self.pressed == Some(id)
    }

    /// Returns true when the widget owns the current secondary-button press.
    #[must_use]
    pub fn is_secondary_pressed(&self, id: WidgetId) -> bool {
        self.secondary_pressed == Some(id)
    }

    /// Returns true when the widget owns pointer capture.
    #[must_use]
    pub fn has_pointer_capture(&self, id: WidgetId) -> bool {
        self.pointer_capture == Some(id)
    }

    /// Returns true when the widget is the active drag source.
    #[must_use]
    pub fn is_drag_source(&self, id: WidgetId) -> bool {
        self.drag_source == Some(id)
    }

    /// Returns true when the widget owns the ordered text-input domain.
    #[must_use]
    pub fn owns_text_input(&self, id: WidgetId) -> bool {
        self.text_input_owner == Some(id)
    }

    /// Claims the current frame's ordered text-editing stream for its owner.
    ///
    /// A frame has at most one successful claim. Changing ownership after a
    /// successful claim never replays the stream for the new owner.
    pub fn claim_text_input_events(&mut self, id: WidgetId) -> bool {
        if self.text_input_owner != Some(id) || self.text_input_event_claim.is_some() {
            return false;
        }
        self.text_input_event_claim = Some(id);
        !matches!(self.root_input_validation, RootInputValidation::Conflict(_))
    }

    /// Resolves ordered text events using the validation authority for this frame.
    ///
    /// Runtime-owned frames validate the root once, then scoped components
    /// revalidate only unchanged text-domain projections. Standalone component
    /// callers retain full canonical/projection validation.
    #[doc(hidden)]
    pub fn effective_text_input_events(
        &self,
        input: &UiInput,
    ) -> Result<Vec<UiInputEvent>, InputStreamConflict> {
        match self.root_input_validation {
            RootInputValidation::Unvalidated => input.effective_text_events(),
            RootInputValidation::Valid => input.effective_scoped_text_events(),
            RootInputValidation::Conflict(conflict) => Err(conflict),
        }
    }

    pub(crate) fn set_root_input_validation(
        &mut self,
        validation: Result<(), InputStreamConflict>,
    ) {
        debug_assert_eq!(self.root_input_validation, RootInputValidation::Unvalidated);
        self.root_input_validation = match validation {
            Ok(()) => RootInputValidation::Valid,
            Err(conflict) => RootInputValidation::Conflict(conflict),
        };
    }

    pub(crate) const fn root_input_conflict(&self) -> Option<InputStreamConflict> {
        match self.root_input_validation {
            RootInputValidation::Conflict(conflict) => Some(conflict),
            RootInputValidation::Unvalidated | RootInputValidation::Valid => None,
        }
    }

    pub(crate) const fn ordered_modifier_state(&self) -> (Modifiers, bool) {
        (self.ordered_modifiers, self.ordered_modifiers_suspended)
    }

    pub(crate) fn set_ordered_modifier_state(&mut self, modifiers: Modifiers, suspended: bool) {
        self.ordered_modifiers = modifiers;
        self.ordered_modifiers_suspended = suspended;
    }

    pub(crate) fn pointer_input_conflicted(&self, input: &UiInput) -> bool {
        match self.root_input_validation {
            RootInputValidation::Unvalidated => input.validate_event_stream().is_err(),
            RootInputValidation::Valid => false,
            RootInputValidation::Conflict(_) => true,
        }
    }

    /// Marks the widget as hovered for this frame.
    pub fn set_hovered(&mut self, id: WidgetId) {
        self.hovered = Some(id);
    }

    /// Moves keyboard focus to a widget.
    pub fn focus(&mut self, id: WidgetId) {
        self.focused = Some(id);
        self.clear_stale_text_input_owner();
    }

    /// Sets keyboard focus explicitly.
    pub fn set_focused(&mut self, focused: Option<WidgetId>) {
        self.focused = focused;
        self.clear_stale_text_input_owner();
    }

    /// Clears keyboard focus.
    pub fn clear_focus(&mut self) {
        self.focused = None;
        self.clear_stale_text_input_owner();
    }

    /// Captures active pointer/modal interaction for a widget.
    pub fn activate(&mut self, id: WidgetId) {
        self.active = Some(id);
    }

    /// Marks a widget as pressed.
    pub fn press(&mut self, id: WidgetId) {
        self.pressed = Some(id);
    }

    /// Marks a widget as owning the current secondary-button press.
    pub fn press_secondary(&mut self, id: WidgetId) {
        self.secondary_pressed = Some(id);
    }

    /// Clears the secondary-button press when held by the provided widget.
    pub fn release_secondary(&mut self, id: WidgetId) {
        if self.secondary_pressed == Some(id) {
            self.secondary_pressed = None;
        }
    }

    /// Captures pointer routing for a widget until explicitly released.
    pub fn capture_pointer(&mut self, id: WidgetId) {
        self.pointer_capture = Some(id);
    }

    pub(crate) fn begin_pointer_gesture(
        &mut self,
        owner: WidgetId,
        press_origin: Point,
        kind: PointerGestureKind,
        click_count: u8,
    ) {
        self.pointer_gesture = Some(PointerGesture {
            owner,
            press_origin,
            threshold_crossed: false,
            kind,
            click_count,
            selection_anchor: None,
        });
    }

    /// Stores an opaque consumer-defined anchor on the active Selection gesture.
    ///
    /// The token has the same retained lifetime as the captured gesture. Core
    /// deliberately does not interpret it.
    #[doc(hidden)]
    pub fn set_selection_gesture_anchor(&mut self, owner: WidgetId, anchor: usize) -> bool {
        let Some(gesture) = &mut self.pointer_gesture else {
            return false;
        };
        if gesture.owner != owner || gesture.kind != PointerGestureKind::Selection {
            return false;
        }
        gesture.selection_anchor = Some(anchor);
        true
    }

    /// Returns the opaque anchor stored on the matching active Selection gesture.
    #[doc(hidden)]
    #[must_use]
    pub fn selection_gesture_anchor(&self, owner: WidgetId) -> Option<usize> {
        self.pointer_gesture
            .filter(|gesture| {
                gesture.owner == owner && gesture.kind == PointerGestureKind::Selection
            })
            .and_then(|gesture| gesture.selection_anchor)
    }

    pub(crate) fn pointer_gesture(&self, owner: WidgetId) -> Option<(Point, bool)> {
        self.pointer_gesture
            .filter(|gesture| gesture.owner == owner)
            .map(|gesture| (gesture.press_origin, gesture.threshold_crossed))
    }

    pub(crate) const fn pointer_gesture_owner(&self) -> Option<WidgetId> {
        match self.pointer_gesture {
            Some(gesture) => Some(gesture.owner),
            None => None,
        }
    }

    pub(crate) fn pointer_gesture_kind(&self, owner: WidgetId) -> Option<PointerGestureKind> {
        self.pointer_gesture
            .filter(|gesture| gesture.owner == owner)
            .map(|gesture| gesture.kind)
    }

    pub(crate) fn domain_drag_gesture(&self) -> Option<(WidgetId, Point, bool)> {
        self.pointer_gesture
            .filter(|gesture| gesture.kind == PointerGestureKind::DomainDrag)
            .map(|gesture| {
                (
                    gesture.owner,
                    gesture.press_origin,
                    gesture.threshold_crossed,
                )
            })
    }

    pub(crate) fn pointer_gesture_click_count(&self, owner: WidgetId) -> Option<u8> {
        self.pointer_gesture
            .filter(|gesture| gesture.owner == owner)
            .map(|gesture| gesture.click_count)
    }

    pub(crate) fn mark_pointer_threshold_crossed(&mut self, owner: WidgetId) {
        if let Some(gesture) = &mut self.pointer_gesture
            && gesture.owner == owner
        {
            gesture.threshold_crossed = true;
        }
    }

    pub(crate) fn take_cancelled_pointer_gesture(
        &mut self,
        owner: WidgetId,
        kind: PointerGestureKind,
        allow_kind_mismatch: bool,
    ) -> Option<u8> {
        self.cancelled_pointer_gesture.filter(|gesture| {
            gesture.owner == owner
                && (gesture.kind == Some(kind)
                    || (gesture.kind.is_none() && kind == PointerGestureKind::Selection)
                    || allow_kind_mismatch)
        })?;
        self.cancelled_pointer_gesture
            .take()
            .map(|gesture| gesture.click_count)
    }

    pub(crate) const fn pointer_release_cleanup_required(&self) -> bool {
        self.pointer_capture.is_some() || self.cancelled_pointer_gesture.is_some()
    }

    pub(crate) fn claim_selection_gesture(&mut self, owner: WidgetId) -> bool {
        self.selection_gesture_claims.insert(owner)
    }

    pub(crate) fn cached_domain_drag_response(&self, owner: WidgetId) -> Option<Response> {
        self.domain_drag_frame_open
            .then(|| self.domain_drag_responses.get(&owner).copied())
            .flatten()
    }

    pub(crate) fn cache_domain_drag_response(&mut self, owner: WidgetId, response: Response) {
        if self.domain_drag_frame_open {
            self.domain_drag_responses.entry(owner).or_insert(response);
        }
    }

    pub(crate) fn install_scoped_pointer_events(
        &mut self,
        event_ordinals: impl IntoIterator<Item = usize>,
        event_indices: impl IntoIterator<Item = usize>,
        primary_transaction_open: impl IntoIterator<Item = bool>,
    ) {
        self.scoped_pointer_event_ordinals.clear();
        self.scoped_pointer_event_ordinals.extend(event_ordinals);
        self.scoped_pointer_cleanup_events.clear();
        self.scoped_pointer_cleanup_events.extend(event_indices);
        self.scoped_primary_transaction_open.clear();
        self.scoped_primary_transaction_open
            .extend(primary_transaction_open);
        debug_assert_eq!(
            self.scoped_pointer_event_ordinals.len(),
            self.scoped_primary_transaction_open.len(),
            "scoped pointer ordinals and transaction provenance must stay aligned"
        );
    }

    pub(crate) fn scoped_pointer_event_is_cleanup(&self, event_index: usize) -> bool {
        self.scoped_pointer_cleanup_events.contains(&event_index)
    }

    pub(crate) fn scoped_pointer_event_ordinal(&self, event_index: usize) -> usize {
        self.scoped_pointer_event_ordinals
            .get(event_index)
            .copied()
            .unwrap_or(event_index)
    }

    pub(crate) fn scoped_primary_transaction_was_open(&self, event_index: usize) -> Option<bool> {
        self.scoped_primary_transaction_open
            .get(event_index)
            .copied()
    }

    /// Marks a widget as the active drag source.
    pub fn start_drag(&mut self, id: WidgetId) {
        self.drag_source = Some(id);
    }

    /// Finishes an active drag and keeps the released source visible this frame.
    pub fn finish_drag(&mut self, id: WidgetId) {
        self.finish_drag_at(id, None);
    }

    pub(crate) fn finish_drag_at(&mut self, id: WidgetId, ordinal: Option<usize>) {
        if self.drag_source == Some(id) {
            self.drag_source = None;
            if self.released_drag_source.is_none() {
                self.released_drag_source = Some(id);
                self.released_drag_ordinal = ordinal;
            }
        }
    }

    pub(crate) fn clear_active_drag(&mut self) {
        self.drag_source = None;
    }

    /// Clears active and released drag source state.
    pub fn clear_drag(&mut self) {
        self.drag_source = None;
        self.released_drag_source = None;
        self.released_drag_ordinal = None;
    }

    /// Cancels active pointer interaction while preserving unrelated retained state.
    pub fn cancel_pointer_interaction(&mut self) -> bool {
        let cancelled_primary = self.cancel_primary_pointer_interaction();
        let cancelled_secondary = self.secondary_pressed.take().is_some();
        let cancelled = cancelled_primary || cancelled_secondary;
        if cancelled {
            self.pointer_interaction_cancelled = true;
        }
        cancelled
    }

    pub(crate) fn cancel_primary_pointer_interaction(&mut self) -> bool {
        let cancelled_owner = self
            .pointer_gesture_owner()
            .or(self.pointer_capture)
            .or(self.active);
        let cancelled_click_count = cancelled_owner
            .and_then(|owner| self.pointer_gesture_click_count(owner))
            .unwrap_or(0);
        let cancelled_kind = cancelled_owner
            .and_then(|owner| self.pointer_gesture_kind(owner))
            .or_else(|| {
                cancelled_owner
                    .filter(|owner| self.drag_source == Some(*owner))
                    .map(|_| PointerGestureKind::DomainDrag)
            });
        let cancelled = self.active.is_some()
            || self.pressed.is_some()
            || self.pointer_capture.is_some()
            || self.drag_source.is_some()
            || self.pointer_gesture.is_some();
        if cancelled {
            self.cancelled_pointer_gesture = cancelled_owner.map(|owner| CancelledPointerGesture {
                owner,
                kind: cancelled_kind,
                click_count: cancelled_click_count,
            });
            self.clear_active_drag();
            self.clear_primary_interaction();
            self.pointer_capture_released = None;
            self.pointer_capture_released_ordinal = None;
        }
        cancelled
    }

    pub(crate) fn cancel_secondary_pointer_interaction(&mut self, id: WidgetId) -> bool {
        if self.secondary_pressed == Some(id) {
            self.secondary_pressed = None;
            true
        } else {
            false
        }
    }

    pub(crate) const fn has_pointer_transaction(&self) -> bool {
        self.has_primary_pointer_transaction() || self.secondary_pressed.is_some()
    }

    pub(crate) const fn has_primary_pointer_transaction(&self) -> bool {
        self.active.is_some()
            || self.pressed.is_some()
            || self.pointer_capture.is_some()
            || self.drag_source.is_some()
            || self.pointer_gesture.is_some()
    }

    pub(crate) fn fence_pointer_stream(&mut self) {
        self.pointer_interaction_cancelled = true;
    }

    /// Reconciles persistent interaction ownership against widgets present this frame.
    pub(crate) fn reconcile_widget_owners(
        &mut self,
        is_present: impl Fn(WidgetId) -> bool,
    ) -> bool {
        let pointer_owner_missing = [
            self.pointer_capture,
            self.active,
            self.pressed,
            self.secondary_pressed,
            self.drag_source,
            self.pointer_gesture_owner(),
        ]
        .into_iter()
        .flatten()
        .any(|owner| !is_present(owner));
        let focused_owner_missing = self.focused.is_some_and(|owner| !is_present(owner));
        let text_owner_invalid = self
            .text_input_owner
            .is_some_and(|owner| !is_present(owner) || self.focused != Some(owner));

        let mut changed = false;
        if pointer_owner_missing {
            changed |= self.cancel_pointer_interaction();
        }
        if focused_owner_missing {
            self.clear_focus();
            changed = true;
        }
        if text_owner_invalid && self.text_input_owner.is_some() {
            self.clear_text_input_owner();
            changed = true;
        }
        changed
    }

    /// Releases pointer capture when it is held by the provided widget.
    pub fn release_pointer_capture(&mut self, id: WidgetId) {
        if self.pointer_capture == Some(id) {
            self.pointer_capture = None;
            if self.pointer_gesture_owner() == Some(id) {
                self.pointer_gesture = None;
            }
        }
    }

    /// Clears interaction capture state at the end of an interaction.
    pub fn clear_interaction(&mut self) {
        self.clear_primary_interaction();
        self.secondary_pressed = None;
    }

    pub(crate) fn clear_primary_interaction(&mut self) {
        self.record_released_pointer_capture(None);
        self.clear_primary_interaction_state();
    }

    pub(crate) fn clear_primary_interaction_at(&mut self, release_ordinal: usize) {
        self.record_released_pointer_capture(Some(release_ordinal));
        self.clear_primary_interaction_state();
    }

    pub(crate) fn discard_primary_interaction(&mut self) {
        self.pointer_capture_released = None;
        self.pointer_capture_released_ordinal = None;
        self.clear_primary_interaction_state();
    }

    fn record_released_pointer_capture(&mut self, release_ordinal: Option<usize>) {
        let Some(owner) = self.pointer_capture else {
            return;
        };
        if let (Some(current), Some(previous)) =
            (release_ordinal, self.pointer_capture_released_ordinal)
            && current < previous
        {
            return;
        }
        if release_ordinal.is_none() && self.pointer_capture_released_ordinal.is_some() {
            return;
        }
        self.pointer_capture_released = Some(owner);
        self.pointer_capture_released_ordinal = release_ordinal;
    }

    fn clear_primary_interaction_state(&mut self) {
        self.active = None;
        self.pressed = None;
        self.pointer_capture = None;
        self.pointer_gesture = None;
    }

    /// Records an already platform-active Editable text-input owner.
    ///
    /// This compatibility setter is primarily useful for retained setup and
    /// tests. Runtime code should prefer mode-aware preparation followed by an
    /// accepted caret rectangle.
    pub fn set_text_input_owner(&mut self, id: WidgetId) {
        if self.text_input_owner == Some(id)
            && self.text_input_owner_mode == Some(TextInputOwnerMode::Editable)
            && self.platform_text_input_state == PlatformTextInputState::Active
        {
            return;
        }

        if self.platform_text_input_state == PlatformTextInputState::Active {
            self.retire_active_platform_text_input();
        }

        if self.pending_text_input_stop == Some(id) {
            self.pending_text_input_stop = None;
        }
        self.text_input_owner = Some(id);
        self.text_input_owner_mode = Some(TextInputOwnerMode::Editable);
        self.platform_text_input_state = if self.pending_text_input_stop.is_none() {
            PlatformTextInputState::Active
        } else {
            PlatformTextInputState::Inactive
        };
    }

    /// Records a logical text-input owner without activating platform IME.
    #[doc(hidden)]
    pub fn set_text_input_owner_mode(&mut self, id: WidgetId, mode: TextInputOwnerMode) {
        if self.text_input_owner == Some(id) && self.text_input_owner_mode == Some(mode) {
            return;
        }

        if self.platform_text_input_state == PlatformTextInputState::Active {
            self.retire_active_platform_text_input();
        }
        self.text_input_owner = Some(id);
        self.text_input_owner_mode = Some(mode);
        self.platform_text_input_state = PlatformTextInputState::Inactive;
    }

    /// Clears the logical text-input owner and retires platform IME when active.
    pub fn clear_text_input_owner(&mut self) {
        if self.text_input_owner.is_none() {
            return;
        }
        self.retire_active_platform_text_input();
        self.text_input_owner = None;
        self.text_input_owner_mode = None;
    }

    /// Takes the text input owner waiting for a platform stop request.
    #[doc(hidden)]
    pub fn take_pending_text_input_stop(&mut self) -> Option<WidgetId> {
        self.pending_text_input_stop.take()
    }

    pub(crate) fn platform_text_input_is_active_for(&self, id: WidgetId) -> bool {
        self.platform_text_input_state == PlatformTextInputState::Active
            && self.text_input_owner == Some(id)
            && self.text_input_owner_mode == Some(TextInputOwnerMode::Editable)
    }

    pub(crate) fn activate_platform_text_input(&mut self, id: WidgetId) {
        debug_assert_eq!(self.text_input_owner, Some(id));
        debug_assert_eq!(
            self.text_input_owner_mode,
            Some(TextInputOwnerMode::Editable)
        );
        debug_assert!(self.pending_text_input_stop.is_none());
        self.platform_text_input_state = PlatformTextInputState::Active;
    }

    pub(crate) fn acknowledge_platform_text_input_stop(&mut self) {
        self.pending_text_input_stop = None;
        self.platform_text_input_state = PlatformTextInputState::Inactive;
    }

    fn retire_active_platform_text_input(&mut self) {
        if self.platform_text_input_state == PlatformTextInputState::Inactive {
            return;
        }
        let owner = self
            .text_input_owner
            .expect("active platform text input has a logical owner");
        if self.pending_text_input_stop.is_none() {
            self.pending_text_input_stop = Some(owner);
        }
        self.platform_text_input_state = PlatformTextInputState::Inactive;
    }

    fn clear_stale_text_input_owner(&mut self) {
        let Some(owner) = self.text_input_owner else {
            return;
        };
        if Some(owner) != self.focused {
            self.clear_text_input_owner();
        }
    }

    /// Returns the scroll offset for a widget.
    #[must_use]
    pub fn scroll_offset(&self, id: WidgetId) -> Vec2 {
        self.scroll_offsets.get(&id).copied().unwrap_or(Vec2::ZERO)
    }

    /// Sets the scroll offset for a widget.
    pub fn set_scroll_offset(&mut self, id: WidgetId, offset: Vec2) {
        self.pending_scroll_offsets.remove(&id);
        self.scroll_offsets.insert(id, offset);
    }

    #[doc(hidden)]
    pub fn stage_scroll_offset(&mut self, id: WidgetId, offset: Vec2) {
        self.pending_scroll_offsets.insert(id, offset);
    }

    /// Returns true when a popover is open for a widget.
    #[must_use]
    pub fn is_popover_open(&self, id: WidgetId) -> bool {
        self.open_popovers.contains(&id)
    }

    /// Opens a popover for a widget.
    pub fn open_popover(&mut self, id: WidgetId) {
        self.open_popovers.insert(id);
    }

    /// Closes a popover for a widget.
    pub fn close_popover(&mut self, id: WidgetId) {
        self.open_popovers.remove(&id);
    }

    /// Returns the retained liveness registry.
    #[must_use]
    pub const fn liveness(&self) -> &LivenessRegistry {
        &self.liveness
    }

    /// Returns mutable access to the retained liveness registry.
    pub fn liveness_mut(&mut self) -> &mut LivenessRegistry {
        &mut self.liveness
    }

    /// Marks an async owner present and returns its stable incarnation token.
    pub fn mark_present_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.liveness.mark_present(target)
    }

    /// Marks an async owner present using the previous live terminology.
    #[deprecated(note = "use mark_present_target")]
    pub fn mark_live_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.mark_present_target(target)
    }

    /// Starts a replacement incarnation and marks it present.
    pub fn restart_liveness_target(
        &mut self,
        target: impl Into<LivenessTargetId>,
    ) -> LivenessToken {
        self.liveness.restart(target)
    }

    /// Cancels the exact active token incarnation.
    pub fn cancel_liveness_token(&mut self, token: LivenessToken) -> LivenessUpdateStatus {
        self.liveness.cancel(token)
    }

    /// Removes a liveness target from the retained registry.
    pub fn remove_live_target(
        &mut self,
        target: impl Into<LivenessTargetId>,
    ) -> LivenessRemovalStatus {
        self.liveness.remove(target)
    }

    /// Runs `update` only when the token matches a currently live target.
    pub fn apply_liveness_update(
        &self,
        token: LivenessToken,
        update: impl FnOnce(),
    ) -> LivenessUpdateStatus {
        self.liveness.apply_update(token, update)
    }

    /// Returns the retained observer registry.
    #[must_use]
    pub const fn observers(&self) -> &ObserverRegistry {
        &self.observers
    }

    /// Returns mutable access to the retained observer registry.
    pub fn observers_mut(&mut self) -> &mut ObserverRegistry {
        &mut self.observers
    }

    /// Retains an observer subscription tied to the provided liveness token.
    pub fn subscribe_observer(&mut self, token: LivenessToken) -> ObserverSubscriptionHandle {
        self.observers.subscribe(token)
    }

    /// Explicitly unsubscribes a retained observer subscription.
    pub fn unsubscribe_observer(&mut self, id: ObserverSubscriptionId) -> bool {
        self.observers.unsubscribe(id)
    }

    /// Publishes an observer notification into the retained queue.
    pub fn publish_observer(
        &mut self,
        subscription_id: ObserverSubscriptionId,
        notification_id: ObserverNotificationId,
    ) -> ObserverPublishStatus {
        self.observers.publish(subscription_id, notification_id)
    }

    /// Enqueues an observer notification into the retained queue.
    pub fn enqueue_observer(
        &mut self,
        notification: ObserverNotification,
    ) -> ObserverPublishStatus {
        self.observers.enqueue(notification)
    }

    /// Drains observer notifications queued at the start of this pass.
    pub fn drain_observers(
        &mut self,
        deliver: impl FnMut(&mut ObserverRegistry, ObserverDelivery),
    ) -> ObserverDrain {
        let Self {
            liveness,
            observers,
            ..
        } = self;
        observers.drain(liveness, deliver)
    }
}

#[cfg(test)]
mod tests {
    use super::{PointerGestureKind, UiMemory};
    use crate::interaction::captured_selection_gesture_with_ordinals;
    use crate::{
        MouseButton, Point, PointerRoute, PointerRoutes, Rect, UiInput, UiInputEvent, Vec2,
        WidgetId,
    };

    fn selection_gesture_memory(owner: WidgetId) -> UiMemory {
        let mut memory = UiMemory::new();
        memory.activate(owner);
        memory.press(owner);
        memory.capture_pointer(owner);
        memory.begin_pointer_gesture(
            owner,
            Point::new(4.0, 4.0),
            PointerGestureKind::Selection,
            1,
        );
        memory
    }

    fn resolve_selection_event(memory: &mut UiMemory, owner: WidgetId, event: UiInputEvent) {
        let mut input = UiInput::default();
        input.push_event(event);
        let _ = captured_selection_gesture_with_ordinals(
            owner,
            Rect::new(0.0, 0.0, 20.0, 20.0),
            &input,
            &[0],
            memory,
            false,
        );
    }

    fn install_ordinary_route(memory: &mut UiMemory, ordinary: PointerRoute) {
        memory.install_pointer_routes(
            PointerRoutes {
                ordinary,
                drop: PointerRoute::Unplanned,
                wheel: PointerRoute::Unplanned,
            },
            [],
            None,
            None,
        );
    }

    #[test]
    fn canonical_primary_route_uses_release_ordinal_then_base_route() {
        let released = WidgetId::from_key("released");
        let next = WidgetId::from_key("next");
        let mut memory = UiMemory::new();
        memory.capture_pointer(released);
        memory.clear_primary_interaction_at(7);

        install_ordinary_route(&mut memory, PointerRoute::Target(next));
        assert!(memory.canonical_primary_route_allows(released, 7));
        assert!(!memory.canonical_primary_route_allows(next, 7));
        assert!(!memory.canonical_primary_route_allows(released, 8));
        assert!(memory.canonical_primary_route_allows(next, 8));

        install_ordinary_route(&mut memory, PointerRoute::Target(released));
        assert!(memory.canonical_primary_route_allows(released, 8));
        assert!(!memory.canonical_primary_route_allows(next, 8));

        install_ordinary_route(&mut memory, PointerRoute::Blocked);
        assert!(!memory.canonical_primary_route_allows(released, 8));
        assert!(!memory.canonical_primary_route_allows(next, 8));

        install_ordinary_route(&mut memory, PointerRoute::Target(next));
        memory.capture_pointer(released);
        assert!(memory.canonical_primary_route_allows(released, 8));
        assert!(!memory.canonical_primary_route_allows(next, 8));
        assert!(memory.cancel_pointer_interaction());
        assert!(!memory.canonical_primary_route_allows(released, 8));
        assert!(!memory.canonical_primary_route_allows(next, 8));
    }

    #[test]
    fn selection_gesture_anchor_is_owner_checked_opaque_and_retained_across_frames() {
        let owner = WidgetId::from_key("owner");
        let other = WidgetId::from_key("other");
        let mut memory = selection_gesture_memory(owner);

        assert_eq!(memory.selection_gesture_anchor(owner), None);
        assert!(!memory.set_selection_gesture_anchor(other, 7));
        assert_eq!(memory.selection_gesture_anchor(other), None);
        assert!(memory.set_selection_gesture_anchor(owner, usize::MAX));
        assert_eq!(memory.selection_gesture_anchor(owner), Some(usize::MAX));

        memory.focus(owner);
        memory.clear_focus();
        assert_eq!(memory.selection_gesture_anchor(owner), Some(usize::MAX));
        memory.begin_frame();
        assert_eq!(memory.selection_gesture_anchor(owner), Some(usize::MAX));
    }

    #[test]
    fn selection_gesture_anchor_rejects_missing_press_and_domain_drag_gestures() {
        let owner = WidgetId::from_key("owner");
        let mut memory = UiMemory::new();
        assert!(!memory.set_selection_gesture_anchor(owner, 1));
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        for kind in [PointerGestureKind::Press, PointerGestureKind::DomainDrag] {
            memory.begin_pointer_gesture(owner, Point::ZERO, kind, 1);
            assert!(!memory.set_selection_gesture_anchor(owner, 2));
            assert_eq!(memory.selection_gesture_anchor(owner), None);
        }
    }

    #[test]
    fn direct_selection_gesture_cleanup_paths_clear_anchor_without_inheritance() {
        let owner = WidgetId::from_key("owner");

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 1));
        memory.release_pointer_capture(owner);
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 2));
        memory.clear_primary_interaction();
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 3));
        memory.clear_primary_interaction_at(9);
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 4));
        memory.discard_primary_interaction();
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 5));
        assert!(memory.cancel_pointer_interaction());
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 6));
        assert!(memory.reconcile_widget_owners(|id| id != owner));
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        memory.begin_pointer_gesture(
            owner,
            Point::new(8.0, 8.0),
            PointerGestureKind::Selection,
            1,
        );
        assert_eq!(memory.selection_gesture_anchor(owner), None);
    }

    #[test]
    fn canonical_release_and_cancellation_fences_clear_selection_gesture_anchor() {
        let owner = WidgetId::from_key("owner");

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 1));
        resolve_selection_event(
            &mut memory,
            owner,
            UiInputEvent::PointerButton {
                button: MouseButton::Primary,
                down: false,
                click_count: 1,
                position: Some(Point::new(4.0, 4.0)),
            },
        );
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 2));
        resolve_selection_event(
            &mut memory,
            owner,
            UiInputEvent::PointerReleaseAll {
                position: Some(Point::new(4.0, 4.0)),
            },
        );
        assert_eq!(memory.selection_gesture_anchor(owner), None);

        let mut memory = selection_gesture_memory(owner);
        assert!(memory.set_selection_gesture_anchor(owner, 3));
        resolve_selection_event(&mut memory, owner, UiInputEvent::WindowFocusChanged(false));
        assert_eq!(memory.selection_gesture_anchor(owner), None);
    }

    #[test]
    fn canonical_release_provenance_keeps_greatest_ordinal_and_resets() {
        let earlier_owner = WidgetId::from_key("earlier-owner");
        let later_owner = WidgetId::from_key("later-owner");
        let base_owner = WidgetId::from_key("base-owner");
        let mut memory = UiMemory::new();
        install_ordinary_route(&mut memory, PointerRoute::Target(base_owner));

        memory.capture_pointer(later_owner);
        memory.clear_primary_interaction_at(9);
        memory.capture_pointer(earlier_owner);
        memory.clear_primary_interaction_at(7);
        memory.capture_pointer(earlier_owner);
        memory.clear_primary_interaction();
        assert!(memory.canonical_primary_route_allows(later_owner, 9));
        assert!(!memory.canonical_primary_route_allows(earlier_owner, 9));
        assert!(memory.canonical_primary_route_allows(base_owner, 10));

        memory.begin_frame();
        assert!(memory.canonical_primary_route_allows(earlier_owner, 0));
        assert!(memory.canonical_primary_route_allows(later_owner, 0));

        memory.capture_pointer(earlier_owner);
        memory.clear_primary_interaction();
        install_ordinary_route(&mut memory, PointerRoute::Target(base_owner));
        assert!(memory.canonical_primary_route_allows(earlier_owner, usize::MAX));
        assert!(!memory.canonical_primary_route_allows(base_owner, usize::MAX));
    }

    #[test]
    fn starts_empty() {
        let memory = UiMemory::new();

        assert_eq!(memory.hovered(), None);
        assert_eq!(memory.focused(), None);
        assert_eq!(memory.active(), None);
        assert_eq!(memory.pressed(), None);
        assert_eq!(memory.secondary_pressed(), None);
        assert_eq!(memory.pointer_capture(), None);
        assert!(!memory.pointer_interaction_cancelled());
        assert_eq!(memory.drag_source(), None);
        assert_eq!(memory.released_drag_source(), None);
        assert_eq!(memory.text_input_owner(), None);
        assert!(memory.liveness().is_empty());
        assert!(memory.observers().is_empty());
    }

    #[test]
    fn begin_frame_clears_hover_but_preserves_focus() {
        let id = WidgetId::from_key("field");
        let mut memory = UiMemory::new();
        memory.set_hovered(id);
        memory.focus(id);

        memory.begin_frame();

        assert_eq!(memory.hovered(), None);
        assert_eq!(memory.focused(), Some(id));
    }

    #[test]
    fn clear_focus_preserves_pointer_capture_and_drag_state() {
        let focused = WidgetId::from_key("focused");
        let pointer_owner = WidgetId::from_key("pointer-owner");
        let drag_owner = WidgetId::from_key("drag-owner");
        let mut memory = UiMemory::new();
        memory.focus(focused);
        memory.capture_pointer(pointer_owner);
        memory.start_drag(drag_owner);

        memory.clear_focus();

        assert_eq!(memory.focused(), None);
        assert_eq!(memory.pointer_capture(), Some(pointer_owner));
        assert_eq!(memory.drag_source(), Some(drag_owner));
        assert_eq!(memory.released_drag_source(), None);
    }

    #[test]
    fn clears_interaction_state() {
        let id = WidgetId::from_key("button");
        let mut memory = UiMemory::new();
        memory.activate(id);
        memory.press(id);
        memory.press_secondary(id);
        memory.capture_pointer(id);
        memory.start_drag(id);

        memory.clear_interaction();

        assert_eq!(memory.active(), None);
        assert_eq!(memory.pressed(), None);
        assert_eq!(memory.secondary_pressed(), None);
        assert_eq!(memory.pointer_capture(), None);
        assert!(!memory.pointer_interaction_cancelled());
        assert_eq!(memory.drag_source(), Some(id));
    }

    #[test]
    fn cancels_pointer_interaction_without_clearing_unrelated_state() {
        let focused = WidgetId::from_key("focused");
        let pointer_owner = WidgetId::from_key("pointer-owner");
        let drag_owner = WidgetId::from_key("drag-owner");
        let mut memory = UiMemory::new();
        memory.focus(focused);
        memory.set_text_input_owner(focused);
        memory.activate(pointer_owner);
        memory.press(pointer_owner);
        memory.press_secondary(pointer_owner);
        memory.capture_pointer(pointer_owner);
        memory.start_drag(drag_owner);

        assert!(memory.cancel_pointer_interaction());

        assert_eq!(memory.active(), None);
        assert_eq!(memory.pressed(), None);
        assert_eq!(memory.secondary_pressed(), None);
        assert_eq!(memory.pointer_capture(), None);
        assert_eq!(memory.drag_source(), None);
        assert_eq!(memory.released_drag_source(), None);
        assert!(memory.pointer_interaction_cancelled());
        assert_eq!(memory.focused(), Some(focused));
        assert_eq!(memory.text_input_owner(), Some(focused));

        memory.begin_frame();
        assert!(!memory.pointer_interaction_cancelled());
    }

    #[test]
    fn exposes_predicates_for_interaction_owners() {
        let hovered = WidgetId::from_key("hovered");
        let focused = WidgetId::from_key("focused");
        let active = WidgetId::from_key("active");
        let pressed = WidgetId::from_key("pressed");
        let other = WidgetId::from_key("other");
        let mut memory = UiMemory::new();

        memory.set_hovered(hovered);
        memory.focus(focused);
        memory.activate(active);
        memory.press(pressed);
        memory.press_secondary(pressed);
        memory.capture_pointer(active);
        memory.start_drag(active);

        assert!(memory.is_hovered(hovered));
        assert!(memory.is_focused(focused));
        assert!(memory.is_active(active));
        assert!(memory.is_pressed(pressed));
        assert!(memory.is_secondary_pressed(pressed));
        assert!(memory.has_pointer_capture(active));
        assert!(memory.is_drag_source(active));
        assert!(!memory.is_hovered(other));
        assert!(!memory.is_focused(other));
        assert!(!memory.is_active(other));
        assert!(!memory.is_pressed(other));
        assert!(!memory.is_secondary_pressed(other));
        assert!(!memory.has_pointer_capture(other));
        assert!(!memory.is_drag_source(other));
    }

    #[test]
    fn tracks_text_input_owner() {
        let field = WidgetId::from_key("field");
        let other = WidgetId::from_key("other");
        let mut memory = UiMemory::new();

        memory.set_text_input_owner(field);
        assert_eq!(memory.text_input_owner(), Some(field));
        assert!(memory.owns_text_input(field));
        assert!(!memory.owns_text_input(other));

        memory.clear_text_input_owner();
        assert_eq!(memory.text_input_owner(), None);
    }

    #[test]
    fn text_input_owner_handoff_preserves_pending_stop_for_old_owner() {
        let old_field = WidgetId::from_key("old-field");
        let new_field = WidgetId::from_key("new-field");
        let mut memory = UiMemory::new();
        memory.focus(old_field);
        memory.set_text_input_owner(old_field);

        memory.focus(new_field);
        assert_eq!(memory.text_input_owner(), None);

        memory.set_text_input_owner(new_field);

        assert_eq!(memory.text_input_owner(), Some(new_field));
        assert_eq!(memory.take_pending_text_input_stop(), Some(old_field));
    }

    #[test]
    fn releases_pointer_capture_for_owner_only() {
        let owner = WidgetId::from_key("owner");
        let other = WidgetId::from_key("other");
        let mut memory = UiMemory::new();

        memory.capture_pointer(owner);
        memory.release_pointer_capture(other);
        assert_eq!(memory.pointer_capture(), Some(owner));

        memory.release_pointer_capture(owner);
        assert_eq!(memory.pointer_capture(), None);
    }

    #[test]
    fn tracks_secondary_press_owner() {
        let owner = WidgetId::from_key("owner");
        let other = WidgetId::from_key("other");
        let mut memory = UiMemory::new();

        memory.press_secondary(owner);
        assert_eq!(memory.secondary_pressed(), Some(owner));
        memory.release_secondary(other);
        assert_eq!(memory.secondary_pressed(), Some(owner));
        memory.release_secondary(owner);
        assert_eq!(memory.secondary_pressed(), None);
    }

    #[test]
    fn tracks_drag_source_and_released_source_for_one_frame() {
        let source = WidgetId::from_key("source");
        let mut memory = UiMemory::new();

        memory.start_drag(source);
        assert_eq!(memory.drag_source(), Some(source));
        assert_eq!(memory.released_drag_source(), None);

        memory.finish_drag(source);
        assert_eq!(memory.drag_source(), None);
        assert_eq!(memory.released_drag_source(), Some(source));

        memory.begin_frame();
        assert_eq!(memory.released_drag_source(), None);
    }

    #[test]
    fn stores_scroll_offsets_by_widget_id() {
        let id = WidgetId::from_key("scroll");
        let mut memory = UiMemory::new();

        assert_eq!(memory.scroll_offset(id), Vec2::ZERO);
        memory.set_scroll_offset(id, Vec2::new(10.0, 20.0));
        assert_eq!(memory.scroll_offset(id), Vec2::new(10.0, 20.0));
    }

    #[test]
    fn tracks_open_popovers() {
        let id = WidgetId::from_key("menu");
        let mut memory = UiMemory::new();

        assert!(!memory.is_popover_open(id));
        memory.open_popover(id);
        assert!(memory.is_popover_open(id));
        memory.close_popover(id);
        assert!(!memory.is_popover_open(id));
    }
}
