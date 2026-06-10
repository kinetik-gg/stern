//! `DockArea`, `Frame`, and `Panel` models for editor layouts.

use kinetik_ui_core::{Axis, Rect};

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
}

/// Passive panel metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Panel {
    /// Panel identity.
    pub id: PanelId,
    /// Display title used by frame tabs.
    pub title: String,
    /// Whether the panel can be dismissed.
    pub dismissible: bool,
}

impl Panel {
    /// Creates a panel.
    #[must_use]
    pub fn new(id: PanelId, title: impl Into<String>) -> Self {
        Self {
            id,
            title: title.into(),
            dismissible: true,
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
}

impl Frame {
    /// Creates a frame with panels.
    #[must_use]
    pub fn new(id: FrameId, panels: Vec<Panel>) -> Self {
        Self {
            id,
            panels,
            active: 0,
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
        let index = self.panels.iter().position(|item| item.id == panel)?;
        let removed = self.panels.remove(index);
        self.active = self.active.min(self.panels.len().saturating_sub(1));
        Some(removed)
    }

    /// Adds a panel at the end.
    pub fn push_panel(&mut self, panel: Panel) {
        self.panels.push(panel);
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
        let Some(panel) = self
            .frame_mut(from)
            .and_then(|frame| frame.remove_panel(panel))
        else {
            return false;
        };
        let Some(target) = self.frame_mut(to) else {
            return false;
        };
        target.push_panel(panel);
        target.active = target.panels.len().saturating_sub(1);
        true
    }

    /// Merges all source frame panels into target frame.
    pub fn merge_frames(&mut self, source: FrameId, target: FrameId) -> bool {
        if source == target {
            return false;
        }
        let Some(source_panels) = self.frame_mut(source).map(|frame| {
            frame.active = 0;
            core::mem::take(&mut frame.panels)
        }) else {
            return false;
        };
        let Some(target_frame) = self.frame_mut(target) else {
            return false;
        };
        target_frame.panels.extend(source_panels);
        target_frame.active = target_frame.panels.len().saturating_sub(1);
        true
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
    /// Returns [`DockRestoreError`] when a frame is empty or its active tab
    /// index does not point at an existing panel.
    pub fn restore(snapshot: DockSnapshot) -> Result<Self, DockRestoreError> {
        validate_snapshot_node(&snapshot.root)?;
        Ok(Self {
            root: restore_node(snapshot.root),
        })
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
            let total = match axis {
                Axis::Horizontal => bounds.width,
                Axis::Vertical => bounds.height,
            };
            let first_size = (total * ratio.clamp(0.0, 1.0))
                .max(*min_first)
                .min(total - *min_second);
            let second_size = (total - first_size).max(0.0);
            let (first_rect, second_rect) = match axis {
                Axis::Horizontal => (
                    Rect::new(bounds.x, bounds.y, first_size, bounds.height),
                    Rect::new(bounds.x + first_size, bounds.y, second_size, bounds.height),
                ),
                Axis::Vertical => (
                    Rect::new(bounds.x, bounds.y, bounds.width, first_size),
                    Rect::new(bounds.x, bounds.y + first_size, bounds.width, second_size),
                ),
            };
            solve_node(first, first_rect, frames);
            solve_node(second, second_rect, frames);
        }
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
            close_visible: panel.dismissible,
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
}

fn snapshot_node(node: &DockNode) -> DockSnapshotNode {
    match node {
        DockNode::Frame(frame) => DockSnapshotNode::Frame {
            id: frame.id,
            panels: frame.panels.clone(),
            active: frame.active,
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
        DockSnapshotNode::Frame { id, panels, active } => {
            DockNode::Frame(Frame { id, panels, active })
        }
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

fn validate_snapshot_node(snapshot: &DockSnapshotNode) -> Result<(), DockRestoreError> {
    match snapshot {
        DockSnapshotNode::Frame { panels, active, .. } => {
            if panels.is_empty() {
                return Err(DockRestoreError::EmptyFrame);
            }
            if *active >= panels.len() {
                return Err(DockRestoreError::InvalidActiveIndex);
            }
            Ok(())
        }
        DockSnapshotNode::Split { first, second, .. } => {
            validate_snapshot_node(first)?;
            validate_snapshot_node(second)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        DockArea, DockNode, DockRestoreError, DockSnapshot, DockSnapshotNode, Frame, FrameId,
        Panel, PanelId, frame_tabs, solve_dock_layout,
    };
    use kinetik_ui_core::{Axis, Rect};

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
        assert!(
            area.frame_mut(FrameId::from_raw(1))
                .expect("source")
                .panels
                .is_empty()
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
    fn frame_tabs_expose_presentation_state() {
        let mut frame = frame(1, vec![panel(1, "A"), panel(2, "B")]);
        frame.select_panel(PanelId::from_raw(2));

        let tabs = frame_tabs(&frame);

        assert!(!tabs[0].active);
        assert!(tabs[1].active);
        assert!(tabs[1].close_visible);
        assert!(tabs[1].draggable);
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
            },
        };

        assert_eq!(
            DockArea::restore(snapshot).expect_err("error"),
            DockRestoreError::EmptyFrame
        );
    }
}
