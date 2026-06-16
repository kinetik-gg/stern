//! `DockArea`, `Frame`, and `Panel` models for editor layouts.

use std::collections::BTreeSet;

use kinetik_ui_core::{Axis, Point, Rect, Vec2};

const DEFAULT_SPLIT_RATIO: f32 = 0.5;
const DEFAULT_SPLIT_MINIMUM: f32 = 100.0;
const DEFAULT_SPLITTER_THICKNESS: f32 = 6.0;
const DROP_EDGE_FRACTION: f32 = 0.25;

/// Stable panel identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PanelId(u64);

impl PanelId {
    /// Creates a panel ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Stable frame identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FrameId(u64);

impl FrameId {
    /// Creates a frame ID from raw bits.
    #[must_use]
    pub const fn from_raw(raw: u64) -> Self {
        Self(raw)
    }

    /// Returns raw ID bits.
    #[must_use]
    pub const fn raw(self) -> u64 {
        self.0
    }
}

/// Address of a split node inside a dock tree.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash)]
pub struct DockSplitPath(Vec<DockPathElement>);

impl DockSplitPath {
    /// Returns the root split path.
    #[must_use]
    pub const fn root() -> Self {
        Self(Vec::new())
    }

    /// Creates a path from child traversal elements.
    #[must_use]
    pub fn new(elements: impl IntoIterator<Item = DockPathElement>) -> Self {
        Self(elements.into_iter().collect())
    }

    /// Returns a child path under this split.
    #[must_use]
    pub fn child(&self, element: DockPathElement) -> Self {
        let mut path = self.clone();
        path.0.push(element);
        path
    }

    /// Returns the traversal elements.
    #[must_use]
    pub fn elements(&self) -> &[DockPathElement] {
        &self.0
    }
}

/// Traversal element for a [`DockSplitPath`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DockPathElement {
    /// Descend into the first split child.
    First,
    /// Descend into the second split child.
    Second,
}

/// Dock placement used when splitting a frame with a dragged tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPlacement {
    /// Insert a new frame to the left of the target frame.
    Left,
    /// Insert a new frame to the right of the target frame.
    Right,
    /// Insert a new frame above the target frame.
    Top,
    /// Insert a new frame below the target frame.
    Bottom,
}

impl DockPlacement {
    const fn axis(self) -> Axis {
        match self {
            Self::Left | Self::Right => Axis::Horizontal,
            Self::Top | Self::Bottom => Axis::Vertical,
        }
    }

    const fn insert_before_target(self) -> bool {
        matches!(self, Self::Left | Self::Top)
    }
}

/// Resolved dock drop zone inside a frame rectangle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockDropZone {
    /// Merge the dragged tab into the target frame.
    Center,
    /// Split to the target frame's left.
    Left,
    /// Split to the target frame's right.
    Right,
    /// Split above the target frame.
    Top,
    /// Split below the target frame.
    Bottom,
}

impl DockDropZone {
    const fn placement(self) -> Option<DockPlacement> {
        match self {
            Self::Center => None,
            Self::Left => Some(DockPlacement::Left),
            Self::Right => Some(DockPlacement::Right),
            Self::Top => Some(DockPlacement::Top),
            Self::Bottom => Some(DockPlacement::Bottom),
        }
    }
}

/// Tab drag state emitted by frame chrome.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DockTabDrag {
    /// Source frame that owns the dragged tab.
    pub source_frame: FrameId,
    /// Dragged panel tab.
    pub panel: PanelId,
}

/// Explicit target for dropping a dragged frame tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DockDropTarget {
    /// Merge the panel into an existing frame tab group.
    Tab {
        /// Target frame.
        frame: FrameId,
    },
    /// Insert the panel as a new frame split adjacent to an existing frame.
    Split {
        /// Existing frame to split around.
        frame: FrameId,
        /// Placement of the new frame relative to the target frame.
        placement: DockPlacement,
        /// Frame ID for the newly inserted frame.
        new_frame: FrameId,
        /// Initial split ratio.
        ratio: f32,
        /// Minimum first child size.
        min_first: f32,
        /// Minimum second child size.
        min_second: f32,
    },
}

impl DockDropTarget {
    /// Creates a tab merge target.
    #[must_use]
    pub const fn tab(frame: FrameId) -> Self {
        Self::Tab { frame }
    }

    /// Creates a split insertion target with editor-friendly defaults.
    #[must_use]
    pub const fn split(frame: FrameId, placement: DockPlacement, new_frame: FrameId) -> Self {
        Self::Split {
            frame,
            placement,
            new_frame,
            ratio: DEFAULT_SPLIT_RATIO,
            min_first: DEFAULT_SPLIT_MINIMUM,
            min_second: DEFAULT_SPLIT_MINIMUM,
        }
    }
}

/// Resolved splitter hit target.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSplitter {
    /// Split path addressed by this splitter.
    pub path: DockSplitPath,
    /// Split axis.
    pub axis: Axis,
    /// Splitter interaction rectangle.
    pub rect: Rect,
    /// Current split ratio.
    pub ratio: f32,
    /// Minimum first child size.
    pub min_first: f32,
    /// Minimum second child size.
    pub min_second: f32,
}

/// Passive panel metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// Panel identity.
    pub id: PanelId,
    /// Display title used by frame tabs.
    pub title: String,
}

impl Panel {
    /// Creates a panel.
    #[must_use]
    pub fn new(id: PanelId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
        }
    }
}

/// Docked frame containing tabbed panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Frame {
    /// Frame identity.
    pub id: FrameId,
    /// Panels in tab order.
    pub panels: Vec<Panel>,
    /// Active panel index.
    pub active: usize,
    /// Panels whose frame tabs expose close/dismiss affordances.
    dismissible_panels: BTreeSet<PanelId>,
}

impl Frame {
    /// Creates a frame with panels.
    #[must_use]
    pub fn new(id: FrameId, panels: Vec<Panel>) -> Self {
        let dismissible_panels = panels.iter().map(|panel| panel.id).collect();
        Self {
            id,
            panels,
            active: 0,
            dismissible_panels,
        }
    }

    /// Returns the active panel.
    #[must_use]
    pub fn active_panel(&self) -> Option<&Panel> {
        self.panels.get(self.active)
    }

    /// Selects a panel by ID.
    pub fn select_panel(&mut self, panel: PanelId) -> bool {
        let Some(index) = self.panels.iter().position(|item| item.id == panel) else {
            return false;
        };
        self.active = index;
        true
    }

    /// Removes a panel by ID.
    pub fn remove_panel(&mut self, panel: PanelId) -> Option<Panel> {
        let (removed, _) = self.remove_panel_with_policy(panel)?;
        Some(removed)
    }

    fn remove_panel_with_policy(&mut self, panel: PanelId) -> Option<(Panel, bool)> {
        let index = self.panels.iter().position(|item| item.id == panel)?;
        let dismissible = self.dismissible_panels.remove(&panel);
        let removed = self.panels.remove(index);
        self.active = self.active.min(self.panels.len().saturating_sub(1));
        Some((removed, dismissible))
    }

    /// Adds a panel at the end.
    pub fn push_panel(&mut self, panel: Panel) {
        self.push_panel_with_policy(panel, true);
    }

    fn push_panel_with_policy(&mut self, panel: Panel, dismissible: bool) {
        let id = panel.id;
        self.panels.push(panel);
        self.set_panel_dismissible(id, dismissible);
    }

    /// Sets whether a frame tab can expose close/dismiss affordances.
    ///
    /// Returns `false` when the panel is not in this frame.
    pub fn set_panel_dismissible(&mut self, panel: PanelId, dismissible: bool) -> bool {
        if !self.panels.iter().any(|item| item.id == panel) {
            return false;
        }
        if dismissible {
            self.dismissible_panels.insert(panel);
        } else {
            self.dismissible_panels.remove(&panel);
        }
        true
    }

    /// Returns true when a frame tab can expose close/dismiss affordances.
    #[must_use]
    pub fn panel_dismissible(&self, panel: PanelId) -> bool {
        self.dismissible_panels.contains(&panel)
    }
}

/// Dock tree node.
#[derive(Debug, Clone, PartialEq)]
pub enum DockNode {
    /// Leaf frame.
    Frame(Frame),
    /// Split between two nodes.
    Split {
        /// Split axis.
        axis: Axis,
        /// First child ratio.
        ratio: f32,
        /// Minimum first child size.
        min_first: f32,
        /// Minimum second child size.
        min_second: f32,
        /// First child.
        first: Box<DockNode>,
        /// Second child.
        second: Box<DockNode>,
    },
}

/// Root dock area.
#[derive(Debug, Clone, PartialEq)]
pub struct DockArea {
    /// Root dock node.
    pub root: DockNode,
}

impl DockArea {
    /// Creates a dock area.
    #[must_use]
    pub const fn new(root: DockNode) -> Self {
        Self { root }
    }

    /// Visits all frames in deterministic tree order.
    #[must_use]
    pub fn frames(&self) -> Vec<&Frame> {
        let mut frames = Vec::new();
        collect_frames(&self.root, &mut frames);
        frames
    }

    /// Finds an immutable frame.
    #[must_use]
    pub fn frame(&self, frame: FrameId) -> Option<&Frame> {
        find_frame(&self.root, frame)
    }

    /// Finds a mutable frame.
    pub fn frame_mut(&mut self, frame: FrameId) -> Option<&mut Frame> {
        find_frame_mut(&mut self.root, frame)
    }

    /// Selects a panel in a frame.
    pub fn select_panel(&mut self, frame: FrameId, panel: PanelId) -> bool {
        self.frame_mut(frame)
            .is_some_and(|frame| frame.select_panel(panel))
    }

    /// Moves a panel between frames.
    pub fn move_panel(&mut self, from: FrameId, to: FrameId, panel: PanelId) -> bool {
        if from == to || self.frame(to).is_none() {
            return false;
        }
        let Some((panel, dismissible)) = self
            .frame_mut(from)
            .and_then(|frame| frame.remove_panel_with_policy(panel))
        else {
            return false;
        };
        let Some(target) = self.frame_mut(to) else {
            return false;
        };
        target.push_panel_with_policy(panel, dismissible);
        target.active = target.panels.len().saturating_sub(1);
        prune_empty_frames(&mut self.root);
        true
    }

    /// Merges all source frame panels into target frame.
    pub fn merge_frames(&mut self, source: FrameId, target: FrameId) -> bool {
        if source == target || self.frame(source).is_none() || self.frame(target).is_none() {
            return false;
        }
        let Some((source_panels, dismissible_panels)) = self.frame_mut(source).map(|frame| {
            frame.active = 0;
            (
                core::mem::take(&mut frame.panels),
                core::mem::take(&mut frame.dismissible_panels),
            )
        }) else {
            return false;
        };
        let Some(target_frame) = self.frame_mut(target) else {
            return false;
        };
        target_frame.panels.extend(source_panels);
        target_frame.dismissible_panels.extend(dismissible_panels);
        target_frame.active = target_frame.panels.len().saturating_sub(1);
        prune_empty_frames(&mut self.root);
        true
    }

    /// Starts a tab drag when the frame owns the panel.
    #[must_use]
    pub fn begin_tab_drag(&self, source_frame: FrameId, panel: PanelId) -> Option<DockTabDrag> {
        self.frame(source_frame)
            .filter(|frame| frame.panels.iter().any(|item| item.id == panel))
            .map(|_| DockTabDrag {
                source_frame,
                panel,
            })
    }

    /// Applies a dragged tab to an explicit dock target.
    pub fn drop_tab(&mut self, drag: DockTabDrag, target: DockDropTarget) -> bool {
        match target {
            DockDropTarget::Tab { frame } => {
                if drag.source_frame == frame {
                    return self.select_panel(frame, drag.panel);
                }
                self.move_panel(drag.source_frame, frame, drag.panel)
            }
            DockDropTarget::Split {
                frame,
                placement,
                new_frame,
                ratio,
                min_first,
                min_second,
            } => self.split_panel(
                drag.source_frame,
                drag.panel,
                DockSplitInsertion {
                    target_frame: frame,
                    placement,
                    new_frame,
                    ratio,
                    min_first,
                    min_second,
                },
            ),
        }
    }

    /// Inserts one panel as a new frame split adjacent to an existing frame.
    pub fn split_panel(
        &mut self,
        source_frame: FrameId,
        panel: PanelId,
        insertion: DockSplitInsertion,
    ) -> bool {
        if self.frame(insertion.target_frame).is_none()
            || self.frame(insertion.new_frame).is_some()
            || !insertion.ratio.is_finite()
            || !(0.0..=1.0).contains(&insertion.ratio)
            || !insertion.min_first.is_finite()
            || insertion.min_first < 0.0
            || !insertion.min_second.is_finite()
            || insertion.min_second < 0.0
        {
            return false;
        }

        if source_frame == insertion.target_frame
            && self
                .frame(source_frame)
                .is_none_or(|frame| frame.panels.len() <= 1)
        {
            return false;
        }

        let Some((panel, dismissible)) = self
            .frame_mut(source_frame)
            .and_then(|frame| frame.remove_panel_with_policy(panel))
        else {
            return false;
        };

        let mut inserted = Frame::new(insertion.new_frame, vec![panel]);
        inserted.set_panel_dismissible(inserted.panels[0].id, dismissible);
        prune_empty_frames(&mut self.root);

        insert_frame_split(&mut self.root, insertion, inserted)
    }

    /// Resizes a split addressed by path using a drag delta in logical units.
    pub fn resize_split(&mut self, path: &DockSplitPath, bounds: Rect, delta: Vec2) -> bool {
        resize_split_at_path(&mut self.root, path.elements(), bounds, delta)
    }

    /// Creates a snapshot for persistence.
    #[must_use]
    pub fn snapshot(&self) -> DockSnapshot {
        DockSnapshot {
            root: snapshot_node(&self.root),
        }
    }

    /// Restores a snapshot after validation.
    ///
    /// # Errors
    ///
    /// Returns [`DockRestoreError`] when persisted dock data is structurally
    /// invalid, contains duplicate identities, or stores invalid split values.
    pub fn restore(snapshot: DockSnapshot) -> Result<Self, DockRestoreError> {
        let mut validation = DockSnapshotValidation::default();
        validate_snapshot_node(&snapshot.root, &mut validation)?;
        Ok(Self {
            root: restore_node(snapshot.root),
        })
    }
}

/// Request for splitting a dragged panel into a new frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockSplitInsertion {
    /// Existing frame to split around.
    pub target_frame: FrameId,
    /// Placement of the new frame relative to the target frame.
    pub placement: DockPlacement,
    /// Frame ID for the newly inserted frame.
    pub new_frame: FrameId,
    /// Initial split ratio.
    pub ratio: f32,
    /// Minimum first child size.
    pub min_first: f32,
    /// Minimum second child size.
    pub min_second: f32,
}

impl DockSplitInsertion {
    /// Creates a split insertion request with editor-friendly defaults.
    #[must_use]
    pub const fn new(target_frame: FrameId, placement: DockPlacement, new_frame: FrameId) -> Self {
        Self {
            target_frame,
            placement,
            new_frame,
            ratio: DEFAULT_SPLIT_RATIO,
            min_first: DEFAULT_SPLIT_MINIMUM,
            min_second: DEFAULT_SPLIT_MINIMUM,
        }
    }
}

fn collect_frames<'a>(node: &'a DockNode, frames: &mut Vec<&'a Frame>) {
    match node {
        DockNode::Frame(frame) => frames.push(frame),
        DockNode::Split { first, second, .. } => {
            collect_frames(first, frames);
            collect_frames(second, frames);
        }
    }
}

fn find_frame_mut(node: &mut DockNode, id: FrameId) -> Option<&mut Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame_mut(first, id).or_else(|| find_frame_mut(second, id))
        }
    }
}

fn find_frame(node: &DockNode, id: FrameId) -> Option<&Frame> {
    match node {
        DockNode::Frame(frame) if frame.id == id => Some(frame),
        DockNode::Frame(_) => None,
        DockNode::Split { first, second, .. } => {
            find_frame(first, id).or_else(|| find_frame(second, id))
        }
    }
}

fn prune_empty_frames(node: &mut DockNode) -> bool {
    match node {
        DockNode::Frame(frame) => !frame.panels.is_empty(),
        DockNode::Split { first, second, .. } => {
            let first_has_panels = prune_empty_frames(first);
            let second_has_panels = prune_empty_frames(second);
            match (first_has_panels, second_has_panels) {
                (true, true) => true,
                (true, false) => {
                    *node = (**first).clone();
                    true
                }
                (false, true) => {
                    *node = (**second).clone();
                    true
                }
                (false, false) => false,
            }
        }
    }
}

fn insert_frame_split(node: &mut DockNode, insertion: DockSplitInsertion, inserted: Frame) -> bool {
    match node {
        DockNode::Frame(frame) if frame.id == insertion.target_frame => {
            let target = DockNode::Frame(frame.clone());
            let inserted = DockNode::Frame(inserted);
            let (first, second) = if insertion.placement.insert_before_target() {
                (inserted, target)
            } else {
                (target, inserted)
            };
            *node = DockNode::Split {
                axis: insertion.placement.axis(),
                ratio: insertion.ratio,
                min_first: insertion.min_first,
                min_second: insertion.min_second,
                first: Box::new(first),
                second: Box::new(second),
            };
            true
        }
        DockNode::Frame(_) => false,
        DockNode::Split { first, second, .. } => {
            insert_frame_split(first, insertion, inserted.clone())
                || insert_frame_split(second, insertion, inserted)
        }
    }
}

fn resize_split_at_path(
    node: &mut DockNode,
    path: &[DockPathElement],
    bounds: Rect,
    delta: Vec2,
) -> bool {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return false;
    };

    if path.is_empty() {
        *ratio = split_ratio_from_drag(*axis, bounds, *ratio, *min_first, *min_second, delta);
        return true;
    }

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    match path[0] {
        DockPathElement::First => resize_split_at_path(first, &path[1..], first_rect, delta),
        DockPathElement::Second => resize_split_at_path(second, &path[1..], second_rect, delta),
    }
}

/// Resolved frame rectangle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FrameLayout {
    /// Frame identity.
    pub frame: FrameId,
    /// Frame rectangle.
    pub rect: Rect,
}

/// Resolves a dock tree into frame rectangles.
#[must_use]
pub fn solve_dock_layout(area: &DockArea, bounds: Rect) -> Vec<FrameLayout> {
    let mut frames = Vec::new();
    solve_node(&area.root, bounds, &mut frames);
    frames
}

/// Resolves splitter interaction rectangles for a dock tree.
#[must_use]
pub fn solve_dock_splitters(area: &DockArea, bounds: Rect, thickness: f32) -> Vec<DockSplitter> {
    let mut splitters = Vec::new();
    solve_splitters(
        &area.root,
        bounds,
        splitter_thickness(thickness),
        &DockSplitPath::root(),
        &mut splitters,
    );
    splitters
}

fn solve_splitters(
    node: &DockNode,
    bounds: Rect,
    thickness: f32,
    path: &DockSplitPath,
    splitters: &mut Vec<DockSplitter>,
) {
    let DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first,
        second,
    } = node
    else {
        return;
    };

    let (first_rect, second_rect) =
        split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
    splitters.push(DockSplitter {
        path: path.clone(),
        axis: *axis,
        rect: splitter_rect(*axis, first_rect, bounds, thickness),
        ratio: finite_ratio(*ratio),
        min_first: finite_non_negative(*min_first),
        min_second: finite_non_negative(*min_second),
    });
    solve_splitters(
        first,
        first_rect,
        thickness,
        &path.child(DockPathElement::First),
        splitters,
    );
    solve_splitters(
        second,
        second_rect,
        thickness,
        &path.child(DockPathElement::Second),
        splitters,
    );
}

fn solve_node(node: &DockNode, bounds: Rect, frames: &mut Vec<FrameLayout>) {
    match node {
        DockNode::Frame(frame) => frames.push(FrameLayout {
            frame: frame.id,
            rect: bounds,
        }),
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => {
            let (first_rect, second_rect) =
                split_child_rects(*axis, bounds, *ratio, *min_first, *min_second);
            solve_node(first, first_rect, frames);
            solve_node(second, second_rect, frames);
        }
    }
}

fn split_child_rects(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
) -> (Rect, Rect) {
    let total = split_total(axis, bounds);
    let first_size = split_first_size(total, ratio, min_first, min_second);
    let second_size = (total - first_size).max(0.0);
    match axis {
        Axis::Horizontal => (
            Rect::new(bounds.x, bounds.y, first_size, bounds.height),
            Rect::new(bounds.x + first_size, bounds.y, second_size, bounds.height),
        ),
        Axis::Vertical => (
            Rect::new(bounds.x, bounds.y, bounds.width, first_size),
            Rect::new(bounds.x, bounds.y + first_size, bounds.width, second_size),
        ),
    }
}

fn split_total(axis: Axis, bounds: Rect) -> f32 {
    match axis {
        Axis::Horizontal => finite_non_negative(bounds.width),
        Axis::Vertical => finite_non_negative(bounds.height),
    }
}

fn split_first_size(total: f32, ratio: f32, min_first: f32, min_second: f32) -> f32 {
    let total = finite_non_negative(total);
    let min_first = finite_non_negative(min_first);
    let min_second = finite_non_negative(min_second);
    let desired = total * finite_ratio(ratio);
    if total >= min_first + min_second {
        desired.clamp(min_first, total - min_second)
    } else {
        desired.max(min_first.min(total)).min(total)
    }
}

fn splitter_rect(axis: Axis, first_rect: Rect, bounds: Rect, thickness: f32) -> Rect {
    let half = thickness * 0.5;
    match axis {
        Axis::Horizontal => Rect::new(
            first_rect.max_x() - half,
            bounds.y,
            thickness,
            bounds.height,
        ),
        Axis::Vertical => Rect::new(bounds.x, first_rect.max_y() - half, bounds.width, thickness),
    }
}

fn splitter_thickness(thickness: f32) -> f32 {
    if thickness.is_finite() && thickness > 0.0 {
        thickness
    } else {
        DEFAULT_SPLITTER_THICKNESS
    }
}

/// Maps a splitter drag delta to a clamped split ratio.
#[must_use]
pub fn split_ratio_from_drag(
    axis: Axis,
    bounds: Rect,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    delta: Vec2,
) -> f32 {
    let total = split_total(axis, bounds);
    if total <= 0.0 {
        return finite_ratio(ratio);
    }
    let delta = match axis {
        Axis::Horizontal => delta.x,
        Axis::Vertical => delta.y,
    };
    let current = split_first_size(total, ratio, min_first, min_second);
    let desired = current + if delta.is_finite() { delta } else { 0.0 };
    split_first_size(total, desired / total, min_first, min_second) / total
}

/// Resolves a frame-local drop zone for a pointer position.
#[must_use]
pub fn resolve_frame_drop_zone(rect: Rect, point: Point) -> Option<DockDropZone> {
    if !rect.contains_point(point) {
        return None;
    }

    let left = (point.x - rect.min_x()).max(0.0);
    let right = (rect.max_x() - point.x).max(0.0);
    let top = (point.y - rect.min_y()).max(0.0);
    let bottom = (rect.max_y() - point.y).max(0.0);
    let edge_x = finite_non_negative(rect.width) * DROP_EDGE_FRACTION;
    let edge_y = finite_non_negative(rect.height) * DROP_EDGE_FRACTION;

    let mut best = (DockDropZone::Center, f32::INFINITY);
    for (zone, distance, limit) in [
        (DockDropZone::Left, left, edge_x),
        (DockDropZone::Right, right, edge_x),
        (DockDropZone::Top, top, edge_y),
        (DockDropZone::Bottom, bottom, edge_y),
    ] {
        if distance <= limit && distance < best.1 {
            best = (zone, distance);
        }
    }

    Some(best.0)
}

/// Resolves a dock drop target from frame layout and a pointer position.
#[must_use]
pub fn resolve_dock_drop_target(
    frames: &[FrameLayout],
    point: Point,
    new_frame: FrameId,
) -> Option<DockDropTarget> {
    frames.iter().find_map(|layout| {
        let zone = resolve_frame_drop_zone(layout.rect, point)?;
        Some(match zone.placement() {
            Some(placement) => DockDropTarget::split(layout.frame, placement, new_frame),
            None => DockDropTarget::tab(layout.frame),
        })
    })
}

fn finite_ratio(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.5
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

/// Tab presentation data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FrameTab {
    /// Panel identity.
    pub panel: PanelId,
    /// Tab title.
    pub title: String,
    /// Whether this tab is active.
    pub active: bool,
    /// Whether this tab can be closed.
    pub close_visible: bool,
    /// Whether this tab can begin a drag operation.
    pub draggable: bool,
}

/// Produces frame tab presentation records.
#[must_use]
pub fn frame_tabs(frame: &Frame) -> Vec<FrameTab> {
    frame
        .panels
        .iter()
        .enumerate()
        .map(|(index, panel)| FrameTab {
            panel: panel.id,
            title: panel.title.clone(),
            active: index == frame.active,
            close_visible: frame.panel_dismissible(panel.id),
            draggable: true,
        })
        .collect()
}

/// Persistable dock snapshot.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSnapshot {
    /// Root snapshot node.
    pub root: DockSnapshotNode,
}

/// Snapshot node.
#[derive(Debug, Clone, PartialEq)]
pub enum DockSnapshotNode {
    /// Frame snapshot.
    Frame {
        /// Frame identity.
        id: FrameId,
        /// Panels.
        panels: Vec<Panel>,
        /// Active panel index.
        active: usize,
        /// Panels whose frame tabs expose close/dismiss affordances.
        dismissible_panels: Vec<PanelId>,
    },
    /// Split snapshot.
    Split {
        /// Split axis.
        axis: Axis,
        /// First child ratio.
        ratio: f32,
        /// Minimum first size.
        min_first: f32,
        /// Minimum second size.
        min_second: f32,
        /// First child.
        first: Box<DockSnapshotNode>,
        /// Second child.
        second: Box<DockSnapshotNode>,
    },
}

/// Snapshot restore error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockRestoreError {
    /// Frame contains no panels.
    EmptyFrame,
    /// Active tab index is outside the panel list.
    InvalidActiveIndex,
    /// Two frames use the same stable frame identity.
    DuplicateFrameId,
    /// Two panels use the same stable panel identity.
    DuplicatePanelId,
    /// Dismissible panel policy references a panel missing from the frame.
    InvalidDismissiblePanel,
    /// Dismissible panel policy contains the same panel more than once.
    DuplicateDismissiblePanel,
    /// Split ratio is not finite or is outside the inclusive 0.0..=1.0 range.
    InvalidSplitRatio,
    /// Split minimum is not finite or is negative.
    InvalidSplitMinimum,
}

fn snapshot_node(node: &DockNode) -> DockSnapshotNode {
    match node {
        DockNode::Frame(frame) => DockSnapshotNode::Frame {
            id: frame.id,
            panels: frame.panels.clone(),
            active: frame.active,
            dismissible_panels: frame.dismissible_panels.iter().copied().collect(),
        },
        DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockSnapshotNode::Split {
            axis: *axis,
            ratio: *ratio,
            min_first: *min_first,
            min_second: *min_second,
            first: Box::new(snapshot_node(first)),
            second: Box::new(snapshot_node(second)),
        },
    }
}

fn restore_node(snapshot: DockSnapshotNode) -> DockNode {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => DockNode::Frame(Frame {
            id,
            panels,
            active,
            dismissible_panels: dismissible_panels.into_iter().collect(),
        }),
        DockSnapshotNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first,
            second,
        } => DockNode::Split {
            axis,
            ratio,
            min_first,
            min_second,
            first: Box::new(restore_node(*first)),
            second: Box::new(restore_node(*second)),
        },
    }
}

#[derive(Default)]
struct DockSnapshotValidation {
    frame_ids: BTreeSet<FrameId>,
    panel_ids: BTreeSet<PanelId>,
}

fn validate_snapshot_node(
    snapshot: &DockSnapshotNode,
    validation: &mut DockSnapshotValidation,
) -> Result<(), DockRestoreError> {
    match snapshot {
        DockSnapshotNode::Frame {
            id,
            panels,
            active,
            dismissible_panels,
        } => {
            if !validation.frame_ids.insert(*id) {
                return Err(DockRestoreError::DuplicateFrameId);
            }
            if panels.is_empty() {
                return Err(DockRestoreError::EmptyFrame);
            }
            if *active >= panels.len() {
                return Err(DockRestoreError::InvalidActiveIndex);
            }

            let mut frame_panel_ids = BTreeSet::new();
            for panel in panels {
                if !frame_panel_ids.insert(panel.id) || !validation.panel_ids.insert(panel.id) {
                    return Err(DockRestoreError::DuplicatePanelId);
                }
            }

            let mut frame_dismissible_ids = BTreeSet::new();
            for id in dismissible_panels {
                if !frame_dismissible_ids.insert(*id) {
                    return Err(DockRestoreError::DuplicateDismissiblePanel);
                }
                if !frame_panel_ids.contains(id) {
                    return Err(DockRestoreError::InvalidDismissiblePanel);
                }
            }
            Ok(())
        }
        DockSnapshotNode::Split {
            ratio,
            min_first,
            min_second,
            first,
            second,
            ..
        } => {
            if !ratio.is_finite() || !(0.0..=1.0).contains(ratio) {
                return Err(DockRestoreError::InvalidSplitRatio);
            }
            if !min_first.is_finite()
                || !min_second.is_finite()
                || *min_first < 0.0
                || *min_second < 0.0
            {
                return Err(DockRestoreError::InvalidSplitMinimum);
            }
            validate_snapshot_node(first, validation)?;
            validate_snapshot_node(second, validation)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DockArea, DockDropTarget, DockDropZone, DockNode, DockPathElement, DockPlacement,
        DockRestoreError, DockSnapshot, DockSnapshotNode, DockSplitInsertion, DockSplitPath, Frame,
        FrameId, Panel, PanelId, frame_tabs, resolve_dock_drop_target, resolve_frame_drop_zone,
        solve_dock_layout, solve_dock_splitters, split_ratio_from_drag,
    };
    use kinetik_ui_core::{Axis, Point, Rect, Vec2};

    fn panel(id: u64, title: &str) -> Panel {
        Panel::new(PanelId::from_raw(id), title)
    }

    fn frame(id: u64, panels: Vec<Panel>) -> Frame {
        Frame::new(FrameId::from_raw(id), panels)
    }

    fn dock_area() -> DockArea {
        DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.25,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "Media")]))),
            second: Box::new(DockNode::Frame(frame(
                2,
                vec![panel(2, "Viewport"), panel(3, "Timeline")],
            ))),
        })
    }

    #[test]
    fn dock_tree_visits_frames_in_order() {
        let area = dock_area();
        let frames = area.frames();

        assert_eq!(frames[0].id, FrameId::from_raw(1));
        assert_eq!(frames[1].id, FrameId::from_raw(2));
    }

    #[test]
    fn selects_and_removes_frame_tabs() {
        let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);

        assert!(frame.select_panel(PanelId::from_raw(2)));
        assert_eq!(
            frame.active_panel().expect("active").id,
            PanelId::from_raw(2)
        );
        assert_eq!(
            frame
                .remove_panel(PanelId::from_raw(2))
                .expect("removed")
                .title,
            "B"
        );
        assert_eq!(frame.active, 0);
    }

    #[test]
    fn moves_panels_between_frames() {
        let mut area = dock_area();

        assert!(area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(1),
            PanelId::from_raw(3)
        ));

        assert_eq!(
            area.frame_mut(FrameId::from_raw(1))
                .expect("frame")
                .panels
                .len(),
            2
        );
    }

    #[test]
    fn moving_panels_preserves_frame_owned_dismissal_policy() {
        let mut area = dock_area();
        area.frame_mut(FrameId::from_raw(2))
            .expect("source")
            .set_panel_dismissible(PanelId::from_raw(3), false);

        assert!(area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(1),
            PanelId::from_raw(3)
        ));

        assert!(
            !area
                .frame(FrameId::from_raw(1))
                .expect("target")
                .panel_dismissible(PanelId::from_raw(3))
        );
    }

    #[test]
    fn moving_panel_to_missing_target_does_not_remove_it() {
        let mut area = dock_area();

        assert!(!area.move_panel(
            FrameId::from_raw(2),
            FrameId::from_raw(99),
            PanelId::from_raw(3)
        ));

        let source = area.frame(FrameId::from_raw(2)).expect("source");
        assert_eq!(source.panels.len(), 2);
        assert!(
            source
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn moving_last_panel_prunes_empty_source_frame() {
        let mut area = dock_area();

        assert!(area.move_panel(
            FrameId::from_raw(1),
            FrameId::from_raw(2),
            PanelId::from_raw(1)
        ));

        assert_eq!(area.frames().len(), 1);
        assert!(area.frame(FrameId::from_raw(1)).is_none());
        assert_eq!(
            area.frame(FrameId::from_raw(2))
                .expect("target")
                .panels
                .len(),
            3
        );
    }

    #[test]
    fn merges_frames_into_target() {
        let mut area = dock_area();

        assert!(area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(2)));

        assert_eq!(
            area.frame_mut(FrameId::from_raw(2))
                .expect("target")
                .panels
                .len(),
            3
        );
        assert_eq!(area.frames().len(), 1);
        assert!(area.frame(FrameId::from_raw(1)).is_none());
    }

    #[test]
    fn merging_missing_target_does_not_remove_source_panels() {
        let mut area = dock_area();

        assert!(!area.merge_frames(FrameId::from_raw(1), FrameId::from_raw(99)));

        assert_eq!(
            area.frame(FrameId::from_raw(1))
                .expect("source")
                .panels
                .len(),
            1
        );
    }

    #[test]
    fn solves_horizontal_split_layout() {
        let area = dock_area();
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));

        assert_eq!(layout.len(), 2);
        assert!((layout[0].rect.width - 250.0).abs() < f32::EPSILON);
        assert!((layout[1].rect.x - 250.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_respects_minimums() {
        let area = DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.05,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 500.0, 200.0));

        assert!((layout[0].rect.width - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_never_emits_negative_sizes_when_minimums_exceed_bounds() {
        let area = DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

        assert_eq!(layout.len(), 2);
        assert!(layout[0].rect.width >= 0.0);
        assert!(layout[1].rect.width >= 0.0);
        assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn split_layout_sanitizes_direct_non_finite_values() {
        let area = DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: f32::NAN,
            min_first: f32::INFINITY,
            min_second: -100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
        });
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 120.0, 200.0));

        assert_eq!(layout.len(), 2);
        assert!(layout[0].rect.width.is_finite());
        assert!(layout[1].rect.width.is_finite());
        assert!((layout[0].rect.width + layout[1].rect.width - 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn splitter_drag_maps_delta_to_clamped_ratio() {
        let bounds = Rect::new(0.0, 0.0, 500.0, 200.0);

        let ratio = split_ratio_from_drag(
            Axis::Horizontal,
            bounds,
            0.5,
            100.0,
            100.0,
            Vec2::new(75.0, 0.0),
        );
        assert!((ratio - 0.65).abs() < f32::EPSILON);

        let clamped = split_ratio_from_drag(
            Axis::Horizontal,
            bounds,
            0.5,
            100.0,
            100.0,
            Vec2::new(-500.0, 0.0),
        );
        assert!((clamped - 0.2).abs() < f32::EPSILON);

        let vertical = split_ratio_from_drag(
            Axis::Vertical,
            bounds,
            0.5,
            50.0,
            50.0,
            Vec2::new(1000.0, 25.0),
        );
        assert!((vertical - 0.625).abs() < f32::EPSILON);
    }

    #[test]
    fn dock_area_resizes_root_and_nested_splits() {
        let mut area = DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.5,
                min_first: 50.0,
                min_second: 50.0,
                first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
                second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
            }),
        });
        let bounds = Rect::new(0.0, 0.0, 600.0, 400.0);

        assert!(area.resize_split(&DockSplitPath::root(), bounds, Vec2::new(60.0, 0.0)));
        assert!(area.resize_split(
            &DockSplitPath::new([DockPathElement::Second]),
            bounds,
            Vec2::new(0.0, -75.0)
        ));
        let layout = solve_dock_layout(&area, bounds);

        assert!((layout[0].rect.width - 360.0).abs() < f32::EPSILON);
        assert!((layout[1].rect.height - 125.0).abs() < f32::EPSILON);
    }

    #[test]
    fn dock_splitters_expose_paths_and_hit_rects() {
        let area = DockArea::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.5,
            min_first: 100.0,
            min_second: 100.0,
            first: Box::new(DockNode::Frame(frame(1, vec![panel(1, "A")]))),
            second: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.25,
                min_first: 50.0,
                min_second: 50.0,
                first: Box::new(DockNode::Frame(frame(2, vec![panel(2, "B")]))),
                second: Box::new(DockNode::Frame(frame(3, vec![panel(3, "C")]))),
            }),
        });

        let splitters = solve_dock_splitters(&area, Rect::new(0.0, 0.0, 600.0, 400.0), 8.0);

        assert_eq!(splitters.len(), 2);
        assert_eq!(splitters[0].path, DockSplitPath::root());
        assert_eq!(splitters[0].rect, Rect::new(296.0, 0.0, 8.0, 400.0));
        assert_eq!(
            splitters[1].path,
            DockSplitPath::new([DockPathElement::Second])
        );
        assert_eq!(splitters[1].axis, Axis::Vertical);
    }

    #[test]
    fn frame_tabs_expose_presentation_state() {
        let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);
        frame.select_panel(PanelId::from_raw(2));
        assert!(frame.set_panel_dismissible(PanelId::from_raw(1), false));

        let tabs = frame_tabs(&frame);

        assert!(!tabs[0].active);
        assert!(!tabs[0].close_visible);
        assert!(tabs[1].active);
        assert!(tabs[1].close_visible);
        assert!(tabs[1].draggable);
    }

    #[test]
    fn tab_drag_starts_only_for_panels_owned_by_the_frame() {
        let area = dock_area();

        assert_eq!(
            area.begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3)),
            Some(super::DockTabDrag {
                source_frame: FrameId::from_raw(2),
                panel: PanelId::from_raw(3),
            })
        );
        assert_eq!(
            area.begin_tab_drag(FrameId::from_raw(1), PanelId::from_raw(3)),
            None
        );
    }

    #[test]
    fn drop_zone_resolution_prefers_edges_over_center() {
        let rect = Rect::new(10.0, 20.0, 200.0, 100.0);

        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(12.0, 70.0)),
            Some(DockDropZone::Left)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(205.0, 70.0)),
            Some(DockDropZone::Right)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 23.0)),
            Some(DockDropZone::Top)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 116.0)),
            Some(DockDropZone::Bottom)
        );
        assert_eq!(
            resolve_frame_drop_zone(rect, Point::new(100.0, 70.0)),
            Some(DockDropZone::Center)
        );
        assert_eq!(resolve_frame_drop_zone(rect, Point::new(0.0, 0.0)), None);
    }

    #[test]
    fn dock_drop_target_resolution_returns_merge_or_split_targets() {
        let area = dock_area();
        let layout = solve_dock_layout(&area, Rect::new(0.0, 0.0, 1000.0, 500.0));
        let new_frame = FrameId::from_raw(9);

        assert_eq!(
            resolve_dock_drop_target(&layout, Point::new(500.0, 250.0), new_frame),
            Some(DockDropTarget::tab(FrameId::from_raw(2)))
        );
        assert_eq!(
            resolve_dock_drop_target(&layout, Point::new(995.0, 250.0), new_frame),
            Some(DockDropTarget::split(
                FrameId::from_raw(2),
                DockPlacement::Right,
                new_frame
            ))
        );
    }

    #[test]
    fn dropping_tab_on_frame_merges_and_selects_panel() {
        let mut area = dock_area();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(area.drop_tab(drag, DockDropTarget::tab(FrameId::from_raw(1))));

        let target = area.frame(FrameId::from_raw(1)).expect("target");
        assert_eq!(target.panels.len(), 2);
        assert_eq!(
            target.active_panel().expect("active").id,
            PanelId::from_raw(3)
        );
        assert_eq!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .len(),
            1
        );
    }

    #[test]
    fn dropping_tab_on_split_edge_inserts_new_frame_and_round_trips() {
        let mut area = dock_area();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");
        let insertion = DockSplitInsertion {
            target_frame: FrameId::from_raw(1),
            placement: DockPlacement::Left,
            new_frame: FrameId::from_raw(9),
            ratio: 0.3,
            min_first: 80.0,
            min_second: 120.0,
        };

        assert!(area.drop_tab(
            drag,
            DockDropTarget::Split {
                frame: insertion.target_frame,
                placement: insertion.placement,
                new_frame: insertion.new_frame,
                ratio: insertion.ratio,
                min_first: insertion.min_first,
                min_second: insertion.min_second,
            }
        ));

        let frames = area.frames();
        assert_eq!(frames.len(), 3);
        assert_eq!(
            area.frame(FrameId::from_raw(9))
                .expect("inserted")
                .active_panel()
                .expect("panel")
                .id,
            PanelId::from_raw(3)
        );
        let restored = DockArea::restore(area.snapshot()).expect("restore");
        assert_eq!(restored.frames().len(), 3);
        assert_eq!(
            restored
                .frame(FrameId::from_raw(9))
                .expect("inserted")
                .active_panel()
                .expect("panel")
                .id,
            PanelId::from_raw(3)
        );
    }

    #[test]
    fn invalid_split_drop_does_not_remove_panel() {
        let mut area = dock_area();
        let drag = area
            .begin_tab_drag(FrameId::from_raw(2), PanelId::from_raw(3))
            .expect("drag");

        assert!(!area.drop_tab(
            drag,
            DockDropTarget::split(
                FrameId::from_raw(99),
                DockPlacement::Left,
                FrameId::from_raw(9)
            )
        ));

        assert!(
            area.frame(FrameId::from_raw(2))
                .expect("source")
                .panels
                .iter()
                .any(|panel| panel.id == PanelId::from_raw(3))
        );
    }

    #[test]
    fn snapshots_round_trip() {
        let area = dock_area();
        let snapshot = area.snapshot();
        let restored = DockArea::restore(snapshot).expect("restore");

        assert_eq!(restored.frames().len(), 2);
    }

    #[test]
    fn invalid_snapshots_are_rejected() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![],
                active: 0,
                dismissible_panels: vec![],
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::EmptyFrame
        );
    }

    #[test]
    fn invalid_snapshot_rejects_invalid_active_panel() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 1,
                dismissible_panels: vec![PanelId::from_raw(1)],
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::InvalidActiveIndex
        );
    }

    #[test]
    fn invalid_snapshot_rejects_unknown_dismissible_panel() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(2)],
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::InvalidDismissiblePanel
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_frame_ids() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicateFrameId
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_panel_ids() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(1, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicatePanelId
        );
    }

    #[test]
    fn invalid_snapshot_rejects_duplicate_dismissible_policy_entries() {
        let snapshot = DockSnapshot {
            root: DockSnapshotNode::Frame {
                id: FrameId::from_raw(1),
                panels: vec![panel(1, "A")],
                active: 0,
                dismissible_panels: vec![PanelId::from_raw(1), PanelId::from_raw(1)],
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::DuplicateDismissiblePanel
        );
    }

    #[test]
    fn invalid_snapshot_rejects_invalid_split_numbers() {
        let invalid_ratio = DockSnapshot {
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: f32::NAN,
                min_first: 0.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };
        assert_eq!(
            DockArea::restore(invalid_ratio).expect_err("error"),
            DockRestoreError::InvalidSplitRatio
        );

        let invalid_minimum = DockSnapshot {
            root: DockSnapshotNode::Split {
                axis: Axis::Horizontal,
                ratio: 0.5,
                min_first: -1.0,
                min_second: 0.0,
                first: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(1),
                    panels: vec![panel(1, "A")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(1)],
                }),
                second: Box::new(DockSnapshotNode::Frame {
                    id: FrameId::from_raw(2),
                    panels: vec![panel(2, "B")],
                    active: 0,
                    dismissible_panels: vec![PanelId::from_raw(2)],
                }),
            },
        };

        assert_eq!(
            DockArea::restore(invalid_minimum).expect_err("error"),
            DockRestoreError::InvalidSplitMinimum
        );
    }
}
