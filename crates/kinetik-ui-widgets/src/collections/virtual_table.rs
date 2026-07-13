use std::collections::{BTreeMap, BTreeSet};

use kinetik_ui_core::{
    PointerOrder, PointerTarget, PointerTargetPlan, Rect, Response, ScrollResponse, Size,
    Transform, Vec2, WidgetId, clamp_scroll_offset,
};

use super::{
    CollectionProjectedItem, CollectionProjection, ItemId, TableCellRect, TableColumn,
    TableColumnConstraints, TableHeaderRect, TableLayout, TableSort, VirtualWindow,
};

/// Configuration for one prepared fixed-height virtual table.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTableConfig {
    /// Visible table viewport in logical coordinates.
    pub bounds: Rect,
    /// Header, column, row-height, and current sort state.
    pub layout: TableLayout,
    /// Stable-column width constraints applied to header and body geometry.
    pub column_constraints: BTreeMap<ItemId, TableColumnConstraints>,
    /// Extra body rows materialized before and after the strict visible range.
    pub overscan: usize,
    /// Accessible table name.
    pub label: String,
    /// Whether scroll and sort interaction are disabled.
    pub disabled: bool,
}

impl VirtualTableConfig {
    /// Creates an enabled table with one overscan row.
    #[must_use]
    pub fn new(bounds: Rect, layout: TableLayout) -> Self {
        Self {
            bounds,
            layout,
            column_constraints: BTreeMap::new(),
            overscan: 1,
            label: "Table".to_owned(),
            disabled: false,
        }
    }

    /// Sets the accessible table name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets stable-column width constraints.
    #[must_use]
    pub fn column_constraints(
        mut self,
        constraints: impl IntoIterator<Item = (ItemId, TableColumnConstraints)>,
    ) -> Self {
        self.column_constraints = constraints.into_iter().collect();
        self
    }

    /// Sets the number of body rows materialized around the strict window.
    #[must_use]
    pub const fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Sets whether table interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
}

/// Presentation returned by the callback for one materialized table row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualTableRow {
    /// Cell labels in table-column order. Missing labels render as empty cells.
    pub cells: Vec<String>,
}

impl VirtualTableRow {
    /// Creates one materialized row presentation.
    #[must_use]
    pub fn new(cells: impl IntoIterator<Item = impl Into<String>>) -> Self {
        Self {
            cells: cells.into_iter().map(Into::into).collect(),
        }
    }
}

/// Frozen two-axis table window.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTableWindow {
    /// Retained offset used by current-frame pointer, paint, and semantics.
    pub offset: Vec2,
    /// Strict and materialized vertical body-row ranges.
    pub body: VirtualWindow,
}

/// Header interaction response for one stable column.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualTableHeaderResponse {
    /// Stable column identity.
    pub column: ItemId,
    /// Shared header interaction response.
    pub response: Response,
}

/// Metadata for one materialized body row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VirtualTableMaterializedRow {
    /// Stable row identity.
    pub id: ItemId,
    /// Row index in the current collection projection.
    pub projected_index: usize,
}

/// Output from one [`crate::Ui::virtual_table`] evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTableOutput {
    /// Two-axis scroll behavior result. Applied deltas affect the next frame.
    pub scroll: ScrollResponse,
    /// Frozen current-frame offset and vertical row window.
    pub window: VirtualTableWindow,
    /// Application-owned sort state requested by a header click.
    pub sort_requested: Option<TableSort>,
    /// Header responses in column order.
    pub headers: Vec<VirtualTableHeaderResponse>,
    /// Materialized body rows in projected order.
    pub rows: Vec<VirtualTableMaterializedRow>,
}

/// Prepared fixed-height virtual-table frame.
///
/// Prepare this snapshot before resolving the frame pointer plan, then share
/// it with pointer declaration and [`crate::Ui::virtual_table`].
#[derive(Debug)]
pub struct VirtualTable<'a> {
    root: WidgetId,
    config: VirtualTableConfig,
    projection: &'a CollectionProjection,
    window: VirtualTableWindow,
    headers: Vec<VirtualTableProjectedHeader>,
    rows: Vec<VirtualTableProjectedRow>,
    header_clip: Rect,
    body_clip: Rect,
    content_size: Size,
    total_width: f32,
}

impl<'a> VirtualTable<'a> {
    pub(crate) fn prepare(
        root: WidgetId,
        config: VirtualTableConfig,
        projection: &'a CollectionProjection,
        retained_scroll_offset: Vec2,
    ) -> Option<Self> {
        valid_viewport(config.bounds)?;
        let header_height = valid_header_height(&config)?;
        let row_height = config.layout.effective_row_height()?;
        validate_columns(&config)?;
        let total_width = config
            .layout
            .total_width_with_constraints(&config.column_constraints);
        if !total_width.is_finite() || total_width <= 0.0 {
            return None;
        }

        let content_size = Size::new(
            total_width.max(config.bounds.width),
            (header_height + config.layout.body_height(projection.len())).max(config.bounds.height),
        );
        let offset = clamp_scroll_offset(
            retained_scroll_offset,
            Size::new(config.bounds.width, config.bounds.height),
            content_size,
        );
        let body = config.layout.body_virtual_window(
            projection.len(),
            offset.y,
            config.bounds.height,
            config.overscan,
        );
        let window = VirtualTableWindow { offset, body };
        let header_clip = Rect::new(
            config.bounds.x,
            config.bounds.y,
            config.bounds.width,
            header_height,
        );
        let body_clip = Rect::new(
            config.bounds.x,
            config.bounds.y + header_height,
            config.bounds.width,
            config.bounds.height - header_height,
        );
        let headers = config
            .layout
            .header_cells_with_constraints(config.bounds, &config.column_constraints)
            .into_iter()
            .map(|cell| VirtualTableProjectedHeader {
                id: header_widget_id(root, cell.column_id),
                cell,
            })
            .collect();
        let cells = config.layout.body_cells_with_constraints(
            config.bounds,
            projection.len(),
            window.body.materialized_range.clone(),
            &config.column_constraints,
        );
        let rows = window
            .body
            .materialized_range
            .clone()
            .filter_map(|projected_index| {
                let item = projection.get(projected_index)?;
                let row_cells = cells
                    .iter()
                    .copied()
                    .filter(|cell| cell.row == projected_index)
                    .map(|cell| VirtualTableProjectedCell {
                        id: cell_widget_id(root, item.id, cell.column_id),
                        cell,
                    })
                    .collect::<Vec<_>>();
                let row_y = row_cells.first()?.cell.rect.y;
                let rect = Rect::new(config.bounds.x, row_y, total_width, row_height);
                Some(VirtualTableProjectedRow {
                    id: row_widget_id(root, item.id),
                    item,
                    projected_index,
                    rect,
                    cells: row_cells,
                })
            })
            .collect();

        Some(Self {
            root,
            config,
            projection,
            window,
            headers,
            rows,
            header_clip,
            body_clip,
            content_size,
            total_width,
        })
    }

    /// Returns the stable table surface and scroll-owner ID.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.root
    }

    /// Returns the stable semantic header-row ID.
    #[must_use]
    pub fn header_row_widget_id(&self) -> WidgetId {
        self.root.child("virtual-table-header-row")
    }

    /// Returns a stable header ID derived from column identity.
    #[must_use]
    pub fn header_widget_id(&self, column: ItemId) -> WidgetId {
        header_widget_id(self.root, column)
    }

    /// Returns a stable body-row ID derived from row identity.
    #[must_use]
    pub fn row_widget_id(&self, row: ItemId) -> WidgetId {
        row_widget_id(self.root, row)
    }

    /// Returns a stable cell ID derived from row and column identities.
    #[must_use]
    pub fn cell_widget_id(&self, row: ItemId, column: ItemId) -> WidgetId {
        cell_widget_id(self.root, row, column)
    }

    /// Returns the frozen current-frame two-axis window.
    #[must_use]
    pub const fn window(&self) -> &VirtualTableWindow {
        &self.window
    }

    /// Adds the table blocker, wheel owner, and horizontally transformed header
    /// targets to one caller-owned pointer plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let mut ordinal = first_order.raw();
        plan.blocker(self.config.bounds, take_order(&mut ordinal));
        plan.target(PointerTarget::wheel_only(
            self.root,
            self.config.bounds,
            take_order(&mut ordinal),
        ));
        plan.with_clip(self.header_clip, |plan| {
            plan.with_transform(
                Transform::translation(Vec2::new(-self.window.offset.x, 0.0)),
                |plan| {
                    if !self.config.disabled {
                        for header in &self.headers {
                            plan.target(PointerTarget::new(
                                header.id,
                                header.cell.rect,
                                take_order(&mut ordinal),
                            ));
                        }
                    }
                },
            );
        });
        PointerOrder::new(ordinal)
    }

    pub(crate) const fn config(&self) -> &VirtualTableConfig {
        &self.config
    }

    /// Returns the frozen row projection used by this table frame.
    #[must_use]
    pub const fn projection(&self) -> &'a CollectionProjection {
        self.projection
    }

    pub(crate) fn headers(&self) -> &[VirtualTableProjectedHeader] {
        &self.headers
    }

    pub(crate) fn rows(&self) -> &[VirtualTableProjectedRow] {
        &self.rows
    }

    pub(crate) const fn header_clip(&self) -> Rect {
        self.header_clip
    }

    pub(crate) const fn body_clip(&self) -> Rect {
        self.body_clip
    }

    pub(crate) const fn content_size(&self) -> Size {
        self.content_size
    }

    pub(crate) const fn total_width(&self) -> f32 {
        self.total_width
    }

    pub(crate) fn header_is_visible(&self, header: &VirtualTableProjectedHeader) -> bool {
        translated_rect(header.cell.rect, -self.window.offset.x, 0.0)
            .intersection(self.header_clip)
            .is_some()
    }

    pub(crate) fn row_is_visible(&self, row: &VirtualTableProjectedRow) -> bool {
        translated_rect(row.rect, -self.window.offset.x, -self.window.offset.y)
            .intersection(self.body_clip)
            .is_some()
    }

    pub(crate) fn cell_is_visible(&self, cell: &VirtualTableProjectedCell) -> bool {
        translated_rect(cell.cell.rect, -self.window.offset.x, -self.window.offset.y)
            .intersection(self.body_clip)
            .is_some()
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VirtualTableProjectedHeader {
    pub(crate) id: WidgetId,
    pub(crate) cell: TableHeaderRect,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VirtualTableProjectedRow {
    pub(crate) id: WidgetId,
    pub(crate) item: CollectionProjectedItem,
    pub(crate) projected_index: usize,
    pub(crate) rect: Rect,
    pub(crate) cells: Vec<VirtualTableProjectedCell>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VirtualTableProjectedCell {
    pub(crate) id: WidgetId,
    pub(crate) cell: TableCellRect,
}

fn valid_viewport(rect: Rect) -> Option<Rect> {
    (rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
        && rect.max_x().is_finite()
        && rect.max_y().is_finite())
    .then_some(rect)
}

fn valid_header_height(config: &VirtualTableConfig) -> Option<f32> {
    let header = config.layout.effective_header_height();
    (header.is_finite() && header > 0.0 && header < config.bounds.height).then_some(header)
}

fn validate_columns(config: &VirtualTableConfig) -> Option<()> {
    if config.layout.columns.is_empty() {
        return None;
    }
    let mut ids = BTreeSet::new();
    for column in &config.layout.columns {
        if !ids.insert(column.id) || effective_column_width(config, column) <= 0.0 {
            return None;
        }
    }
    Some(())
}

fn effective_column_width(config: &VirtualTableConfig, column: &TableColumn) -> f32 {
    column.clamped_width(
        config
            .column_constraints
            .get(&column.id)
            .copied()
            .unwrap_or_default(),
    )
}

fn translated_rect(rect: Rect, x: f32, y: f32) -> Rect {
    Rect::new(rect.x + x, rect.y + y, rect.width, rect.height)
}

fn header_widget_id(root: WidgetId, column: ItemId) -> WidgetId {
    root.child(("virtual-table-header", column.raw()))
}

fn row_widget_id(root: WidgetId, row: ItemId) -> WidgetId {
    root.child(("virtual-table-row", row.raw()))
}

fn cell_widget_id(root: WidgetId, row: ItemId, column: ItemId) -> WidgetId {
    root.child(("virtual-table-cell", row.raw(), column.raw()))
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
