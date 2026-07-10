use std::ops::Range;

use kinetik_ui_core::Rect;

use super::math::{
    finite_coordinate, finite_index_extent, finite_non_negative, finite_positive, finite_sum,
};
use super::{
    ItemId, ListLayout, VirtualWindow, VirtualWindowRequest, clamp_virtual_scroll_offset,
    virtual_content_extent, virtual_max_scroll_offset, virtual_window,
};

/// Visible tree row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeRow {
    /// Visible row index.
    pub row: usize,
    /// Source item index.
    pub item_index: usize,
    /// Item identity.
    pub id: ItemId,
    /// Parent identity, if any.
    pub parent: Option<ItemId>,
    /// Nesting depth.
    pub depth: usize,
    /// Whether this row can expose children.
    pub has_children: bool,
    /// Whether this row is currently expanded.
    pub expanded: bool,
}

/// Rectangle assigned to a visible tree row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeRowRect {
    /// Visible row metadata.
    pub row: TreeRow,
    /// Full row rectangle.
    pub rect: Rect,
    /// Content rectangle after indentation.
    pub content_rect: Rect,
}

/// Tree layout model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TreeLayout {
    /// Row height in logical units.
    pub row_height: f32,
    /// Per-depth indentation in logical units.
    pub indent_width: f32,
}

impl TreeLayout {
    /// Creates a tree layout.
    #[must_use]
    pub const fn new(row_height: f32, indent_width: f32) -> Self {
        Self {
            row_height,
            indent_width,
        }
    }

    /// Returns the sanitized row height, or `None` when rows cannot be laid out.
    #[must_use]
    pub fn effective_row_height(self) -> Option<f32> {
        finite_positive(self.row_height)
    }

    /// Returns the sanitized indentation width.
    #[must_use]
    pub fn effective_indent_width(self) -> f32 {
        finite_non_negative(self.indent_width)
    }

    /// Computes total content height for visible rows.
    #[must_use]
    pub fn content_height(self, rows: usize) -> f32 {
        self.effective_row_height()
            .map_or(0.0, |row_height| virtual_content_extent(rows, row_height))
    }

    /// Computes the maximum vertical scroll offset.
    #[must_use]
    pub fn max_scroll_offset(self, rows: usize, viewport_height: f32) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            virtual_max_scroll_offset(rows, row_height, viewport_height)
        })
    }

    /// Clamps a scroll offset to visible tree bounds.
    #[must_use]
    pub fn clamp_scroll_offset(self, rows: usize, viewport_height: f32, scroll_offset: f32) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            clamp_virtual_scroll_offset(scroll_offset, rows, row_height, viewport_height)
        })
    }

    /// Computes the virtual window for visible tree rows.
    #[must_use]
    pub fn virtual_window(
        self,
        rows: usize,
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> VirtualWindow {
        virtual_window(VirtualWindowRequest {
            item_count: rows,
            scroll_offset,
            viewport_extent: viewport_height,
            item_extent: self.row_height,
            overscan,
        })
    }

    /// Computes the virtualized row range for a visible tree.
    #[must_use]
    pub fn visible_range(
        self,
        rows: usize,
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        self.virtual_window(rows, scroll_offset, viewport_height, overscan)
            .materialized_range
    }

    /// Computes visible row rectangles in viewport coordinates.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects(
        self,
        bounds: Rect,
        rows: &[TreeRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<TreeRowRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let indent_width = self.effective_indent_width();
        let clamped_scroll =
            clamp_virtual_scroll_offset(scroll_offset, rows.len(), row_height, bounds.height);
        let row_bounds = Rect::new(
            finite_coordinate(bounds.x),
            finite_sum(finite_coordinate(bounds.y), -clamped_scroll),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        let list = ListLayout::new(row_height);
        list.row_rects(
            row_bounds,
            rows.len(),
            self.visible_range(rows.len(), clamped_scroll, bounds.height, overscan),
        )
        .into_iter()
        .map(|item| {
            let row = rows[item.index];
            let indent = finite_index_extent(row.depth, indent_width);
            let rect = item.rect;
            TreeRowRect {
                row,
                rect,
                content_rect: Rect::new(
                    finite_sum(rect.x, indent),
                    rect.y,
                    (rect.width - indent).max(0.0),
                    rect.height,
                ),
            }
        })
        .collect()
    }

    /// Computes visible row rectangles in content coordinates.
    ///
    /// Use this variant inside a runtime-owned scroll transform. The scroll
    /// offset selects the materialized range but is not subtracted from emitted
    /// geometry, because the enclosing runtime scope owns that translation.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects_content(
        self,
        bounds: Rect,
        rows: &[TreeRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<TreeRowRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let indent_width = self.effective_indent_width();
        let clamped_scroll =
            clamp_virtual_scroll_offset(scroll_offset, rows.len(), row_height, bounds.height);
        let row_bounds = Rect::new(
            finite_coordinate(bounds.x),
            finite_coordinate(bounds.y),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        let list = ListLayout::new(row_height);
        list.row_rects(
            row_bounds,
            rows.len(),
            self.visible_range(rows.len(), clamped_scroll, bounds.height, overscan),
        )
        .into_iter()
        .map(|item| {
            let row = rows[item.index];
            let indent = finite_index_extent(row.depth, indent_width);
            let rect = item.rect;
            TreeRowRect {
                row,
                rect,
                content_rect: Rect::new(
                    finite_sum(rect.x, indent),
                    rect.y,
                    (rect.width - indent).max(0.0),
                    rect.height,
                ),
            }
        })
        .collect()
    }

    /// Computes visible row rectangles from rows collected for a global range.
    ///
    /// Unlike [`Self::visible_row_rects`], `rows` may be a materialized subset
    /// whose [`TreeRow::row`] values refer to global visible row indices.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects_in_range(
        self,
        bounds: Rect,
        total_rows: usize,
        rows: &[TreeRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<TreeRowRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let indent_width = self.effective_indent_width();
        let clamped_scroll =
            clamp_virtual_scroll_offset(scroll_offset, total_rows, row_height, bounds.height);
        let row_bounds = Rect::new(
            finite_coordinate(bounds.x),
            finite_sum(finite_coordinate(bounds.y), -clamped_scroll),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        let visible_range = self.visible_range(total_rows, clamped_scroll, bounds.height, overscan);
        let list = ListLayout::new(row_height);

        rows.iter()
            .copied()
            .filter(|row| row.row < total_rows && visible_range.contains(&row.row))
            .filter_map(|row| {
                let rect = list.row_rect(row_bounds, row.row)?;
                let indent = finite_index_extent(row.depth, indent_width);
                Some(TreeRowRect {
                    row,
                    rect,
                    content_rect: Rect::new(
                        finite_sum(rect.x, indent),
                        rect.y,
                        (rect.width - indent).max(0.0),
                        rect.height,
                    ),
                })
            })
            .collect()
    }

    /// Computes range-collected row rectangles in content coordinates.
    ///
    /// This is the runtime-scroll counterpart to
    /// [`Self::visible_row_rects_in_range`]. The offset selects the global
    /// materialized range but the enclosing runtime scope owns translation.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects_in_range_content(
        self,
        bounds: Rect,
        total_rows: usize,
        rows: &[TreeRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<TreeRowRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let indent_width = self.effective_indent_width();
        let clamped_scroll =
            clamp_virtual_scroll_offset(scroll_offset, total_rows, row_height, bounds.height);
        let row_bounds = Rect::new(
            finite_coordinate(bounds.x),
            finite_coordinate(bounds.y),
            finite_non_negative(bounds.width),
            finite_non_negative(bounds.height),
        );
        let visible_range = self.visible_range(total_rows, clamped_scroll, bounds.height, overscan);
        let list = ListLayout::new(row_height);

        rows.iter()
            .copied()
            .filter(|row| row.row < total_rows && visible_range.contains(&row.row))
            .filter_map(|row| {
                let rect = list.row_rect(row_bounds, row.row)?;
                let indent = finite_index_extent(row.depth, indent_width);
                Some(TreeRowRect {
                    row,
                    rect,
                    content_rect: Rect::new(
                        finite_sum(rect.x, indent),
                        rect.y,
                        (rect.width - indent).max(0.0),
                        rect.height,
                    ),
                })
            })
            .collect()
    }
}
