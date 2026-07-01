use super::super::{DEFAULT_SPLIT_MINIMUM, DEFAULT_SPLIT_RATIO, DockPlacement, FrameId};

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
