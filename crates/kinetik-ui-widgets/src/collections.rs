//! Collection models for lists, grids, tables, trees, virtualization, and selection.

use std::collections::{BTreeMap, BTreeSet};
use std::ops::Range;

use kinetik_ui_core::{Rect, Size};

/// Stable collection item identity.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ItemId(u64);

impl ItemId {
    /// Creates an item ID from raw bits.
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

/// Rectangle assigned to an item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ItemRect {
    /// Item index.
    pub index: usize,
    /// Item rectangle.
    pub rect: Rect,
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn finite_coordinate(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_sum(lhs: f32, rhs: f32) -> f32 {
    let sum = lhs + rhs;
    if sum.is_finite() {
        sum
    } else if sum.is_sign_negative() {
        f32::MIN
    } else {
        f32::MAX
    }
}

#[allow(clippy::cast_precision_loss)]
fn finite_index_extent(index: usize, extent: f32) -> f32 {
    let offset = index as f32 * extent;
    if offset.is_finite() { offset } else { f32::MAX }
}

/// List layout model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ListLayout {
    /// Row height in logical units.
    pub row_height: f32,
}

impl ListLayout {
    /// Creates a list layout.
    #[must_use]
    pub const fn new(row_height: f32) -> Self {
        Self { row_height }
    }

    /// Returns the sanitized row height, or `None` when rows cannot be laid out.
    #[must_use]
    pub fn effective_row_height(self) -> Option<f32> {
        finite_positive(self.row_height)
    }

    /// Computes total content height for the row count.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn content_height(self, rows: usize) -> f32 {
        self.effective_row_height()
            .map_or(0.0, |row_height| virtual_content_extent(rows, row_height))
    }

    /// Computes the maximum vertical scroll offset for this list.
    #[must_use]
    pub fn max_scroll_offset(self, rows: usize, viewport_height: f32) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            virtual_max_scroll_offset(rows, row_height, viewport_height)
        })
    }

    /// Clamps a scroll offset to the valid list range.
    #[must_use]
    pub fn clamp_scroll_offset(self, rows: usize, viewport_height: f32, scroll_offset: f32) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            clamp_virtual_scroll_offset(scroll_offset, rows, row_height, viewport_height)
        })
    }

    /// Computes the virtualized row range for a viewport.
    #[must_use]
    pub fn visible_range(
        self,
        rows: usize,
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        self.effective_row_height().map_or(0..0, |row_height| {
            virtual_range(VirtualRangeRequest {
                item_count: rows,
                scroll_offset,
                viewport_extent: viewport_height,
                item_extent: row_height,
                overscan,
            })
        })
    }

    /// Computes one row rectangle in content coordinates.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn row_rect(self, bounds: Rect, index: usize) -> Option<Rect> {
        let row_height = self.effective_row_height()?;
        Some(Rect::new(
            finite_coordinate(bounds.x),
            finite_sum(
                finite_coordinate(bounds.y),
                finite_index_extent(index, row_height),
            ),
            finite_non_negative(bounds.width),
            row_height,
        ))
    }

    /// Computes row rectangles.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn row_rects(self, bounds: Rect, rows: usize, visible: Range<usize>) -> Vec<ItemRect> {
        visible
            .take_while(|index| *index < rows)
            .filter_map(|index| {
                self.row_rect(bounds, index)
                    .map(|rect| ItemRect { index, rect })
            })
            .collect()
    }

    /// Computes visible row rectangles in viewport coordinates.
    #[must_use]
    pub fn visible_row_rects(
        self,
        bounds: Rect,
        rows: usize,
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<ItemRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let clamped_scroll =
            clamp_virtual_scroll_offset(scroll_offset, rows, row_height, bounds.height);
        self.row_rects(
            Rect::new(
                finite_coordinate(bounds.x),
                finite_sum(finite_coordinate(bounds.y), -clamped_scroll),
                finite_non_negative(bounds.width),
                finite_non_negative(bounds.height),
            ),
            rows,
            self.visible_range(rows, clamped_scroll, bounds.height, overscan),
        )
    }
}

/// Grid column behavior.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum GridColumns {
    /// Fixed column count.
    Fixed(usize),
    /// Adaptive columns based on minimum item width.
    Adaptive {
        /// Minimum item width used to derive column count.
        min_width: f32,
    },
}

/// Grid layout model.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct GridLayout {
    /// Column behavior.
    pub columns: GridColumns,
    /// Item size.
    pub item_size: Size,
    /// Gap between items.
    pub gap: f32,
}

impl GridLayout {
    /// Returns sanitized item size, or `None` when grid items cannot be laid out.
    #[must_use]
    pub fn effective_item_size(self) -> Option<Size> {
        let width = finite_positive(self.item_size.width)?;
        let height = finite_positive(self.item_size.height)?;
        Some(Size::new(width, height))
    }

    /// Returns the sanitized gap between grid items.
    #[must_use]
    pub fn effective_gap(self) -> f32 {
        finite_non_negative(self.gap)
    }

    /// Resolves the number of columns for bounds.
    #[must_use]
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    pub fn column_count(self, bounds: Rect) -> usize {
        match self.columns {
            GridColumns::Fixed(count) => count.max(1),
            GridColumns::Adaptive { min_width } => {
                let gap = self.effective_gap();
                let available = finite_non_negative(bounds.width);
                let item_width = self
                    .effective_item_size()
                    .map_or(1.0, |size| size.width)
                    .max(1.0);
                let min_width = finite_positive(min_width).unwrap_or(item_width);
                ((available + gap) / (min_width + gap)).floor().max(1.0) as usize
            }
        }
    }

    /// Computes grid item rectangles.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn item_rects(self, bounds: Rect, count: usize, visible: Range<usize>) -> Vec<ItemRect> {
        let Some(item_size) = self.effective_item_size() else {
            return Vec::new();
        };
        let gap = self.effective_gap();
        let columns = self.column_count(bounds);
        visible
            .take_while(|index| *index < count)
            .map(|index| {
                let column = index % columns;
                let row = index / columns;
                ItemRect {
                    index,
                    rect: Rect::new(
                        bounds.x + column as f32 * (item_size.width + gap),
                        bounds.y + row as f32 * (item_size.height + gap),
                        item_size.width,
                        item_size.height,
                    ),
                }
            })
            .collect()
    }
}

/// Table column.
#[derive(Debug, Clone, PartialEq)]
pub struct TableColumn {
    /// Column ID.
    pub id: ItemId,
    /// Header label.
    pub header: String,
    /// Column width.
    pub width: f32,
}

/// Sort direction requested by table headers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    /// Ascending sort.
    Ascending,
    /// Descending sort.
    Descending,
}

/// Table sort intent.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TableSort {
    /// Column to sort by.
    pub column: ItemId,
    /// Direction.
    pub direction: SortDirection,
}

/// Rectangle assigned to a table header cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableHeaderRect {
    /// Column index.
    pub column: usize,
    /// Column identity.
    pub column_id: ItemId,
    /// Header rectangle.
    pub rect: Rect,
}

/// Rectangle assigned to a table body cell.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TableCellRect {
    /// Row index.
    pub row: usize,
    /// Column index.
    pub column: usize,
    /// Column identity.
    pub column_id: ItemId,
    /// Flat cell index in row-major order.
    pub index: usize,
    /// Cell rectangle.
    pub rect: Rect,
}

/// Table layout model.
#[derive(Debug, Clone, PartialEq)]
pub struct TableLayout {
    /// Columns.
    pub columns: Vec<TableColumn>,
    /// Header height.
    pub header_height: f32,
    /// Row height.
    pub row_height: f32,
    /// Requested sort.
    pub sort: Option<TableSort>,
}

impl TableLayout {
    /// Returns the sanitized header height.
    #[must_use]
    pub fn effective_header_height(&self) -> f32 {
        finite_non_negative(self.header_height)
    }

    /// Returns the sanitized row height, or `None` when rows cannot be laid out.
    #[must_use]
    pub fn effective_row_height(&self) -> Option<f32> {
        finite_positive(self.row_height)
    }

    /// Returns the sanitized total width of all columns.
    #[must_use]
    pub fn total_width(&self) -> f32 {
        self.columns
            .iter()
            .map(|column| finite_non_negative(column.width))
            .sum()
    }

    /// Computes the total body height for a row count.
    #[must_use]
    pub fn body_height(&self, rows: usize) -> f32 {
        self.effective_row_height()
            .map_or(0.0, |row_height| virtual_content_extent(rows, row_height))
    }

    /// Computes the total table content size for a row count.
    #[must_use]
    pub fn content_size(&self, rows: usize) -> Size {
        Size::new(
            self.total_width(),
            self.effective_header_height() + self.body_height(rows),
        )
    }

    /// Computes the maximum vertical body scroll offset for this table.
    #[must_use]
    pub fn max_scroll_offset(&self, rows: usize, viewport_height: f32) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            let body_viewport =
                finite_non_negative(viewport_height) - self.effective_header_height();
            virtual_max_scroll_offset(rows, row_height, body_viewport)
        })
    }

    /// Clamps a vertical body scroll offset to the valid table range.
    #[must_use]
    pub fn clamp_scroll_offset(
        &self,
        rows: usize,
        viewport_height: f32,
        scroll_offset: f32,
    ) -> f32 {
        self.effective_row_height().map_or(0.0, |row_height| {
            let body_viewport =
                finite_non_negative(viewport_height) - self.effective_header_height();
            clamp_virtual_scroll_offset(scroll_offset, rows, row_height, body_viewport)
        })
    }

    /// Computes the virtualized body row range for a viewport.
    #[must_use]
    pub fn visible_row_range(
        &self,
        rows: usize,
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        self.effective_row_height().map_or(0..0, |row_height| {
            let body_viewport =
                finite_non_negative(viewport_height) - self.effective_header_height();
            virtual_range(VirtualRangeRequest {
                item_count: rows,
                scroll_offset,
                viewport_extent: body_viewport,
                item_extent: row_height,
                overscan,
            })
        })
    }

    /// Computes header cell rectangles.
    #[must_use]
    pub fn header_rects(&self, bounds: Rect) -> Vec<ItemRect> {
        self.header_cells(bounds)
            .into_iter()
            .map(|cell| ItemRect {
                index: cell.column,
                rect: cell.rect,
            })
            .collect()
    }

    /// Computes header cell rectangles with table-specific metadata.
    #[must_use]
    pub fn header_cells(&self, bounds: Rect) -> Vec<TableHeaderRect> {
        let mut x = finite_coordinate(bounds.x);
        let y = finite_coordinate(bounds.y);
        self.columns
            .iter()
            .enumerate()
            .map(|(index, column)| {
                let width = finite_non_negative(column.width);
                let rect = Rect::new(x, y, width, self.effective_header_height());
                x = finite_sum(x, width);
                TableHeaderRect {
                    column: index,
                    column_id: column.id,
                    rect,
                }
            })
            .collect()
    }

    /// Computes visible table cell rectangles.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn cell_rects(&self, bounds: Rect, rows: usize, visible: Range<usize>) -> Vec<ItemRect> {
        self.body_cells(bounds, rows, visible)
            .into_iter()
            .map(|cell| ItemRect {
                index: cell.index,
                rect: cell.rect,
            })
            .collect()
    }

    /// Computes visible table cell rectangles with row and column metadata.
    #[must_use]
    #[allow(clippy::cast_precision_loss)]
    pub fn body_cells(
        &self,
        bounds: Rect,
        rows: usize,
        visible: Range<usize>,
    ) -> Vec<TableCellRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let mut rects = Vec::new();
        for row in visible.take_while(|row| *row < rows) {
            let mut x = finite_coordinate(bounds.x);
            for (column, model) in self.columns.iter().enumerate() {
                let width = finite_non_negative(model.width);
                rects.push(TableCellRect {
                    row,
                    column,
                    column_id: model.id,
                    index: row
                        .saturating_mul(self.columns.len())
                        .saturating_add(column),
                    rect: Rect::new(
                        x,
                        finite_sum(
                            finite_sum(finite_coordinate(bounds.y), self.effective_header_height()),
                            finite_index_extent(row, row_height),
                        ),
                        width,
                        row_height,
                    ),
                });
                x = finite_sum(x, width);
            }
        }
        rects
    }

    /// Computes visible table body cells in viewport coordinates.
    #[must_use]
    pub fn visible_body_cells(
        &self,
        bounds: Rect,
        rows: usize,
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<TableCellRect> {
        let Some(row_height) = self.effective_row_height() else {
            return Vec::new();
        };
        let clamped_scroll =
            self.clamp_scroll_offset(rows, finite_non_negative(bounds.height), scroll_offset);
        self.body_cells(
            Rect::new(
                finite_coordinate(bounds.x),
                finite_sum(finite_coordinate(bounds.y), -clamped_scroll),
                finite_non_negative(bounds.width),
                finite_non_negative(bounds.height),
            ),
            rows,
            virtual_range(VirtualRangeRequest {
                item_count: rows,
                scroll_offset: clamped_scroll,
                viewport_extent: finite_non_negative(bounds.height)
                    - self.effective_header_height(),
                item_extent: row_height,
                overscan,
            }),
        )
    }
}

/// One item in a tree model.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeItem {
    /// Stable item identity.
    pub id: ItemId,
    /// Parent item, or `None` for a root item.
    pub parent: Option<ItemId>,
    /// Whether the item should expose an expansion affordance even before children are loaded.
    pub has_children: bool,
}

/// Structural tree model error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreeModelError {
    /// More than one item uses the same ID.
    DuplicateItemId {
        /// Duplicated item identity.
        id: ItemId,
    },
    /// An item points at itself as parent.
    SelfParent {
        /// Invalid item identity.
        id: ItemId,
    },
    /// An item points at a parent that is not present in the model.
    UnknownParent {
        /// Item carrying the invalid parent reference.
        id: ItemId,
        /// Missing parent identity.
        parent: ItemId,
    },
    /// Parent links contain a cycle.
    Cycle {
        /// First repeated item detected while walking parent links.
        id: ItemId,
    },
}

/// Flat tree model with parent links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeModel {
    items: Vec<TreeItem>,
}

impl TreeModel {
    /// Creates a tree model from items in deterministic presentation order.
    #[must_use]
    pub fn new(items: impl Into<Vec<TreeItem>>) -> Self {
        Self {
            items: items.into(),
        }
    }

    /// Returns all items in source order.
    #[must_use]
    pub fn items(&self) -> &[TreeItem] {
        &self.items
    }

    /// Returns the number of items in the model.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when the model has no items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Validates tree identity and parent-link invariants.
    ///
    /// # Errors
    ///
    /// Returns [`TreeModelError`] for duplicate IDs, unknown parents, self
    /// parents, or cyclic parent links.
    pub fn validate(&self) -> Result<(), TreeModelError> {
        let mut ids = BTreeSet::new();
        for item in &self.items {
            if !ids.insert(item.id) {
                return Err(TreeModelError::DuplicateItemId { id: item.id });
            }
        }

        for item in &self.items {
            if item.parent == Some(item.id) {
                return Err(TreeModelError::SelfParent { id: item.id });
            }
            if let Some(parent) = item.parent
                && !ids.contains(&parent)
            {
                return Err(TreeModelError::UnknownParent {
                    id: item.id,
                    parent,
                });
            }
        }

        let index_by_id = self.index_by_id();
        for item in &self.items {
            let mut visited = BTreeSet::new();
            let mut current = Some(item.id);
            while let Some(id) = current {
                if !visited.insert(id) {
                    return Err(TreeModelError::Cycle { id });
                }
                current = self.items[index_by_id[&id]].parent;
            }
        }

        Ok(())
    }

    /// Returns direct child IDs for a parent in source order.
    #[must_use]
    pub fn child_ids(&self, parent: Option<ItemId>) -> Vec<ItemId> {
        self.items
            .iter()
            .filter(|item| item.parent == parent)
            .map(|item| item.id)
            .collect()
    }

    /// Returns all descendant IDs for an item in source order.
    #[must_use]
    pub fn descendant_ids(&self, item: ItemId) -> Vec<ItemId> {
        let children_by_parent = self.children_by_parent();
        let mut descendants = Vec::new();
        let mut visited = BTreeSet::new();
        collect_descendant_ids(
            item,
            &children_by_parent,
            &self.items,
            &mut visited,
            &mut descendants,
        );
        descendants
    }

    /// Computes visible tree rows from the current expansion state.
    ///
    /// Invalid models return no visible rows; call [`Self::validate`] to
    /// distinguish an empty tree from a malformed one.
    #[must_use]
    pub fn visible_rows(&self, expansion: &TreeExpansion) -> Vec<TreeRow> {
        if self.validate().is_err() {
            return Vec::new();
        }

        let children_by_parent = self.children_by_parent();
        let mut rows = Vec::new();
        let mut visited = BTreeSet::new();
        self.push_visible_children(
            None,
            0,
            expansion,
            &children_by_parent,
            &mut visited,
            &mut rows,
        );
        rows
    }

    fn index_by_id(&self) -> BTreeMap<ItemId, usize> {
        self.items
            .iter()
            .enumerate()
            .map(|(index, item)| (item.id, index))
            .collect()
    }

    fn children_by_parent(&self) -> BTreeMap<Option<ItemId>, Vec<usize>> {
        let mut children = BTreeMap::<Option<ItemId>, Vec<usize>>::new();
        for (index, item) in self.items.iter().enumerate() {
            children.entry(item.parent).or_default().push(index);
        }
        children
    }

    fn push_visible_children(
        &self,
        parent: Option<ItemId>,
        depth: usize,
        expansion: &TreeExpansion,
        children_by_parent: &BTreeMap<Option<ItemId>, Vec<usize>>,
        visited: &mut BTreeSet<ItemId>,
        rows: &mut Vec<TreeRow>,
    ) {
        let Some(children) = children_by_parent.get(&parent) else {
            return;
        };

        for index in children {
            let item = self.items[*index];
            if !visited.insert(item.id) {
                continue;
            }

            let has_loaded_children = children_by_parent.contains_key(&Some(item.id));
            let has_children = item.has_children || has_loaded_children;
            let expanded = has_children && expansion.is_expanded(item.id);
            rows.push(TreeRow {
                row: rows.len(),
                item_index: *index,
                id: item.id,
                parent: item.parent,
                depth,
                has_children,
                expanded,
            });

            if expanded {
                self.push_visible_children(
                    Some(item.id),
                    depth.saturating_add(1),
                    expansion,
                    children_by_parent,
                    visited,
                    rows,
                );
            }
        }
    }
}

/// Retained tree expansion state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TreeExpansion {
    expanded: BTreeSet<ItemId>,
}

impl TreeExpansion {
    /// Creates empty expansion state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when an item is expanded.
    #[must_use]
    pub fn is_expanded(&self, item: ItemId) -> bool {
        self.expanded.contains(&item)
    }

    /// Returns expanded item IDs in sorted order.
    #[must_use]
    pub fn expanded(&self) -> Vec<ItemId> {
        self.expanded.iter().copied().collect()
    }

    /// Expands an item.
    pub fn expand(&mut self, item: ItemId) -> bool {
        self.expanded.insert(item)
    }

    /// Collapses an item.
    pub fn collapse(&mut self, item: ItemId) -> bool {
        self.expanded.remove(&item)
    }

    /// Toggles expansion and returns whether the item is now expanded.
    pub fn toggle(&mut self, item: ItemId) -> bool {
        if self.expanded.remove(&item) {
            false
        } else {
            self.expanded.insert(item);
            true
        }
    }

    /// Collapses an item and any currently expanded descendants.
    pub fn collapse_descendants(&mut self, model: &TreeModel, item: ItemId) -> bool {
        let mut changed = self.collapse(item);
        for descendant in model.descendant_ids(item) {
            changed |= self.collapse(descendant);
        }
        changed
    }

    /// Clears all expansion state.
    pub fn clear(&mut self) {
        self.expanded.clear();
    }
}

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

    /// Computes the virtualized row range for a visible tree.
    #[must_use]
    pub fn visible_range(
        self,
        rows: usize,
        scroll_offset: f32,
        viewport_height: f32,
        overscan: usize,
    ) -> Range<usize> {
        self.effective_row_height().map_or(0..0, |row_height| {
            virtual_range(VirtualRangeRequest {
                item_count: rows,
                scroll_offset,
                viewport_extent: viewport_height,
                item_extent: row_height,
                overscan,
            })
        })
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
}

fn collect_descendant_ids(
    parent: ItemId,
    children_by_parent: &BTreeMap<Option<ItemId>, Vec<usize>>,
    items: &[TreeItem],
    visited: &mut BTreeSet<ItemId>,
    descendants: &mut Vec<ItemId>,
) {
    let Some(children) = children_by_parent.get(&Some(parent)) else {
        return;
    };
    for index in children {
        let child = items[*index].id;
        if !visited.insert(child) {
            continue;
        }
        descendants.push(child);
        collect_descendant_ids(child, children_by_parent, items, visited, descendants);
    }
}

/// Virtualization request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualRangeRequest {
    /// Total item count.
    pub item_count: usize,
    /// Scroll offset in logical units.
    pub scroll_offset: f32,
    /// Viewport extent in logical units.
    pub viewport_extent: f32,
    /// Item extent in logical units.
    pub item_extent: f32,
    /// Extra items before and after the visible range.
    pub overscan: usize,
}

/// Fixed-extent virtualization request.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualWindowRequest {
    /// Total item count.
    pub item_count: usize,
    /// Scroll offset in logical units.
    pub scroll_offset: f32,
    /// Viewport extent in logical units.
    pub viewport_extent: f32,
    /// Item extent in logical units.
    pub item_extent: f32,
    /// Extra items before and after the visible range.
    pub overscan: usize,
}

impl From<VirtualRangeRequest> for VirtualWindowRequest {
    fn from(request: VirtualRangeRequest) -> Self {
        Self {
            item_count: request.item_count,
            scroll_offset: request.scroll_offset,
            viewport_extent: request.viewport_extent,
            item_extent: request.item_extent,
            overscan: request.overscan,
        }
    }
}

/// Fixed-extent virtualization result.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualWindow {
    /// Total content extent in logical units.
    pub content_extent: f32,
    /// Maximum valid scroll offset in logical units.
    pub max_scroll_offset: f32,
    /// Scroll offset clamped to finite valid bounds.
    pub clamped_scroll_offset: f32,
    /// Strict visible item range before overscan.
    pub visible_range: Range<usize>,
    /// Overscanned range to materialize for layout and painting.
    pub materialized_range: Range<usize>,
}

impl VirtualWindow {
    fn empty() -> Self {
        Self {
            content_extent: 0.0,
            max_scroll_offset: 0.0,
            clamped_scroll_offset: 0.0,
            visible_range: 0..0,
            materialized_range: 0..0,
        }
    }
}

/// Computes a fixed-extent virtual window.
#[must_use]
#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub fn virtual_window(request: VirtualWindowRequest) -> VirtualWindow {
    let Some(item_extent) = finite_positive(request.item_extent) else {
        return VirtualWindow::empty();
    };
    let Some(viewport_extent) = finite_positive(request.viewport_extent) else {
        return VirtualWindow::empty();
    };
    if request.item_count == 0 {
        return VirtualWindow::empty();
    }

    let content_extent = virtual_content_extent(request.item_count, item_extent);
    let max_scroll_offset =
        virtual_max_scroll_offset(request.item_count, item_extent, viewport_extent);
    let clamped_scroll_offset = finite_non_negative(request.scroll_offset).min(max_scroll_offset);

    let first = ((clamped_scroll_offset / item_extent).floor() as usize).min(request.item_count);
    let visible_start = first;
    let visible_end =
        (finite_sum(clamped_scroll_offset, viewport_extent) / item_extent).ceil() as usize;
    let visible_end = visible_end.min(request.item_count).max(visible_start);
    let visible_range = visible_start..visible_end;

    let materialized_visible = ((viewport_extent / item_extent).ceil() as usize)
        .saturating_add(1)
        .min(request.item_count);
    let start = first.saturating_sub(request.overscan);
    let end = first
        .saturating_add(materialized_visible)
        .saturating_add(request.overscan)
        .min(request.item_count);
    let materialized_range = start..end;

    VirtualWindow {
        content_extent,
        max_scroll_offset,
        clamped_scroll_offset,
        visible_range,
        materialized_range,
    }
}

/// Computes an overscanned item range for compatibility with existing callers.
#[must_use]
pub fn virtual_range(request: VirtualRangeRequest) -> Range<usize> {
    virtual_window(request.into()).materialized_range
}

/// Computes virtualized content extent for a fixed item extent.
#[must_use]
#[allow(clippy::cast_precision_loss)]
pub fn virtual_content_extent(item_count: usize, item_extent: f32) -> f32 {
    finite_positive(item_extent).map_or(0.0, |item_extent| {
        let extent = item_count as f32 * item_extent;
        if extent.is_finite() { extent } else { f32::MAX }
    })
}

/// Computes the maximum valid scroll offset for virtualized fixed-extent items.
#[must_use]
pub fn virtual_max_scroll_offset(item_count: usize, item_extent: f32, viewport_extent: f32) -> f32 {
    let content_extent = virtual_content_extent(item_count, item_extent);
    (content_extent - finite_non_negative(viewport_extent)).max(0.0)
}

/// Clamps a virtualized scroll offset to finite, valid bounds.
#[must_use]
pub fn clamp_virtual_scroll_offset(
    scroll_offset: f32,
    item_count: usize,
    item_extent: f32,
    viewport_extent: f32,
) -> f32 {
    let scroll_offset = finite_non_negative(scroll_offset);
    scroll_offset.min(virtual_max_scroll_offset(
        item_count,
        item_extent,
        viewport_extent,
    ))
}

/// Shared multi-selection state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Selection {
    selected: BTreeSet<ItemId>,
    /// Active item.
    pub active: Option<ItemId>,
    anchor: Option<ItemId>,
}

impl Selection {
    /// Creates an empty selection.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns true when an item is selected.
    #[must_use]
    pub fn contains(&self, item: ItemId) -> bool {
        self.selected.contains(&item)
    }

    /// Returns selected items in sorted order.
    #[must_use]
    pub fn selected(&self) -> Vec<ItemId> {
        self.selected.iter().copied().collect()
    }

    /// Clears selection.
    pub fn clear(&mut self) {
        self.selected.clear();
        self.active = None;
        self.anchor = None;
    }

    /// Replaces selection with one item.
    pub fn replace(&mut self, item: ItemId) {
        self.selected.clear();
        self.selected.insert(item);
        self.active = Some(item);
        self.anchor = Some(item);
    }

    /// Toggles an item.
    pub fn toggle(&mut self, item: ItemId) {
        if !self.selected.remove(&item) {
            self.selected.insert(item);
        }
        self.active = Some(item);
        self.anchor = Some(item);
    }

    /// Selects a range using the current anchor or the provided end as anchor.
    pub fn select_range(&mut self, ordered_items: &[ItemId], end: ItemId) -> bool {
        let anchor = self.anchor.unwrap_or(end);
        let Some(anchor_index) = ordered_items.iter().position(|item| *item == anchor) else {
            return false;
        };
        let Some(end_index) = ordered_items.iter().position(|item| *item == end) else {
            return false;
        };
        let range = anchor_index.min(end_index)..=anchor_index.max(end_index);
        self.selected.clear();
        self.selected.extend(ordered_items[range].iter().copied());
        self.active = Some(end);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::{
        GridColumns, GridLayout, ItemId, ListLayout, Selection, SortDirection, TableColumn,
        TableLayout, TableSort, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeModelError,
        VirtualRangeRequest, clamp_virtual_scroll_offset, virtual_range,
    };
    use kinetik_ui_core::{Rect, Size};

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_rect_finite(rect: Rect) {
        assert!(rect.x.is_finite(), "rect x must be finite: {rect:?}");
        assert!(rect.y.is_finite(), "rect y must be finite: {rect:?}");
        assert!(
            rect.width.is_finite(),
            "rect width must be finite: {rect:?}"
        );
        assert!(
            rect.height.is_finite(),
            "rect height must be finite: {rect:?}"
        );
        assert!(rect.width >= 0.0, "rect width must be bounded: {rect:?}");
        assert!(rect.height >= 0.0, "rect height must be bounded: {rect:?}");
    }

    #[test]
    fn list_layout_computes_row_rectangles() {
        let rows = ListLayout::new(20.0).row_rects(Rect::new(0.0, 0.0, 100.0, 200.0), 10, 2..5);

        assert_eq!(rows.len(), 3);
        assert!((rows[0].rect.y - 40.0).abs() < f32::EPSILON);
    }

    #[test]
    fn list_layout_exposes_scroll_extent_and_visible_rows() {
        let list = ListLayout::new(10.0);

        assert_approx(list.content_height(100), 1000.0);
        assert_approx(list.max_scroll_offset(100, 30.0), 970.0);
        assert_approx(list.clamp_scroll_offset(100, 30.0, 5000.0), 970.0);
        assert_eq!(list.visible_range(100, 5000.0, 30.0, 0), 97..100);

        let rows = list.visible_row_rects(Rect::new(0.0, 100.0, 200.0, 30.0), 100, 5000.0, 0);
        assert_eq!(rows[0].index, 97);
        assert!((rows[0].rect.y - 100.0).abs() < f32::EPSILON);
    }

    #[test]
    fn list_layout_rejects_invalid_row_height() {
        let list = ListLayout::new(f32::NAN);

        assert_approx(list.content_height(100), 0.0);
        assert_eq!(list.visible_range(100, 0.0, 30.0, 0), 0..0);
        assert!(
            list.row_rects(Rect::new(0.0, 0.0, 200.0, 30.0), 100, 0..3)
                .is_empty()
        );
    }

    #[test]
    fn list_layout_sanitizes_visible_rect_inputs() {
        let list = ListLayout::new(10.0);

        let direct = list.row_rect(Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, 30.0), 2);
        assert_rect_finite(direct.expect("valid row height"));

        let rows = list.visible_row_rects(
            Rect::new(f32::NAN, f32::NEG_INFINITY, f32::INFINITY, 30.0),
            8,
            f32::INFINITY,
            0,
        );
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].index, 0);
        for row in rows {
            assert_rect_finite(row.rect);
        }
    }

    #[test]
    fn grid_layout_supports_fixed_columns() {
        let grid = GridLayout {
            columns: GridColumns::Fixed(2),
            item_size: Size::new(10.0, 10.0),
            gap: 2.0,
        };
        let items = grid.item_rects(Rect::new(0.0, 0.0, 100.0, 100.0), 4, 0..4);

        assert!((items[2].rect.y - 12.0).abs() < f32::EPSILON);
    }

    #[test]
    fn grid_layout_supports_adaptive_columns() {
        let grid = GridLayout {
            columns: GridColumns::Adaptive { min_width: 20.0 },
            item_size: Size::new(20.0, 20.0),
            gap: 5.0,
        };

        assert_eq!(grid.column_count(Rect::new(0.0, 0.0, 75.0, 100.0)), 3);
    }

    #[test]
    fn grid_layout_sanitizes_bad_adaptive_inputs() {
        let grid = GridLayout {
            columns: GridColumns::Adaptive {
                min_width: f32::NAN,
            },
            item_size: Size::new(20.0, 20.0),
            gap: -5.0,
        };

        assert_eq!(grid.column_count(Rect::new(0.0, 0.0, 75.0, 100.0)), 3);

        let invalid_grid = GridLayout {
            columns: GridColumns::Fixed(2),
            item_size: Size::new(0.0, 20.0),
            gap: 4.0,
        };
        assert!(
            invalid_grid
                .item_rects(Rect::new(0.0, 0.0, 100.0, 100.0), 4, 0..4)
                .is_empty()
        );
    }

    #[test]
    fn table_layout_computes_header_and_cell_rectangles() {
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 100.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "Kind".to_owned(),
                    width: 50.0,
                },
            ],
            header_height: 24.0,
            row_height: 18.0,
            sort: Some(TableSort {
                column: ItemId::from_raw(1),
                direction: SortDirection::Ascending,
            }),
        };

        assert_eq!(
            table.header_rects(Rect::new(0.0, 0.0, 200.0, 200.0)).len(),
            2
        );
        assert_eq!(
            table
                .cell_rects(Rect::new(0.0, 0.0, 200.0, 200.0), 2, 0..2)
                .len(),
            4
        );
    }

    #[test]
    fn table_layout_exposes_content_size_scroll_and_cell_metadata() {
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 100.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "State".to_owned(),
                    width: f32::NAN,
                },
            ],
            header_height: 30.0,
            row_height: 20.0,
            sort: None,
        };

        assert_approx(table.total_width(), 100.0);
        let content_size = table.content_size(3);
        assert_approx(content_size.width, 100.0);
        assert_approx(content_size.height, 90.0);
        assert_approx(table.max_scroll_offset(100, 70.0), 1960.0);

        let headers = table.header_cells(Rect::new(10.0, 20.0, 200.0, 70.0));
        assert_eq!(headers[1].column_id, ItemId::from_raw(2));
        assert_approx(headers[1].rect.width, 0.0);

        let cells = table.visible_body_cells(Rect::new(10.0, 20.0, 200.0, 70.0), 100, 5000.0, 0);
        assert_eq!(cells[0].row, 98);
        assert_eq!(cells[0].column, 0);
        assert_eq!(cells[0].column_id, ItemId::from_raw(1));
        assert!((cells[0].rect.y - 50.0).abs() < f32::EPSILON);
    }

    #[test]
    fn table_layout_rejects_invalid_row_height() {
        let table = TableLayout {
            columns: vec![TableColumn {
                id: ItemId::from_raw(1),
                header: "Name".to_owned(),
                width: 100.0,
            }],
            header_height: 24.0,
            row_height: 0.0,
            sort: None,
        };

        assert_approx(table.body_height(10), 0.0);
        assert_eq!(table.visible_row_range(10, 0.0, 100.0, 0), 0..0);
        assert!(
            table
                .body_cells(Rect::new(0.0, 0.0, 200.0, 100.0), 10, 0..3)
                .is_empty()
        );
    }

    #[test]
    fn table_layout_sanitizes_visible_ranges_and_rects() {
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: ItemId::from_raw(1),
                    header: "Name".to_owned(),
                    width: 100.0,
                },
                TableColumn {
                    id: ItemId::from_raw(2),
                    header: "Bad".to_owned(),
                    width: f32::INFINITY,
                },
            ],
            header_height: f32::NAN,
            row_height: 20.0,
            sort: None,
        };

        let headers = table.header_cells(Rect::new(f32::NAN, f32::INFINITY, 200.0, 80.0));
        assert_eq!(headers.len(), 2);
        for header in headers {
            assert_rect_finite(header.rect);
        }

        let cells = table.visible_body_cells(
            Rect::new(f32::NAN, f32::NEG_INFINITY, f32::INFINITY, 60.0),
            10,
            f32::INFINITY,
            0,
        );
        assert_eq!(cells.len(), 8);
        assert_eq!(cells[0].row, 0);
        assert_eq!(cells[0].index, 0);
        for cell in cells {
            assert_rect_finite(cell.rect);
        }
    }

    fn tree_model() -> TreeModel {
        TreeModel::new(vec![
            TreeItem {
                id: ItemId::from_raw(1),
                parent: None,
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(2),
                parent: Some(ItemId::from_raw(1)),
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(3),
                parent: Some(ItemId::from_raw(2)),
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(4),
                parent: None,
                has_children: true,
            },
        ])
    }

    #[test]
    fn tree_model_validates_structure() {
        assert!(tree_model().validate().is_ok());

        let duplicate = TreeModel::new(vec![
            TreeItem {
                id: ItemId::from_raw(1),
                parent: None,
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(1),
                parent: None,
                has_children: false,
            },
        ]);
        assert_eq!(
            duplicate.validate(),
            Err(TreeModelError::DuplicateItemId {
                id: ItemId::from_raw(1)
            })
        );

        let unknown = TreeModel::new(vec![TreeItem {
            id: ItemId::from_raw(2),
            parent: Some(ItemId::from_raw(99)),
            has_children: false,
        }]);
        assert_eq!(
            unknown.validate(),
            Err(TreeModelError::UnknownParent {
                id: ItemId::from_raw(2),
                parent: ItemId::from_raw(99),
            })
        );

        let self_parent = TreeModel::new(vec![TreeItem {
            id: ItemId::from_raw(7),
            parent: Some(ItemId::from_raw(7)),
            has_children: false,
        }]);
        assert_eq!(
            self_parent.validate(),
            Err(TreeModelError::SelfParent {
                id: ItemId::from_raw(7)
            })
        );

        let cycle = TreeModel::new(vec![
            TreeItem {
                id: ItemId::from_raw(1),
                parent: Some(ItemId::from_raw(2)),
                has_children: false,
            },
            TreeItem {
                id: ItemId::from_raw(2),
                parent: Some(ItemId::from_raw(1)),
                has_children: false,
            },
        ]);
        assert_eq!(
            cycle.validate(),
            Err(TreeModelError::Cycle {
                id: ItemId::from_raw(1)
            })
        );
    }

    #[test]
    fn invalid_tree_models_have_empty_visible_rows() {
        let invalid = TreeModel::new(vec![TreeItem {
            id: ItemId::from_raw(1),
            parent: Some(ItemId::from_raw(99)),
            has_children: false,
        }]);

        assert!(invalid.visible_rows(&TreeExpansion::new()).is_empty());
    }

    #[test]
    fn tree_model_flattens_visible_rows_by_expansion() {
        let tree = tree_model();
        let mut expansion = TreeExpansion::new();

        let rows = tree.visible_rows(&expansion);
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![ItemId::from_raw(1), ItemId::from_raw(4)]
        );
        assert!(rows[0].has_children);
        assert!(!rows[0].expanded);
        assert!(rows[1].has_children);

        assert!(expansion.expand(ItemId::from_raw(1)));
        let rows = tree.visible_rows(&expansion);
        assert_eq!(
            rows.iter()
                .map(|row| (row.id, row.depth))
                .collect::<Vec<_>>(),
            vec![
                (ItemId::from_raw(1), 0),
                (ItemId::from_raw(2), 1),
                (ItemId::from_raw(4), 0),
            ]
        );

        assert!(expansion.expand(ItemId::from_raw(2)));
        let rows = tree.visible_rows(&expansion);
        assert_eq!(
            rows.iter()
                .map(|row| (row.id, row.depth))
                .collect::<Vec<_>>(),
            vec![
                (ItemId::from_raw(1), 0),
                (ItemId::from_raw(2), 1),
                (ItemId::from_raw(3), 2),
                (ItemId::from_raw(4), 0),
            ]
        );
    }

    #[test]
    fn tree_expansion_collapses_descendants() {
        let tree = tree_model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(ItemId::from_raw(1));
        expansion.expand(ItemId::from_raw(2));

        assert_eq!(
            tree.descendant_ids(ItemId::from_raw(1)),
            vec![ItemId::from_raw(2), ItemId::from_raw(3)]
        );
        assert!(expansion.collapse_descendants(&tree, ItemId::from_raw(1)));
        assert!(expansion.expanded().is_empty());
    }

    #[test]
    fn tree_expansion_toggle_clear_and_visible_rows_are_deterministic() {
        let tree = tree_model();
        let mut expansion = TreeExpansion::new();

        assert!(expansion.toggle(ItemId::from_raw(2)));
        assert_eq!(expansion.expanded(), vec![ItemId::from_raw(2)]);
        assert!(
            tree.visible_rows(&expansion)
                .iter()
                .all(|row| row.id != ItemId::from_raw(3))
        );

        assert!(expansion.toggle(ItemId::from_raw(1)));
        let rows = tree.visible_rows(&expansion);
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![
                ItemId::from_raw(1),
                ItemId::from_raw(2),
                ItemId::from_raw(3),
                ItemId::from_raw(4)
            ]
        );

        assert!(!expansion.toggle(ItemId::from_raw(2)));
        let rows = tree.visible_rows(&expansion);
        assert_eq!(
            rows.iter().map(|row| row.id).collect::<Vec<_>>(),
            vec![
                ItemId::from_raw(1),
                ItemId::from_raw(2),
                ItemId::from_raw(4)
            ]
        );

        expansion.clear();
        assert!(expansion.expanded().is_empty());
        assert_eq!(
            tree.visible_rows(&expansion)
                .iter()
                .map(|row| row.id)
                .collect::<Vec<_>>(),
            vec![ItemId::from_raw(1), ItemId::from_raw(4)]
        );
    }

    #[test]
    fn tree_layout_virtualizes_indented_visible_rows() {
        let tree = tree_model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(ItemId::from_raw(1));
        expansion.expand(ItemId::from_raw(2));
        let rows = tree.visible_rows(&expansion);
        let layout = TreeLayout::new(20.0, 12.0);

        assert_approx(layout.content_height(rows.len()), 80.0);
        assert_approx(layout.max_scroll_offset(rows.len(), 40.0), 40.0);
        assert_approx(layout.clamp_scroll_offset(rows.len(), 40.0, 500.0), 40.0);
        assert_eq!(layout.visible_range(rows.len(), 20.0, 40.0, 0), 1..4);

        let rects = layout.visible_row_rects(Rect::new(10.0, 100.0, 200.0, 40.0), &rows, 20.0, 0);
        assert_eq!(rects.len(), 3);
        assert_eq!(rects[0].row.id, ItemId::from_raw(2));
        assert_approx(rects[0].rect.y, 100.0);
        assert_approx(rects[0].content_rect.x, 22.0);
        assert_approx(rects[1].content_rect.x, 34.0);
    }

    #[test]
    fn tree_layout_rejects_invalid_row_height_and_sanitizes_indent() {
        let layout = TreeLayout::new(f32::NAN, f32::NAN);
        let rows = tree_model().visible_rows(&TreeExpansion::new());

        assert_approx(layout.content_height(rows.len()), 0.0);
        assert_eq!(layout.visible_range(rows.len(), 0.0, 100.0, 0), 0..0);
        assert!(
            layout
                .visible_row_rects(Rect::new(0.0, 0.0, 100.0, 100.0), &rows, 0.0, 0)
                .is_empty()
        );

        let layout = TreeLayout::new(20.0, -12.0);
        let rects = layout.visible_row_rects(Rect::new(10.0, 20.0, 100.0, 40.0), &rows, 0.0, 0);
        assert_eq!(rects[0].rect, rects[0].content_rect);
    }

    #[test]
    fn tree_layout_sanitizes_visible_row_rects() {
        let tree = tree_model();
        let mut expansion = TreeExpansion::new();
        expansion.expand(ItemId::from_raw(1));
        let rows = tree.visible_rows(&expansion);
        let layout = TreeLayout::new(20.0, 12.0);

        let rects = layout.visible_row_rects(
            Rect::new(f32::NAN, f32::INFINITY, f32::INFINITY, 40.0),
            &rows,
            f32::NEG_INFINITY,
            usize::MAX,
        );
        assert_eq!(rects.len(), rows.len());
        for rect in rects {
            assert_rect_finite(rect.rect);
            assert_rect_finite(rect.content_rect);
        }
    }

    #[test]
    fn virtual_range_applies_overscan_and_bounds() {
        let range = virtual_range(VirtualRangeRequest {
            item_count: 100,
            scroll_offset: 50.0,
            viewport_extent: 40.0,
            item_extent: 10.0,
            overscan: 2,
        });

        assert_eq!(range, 3..12);
    }

    #[test]
    fn virtual_range_clamps_overscrolled_offsets() {
        let range = virtual_range(VirtualRangeRequest {
            item_count: 100,
            scroll_offset: 5000.0,
            viewport_extent: 40.0,
            item_extent: 10.0,
            overscan: 0,
        });

        assert_eq!(range, 96..100);
        assert_approx(clamp_virtual_scroll_offset(5000.0, 100, 10.0, 40.0), 960.0);
    }

    #[test]
    fn virtual_range_clamps_negative_and_extreme_overscan() {
        assert_eq!(
            virtual_range(VirtualRangeRequest {
                item_count: 6,
                scroll_offset: -200.0,
                viewport_extent: 20.0,
                item_extent: 10.0,
                overscan: usize::MAX,
            }),
            0..6
        );
        assert_eq!(
            virtual_range(VirtualRangeRequest {
                item_count: 6,
                scroll_offset: f32::INFINITY,
                viewport_extent: 20.0,
                item_extent: 10.0,
                overscan: 0,
            }),
            0..3
        );
    }

    #[test]
    fn virtual_range_handles_empty_inputs() {
        assert_eq!(
            virtual_range(VirtualRangeRequest {
                item_count: 0,
                scroll_offset: 0.0,
                viewport_extent: 100.0,
                item_extent: 20.0,
                overscan: 1,
            }),
            0..0
        );
        assert_eq!(
            virtual_range(VirtualRangeRequest {
                item_count: 10,
                scroll_offset: 0.0,
                viewport_extent: f32::NAN,
                item_extent: 20.0,
                overscan: 1,
            }),
            0..0
        );
        assert_eq!(
            virtual_range(VirtualRangeRequest {
                item_count: 10,
                scroll_offset: 0.0,
                viewport_extent: 100.0,
                item_extent: f32::NAN,
                overscan: 1,
            }),
            0..0
        );
    }

    #[test]
    fn selection_supports_replace_toggle_clear() {
        let mut selection = Selection::new();
        let one = ItemId::from_raw(1);
        let two = ItemId::from_raw(2);

        selection.replace(one);
        assert!(selection.contains(one));
        assert_eq!(selection.active, Some(one));
        assert_eq!(selection.selected(), vec![one]);
        selection.toggle(two);
        assert_eq!(selection.selected(), vec![one, two]);
        assert_eq!(selection.active, Some(two));
        selection.toggle(one);
        assert!(!selection.contains(one));
        selection.clear();
        assert!(selection.selected().is_empty());
        assert_eq!(selection.active, None);
    }

    #[test]
    fn selection_supports_ranges_from_anchor() {
        let items = [
            ItemId::from_raw(1),
            ItemId::from_raw(2),
            ItemId::from_raw(3),
            ItemId::from_raw(4),
        ];
        let mut selection = Selection::new();

        selection.replace(ItemId::from_raw(2));
        assert!(selection.select_range(&items, ItemId::from_raw(4)));

        assert_eq!(
            selection.selected(),
            vec![
                ItemId::from_raw(2),
                ItemId::from_raw(3),
                ItemId::from_raw(4)
            ]
        );
    }

    #[test]
    fn selection_range_failure_preserves_deterministic_state() {
        let items = [
            ItemId::from_raw(5),
            ItemId::from_raw(3),
            ItemId::from_raw(9),
            ItemId::from_raw(1),
        ];
        let mut selection = Selection::new();

        selection.replace(ItemId::from_raw(3));
        selection.toggle(ItemId::from_raw(1));
        assert_eq!(
            selection.selected(),
            vec![ItemId::from_raw(1), ItemId::from_raw(3)]
        );
        assert!(!selection.select_range(&items, ItemId::from_raw(99)));
        assert_eq!(selection.active, Some(ItemId::from_raw(1)));
        assert_eq!(
            selection.selected(),
            vec![ItemId::from_raw(1), ItemId::from_raw(3)]
        );
    }
}
