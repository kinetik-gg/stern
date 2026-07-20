mod split;
mod tree;

pub use split::DockSplitInsertion;
pub(crate) use tree::{
    DockSplitTopologyFingerprint, collect_frame_ids, frame_is_valid, split_children_at_path,
};
use tree::{
    collect_frames, find_frame, find_frame_mut, first_valid_frame_id, insert_frame_split,
    prune_empty_frames, resize_split_at_path, resize_split_at_path_with_gutter,
    set_split_ratio_at_path, split_ratio_at_path as node_split_ratio_at_path,
    split_topology_fingerprint_at_path, swap_frame_leaves,
};

use super::{
    Axis, BTreeSet, DEFAULT_SPLIT_MINIMUM, DEFAULT_SPLIT_RATIO, DockChromeStyle,
    DockInteractionPolicy, DockJoinRequest, DockNeighborDirection, DockPlacement, DockRestoreError,
    DockSnapshot, DockSplitPath, DockSwapRequest, FrameId, PanelId, PanelInstanceId,
    PanelInstanceSnapshot, PanelTypeDescriptor, Rect, Vec2, WorkspaceRestoreError,
    WorkspaceSnapshot, frame_neighbor, join_request_matches_layout, restore_node, snapshot_node,
    solve_dock_layout, swap_request_matches_layout, validate_dock_snapshot,
};

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
    pub(crate) const fn placement(self) -> Option<DockPlacement> {
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

    /// Creates a panel from the panel instance vocabulary.
    #[must_use]
    pub fn from_instance_id(id: PanelInstanceId, title: impl Into<String>) -> Self {
        Self::new(PanelId::from_instance_id(id), title)
    }

    /// Returns this panel identity as a panel instance ID.
    #[must_use]
    pub const fn instance_id(&self) -> PanelInstanceId {
        self.id.instance_id()
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
    pub(crate) dismissible_panels: BTreeSet<PanelId>,
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

/// Root dock.
#[derive(Debug, Clone, PartialEq)]
pub struct Dock {
    /// Root dock node.
    pub root: DockNode,
    /// Root-owned active frame identity.
    active_frame: Option<FrameId>,
}

impl Dock {
    /// Creates a dock.
    #[must_use]
    pub fn new(root: DockNode) -> Self {
        let active_frame = first_valid_frame_id(&root);
        Self { root, active_frame }
    }

    /// Returns the root-owned active frame identity.
    #[must_use]
    pub fn active_frame(&self) -> Option<FrameId> {
        self.active_frame
            .filter(|frame| self.frame(*frame).is_some_and(frame_is_valid))
    }

    /// Sets the root-owned active frame when the frame exists.
    pub fn set_active_frame(&mut self, frame: FrameId) -> bool {
        if !self.frame(frame).is_some_and(frame_is_valid) {
            return false;
        }
        self.active_frame = Some(frame);
        true
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
        let selected = self
            .frame_mut(frame)
            .is_some_and(|frame| frame.select_panel(panel));
        if selected {
            self.active_frame = Some(frame);
        }
        selected
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
        self.active_frame = Some(to);
        self.refresh_active_frame();
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
        self.active_frame = Some(target);
        self.refresh_active_frame();
        true
    }

    /// Applies a resolved neighbor join request against the current dock layout.
    pub fn apply_join_request(&mut self, bounds: Rect, request: DockJoinRequest) -> bool {
        self.apply_join_request_with_policy(bounds, request, DockInteractionPolicy::default())
    }

    /// Applies a resolved neighbor join request when policy allows join actions.
    pub fn apply_join_request_with_policy(
        &mut self,
        bounds: Rect,
        request: DockJoinRequest,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_join {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        if !join_request_matches_layout(&layout, request) {
            return false;
        }

        self.merge_frames(request.source_frame(), request.target_frame())
    }

    /// Resolves and applies a neighbor join against the current dock layout.
    pub fn join_neighbor(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
    ) -> bool {
        self.join_neighbor_with_policy(
            bounds,
            source_frame,
            direction,
            DockInteractionPolicy::default(),
        )
    }

    /// Resolves and applies a neighbor join when policy allows join actions.
    pub fn join_neighbor_with_policy(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_join {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        let Some(target_frame) = frame_neighbor(&layout, source_frame, direction) else {
            return false;
        };
        let request = DockJoinRequest {
            source_frame,
            direction,
            target_frame,
        };

        if !join_request_matches_layout(&layout, request) {
            return false;
        }

        self.merge_frames(source_frame, target_frame)
    }

    /// Applies a resolved neighbor swap request against the current dock layout.
    pub fn apply_swap_request(&mut self, bounds: Rect, request: DockSwapRequest) -> bool {
        self.apply_swap_request_with_policy(bounds, request, DockInteractionPolicy::default())
    }

    /// Applies a resolved neighbor swap request when policy allows swap actions.
    pub fn apply_swap_request_with_policy(
        &mut self,
        bounds: Rect,
        request: DockSwapRequest,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_swap {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        if !swap_request_matches_layout(&layout, request) {
            return false;
        }

        swap_frame_leaves(
            &mut self.root,
            request.source_frame(),
            request.target_frame(),
        )
    }

    /// Resolves and applies a neighbor swap against the current dock layout.
    pub fn swap_neighbor(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
    ) -> bool {
        self.swap_neighbor_with_policy(
            bounds,
            source_frame,
            direction,
            DockInteractionPolicy::default(),
        )
    }

    /// Resolves and applies a neighbor swap when policy allows swap actions.
    pub fn swap_neighbor_with_policy(
        &mut self,
        bounds: Rect,
        source_frame: FrameId,
        direction: DockNeighborDirection,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_swap {
            return false;
        }

        let layout = solve_dock_layout(self, bounds);
        let Some(target_frame) = frame_neighbor(&layout, source_frame, direction) else {
            return false;
        };
        let request = DockSwapRequest {
            source_frame,
            direction,
            target_frame,
        };

        if !swap_request_matches_layout(&layout, request) {
            return false;
        }

        swap_frame_leaves(&mut self.root, source_frame, target_frame)
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

        let inserted = insert_frame_split(&mut self.root, insertion, inserted);
        if inserted {
            self.active_frame = Some(insertion.new_frame);
        } else {
            self.refresh_active_frame();
        }
        inserted
    }

    /// Resizes a split addressed by path using a drag delta in logical units.
    pub fn resize_split(&mut self, path: &DockSplitPath, bounds: Rect, delta: Vec2) -> bool {
        self.resize_split_with_policy(path, bounds, delta, DockInteractionPolicy::default())
    }

    /// Resizes a split when policy allows splitter drag resize.
    pub fn resize_split_with_policy(
        &mut self,
        path: &DockSplitPath,
        bounds: Rect,
        delta: Vec2,
        policy: DockInteractionPolicy,
    ) -> bool {
        if !policy.sanitized().splitters.allow_resize {
            return false;
        }

        resize_split_at_path(&mut self.root, path.elements(), bounds, delta)
    }

    pub(crate) fn resize_split_with_policy_and_style(
        &mut self,
        path: &DockSplitPath,
        bounds: Rect,
        delta: Vec2,
        policy: DockInteractionPolicy,
        style: DockChromeStyle,
    ) -> bool {
        if !policy.sanitized().splitters.allow_resize {
            return false;
        }

        resize_split_at_path_with_gutter(
            &mut self.root,
            path.elements(),
            bounds,
            delta,
            style.sanitized().splitter_hit_thickness,
        )
    }

    pub(crate) fn split_ratio_at_path(&self, path: &DockSplitPath) -> Option<f32> {
        node_split_ratio_at_path(&self.root, path.elements())
    }

    pub(crate) fn split_topology_fingerprint_at_path(
        &self,
        path: &DockSplitPath,
    ) -> Option<DockSplitTopologyFingerprint> {
        split_topology_fingerprint_at_path(&self.root, path.elements())
    }

    pub(crate) fn restore_split_ratio_at_path(&mut self, path: &DockSplitPath, ratio: f32) -> bool {
        set_split_ratio_at_path(&mut self.root, path.elements(), ratio)
    }

    /// Creates a snapshot for persistence.
    #[must_use]
    pub fn snapshot(&self) -> DockSnapshot {
        DockSnapshot {
            active_frame: self.active_frame(),
            root: snapshot_node(&self.root),
        }
    }

    /// Creates an additive workspace snapshot shell around the dock snapshot.
    ///
    /// Panel instance records are supplied by the application because the UI
    /// toolkit does not own panel type registration or panel state storage.
    #[must_use]
    pub fn workspace_snapshot(
        &self,
        panel_instances: Vec<PanelInstanceSnapshot>,
    ) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            dock: self.snapshot(),
            panel_instances,
        }
    }

    /// Restores a snapshot after validation.
    ///
    /// # Errors
    ///
    /// Returns [`DockRestoreError`] when persisted dock data is structurally
    /// invalid, contains duplicate identities, or stores invalid split values.
    pub fn restore(snapshot: DockSnapshot) -> Result<Self, DockRestoreError> {
        validate_dock_snapshot(&snapshot)?;
        let root = restore_node(snapshot.root);
        let active_frame = snapshot
            .active_frame
            .or_else(|| first_valid_frame_id(&root));
        Ok(Self { root, active_frame })
    }

    /// Restores a workspace snapshot after validating its panel instance shell.
    ///
    /// # Errors
    ///
    /// Returns [`WorkspaceRestoreError`] when the dock snapshot is invalid, an
    /// instance record is missing or stale, or the supplied panel type
    /// descriptors are not a deterministic set.
    pub fn restore_workspace(
        snapshot: WorkspaceSnapshot,
        descriptors: &[PanelTypeDescriptor],
    ) -> Result<Self, WorkspaceRestoreError> {
        snapshot.restore_dock(descriptors)
    }

    fn refresh_active_frame(&mut self) {
        if self.active_frame().is_none() {
            self.active_frame = first_valid_frame_id(&self.root);
        }
    }
}
