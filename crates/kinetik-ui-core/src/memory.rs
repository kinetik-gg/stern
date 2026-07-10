//! Retained UI memory.

use std::collections::{HashMap, HashSet};

use crate::{
    LivenessRegistry, LivenessTargetId, LivenessToken, LivenessUpdateStatus, ObserverDelivery,
    ObserverDrain, ObserverNotification, ObserverNotificationId, ObserverPublishStatus,
    ObserverRegistry, ObserverSubscriptionHandle, ObserverSubscriptionId, Vec2, WidgetId,
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

impl PointerRoutes {
    pub(crate) const BLOCKED: Self = Self {
        ordinary: PointerRoute::Blocked,
        drop: PointerRoute::Blocked,
        wheel: PointerRoute::Blocked,
    };
}

/// Retained interaction and widget state owned by the UI runtime.
#[derive(Debug, Clone, Default, PartialEq)]
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
    /// Pointer interaction was cancelled at this frame's runtime boundary.
    pointer_interaction_cancelled: bool,
    pointer_routes: PointerRoutes,
    pointer_cursor_equivalents: HashSet<WidgetId>,
    /// Widget currently acting as an active drag source.
    drag_source: Option<WidgetId>,
    /// Drag source released during this frame.
    released_drag_source: Option<WidgetId>,
    /// Text-editing widget currently owning platform text input or IME.
    text_input_owner: Option<WidgetId>,
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
        self.released_drag_source = None;
        self.pointer_interaction_cancelled = false;
        self.pointer_routes = PointerRoutes::default();
        self.pointer_cursor_equivalents.clear();
        self.liveness.begin_frame();
        self.observers.prune_inactive_subscriptions();
    }

    /// Removes liveness targets not seen during the current frame.
    pub(crate) fn end_frame(&mut self) {
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
    ) {
        self.pointer_routes = routes;
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

    pub(crate) fn pointer_drop_route_allows(&self, id: WidgetId) -> bool {
        !self.pointer_interaction_cancelled && self.pointer_routes.drop.allows(id)
    }

    pub(crate) fn pointer_wheel_route_allows(&self, id: WidgetId) -> bool {
        !self.pointer_interaction_cancelled && self.pointer_routes.wheel.allows(id)
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

    /// Returns the widget currently owning platform text input or IME.
    #[must_use]
    pub const fn text_input_owner(&self) -> Option<WidgetId> {
        self.text_input_owner
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

    /// Returns true when the widget owns platform text input or IME.
    #[must_use]
    pub fn owns_text_input(&self, id: WidgetId) -> bool {
        self.text_input_owner == Some(id)
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

    /// Marks a widget as the active drag source.
    pub fn start_drag(&mut self, id: WidgetId) {
        self.drag_source = Some(id);
    }

    /// Finishes an active drag and keeps the released source visible this frame.
    pub fn finish_drag(&mut self, id: WidgetId) {
        if self.drag_source == Some(id) {
            self.drag_source = None;
            self.released_drag_source = Some(id);
        }
    }

    /// Clears active and released drag source state.
    pub fn clear_drag(&mut self) {
        self.drag_source = None;
        self.released_drag_source = None;
    }

    /// Cancels active pointer interaction while preserving unrelated retained state.
    pub fn cancel_pointer_interaction(&mut self) -> bool {
        let cancelled = self.active.is_some()
            || self.pressed.is_some()
            || self.secondary_pressed.is_some()
            || self.pointer_capture.is_some()
            || self.drag_source.is_some();
        if cancelled {
            self.clear_drag();
            self.clear_interaction();
            self.pointer_capture_released = None;
            self.pointer_interaction_cancelled = true;
        }
        cancelled
    }

    /// Releases pointer capture when it is held by the provided widget.
    pub fn release_pointer_capture(&mut self, id: WidgetId) {
        if self.pointer_capture == Some(id) {
            self.pointer_capture = None;
        }
    }

    /// Clears interaction capture state at the end of an interaction.
    pub fn clear_interaction(&mut self) {
        self.pointer_capture_released = self.pointer_capture;
        self.active = None;
        self.pressed = None;
        self.secondary_pressed = None;
        self.pointer_capture = None;
    }

    /// Records the widget that should receive platform text input or IME events.
    pub fn set_text_input_owner(&mut self, id: WidgetId) {
        if self.text_input_owner == Some(id) {
            return;
        }
        if self.pending_text_input_stop == Some(id) {
            self.pending_text_input_stop = None;
        }
        self.text_input_owner = Some(id);
    }

    /// Clears the active platform text input owner.
    pub fn clear_text_input_owner(&mut self) {
        if let Some(owner) = self.text_input_owner.take() {
            self.pending_text_input_stop = Some(owner);
        }
    }

    /// Takes the text input owner waiting for a platform stop request.
    #[doc(hidden)]
    pub fn take_pending_text_input_stop(&mut self) -> Option<WidgetId> {
        self.pending_text_input_stop.take()
    }

    fn clear_stale_text_input_owner(&mut self) {
        let Some(owner) = self.text_input_owner else {
            return;
        };
        if Some(owner) != self.focused {
            self.text_input_owner = None;
            self.pending_text_input_stop = Some(owner);
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

    pub(crate) fn stage_scroll_offset(&mut self, id: WidgetId, offset: Vec2) {
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

    /// Marks a liveness target live and returns a renewed token.
    pub fn mark_live_target(&mut self, target: impl Into<LivenessTargetId>) -> LivenessToken {
        self.liveness.mark_live(target)
    }

    /// Removes a liveness target from the retained registry.
    pub fn remove_live_target(&mut self, target: impl Into<LivenessTargetId>) -> bool {
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

    /// Updates an active observer subscription to a renewed liveness token.
    pub fn update_observer_subscription_token(
        &mut self,
        id: ObserverSubscriptionId,
        token: LivenessToken,
    ) -> bool {
        self.observers.update_subscription_token(id, token)
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
    use super::UiMemory;
    use crate::{Vec2, WidgetId};

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
