use super::{Frame, PanelId};

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
