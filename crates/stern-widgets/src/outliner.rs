//! Data-only outliner contracts for hierarchy navigation.

mod component;

pub use component::{
    OutlinerConfig, OutlinerContextMenuConfig, OutlinerOutput, OutlinerRequest,
    OutlinerRowResponse, OutlinerScene, OutlinerSelectionMode, OutlinerState,
};
pub(crate) use component::{
    OutlinerContextState, OutlinerDragState, background_widget_id, context_overlay_id,
    disclosure_widget_id, drop_widget_id, lock_widget_id, visibility_widget_id,
};

use std::collections::BTreeSet;
use std::ops::Range;

use stern_core::{
    Point, Rect, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, SemanticValue,
    StaticIcon, WidgetId,
};

use crate::{
    CollectionContextTarget, CollectionDragSource, InlineEditBeginRequest, InlineEditEligibility,
    ItemId, Selection, TreeExpansion, TreeItem, TreeLayout, TreeModel, TreeModelError,
    inline_edit_widget_id,
};

/// Optional app-owned resource metadata attached to an outliner item.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlinerResourceMetadata {
    /// Stable resource kind such as `scene`, `collection`, or `material`.
    pub kind: String,
    /// App-owned resource reference or display metadata.
    pub reference: String,
}

impl OutlinerResourceMetadata {
    /// Creates app-owned resource metadata.
    #[must_use]
    pub fn new(kind: impl Into<String>, reference: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            reference: reference.into(),
        }
    }
}

/// App-owned row state and affordance availability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct OutlinerRowFlags {
    /// Row can be selected.
    pub selectable: bool,
    /// Row is disabled and should not emit interaction requests.
    pub disabled: bool,
    /// Row is read-only and cannot emit visibility or lock toggle requests.
    pub read_only: bool,
    /// Row can request inline rename/edit.
    pub renamable: bool,
    /// App-owned visibility state.
    pub visible: bool,
    /// Visibility toggle affordance is available.
    pub visibility_toggle_available: bool,
    /// App-owned lock state.
    pub locked: bool,
    /// Lock toggle affordance is available.
    pub lock_toggle_available: bool,
}

impl OutlinerRowFlags {
    /// Creates neutral row flags for an enabled, selectable row.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            selectable: true,
            disabled: false,
            read_only: false,
            renamable: true,
            visible: true,
            visibility_toggle_available: true,
            locked: false,
            lock_toggle_available: true,
        }
    }

    /// Returns true when the row can request selection.
    #[must_use]
    pub const fn can_request_selection(self) -> bool {
        self.selectable && !self.disabled
    }

    /// Returns true when the row can request a visibility toggle.
    #[must_use]
    pub const fn can_request_visibility_toggle(self) -> bool {
        self.visibility_toggle_available && !self.disabled && !self.read_only
    }

    /// Returns true when the row can request a lock toggle.
    #[must_use]
    pub const fn can_request_lock_toggle(self) -> bool {
        self.lock_toggle_available && !self.disabled && !self.read_only
    }

    /// Returns true when the row can request inline rename/edit.
    #[must_use]
    pub const fn can_request_rename(self) -> bool {
        !self.disabled && !self.read_only && self.renamable
    }

    /// Returns the row's inline edit eligibility.
    #[must_use]
    pub const fn inline_edit_eligibility(self) -> InlineEditEligibility {
        InlineEditEligibility::new(self.disabled, self.read_only, self.renamable)
    }
}

impl Default for OutlinerRowFlags {
    fn default() -> Self {
        Self::new()
    }
}

/// Generic outliner item descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlinerItem {
    /// Stable item identity.
    pub id: ItemId,
    /// Parent item, or `None` for a root row.
    pub parent: Option<ItemId>,
    /// Row label.
    pub label: String,
    /// Optional static icon metadata.
    pub icon: Option<StaticIcon>,
    /// Optional app-owned resource metadata.
    pub resource: Option<OutlinerResourceMetadata>,
    /// Whether the row should expose expansion before child descriptors are loaded.
    pub has_children: bool,
    /// Row state and affordance availability.
    pub flags: OutlinerRowFlags,
}

impl OutlinerItem {
    /// Creates an outliner item.
    #[must_use]
    pub fn new(id: ItemId, label: impl Into<String>) -> Self {
        Self {
            id,
            parent: None,
            label: label.into(),
            icon: None,
            resource: None,
            has_children: false,
            flags: OutlinerRowFlags::default(),
        }
    }

    /// Assigns a parent item.
    #[must_use]
    pub const fn with_parent(mut self, parent: ItemId) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Assigns static icon metadata.
    #[must_use]
    pub fn with_icon(mut self, icon: impl Into<StaticIcon>) -> Self {
        self.icon = Some(icon.into());
        self
    }

    /// Assigns app-owned resource metadata.
    #[must_use]
    pub fn with_resource(mut self, resource: OutlinerResourceMetadata) -> Self {
        self.resource = Some(resource);
        self
    }

    /// Assigns a child affordance hint.
    #[must_use]
    pub const fn with_has_children(mut self, has_children: bool) -> Self {
        self.has_children = has_children;
        self
    }

    /// Assigns row flags.
    #[must_use]
    pub const fn with_flags(mut self, flags: OutlinerRowFlags) -> Self {
        self.flags = flags;
        self
    }

    fn tree_item(&self) -> TreeItem {
        TreeItem {
            id: self.id,
            parent: self.parent,
            has_children: self.has_children,
        }
    }
}

/// Generic outliner model in deterministic presentation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlinerModel {
    items: Vec<OutlinerItem>,
}

impl OutlinerModel {
    /// Creates an outliner model from descriptors in presentation order.
    #[must_use]
    pub fn new(items: impl Into<Vec<OutlinerItem>>) -> Self {
        Self {
            items: items.into(),
        }
    }

    /// Returns all item descriptors in source order.
    #[must_use]
    pub fn items(&self) -> &[OutlinerItem] {
        &self.items
    }

    /// Returns a tree model projection.
    #[must_use]
    pub fn tree_model(&self) -> TreeModel {
        TreeModel::new(
            self.items
                .iter()
                .map(OutlinerItem::tree_item)
                .collect::<Vec<_>>(),
        )
    }

    /// Validates item IDs and parent links.
    ///
    /// # Errors
    ///
    /// Returns [`TreeModelError`] for duplicate IDs, unknown parents, self
    /// parents, or cycles.
    pub fn validate(&self) -> Result<(), TreeModelError> {
        self.tree_model().validate()
    }

    /// Resolves visible rows from expansion state.
    ///
    /// Invalid models return no visible rows. Use [`Self::validate`] when the
    /// caller needs diagnostics.
    #[must_use]
    pub fn visible_rows(&self, expansion: &TreeExpansion) -> Vec<OutlinerRow> {
        self.outliner_rows_from_tree_rows(self.tree_model().visible_rows(expansion))
    }

    /// Counts visible rows from expansion state without materializing row metadata.
    ///
    /// Invalid models return zero visible rows. Use [`Self::validate`] when the
    /// caller needs diagnostics.
    #[must_use]
    pub fn visible_row_count(&self, expansion: &TreeExpansion) -> usize {
        self.tree_model().visible_row_count(expansion)
    }

    /// Resolves visible rows inside a global visible row range.
    ///
    /// Returned rows preserve their global visible row indices instead of
    /// rebasing [`OutlinerRow::row`] to zero.
    ///
    /// Invalid models return no visible rows. Use [`Self::validate`] when the
    /// caller needs diagnostics.
    #[must_use]
    pub fn visible_rows_in_range(
        &self,
        expansion: &TreeExpansion,
        range: Range<usize>,
    ) -> Vec<OutlinerRow> {
        self.outliner_rows_from_tree_rows(self.tree_model().visible_rows_in_range(expansion, range))
    }

    fn outliner_rows_from_tree_rows(&self, rows: Vec<crate::TreeRow>) -> Vec<OutlinerRow> {
        rows.into_iter()
            .filter_map(|row| {
                self.items
                    .get(row.item_index)
                    .map(|item| OutlinerRow::from_item(row, item))
            })
            .collect()
    }

    /// Resolves visible rows after applying an app-owned deterministic filter.
    ///
    /// Ancestors of matched descendants remain visible so tree context is
    /// preserved. Expansion state is not mutated, allowing search and filter
    /// changes to reveal descendants again without losing expansion intent.
    #[must_use]
    pub fn filtered_visible_rows(
        &self,
        expansion: &TreeExpansion,
        mut include: impl FnMut(&OutlinerItem) -> bool,
    ) -> Vec<OutlinerRow> {
        let tree = self.tree_model();
        let rows = tree.filtered_visible_rows(expansion, |tree_item| {
            self.item_by_id(tree_item.id).is_some_and(&mut include)
        });
        rows.into_iter()
            .filter_map(|row| {
                self.items
                    .get(row.item_index)
                    .map(|item| OutlinerRow::from_item(row, item))
            })
            .collect()
    }

    /// Returns visible item IDs in row order.
    #[must_use]
    pub fn visible_item_ids(&self, expansion: &TreeExpansion) -> Vec<ItemId> {
        self.visible_rows(expansion)
            .into_iter()
            .map(|row| row.id)
            .collect()
    }

    /// Returns an item descriptor by stable ID.
    #[must_use]
    pub fn item_by_id(&self, id: ItemId) -> Option<&OutlinerItem> {
        self.items.iter().find(|item| item.id == id)
    }

    /// Creates an inline rename begin request for the active selected item.
    #[must_use]
    pub fn inline_rename_begin_from_selection(
        &self,
        selection: &Selection,
        root: WidgetId,
    ) -> Option<InlineEditBeginRequest> {
        let active = selection.active?;
        if !selection.contains(active) {
            return None;
        }

        let item = self.item_by_id(active)?;
        item.inline_rename_begin_request(root)
    }

    /// Returns expansion state containing only IDs present in a valid model.
    #[must_use]
    pub fn preserved_expansion(&self, expansion: &TreeExpansion) -> TreeExpansion {
        let mut preserved = TreeExpansion::new();
        if self.validate().is_err() {
            return preserved;
        }

        let ids = self
            .items
            .iter()
            .map(|item| item.id)
            .collect::<BTreeSet<_>>();
        for id in expansion.expanded() {
            if ids.contains(&id) {
                preserved.expand(id);
            }
        }
        preserved
    }
}

/// Resolved outliner row metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlinerRow {
    /// Visible row index.
    pub row: usize,
    /// Source item index.
    pub item_index: usize,
    /// Stable item identity.
    pub id: ItemId,
    /// Parent identity, if any.
    pub parent: Option<ItemId>,
    /// Nesting depth.
    pub depth: usize,
    /// Whether this row can expose children.
    pub has_children: bool,
    /// Whether this row is currently expanded.
    pub expanded: bool,
    /// Row label.
    pub label: String,
    /// Optional static icon metadata.
    pub icon: Option<StaticIcon>,
    /// Optional app-owned resource metadata.
    pub resource: Option<OutlinerResourceMetadata>,
    /// Row state and affordance availability.
    pub flags: OutlinerRowFlags,
}

impl OutlinerRow {
    fn from_item(row: crate::TreeRow, item: &OutlinerItem) -> Self {
        Self {
            row: row.row,
            item_index: row.item_index,
            id: row.id,
            parent: row.parent,
            depth: row.depth,
            has_children: row.has_children,
            expanded: row.expanded,
            label: item.label.clone(),
            icon: item.icon,
            resource: item.resource.clone(),
            flags: item.flags,
        }
    }

    /// Creates an expansion request for this row.
    #[must_use]
    pub const fn expansion_request(&self, expanded: bool) -> Option<OutlinerExpansionRequest> {
        if self.has_children && !self.flags.disabled {
            Some(OutlinerExpansionRequest {
                target: self.id,
                expanded,
            })
        } else {
            None
        }
    }

    /// Creates an expand request for this row.
    #[must_use]
    pub const fn expand_request(&self) -> Option<OutlinerExpansionRequest> {
        self.expansion_request(true)
    }

    /// Creates a collapse request for this row.
    #[must_use]
    pub const fn collapse_request(&self) -> Option<OutlinerExpansionRequest> {
        self.expansion_request(false)
    }

    /// Creates a selection request for this row.
    #[must_use]
    pub const fn selection_request(
        &self,
        operation: OutlinerSelectionOperation,
    ) -> Option<OutlinerSelectionRequest> {
        if self.flags.can_request_selection() {
            Some(OutlinerSelectionRequest {
                target: self.id,
                operation,
            })
        } else {
            None
        }
    }

    /// Creates a visibility toggle request for this row.
    #[must_use]
    pub const fn visibility_toggle_request(&self) -> Option<OutlinerVisibilityToggleRequest> {
        if self.flags.can_request_visibility_toggle() {
            Some(OutlinerVisibilityToggleRequest {
                target: self.id,
                visible: self.flags.visible,
            })
        } else {
            None
        }
    }

    /// Creates a lock toggle request for this row.
    #[must_use]
    pub const fn lock_toggle_request(&self) -> Option<OutlinerLockToggleRequest> {
        if self.flags.can_request_lock_toggle() {
            Some(OutlinerLockToggleRequest {
                target: self.id,
                locked: self.flags.locked,
            })
        } else {
            None
        }
    }

    /// Creates an inline rename begin request for this row.
    #[must_use]
    pub fn inline_rename_begin_request(&self, root: WidgetId) -> Option<InlineEditBeginRequest> {
        InlineEditBeginRequest::new(
            self.id,
            self.label.clone(),
            inline_edit_widget_id(root, self.id),
            self.flags.inline_edit_eligibility(),
        )
    }

    /// Creates stable drag source metadata for this row.
    #[must_use]
    pub fn drag_source(&self, selection: &Selection) -> Option<CollectionDragSource> {
        (!self.flags.disabled).then(|| CollectionDragSource::from_selection(self.id, selection))
    }

    /// Creates a context-menu target for this row.
    #[must_use]
    pub fn context_target(&self, selection: &Selection) -> Option<CollectionContextTarget> {
        if self.flags.disabled {
            return None;
        }

        if selection.contains(self.id) {
            CollectionContextTarget::selection(selection.selected())
        } else {
            Some(CollectionContextTarget::item(self.id))
        }
    }
}

impl OutlinerItem {
    /// Creates an inline rename begin request for this item.
    #[must_use]
    pub fn inline_rename_begin_request(&self, root: WidgetId) -> Option<InlineEditBeginRequest> {
        InlineEditBeginRequest::new(
            self.id,
            self.label.clone(),
            inline_edit_widget_id(root, self.id),
            self.flags.inline_edit_eligibility(),
        )
    }
}

/// Selection operation requested by an outliner row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinerSelectionOperation {
    /// Replace selection with the target row.
    Replace,
    /// Toggle the target row in the selection.
    Toggle,
    /// Extend selection to the target row.
    Extend,
}

/// Expansion or collapse request emitted by the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinerExpansionRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Requested expansion state.
    pub expanded: bool,
}

/// Selection request emitted by the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinerSelectionRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Requested selection operation.
    pub operation: OutlinerSelectionOperation,
}

/// Visibility toggle request emitted by the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinerVisibilityToggleRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Current app-owned visibility state.
    pub visible: bool,
}

/// Lock toggle request emitted by the UI layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OutlinerLockToggleRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Current app-owned lock state.
    pub locked: bool,
}

/// Outliner row drop zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinerDropZoneKind {
    /// Drop before the target row.
    Before,
    /// Drop inside the target row.
    Inside,
    /// Drop after the target row.
    After,
}

/// Data-only outliner drop target metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutlinerDropTarget {
    /// Drag source represented by the drop.
    pub source: CollectionDragSource,
    /// Target row.
    pub target: ItemId,
    /// Target row zone.
    pub zone: OutlinerDropZoneKind,
}

/// Row zone hit target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutlinerRowZoneKind {
    /// Disclosure expand/collapse zone.
    Disclosure,
    /// Visibility toggle zone.
    VisibilityToggle,
    /// Lock toggle zone.
    LockToggle,
    /// Label zone.
    Label,
    /// Row context menu target.
    Context,
}

/// Outliner layout contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinerLayout {
    /// Tree row and indentation layout.
    pub tree: TreeLayout,
    /// Disclosure zone width.
    pub disclosure_width: f32,
    /// Visibility toggle zone width.
    pub visibility_toggle_width: f32,
    /// Lock toggle zone width.
    pub lock_toggle_width: f32,
    /// Gap between row zones.
    pub gap: f32,
}

impl OutlinerLayout {
    /// Creates an outliner layout.
    #[must_use]
    pub const fn new(row_height: f32, indent_width: f32) -> Self {
        Self {
            tree: TreeLayout::new(row_height, indent_width),
            disclosure_width: 16.0,
            visibility_toggle_width: 18.0,
            lock_toggle_width: 18.0,
            gap: 4.0,
        }
    }

    /// Computes visible row zones in viewport coordinates.
    #[must_use]
    pub fn visible_row_zones(
        self,
        bounds: Rect,
        rows: &[OutlinerRow],
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<OutlinerRowZones> {
        let tree_rows = rows.iter().map(outliner_tree_row).collect::<Vec<_>>();
        self.tree
            .visible_row_rects(bounds, &tree_rows, scroll_offset, overscan)
            .into_iter()
            .filter_map(|rect| rows.get(rect.row.row).cloned().map(|row| (rect, row)))
            .map(|(rect, row)| self.row_zones(row, rect.rect, rect.content_rect))
            .collect()
    }

    /// Computes visible row zones directly from a model and expansion state.
    ///
    /// This path computes the virtual row window first, then materializes only
    /// the row range needed for layout while preserving global visible row
    /// indices.
    #[must_use]
    pub fn visible_model_row_zones(
        self,
        bounds: Rect,
        model: &OutlinerModel,
        expansion: &TreeExpansion,
        scroll_offset: f32,
        overscan: usize,
    ) -> Vec<OutlinerRowZones> {
        let tree = model.tree_model();
        let total_rows = tree.visible_row_count(expansion);
        let visible_range =
            self.tree
                .visible_range(total_rows, scroll_offset, bounds.height, overscan);
        let tree_rows = tree.visible_rows_in_range(expansion, visible_range);
        let rects = self.tree.visible_row_rects_in_range(
            bounds,
            total_rows,
            &tree_rows,
            scroll_offset,
            overscan,
        );
        let rows = model.outliner_rows_from_tree_rows(tree_rows);

        rects
            .into_iter()
            .zip(rows)
            .map(|(rect, row)| self.row_zones(row, rect.rect, rect.content_rect))
            .collect()
    }

    fn row_zones(self, row: OutlinerRow, rect: Rect, content_rect: Rect) -> OutlinerRowZones {
        let rect = finite_rect(rect);
        let content_rect = finite_rect(content_rect);
        let max_x = content_rect.max_x();
        let gap = finite_non_negative(self.gap);
        let mut cursor = content_rect.x;
        let disclosure_rect = take_zone(&mut cursor, max_x, self.disclosure_width);
        cursor = finite_sum(cursor, gap).min(max_x);
        let visibility_toggle_rect = take_zone(&mut cursor, max_x, self.visibility_toggle_width);
        cursor = finite_sum(cursor, gap).min(max_x);
        let lock_toggle_rect = take_zone(&mut cursor, max_x, self.lock_toggle_width);
        cursor = finite_sum(cursor, gap).min(max_x);
        let label_rect = Rect::new(
            cursor,
            content_rect.y,
            (max_x - cursor).max(0.0),
            content_rect.height,
        );

        OutlinerRowZones {
            row,
            rect,
            disclosure_rect: with_row_y(disclosure_rect, content_rect),
            visibility_toggle_rect: with_row_y(visibility_toggle_rect, content_rect),
            lock_toggle_rect: with_row_y(lock_toggle_rect, content_rect),
            label_rect: finite_rect(label_rect),
            context_rect: rect,
        }
    }
}

/// Rectangles assigned to one outliner row.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinerRowZones {
    /// Resolved row metadata.
    pub row: OutlinerRow,
    /// Full row rectangle.
    pub rect: Rect,
    /// Disclosure zone rectangle.
    pub disclosure_rect: Rect,
    /// Visibility toggle zone rectangle.
    pub visibility_toggle_rect: Rect,
    /// Lock toggle zone rectangle.
    pub lock_toggle_rect: Rect,
    /// Label zone rectangle.
    pub label_rect: Rect,
    /// Context target rectangle.
    pub context_rect: Rect,
}

impl OutlinerRowZones {
    /// Returns the row zone containing a point.
    #[must_use]
    pub fn hit_zone(&self, point: Point) -> Option<OutlinerRowZoneKind> {
        if self.disclosure_rect.contains_point(point) {
            Some(OutlinerRowZoneKind::Disclosure)
        } else if self.visibility_toggle_rect.contains_point(point) {
            Some(OutlinerRowZoneKind::VisibilityToggle)
        } else if self.lock_toggle_rect.contains_point(point) {
            Some(OutlinerRowZoneKind::LockToggle)
        } else if self.label_rect.contains_point(point) {
            Some(OutlinerRowZoneKind::Label)
        } else if self.context_rect.contains_point(point) {
            Some(OutlinerRowZoneKind::Context)
        } else {
            None
        }
    }

    /// Resolves a deterministic drop target for a point over this row.
    #[must_use]
    pub fn drop_target(
        &self,
        point: Point,
        source: &CollectionDragSource,
    ) -> Option<OutlinerDropTarget> {
        if self.row.flags.disabled
            || source.contains(self.row.id)
            || !self.context_rect.contains_point(point)
        {
            return None;
        }

        Some(OutlinerDropTarget {
            source: source.clone(),
            target: self.row.id,
            zone: outliner_drop_zone(self.rect, point),
        })
    }
}

/// Builds semantic list and row nodes for visible outliner rows.
#[must_use]
pub fn outliner_semantics(
    id: WidgetId,
    bounds: Rect,
    rows: &[OutlinerRowZones],
    selection: &Selection,
    label: impl Into<String>,
) -> Vec<SemanticNode> {
    let children = rows
        .iter()
        .map(|row| outliner_row_widget_id(id, row.row.id))
        .collect::<Vec<_>>();
    let mut semantics = vec![
        SemanticNode::new(id, SemanticRole::List, finite_rect(bounds))
            .with_label(label)
            .with_children(children),
    ];
    semantics[0].state.value = Some(SemanticValue::Text(format!("{} rows", rows.len())));

    semantics.extend(rows.iter().map(|zones| {
        let mut node = SemanticNode::new(
            outliner_row_widget_id(id, zones.row.id),
            SemanticRole::ListItem,
            zones.rect,
        )
        .with_label(zones.row.label.clone())
        .focusable(zones.row.flags.can_request_selection());
        node.state.disabled = zones.row.flags.disabled;
        node.state.selected = selection.contains(zones.row.id);
        node.state.expanded = zones.row.has_children.then_some(zones.row.expanded);
        node.state.value = Some(SemanticValue::Text(zones.row.label.clone()));
        if zones.row.flags.can_request_selection() {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Invoke,
                "Select row",
            ));
        }
        if zones.row.has_children && !zones.row.flags.disabled {
            node.actions.push(SemanticAction::new(
                if zones.row.expanded {
                    SemanticActionKind::Close
                } else {
                    SemanticActionKind::Open
                },
                if zones.row.expanded {
                    "Collapse row"
                } else {
                    "Expand row"
                },
            ));
        }
        if zones.row.flags.can_request_visibility_toggle() {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Custom("toggle-visibility".to_owned()),
                "Toggle visibility",
            ));
        }
        if zones.row.flags.can_request_lock_toggle() {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Custom("toggle-lock".to_owned()),
                "Toggle lock",
            ));
        }
        if zones.row.flags.can_request_rename() {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Custom("rename".to_owned()),
                "Rename row",
            ));
        }
        node
    }));

    semantics
}

/// Resolves an item, selection, or background context target at a point.
#[must_use]
pub fn outliner_context_target_at(
    bounds: Rect,
    rows: &[OutlinerRowZones],
    point: Point,
    selection: &Selection,
) -> Option<CollectionContextTarget> {
    let bounds = finite_rect(bounds);
    if !bounds.contains_point(point) {
        return None;
    }

    if let Some(row) = rows
        .iter()
        .find(|row| row.context_rect.contains_point(point))
    {
        return row.row.context_target(selection);
    }

    Some(CollectionContextTarget::background())
}

/// Derives a stable semantic widget ID for an outliner row.
#[must_use]
pub fn outliner_row_widget_id(root: WidgetId, item: ItemId) -> WidgetId {
    root.child(("outliner-row", item.raw()))
}

fn outliner_tree_row(row: &OutlinerRow) -> crate::TreeRow {
    crate::TreeRow {
        row: row.row,
        item_index: row.item_index,
        id: row.id,
        parent: row.parent,
        depth: row.depth,
        has_children: row.has_children,
        expanded: row.expanded,
    }
}

fn outliner_drop_zone(rect: Rect, point: Point) -> OutlinerDropZoneKind {
    let rect = finite_rect(rect);
    let y = finite_coordinate(point.y);
    let top = rect.y + rect.height * 0.25;
    let bottom = rect.y + rect.height * 0.75;

    if y < top {
        OutlinerDropZoneKind::Before
    } else if y >= bottom {
        OutlinerDropZoneKind::After
    } else {
        OutlinerDropZoneKind::Inside
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
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

fn finite_rect(rect: Rect) -> Rect {
    Rect::new(
        finite_coordinate(rect.x),
        finite_coordinate(rect.y),
        finite_non_negative(rect.width),
        finite_non_negative(rect.height),
    )
}

fn take_zone(cursor: &mut f32, max_x: f32, requested_width: f32) -> Rect {
    let x = (*cursor).min(max_x);
    let width = finite_non_negative(requested_width).min((max_x - x).max(0.0));
    *cursor = finite_sum(x, width).min(max_x);
    Rect::new(x, 0.0, width, 0.0)
}

fn with_row_y(rect: Rect, row: Rect) -> Rect {
    finite_rect(Rect::new(rect.x, row.y, rect.width, row.height))
}
