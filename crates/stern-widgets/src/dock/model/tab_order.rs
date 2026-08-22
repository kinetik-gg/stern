//! Deterministic tab-strip insertion and local reordering targets.
//!
//! Everything here is pure: no time sources, no platform services, and no
//! dependence on iteration order beyond the caller-supplied slice order.
//! Strip slices follow the deterministic dock-tree order produced by the
//! prepared [`crate::dock::DockScene`] layout.

use stern_core::{Point, Rect};

use super::super::{Dock, DockTabDrag, FrameId, PanelId};
use super::prune_empty_frames;

/// Prepared geometry for one frame's tab strip.
#[derive(Debug, Clone, PartialEq)]
pub struct DockTabStripGeometry {
    /// Frame owning the strip.
    pub frame: FrameId,
    /// Full frame rectangle.
    pub rect: Rect,
    /// Tab-strip rectangle used as the drop surface.
    pub tab_list_rect: Rect,
    /// Tab slots in model order.
    pub tabs: Vec<DockTabSlotGeometry>,
}

/// One prepared tab slot inside a [`DockTabStripGeometry`].
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockTabSlotGeometry {
    /// Stable panel identity.
    pub panel: PanelId,
    /// Clipped tab rectangle.
    pub rect: Rect,
}

/// Pure tab-strip placement target for a dragged frame tab.
///
/// The same placement is expressed two ways on purpose: reordering inside
/// the source frame is addressed by slot index, while inserting into another
/// frame is addressed by the stable anchor panel the dragged tab lands
/// before. `anchor: None` always means append after the last tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockTabStripTarget {
    /// Reorder within `frame`; `index` is the dragged tab's slot in the
    /// frame order after removal of the dragged panel (`0..=len-1`).
    Reorder {
        /// Frame receiving the reordered tab; always the source frame.
        frame: FrameId,
        /// Resulting full-list index of the dragged tab.
        index: usize,
    },
    /// Insert into another frame immediately before `anchor`.
    Insert {
        /// Target frame receiving the dragged tab.
        frame: FrameId,
        /// Anchor panel the dragged tab is placed before; `None` appends.
        anchor: Option<PanelId>,
    },
}

impl DockTabStripTarget {
    /// Returns the receiving frame identity.
    #[must_use]
    pub const fn frame(self) -> FrameId {
        match self {
            Self::Reorder { frame, .. } | Self::Insert { frame, .. } => frame,
        }
    }

    /// Converts this target into the anchor-keyed [`super::super::DockDropTarget`]
    /// form using the current source-strip order.
    ///
    /// Reorder slots are mapped onto anchors deterministically against the
    /// source strip with the dragged panel removed: slot `k` places the
    /// dragged tab before the panel currently at slot `k`, and the last slot
    /// maps to append (`None`). Returns `None` when the slot is out of range
    /// or the geometry no longer contains the dragged panel.
    #[must_use]
    pub fn to_anchor_target(
        self,
        strips: &[DockTabStripGeometry],
        drag: DockTabDrag,
    ) -> Option<(FrameId, Option<PanelId>)> {
        match self {
            Self::Reorder { frame, index } => {
                let strip = strips.iter().find(|strip| strip.frame == frame)?;
                let remaining = remaining_strip_panels(strip, drag);
                let anchor = dock_tab_slot_anchor(&remaining, index)?;
                Some((frame, anchor))
            }
            Self::Insert { frame, anchor } => Some((frame, anchor)),
        }
    }
}

/// Resolves a pure tab-strip target from pointer geometry over prepared
/// strips.
///
/// Deterministic resolution rules:
///
/// 1. Strips are scanned in slice order (dock-tree order); the first strip
///    whose `tab_list_rect` contains the pointer wins. Overlapping strips
///    therefore resolve to the earlier frame.
/// 2. A pointer over the source frame's own strip resolves to
///    [`DockTabStripTarget::Reorder`]. The slot is the number of remaining
///    tabs (all tabs except the dragged one) whose center is at or left of
///    the pointer, so equal centers place the dragged tab to the right of
///    that tab. Slot equality with the dragged tab's current position is a
///    no-op and resolves to `None`.
/// 3. A pointer over another frame's strip resolves to
///    [`DockTabStripTarget::Insert`]: the hovered tab is the anchor, and a
///    pointer in a gap, past the last tab, or into an empty strip appends.
/// 4. A pointer outside every strip returns `None`, which allows callers to
///    fall through to generic edge-split targeting.
/// 5. Non-finite pointers never resolve.
#[must_use]
pub fn resolve_dock_tab_strip_target(
    strips: &[DockTabStripGeometry],
    pointer: Point,
    drag: DockTabDrag,
) -> Option<DockTabStripTarget> {
    if !valid_pointer(pointer) {
        return None;
    }

    let strip = strips
        .iter()
        .find(|strip| valid_rect(strip.tab_list_rect) && strip.tab_list_rect.contains_point(pointer))?;

    if strip.frame == drag.source_frame {
        if !strip.tabs.iter().any(|slot| slot.panel == drag.panel) {
            return None;
        }
        let original_index = strip
            .tabs
            .iter()
            .position(|slot| slot.panel == drag.panel)
            .map_or(0, |index| index.min(strip.tabs.len().saturating_sub(1)));
        let slot = strip
            .tabs
            .iter()
            .filter(|candidate| candidate.panel != drag.panel)
            .filter(|candidate| {
                valid_rect(candidate.rect) && tab_center_x(candidate) <= pointer.x
            })
            .count();
        (slot != original_index).then_some(DockTabStripTarget::Reorder {
            frame: strip.frame,
            index: slot,
        })
    } else {
        let anchor = strip
            .tabs
            .iter()
            .find(|slot| valid_rect(slot.rect) && slot.rect.contains_point(pointer))
            .map(|slot| slot.panel);
        Some(DockTabStripTarget::Insert {
            frame: strip.frame,
            anchor,
        })
    }
}

/// Returns true when any prepared strip surface contains the pointer.
///
/// Callers use this to distinguish "over a strip but idle" — which must show
/// no preview and must not fall through to split targeting — from "outside
/// every strip", which may fall through.
#[must_use]
pub fn dock_tab_strip_contains_point(
    strips: &[DockTabStripGeometry],
    pointer: Point,
) -> bool {
    valid_pointer(pointer)
        && strips.iter().any(|strip| {
            valid_rect(strip.tab_list_rect) && strip.tab_list_rect.contains_point(pointer)
        })
}

/// Maps an insertion slot onto a stable anchor panel.
///
/// With `remaining` holding the source strip panels in order excluding the
/// dragged tab, slot `k` places the dragged tab immediately before
/// `remaining[k]`; the final slot appends (`Some(None)`). Returns `None`
/// when the slot is out of range, which callers must treat as an invalid
/// target rather than an append.
#[must_use]
pub fn dock_tab_slot_anchor(remaining: &[PanelId], slot: usize) -> Option<Option<PanelId>> {
    if slot > remaining.len() {
        return None;
    }
    Some(remaining.get(slot).copied())
}

impl Dock {
    /// Pure current-dock validation query for an anchor-keyed tab insertion.
    ///
    /// This is the single validation shared by preview resolution and by
    /// [`Self::drop_tab`] at release-commit time. It checks that the source
    /// frame still owns the dragged panel, that the target frame exists and
    /// is non-empty, that cross-frame anchors are present in the target
    /// frame, and that same-frame placements actually move the tab.
    #[must_use]
    pub fn tab_insertion_is_current(
        &self,
        drag: DockTabDrag,
        frame: FrameId,
        anchor: Option<PanelId>,
    ) -> bool {
        let Some(source) = self.frame(drag.source_frame) else {
            return false;
        };
        let Some(position) = source.panels.iter().position(|panel| panel.id == drag.panel) else {
            return false;
        };
        let Some(target) = self.frame(frame) else {
            return false;
        };
        if target.panels.is_empty() {
            return false;
        }
        if frame != drag.source_frame {
            return anchor.is_none_or(|anchor| {
                target.panels.iter().any(|panel| panel.id == anchor)
            });
        }

        // Same-frame placement is only current when it moves the tab: the
        // anchor must not be exactly the panel after the dragged tab, and
        // appending is only meaningful when the dragged tab is not last.
        match anchor {
            None => position + 1 < source.panels.len(),
            Some(anchor) => source.panels.get(position + 1).is_some_and(|next| next.id != anchor),
        }
    }

    /// Validates and applies an anchor-keyed tab insertion.
    ///
    /// Validation goes through [`Self::tab_insertion_is_current`], so invalid
    /// targets leave the committed tree untouched. Same-frame placements
    /// preserve the active panel identity; cross-frame insertions activate
    /// the moved tab in the target frame like every other docking move.
    pub fn apply_tab_insertion(
        &mut self,
        drag: DockTabDrag,
        frame: FrameId,
        anchor: Option<PanelId>,
    ) -> bool {
        if !self.tab_insertion_is_current(drag, frame, anchor) {
            return false;
        }

        let active_before = self
            .frame(drag.source_frame)
            .and_then(|source| source.active_panel())
            .map(|panel| panel.id);
        let Some((panel, dismissible)) = self
            .frame_mut(drag.source_frame)
            .and_then(|source| source.remove_panel_with_policy(drag.panel))
        else {
            return false;
        };

        let Some(target) = self.frame_mut(frame) else {
            return false;
        };
        let index = anchor
            .and_then(|anchor| target.panels.iter().position(|item| item.id == anchor))
            .unwrap_or(target.panels.len());
        let panel_id = panel.id;
        target.panels.insert(index, panel);
        target.set_panel_dismissible(panel_id, dismissible);

        if frame == drag.source_frame {
            if let Some(active_before) = active_before
                && let Some(active_index) =
                    target.panels.iter().position(|item| item.id == active_before)
            {
                target.active = active_index;
            }
        } else {
            target.active = index;
            prune_empty_frames(&mut self.root);
            self.active_frame = Some(frame);
            self.refresh_active_frame();
        }
        true
    }
}

fn remaining_strip_panels(strip: &DockTabStripGeometry, drag: DockTabDrag) -> Vec<PanelId> {
    strip
        .tabs
        .iter()
        .map(|slot| slot.panel)
        .filter(|panel| *panel != drag.panel)
        .collect()
}

fn tab_center_x(slot: &DockTabSlotGeometry) -> f32 {
    slot.rect.x + slot.rect.width * 0.5
}

fn valid_pointer(point: Point) -> bool {
    point.x.is_finite() && point.y.is_finite()
}

fn valid_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
}
