//! Viewport-driven vertical band layout for the standard editor chrome.

use stern_core::{Rect, Theme};

/// Vertical band layout stacking the standard editor-chrome surfaces and the
/// dock content band over one viewport rectangle.
///
/// Band heights are resolved from theme size tokens instead of hardcoded
/// constants (`docs/visual-spec/05-chrome-dock.md` §Vertical bar ladder):
/// menu bar `size.control.md`, toolbar `size.control.lg`, tab strip
/// `size.tab`, status bar `size.control.sm`. The content band absorbs every
/// remaining logical pixel between the tab strip and the status bar, so a
/// layout recomputed from the current frame's viewport tracks every resize
/// exactly — no stale band geometry, no dead space, no overlap.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ChromeBandLayout {
    /// Menu-bar band pinned to the top edge.
    pub menu_bar: Rect,
    /// Toolbar band below the menu bar.
    pub toolbar: Rect,
    /// Tab-strip band below the toolbar.
    pub tab_strip: Rect,
    /// Content (dock) band between the tab strip and the status bar.
    pub content: Rect,
    /// Status-bar band pinned to the bottom edge.
    pub status_bar: Rect,
}

impl ChromeBandLayout {
    /// Computes the band layout for `bounds` from `theme` size tokens.
    ///
    /// When `bounds` is shorter than the full ladder, bands clamp in
    /// stacking order: earlier bands keep their token height while later
    /// bands and the content band shrink toward zero. No band ever extends
    /// past `bounds`, and the five bands always tile `bounds` exactly.
    #[must_use]
    pub fn from_viewport(bounds: Rect, theme: &Theme) -> Self {
        let width = bounds.width.max(0.0);
        let height = bounds.height.max(0.0);
        let menu_height = theme.sizes.control.md.clamp(0.0, height);
        let mut remaining = height - menu_height;
        let toolbar_height = theme.sizes.control.lg.clamp(0.0, remaining);
        remaining -= toolbar_height;
        let tab_height = theme.sizes.tab.clamp(0.0, remaining);
        remaining -= tab_height;
        let status_height = theme.sizes.control.sm.clamp(0.0, remaining);
        let content_height = remaining - status_height;
        let menu_y = bounds.y;
        let toolbar_y = menu_y + menu_height;
        let tab_y = toolbar_y + toolbar_height;
        let content_y = tab_y + tab_height;
        let status_y = content_y + content_height;
        Self {
            menu_bar: Rect::new(bounds.x, menu_y, width, menu_height),
            toolbar: Rect::new(bounds.x, toolbar_y, width, toolbar_height),
            tab_strip: Rect::new(bounds.x, tab_y, width, tab_height),
            content: Rect::new(bounds.x, content_y, width, content_height),
            status_bar: Rect::new(bounds.x, status_y, width, status_height),
        }
    }
}
