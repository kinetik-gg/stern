use kinetik_ui_core::Point;

use super::{OverlayEntry, OverlayId};

/// Retained overlay stack.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayStack {
    entries: Vec<OverlayEntry>,
}

impl OverlayStack {
    /// Creates an empty overlay stack.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds or replaces an overlay at the top of the stack.
    pub fn open(&mut self, entry: OverlayEntry) {
        self.close(entry.id);
        self.entries.push(entry);
    }

    /// Opens a child overlay when its parent is still present.
    ///
    /// Returns `false` when the parent is missing and leaves the stack unchanged.
    pub fn open_child(&mut self, parent: OverlayId, entry: OverlayEntry) -> bool {
        if !self.entries.iter().any(|candidate| candidate.id == parent) {
            return false;
        }
        if self.would_parent_cycle(parent, entry.id) {
            return false;
        }
        self.open(entry.with_parent(parent));
        true
    }

    /// Closes an overlay and any nested descendants by ID.
    pub fn close(&mut self, id: OverlayId) -> Option<OverlayEntry> {
        let closed = self.entries.iter().find(|entry| entry.id == id).cloned()?;
        let closing = self.descendant_ids(id);
        self.entries.retain(|entry| !closing.contains(&entry.id));
        Some(closed)
    }

    /// Closes and returns the top overlay.
    pub fn close_top(&mut self) -> Option<OverlayEntry> {
        let id = self.top()?.id;
        self.close(id)
    }

    /// Returns the top overlay.
    #[must_use]
    pub fn top(&self) -> Option<&OverlayEntry> {
        self.entries.last()
    }

    /// Returns overlays in bottom-to-top order.
    #[must_use]
    pub fn entries(&self) -> &[OverlayEntry] {
        &self.entries
    }

    /// Returns true when any modal overlay is open.
    #[must_use]
    pub fn has_modal(&self) -> bool {
        self.entries.iter().any(OverlayEntry::captures_lower_layers)
    }

    /// Returns the overlay that should receive focus by default.
    #[must_use]
    pub fn focus_target(&self) -> Option<OverlayId> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.receives_focus())
            .map(|entry| entry.id)
    }

    /// Returns the topmost overlay containing a point.
    #[must_use]
    pub fn topmost_at(&self, point: Point) -> Option<&OverlayEntry> {
        self.entries
            .iter()
            .rev()
            .find(|entry| entry.rect.contains_point(point))
    }

    /// Returns the overlay that captures pointer routing for a point.
    ///
    /// A point inside the topmost overlay routes to that overlay. A modal
    /// overlay captures any point that was not already claimed by a higher
    /// overlay, so lower UI cannot receive interaction through it.
    #[must_use]
    pub fn pointer_capture_target(&self, point: Point) -> Option<OverlayId> {
        self.entries.iter().rev().find_map(|entry| {
            if entry.rect.contains_point(point) || entry.captures_lower_layers() {
                Some(entry.id)
            } else {
                None
            }
        })
    }

    /// Returns overlays that should close for an outside activation point.
    #[must_use]
    pub fn outside_click_close_requests(&self, point: Point) -> Vec<OverlayId> {
        let mut requests = Vec::new();
        for entry in self.entries.iter().rev() {
            if entry.rect.contains_point(point) {
                break;
            }
            if entry.dismissal.closes_on_outside_click() {
                requests.push(entry.id);
            }
            if entry.captures_lower_layers()
                || (!entry.dismissal.closes_on_outside_click() && entry.receives_focus())
            {
                break;
            }
        }
        requests
    }

    /// Returns the top overlay that should close for Escape.
    #[must_use]
    pub fn escape_close_request(&self) -> Option<OverlayId> {
        for entry in self.entries.iter().rev() {
            if entry.dismissal.closes_on_escape() {
                return Some(entry.id);
            }
            if entry.captures_lower_layers() || entry.receives_focus() {
                return None;
            }
        }
        None
    }

    /// Returns dismissal requests for a frame's overlay input.
    #[must_use]
    pub fn dismissal_requests(
        &self,
        outside_activation: Option<Point>,
        escape_pressed: bool,
    ) -> Vec<OverlayId> {
        let mut requests = outside_activation
            .map_or_else(Vec::new, |point| self.outside_click_close_requests(point));
        if escape_pressed
            && let Some(id) = self.escape_close_request()
            && !requests.contains(&id)
        {
            requests.push(id);
        }
        requests
    }

    fn descendant_ids(&self, root: OverlayId) -> Vec<OverlayId> {
        let mut ids = vec![root];
        let mut changed = true;
        while changed {
            changed = false;
            for entry in &self.entries {
                if entry.parent.is_some_and(|parent| ids.contains(&parent))
                    && !ids.contains(&entry.id)
                {
                    ids.push(entry.id);
                    changed = true;
                }
            }
        }
        ids
    }

    fn would_parent_cycle(&self, parent: OverlayId, child: OverlayId) -> bool {
        let mut current = Some(parent);
        let mut visited = Vec::new();
        while let Some(id) = current {
            if id == child || visited.contains(&id) {
                return true;
            }
            visited.push(id);
            current = self
                .entries
                .iter()
                .find(|entry| entry.id == id)
                .and_then(|entry| entry.parent);
        }
        false
    }
}
