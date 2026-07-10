use std::collections::BTreeSet;
use std::ops::Range;

use kinetik_ui_core::Rect;

use crate::collections::ItemId;

use super::row::{PropertyGridRow, PropertyGridRowKind};
use super::util::{finite_non_negative, finite_positive};

/// Property-grid structural error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropertyGridError {
    /// More than one row uses the same ID.
    DuplicateRowId {
        /// Duplicated row identity.
        id: ItemId,
    },
}

/// Rectangle assigned to one property-grid row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridRowRect {
    /// Source row index.
    pub index: usize,
    /// Stable row identity.
    pub id: ItemId,
    /// Row kind.
    pub kind: PropertyGridRowKind,
    /// Full row rectangle.
    pub rect: Rect,
    /// Label or section-title rectangle.
    pub label_rect: Rect,
    /// Value/control rectangle.
    pub value_rect: Rect,
}

/// Inspector-style property-grid layout.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PropertyGridLayout {
    /// Regular property row height.
    pub row_height: f32,
    /// Section heading row height.
    pub section_height: f32,
    /// Preferred label column width.
    pub label_width: f32,
    /// Gap between label and value columns.
    pub column_gap: f32,
    /// Per-depth indentation.
    pub indent_width: f32,
}

impl PropertyGridLayout {
    /// Creates a property-grid layout.
    #[must_use]
    pub const fn new(
        row_height: f32,
        section_height: f32,
        label_width: f32,
        column_gap: f32,
        indent_width: f32,
    ) -> Self {
        Self {
            row_height,
            section_height,
            label_width,
            column_gap,
            indent_width,
        }
    }

    /// Returns the sanitized property row height.
    #[must_use]
    pub fn effective_row_height(self) -> Option<f32> {
        finite_positive(self.row_height)
    }

    /// Returns the sanitized section heading height.
    #[must_use]
    pub fn effective_section_height(self) -> Option<f32> {
        finite_positive(self.section_height)
    }

    /// Returns the sanitized label column width.
    #[must_use]
    pub fn effective_label_width(self) -> f32 {
        finite_non_negative(self.label_width)
    }

    /// Returns the sanitized gap between label and value columns.
    #[must_use]
    pub fn effective_column_gap(self) -> f32 {
        finite_non_negative(self.column_gap)
    }

    /// Returns the sanitized per-depth indentation.
    #[must_use]
    pub fn effective_indent_width(self) -> f32 {
        finite_non_negative(self.indent_width)
    }

    /// Validates row identity invariants.
    ///
    /// # Errors
    ///
    /// Returns [`PropertyGridError`] when duplicate row IDs are present.
    pub fn validate_rows(rows: &[PropertyGridRow]) -> Result<(), PropertyGridError> {
        let mut ids = BTreeSet::new();
        for row in rows {
            if !ids.insert(row.id) {
                return Err(PropertyGridError::DuplicateRowId { id: row.id });
            }
        }
        Ok(())
    }

    /// Computes the height for one row kind.
    #[must_use]
    pub fn row_extent(self, kind: PropertyGridRowKind) -> f32 {
        match kind {
            PropertyGridRowKind::Section => self.effective_section_height(),
            PropertyGridRowKind::Property { .. } => self.effective_row_height(),
        }
        .unwrap_or(0.0)
    }

    /// Computes total content height.
    #[must_use]
    pub fn content_height(self, rows: &[PropertyGridRow]) -> f32 {
        rows.iter()
            .map(|row| self.row_extent(row.kind))
            .sum::<f32>()
    }

    /// Computes the maximum vertical scroll offset.
    #[must_use]
    pub fn max_scroll_offset(self, rows: &[PropertyGridRow], viewport_height: f32) -> f32 {
        (self.content_height(rows) - finite_non_negative(viewport_height)).max(0.0)
    }

    /// Clamps a vertical scroll offset to the valid range.
    #[must_use]
    pub fn clamp_scroll_offset(
        self,
        rows: &[PropertyGridRow],
        viewport_height: f32,
        scroll_offset: f32,
    ) -> f32 {
        finite_non_negative(scroll_offset).min(self.max_scroll_offset(rows, viewport_height))
    }

    /// Computes visible row indexes for a viewport.
    #[must_use]
    pub fn visible_range(
        self,
        rows: &[PropertyGridRow],
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        let Some(viewport_height) = finite_positive(viewport_height) else {
            return 0..0;
        };
        if rows.is_empty() {
            return 0..0;
        }
        if self.content_height(rows) <= 0.0 {
            return 0..0;
        }

        let scroll_offset = self.clamp_scroll_offset(rows, viewport_height, scroll_offset);
        let viewport_end = scroll_offset + viewport_height;
        let mut y = 0.0;
        let mut start = None;
        let mut end = rows.len();

        for (index, row) in rows.iter().enumerate() {
            let height = self.row_extent(row.kind);
            let row_end = y + height;
            if start.is_none() && row_end > scroll_offset {
                start = Some(index);
            }
            if y >= viewport_end {
                end = index;
                break;
            }
            y = row_end;
        }

        let start = start.unwrap_or(rows.len()).saturating_sub(overscan);
        let end = end.saturating_add(overscan).min(rows.len());
        start..end
    }

    /// Computes row rectangles in viewport coordinates.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn visible_row_rects(
        self,
        bounds: Rect,
        rows: &[PropertyGridRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<PropertyGridRowRect> {
        let scroll_offset = self.clamp_scroll_offset(rows, bounds.height, scroll_offset);
        let visible = self.visible_range(rows, scroll_offset, bounds.height, overscan);
        let mut y = bounds.y - scroll_offset;
        for row in rows.iter().take(visible.start) {
            y += self.row_extent(row.kind);
        }

        visible
            .map(|index| {
                let row = &rows[index];
                let height = self.row_extent(row.kind);
                let rect = Rect::new(
                    bounds.x,
                    y,
                    finite_non_negative(bounds.width),
                    finite_non_negative(height),
                );
                y += height;
                self.row_rect(index, row, rect)
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
        rows: &[PropertyGridRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<PropertyGridRowRect> {
        let scroll_offset = self.clamp_scroll_offset(rows, bounds.height, scroll_offset);
        let visible = self.visible_range(rows, scroll_offset, bounds.height, overscan);
        let mut y = bounds.y;
        for row in rows.iter().take(visible.start) {
            y += self.row_extent(row.kind);
        }

        visible
            .map(|index| {
                let row = &rows[index];
                let height = self.row_extent(row.kind);
                let rect = Rect::new(
                    bounds.x,
                    y,
                    finite_non_negative(bounds.width),
                    finite_non_negative(height),
                );
                y += height;
                self.row_rect(index, row, rect)
            })
            .collect()
    }

    #[allow(clippy::cast_precision_loss)]
    fn row_rect(self, index: usize, row: &PropertyGridRow, rect: Rect) -> PropertyGridRowRect {
        match row.kind {
            PropertyGridRowKind::Section => PropertyGridRowRect {
                index,
                id: row.id,
                kind: row.kind,
                rect,
                label_rect: rect,
                value_rect: Rect::new(rect.max_x(), rect.y, 0.0, rect.height),
            },
            PropertyGridRowKind::Property { depth } => {
                let indent = depth as f32 * self.effective_indent_width();
                let x = rect.x + indent;
                let available = (rect.width - indent).max(0.0);
                let label_width = self.effective_label_width().min(available);
                let gap = if available > label_width {
                    self.effective_column_gap().min(available - label_width)
                } else {
                    0.0
                };
                let value_x = x + label_width + gap;
                let value_width = (rect.max_x() - value_x).max(0.0);
                PropertyGridRowRect {
                    index,
                    id: row.id,
                    kind: row.kind,
                    rect,
                    label_rect: Rect::new(x, rect.y, label_width, rect.height),
                    value_rect: Rect::new(value_x, rect.y, value_width, rect.height),
                }
            }
        }
    }
}
