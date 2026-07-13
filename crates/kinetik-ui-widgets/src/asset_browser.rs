//! Data-only asset browser contracts for app-owned asset collections.

mod component;

pub use component::{
    AssetBrowserConfig, AssetBrowserContextMenuConfig, AssetBrowserItemResponse,
    AssetBrowserOutput, AssetBrowserRenameConflict, AssetBrowserRequest, AssetBrowserScene,
    AssetBrowserSelectionMode, AssetBrowserState,
};
pub(crate) use component::{
    AssetBrowserContextState, AssetBrowserDragState, background_drop_widget_id,
    background_widget_id, context_overlay_id, drop_widget_id,
};

use std::cmp::Ordering;
use std::ops::Range;

use kinetik_ui_core::{
    ImageId, Point, Rect, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    SemanticValue, Size, WidgetId,
};

use crate::{
    CollectionContextTarget, CollectionDragSource, CollectionProjectedItem, CollectionProjection,
    GridLayout, InlineEditBeginRequest, InlineEditEligibility, ItemId, ListLayout, Selection,
    SortDirection, VirtualWindowRequest, inline_edit_widget_id, virtual_window,
};

/// Generic asset browser view mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserViewMode {
    /// Tile assets in a virtualized grid.
    Grid,
    /// Show assets as fixed-height list rows.
    List,
}

/// Asset browser sort key supplied by app-owned asset descriptors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserSortKey {
    /// Keep source collection order.
    Source,
    /// Sort by the asset name field.
    Name,
    /// Sort by the asset kind field.
    Kind,
    /// Sort by the asset tag list.
    Tags,
}

/// Asset browser sort contract.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetBrowserSort {
    /// App-provided key to sort by.
    pub key: AssetBrowserSortKey,
    /// Sort direction.
    pub direction: SortDirection,
}

impl AssetBrowserSort {
    /// Creates an asset sort contract.
    #[must_use]
    pub const fn new(key: AssetBrowserSortKey, direction: SortDirection) -> Self {
        Self { key, direction }
    }
}

/// App-owned fallback metadata used when an asset has no thumbnail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetIconFallback {
    /// Stable fallback kind such as `image`, `material`, or `scene`.
    pub kind: String,
    /// Short display label for the fallback icon surface.
    pub label: String,
}

impl AssetIconFallback {
    /// Creates fallback icon metadata.
    #[must_use]
    pub fn new(kind: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            kind: kind.into(),
            label: label.into(),
        }
    }
}

/// Generic app-provided asset descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetBrowserItem {
    /// Stable application-owned asset identity.
    pub id: ItemId,
    /// User-visible asset name.
    pub name: String,
    /// Stable kind label used for grouping, filtering, or list detail text.
    pub kind: String,
    /// App-owned tags exposed for later filtering.
    pub tags: Vec<String>,
    /// Optional decoded image resource handle for a thumbnail.
    pub thumbnail: Option<ImageId>,
    /// Non-resource fallback metadata used when the thumbnail is absent.
    pub fallback: AssetIconFallback,
    /// Whether this item is presented as unavailable.
    pub disabled: bool,
    /// Whether this item is visible but not mutable.
    pub read_only: bool,
    /// Whether this item exposes inline rename/edit.
    pub renamable: bool,
}

impl AssetBrowserItem {
    /// Creates an asset descriptor.
    #[must_use]
    pub fn new(id: ItemId, name: impl Into<String>, kind: impl Into<String>) -> Self {
        let kind = kind.into();
        Self {
            id,
            name: name.into(),
            tags: Vec::new(),
            thumbnail: None,
            fallback: AssetIconFallback::new(kind.clone(), fallback_label(&kind)),
            kind,
            disabled: false,
            read_only: false,
            renamable: true,
        }
    }

    /// Assigns tags for later filtering.
    #[must_use]
    pub fn with_tags(mut self, tags: impl IntoIterator<Item = impl Into<String>>) -> Self {
        self.tags = tags.into_iter().map(Into::into).collect();
        self
    }

    /// Assigns thumbnail image metadata.
    #[must_use]
    pub const fn with_thumbnail(mut self, thumbnail: ImageId) -> Self {
        self.thumbnail = Some(thumbnail);
        self
    }

    /// Assigns fallback icon metadata.
    #[must_use]
    pub fn with_fallback(mut self, fallback: AssetIconFallback) -> Self {
        self.fallback = fallback;
        self
    }

    /// Assigns disabled presentation state.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Assigns read-only presentation state.
    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    /// Assigns inline rename availability.
    #[must_use]
    pub const fn renamable(mut self, renamable: bool) -> Self {
        self.renamable = renamable;
        self
    }

    /// Creates an inline rename begin request for this asset.
    #[must_use]
    pub fn inline_rename_begin_request(&self, root: WidgetId) -> Option<InlineEditBeginRequest> {
        InlineEditBeginRequest::new(
            self.id,
            self.name.clone(),
            inline_edit_widget_id(root, self.id),
            InlineEditEligibility::new(self.disabled, self.read_only, self.renamable),
        )
    }
}

/// Asset browser model in deterministic presentation order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetBrowserModel {
    items: Vec<AssetBrowserItem>,
}

impl AssetBrowserModel {
    /// Creates an asset browser model from descriptors in presentation order.
    #[must_use]
    pub fn new(items: impl Into<Vec<AssetBrowserItem>>) -> Self {
        Self {
            items: items.into(),
        }
    }

    /// Returns all asset descriptors in presentation order.
    #[must_use]
    pub fn items(&self) -> &[AssetBrowserItem] {
        &self.items
    }

    /// Returns the number of asset descriptors.
    #[must_use]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    /// Returns true when the model contains no descriptors.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// Returns asset IDs in presentation order.
    #[must_use]
    pub fn item_ids(&self) -> Vec<ItemId> {
        self.items.iter().map(|item| item.id).collect()
    }

    /// Returns an identity projection over all asset descriptors.
    #[must_use]
    pub fn projection(&self) -> CollectionProjection {
        CollectionProjection::from_source_ids(&self.item_ids())
    }

    /// Returns a filtered asset projection without mutating source descriptors.
    #[must_use]
    pub fn filtered_projection(
        &self,
        mut include: impl FnMut(&AssetBrowserItem) -> bool,
    ) -> CollectionProjection {
        CollectionProjection::from_items(
            self.items
                .iter()
                .enumerate()
                .filter(|(_, item)| include(item))
                .map(|(source_index, item)| CollectionProjectedItem::new(item.id, source_index)),
        )
    }

    /// Returns a filtered and optionally sorted asset projection.
    #[must_use]
    pub fn projected(
        &self,
        include: impl FnMut(&AssetBrowserItem) -> bool,
        sort: Option<AssetBrowserSort>,
    ) -> CollectionProjection {
        let projection = self.filtered_projection(include);
        if let Some(sort) = sort {
            self.sorted_projection(&projection, sort)
        } else {
            projection
        }
    }

    /// Sorts an existing projection by app-provided asset keys.
    #[must_use]
    pub fn sorted_projection(
        &self,
        projection: &CollectionProjection,
        sort: AssetBrowserSort,
    ) -> CollectionProjection {
        projection.sorted_by(|lhs, rhs| self.compare_projected_assets(lhs, rhs, sort))
    }

    /// Returns an asset descriptor by stable ID.
    #[must_use]
    pub fn item_by_id(&self, id: ItemId) -> Option<&AssetBrowserItem> {
        self.items.iter().find(|item| item.id == id)
    }

    fn compare_projected_assets(
        &self,
        lhs: &CollectionProjectedItem,
        rhs: &CollectionProjectedItem,
        sort: AssetBrowserSort,
    ) -> Ordering {
        let Some(lhs_item) = self.items.get(lhs.source_index) else {
            return lhs.source_index.cmp(&rhs.source_index);
        };
        let Some(rhs_item) = self.items.get(rhs.source_index) else {
            return lhs.source_index.cmp(&rhs.source_index);
        };
        let order = match sort.key {
            AssetBrowserSortKey::Source => lhs.source_index.cmp(&rhs.source_index),
            AssetBrowserSortKey::Name => lhs_item.name.cmp(&rhs_item.name),
            AssetBrowserSortKey::Kind => lhs_item.kind.cmp(&rhs_item.kind),
            AssetBrowserSortKey::Tags => lhs_item.tags.cmp(&rhs_item.tags),
        };

        match sort.direction {
            SortDirection::Ascending => order,
            SortDirection::Descending => order.reverse(),
        }
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

    /// Validates model identity.
    ///
    /// # Errors
    ///
    /// Returns [`AssetBrowserModelError::DuplicateItemId`] when descriptors
    /// reuse the same stable ID.
    pub fn validate(&self) -> Result<(), AssetBrowserModelError> {
        let mut ids = std::collections::BTreeSet::new();
        for item in &self.items {
            if !ids.insert(item.id) {
                return Err(AssetBrowserModelError::DuplicateItemId { id: item.id });
            }
        }
        Ok(())
    }
}

/// Asset browser model validation error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserModelError {
    /// Two asset descriptors use the same stable ID.
    DuplicateItemId {
        /// Duplicated item ID.
        id: ItemId,
    },
}

/// App-owned presentation state resolved for an asset cell or row.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct AssetBrowserItemState {
    /// Item ID is present in the current multi-selection.
    pub selected: bool,
    /// Item ID is the current hover target.
    pub hovered: bool,
    /// Item is unavailable for interaction.
    pub disabled: bool,
    /// Item is visible but not mutable.
    pub read_only: bool,
    /// Item exposes inline rename/edit.
    pub renamable: bool,
}

/// Selection operation requested from an asset browser item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserSelectionOperation {
    /// Replace selection with the target asset.
    Replace,
    /// Toggle the target asset in the selection.
    Toggle,
    /// Extend selection to the target asset.
    Extend,
}

/// Selection request emitted by an asset browser surface.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AssetBrowserSelectionRequest {
    /// Target item ID.
    pub target: ItemId,
    /// Requested selection operation.
    pub operation: AssetBrowserSelectionOperation,
}

/// Resolved asset browser item metadata for a materialized row or tile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetBrowserResolvedItem {
    /// Source item index.
    pub index: usize,
    /// Stable item identity.
    pub id: ItemId,
    /// User-visible asset name.
    pub name: String,
    /// Stable kind label.
    pub kind: String,
    /// App-owned tags.
    pub tags: Vec<String>,
    /// Optional thumbnail resource handle.
    pub thumbnail: Option<ImageId>,
    /// Fallback metadata for assets without thumbnails.
    pub fallback: AssetIconFallback,
    /// Presentation state.
    pub state: AssetBrowserItemState,
}

impl AssetBrowserResolvedItem {
    fn from_item(
        index: usize,
        item: &AssetBrowserItem,
        selection: &Selection,
        hovered: Option<ItemId>,
    ) -> Self {
        Self {
            index,
            id: item.id,
            name: item.name.clone(),
            kind: item.kind.clone(),
            tags: item.tags.clone(),
            thumbnail: item.thumbnail,
            fallback: item.fallback.clone(),
            state: AssetBrowserItemState {
                selected: selection.contains(item.id),
                hovered: hovered == Some(item.id),
                disabled: item.disabled,
                read_only: item.read_only,
                renamable: item.renamable,
            },
        }
    }

    /// Creates a selection request for this asset item.
    #[must_use]
    pub const fn selection_request(
        &self,
        operation: AssetBrowserSelectionOperation,
    ) -> Option<AssetBrowserSelectionRequest> {
        if self.state.disabled {
            None
        } else {
            Some(AssetBrowserSelectionRequest {
                target: self.id,
                operation,
            })
        }
    }

    /// Creates an inline rename begin request for this asset item.
    #[must_use]
    pub fn inline_rename_begin_request(&self, root: WidgetId) -> Option<InlineEditBeginRequest> {
        InlineEditBeginRequest::new(
            self.id,
            self.name.clone(),
            inline_edit_widget_id(root, self.id),
            InlineEditEligibility::new(
                self.state.disabled,
                self.state.read_only,
                self.state.renamable,
            ),
        )
    }

    /// Creates stable drag source metadata for this asset item.
    #[must_use]
    pub fn drag_source(&self, selection: &Selection) -> Option<CollectionDragSource> {
        (!self.state.disabled && !self.state.read_only)
            .then(|| CollectionDragSource::from_selection(self.id, selection))
    }

    /// Creates a context-menu target for this asset item.
    #[must_use]
    pub fn context_target(&self, selection: &Selection) -> Option<CollectionContextTarget> {
        if self.state.disabled {
            return None;
        }

        if selection.contains(self.id) {
            CollectionContextTarget::selection(selection.selected())
        } else {
            Some(CollectionContextTarget::item(self.id))
        }
    }
}

/// Rectangle and metadata assigned to one materialized asset item.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBrowserItemRect {
    /// Resolved item metadata.
    pub item: AssetBrowserResolvedItem,
    /// Full row or tile rectangle in viewport coordinates.
    pub rect: Rect,
    /// Thumbnail or fallback icon rectangle.
    pub preview_rect: Rect,
    /// Name label rectangle.
    pub name_rect: Rect,
    /// Kind label rectangle.
    pub kind_rect: Rect,
}

impl AssetBrowserItemRect {
    /// Resolves this item as a drop target for a drag source.
    #[must_use]
    pub fn drop_target(&self, source: &CollectionDragSource) -> Option<AssetBrowserDropTarget> {
        if self.item.state.disabled || self.item.state.read_only || source.contains(self.item.id) {
            return None;
        }

        Some(AssetBrowserDropTarget {
            source: source.clone(),
            kind: AssetBrowserDropTargetKind::Item {
                target: self.item.id,
            },
        })
    }
}

/// Data-only asset browser drop target metadata.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetBrowserDropTarget {
    /// Drag source represented by the drop.
    pub source: CollectionDragSource,
    /// Target kind resolved by the asset browser surface.
    pub kind: AssetBrowserDropTargetKind,
}

/// Asset browser drop target kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssetBrowserDropTargetKind {
    /// Drop on a materialized asset item.
    Item {
        /// Target item.
        target: ItemId,
    },
    /// Drop on empty grid/list space.
    EmptySpace {
        /// Stable insertion index derived from the resolved layout.
        index: usize,
    },
}

/// Asset browser layout contract.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetBrowserLayout {
    /// Current view mode.
    pub view_mode: AssetBrowserViewMode,
    /// Grid layout used by [`AssetBrowserViewMode::Grid`].
    pub grid: GridLayout,
    /// List layout used by [`AssetBrowserViewMode::List`].
    pub list: ListLayout,
    /// Extra rows to materialize before and after the visible window.
    pub overscan: usize,
}

impl AssetBrowserLayout {
    /// Creates an asset browser layout contract.
    #[must_use]
    pub const fn new(view_mode: AssetBrowserViewMode, grid: GridLayout, list: ListLayout) -> Self {
        Self {
            view_mode,
            grid,
            list,
            overscan: 0,
        }
    }

    /// Assigns overscan rows for materialization.
    #[must_use]
    pub const fn with_overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Resolves item rectangles and presentation metadata.
    #[must_use]
    pub fn resolve(
        self,
        bounds: Rect,
        model: &AssetBrowserModel,
        scroll_offset: f32,
        selection: &Selection,
        hovered: Option<ItemId>,
    ) -> AssetBrowserLayoutResult {
        self.resolve_projected(
            bounds,
            model,
            &model.projection(),
            scroll_offset,
            selection,
            hovered,
        )
    }

    /// Resolves item rectangles and presentation metadata through a projection.
    #[must_use]
    pub fn resolve_projected(
        self,
        bounds: Rect,
        model: &AssetBrowserModel,
        projection: &CollectionProjection,
        scroll_offset: f32,
        selection: &Selection,
        hovered: Option<ItemId>,
    ) -> AssetBrowserLayoutResult {
        let bounds = finite_rect(bounds);
        match self.view_mode {
            AssetBrowserViewMode::Grid => {
                self.resolve_grid(bounds, model, projection, scroll_offset, selection, hovered)
            }
            AssetBrowserViewMode::List => {
                self.resolve_list(bounds, model, projection, scroll_offset, selection, hovered)
            }
        }
    }

    fn resolve_grid(
        self,
        bounds: Rect,
        model: &AssetBrowserModel,
        projection: &CollectionProjection,
        scroll_offset: f32,
        selection: &Selection,
        hovered: Option<ItemId>,
    ) -> AssetBrowserLayoutResult {
        let Some(item_size) = self.grid.effective_item_size() else {
            return AssetBrowserLayoutResult::empty(AssetBrowserViewMode::Grid, bounds);
        };
        let gap = self.grid.effective_gap();
        let columns = self.grid.column_count(bounds);
        let row_count = row_count(projection.len(), columns);
        let row_extent = item_size.height + gap;
        let window = virtual_window(VirtualWindowRequest {
            item_count: row_count,
            scroll_offset,
            viewport_extent: bounds.height,
            item_extent: row_extent,
            overscan: self.overscan,
        });
        let materialized_range =
            item_range_for_rows(&window.materialized_range, columns, projection.len());
        let visible_range = item_range_for_rows(&window.visible_range, columns, projection.len());
        let rect_bounds = Rect::new(
            bounds.x,
            finite_sum(bounds.y, -window.clamped_scroll_offset),
            bounds.width,
            bounds.height,
        );
        let items = self
            .grid
            .item_rects(rect_bounds, projection.len(), materialized_range.clone())
            .into_iter()
            .filter_map(|item_rect| {
                let projected = projection.get(item_rect.index)?;
                model.items.get(projected.source_index).map(|item| {
                    let item = AssetBrowserResolvedItem::from_item(
                        projected.source_index,
                        item,
                        selection,
                        hovered,
                    );
                    grid_item_rects(item, item_rect.rect)
                })
            })
            .collect();

        AssetBrowserLayoutResult {
            view_mode: AssetBrowserViewMode::Grid,
            columns,
            content_size: Size::new(bounds.width, window.content_extent),
            visible_range,
            materialized_range,
            max_scroll_offset: window.max_scroll_offset,
            scroll_offset: window.clamped_scroll_offset,
            items,
        }
    }

    fn resolve_list(
        self,
        bounds: Rect,
        model: &AssetBrowserModel,
        projection: &CollectionProjection,
        scroll_offset: f32,
        selection: &Selection,
        hovered: Option<ItemId>,
    ) -> AssetBrowserLayoutResult {
        let window = self.list.virtual_window(
            projection.len(),
            scroll_offset,
            bounds.height,
            self.overscan,
        );
        let rect_bounds = Rect::new(
            bounds.x,
            finite_sum(bounds.y, -window.clamped_scroll_offset),
            bounds.width,
            bounds.height,
        );
        let items = self
            .list
            .row_rects(
                rect_bounds,
                projection.len(),
                window.materialized_range.clone(),
            )
            .into_iter()
            .filter_map(|item_rect| {
                let projected = projection.get(item_rect.index)?;
                model.items.get(projected.source_index).map(|item| {
                    let item = AssetBrowserResolvedItem::from_item(
                        projected.source_index,
                        item,
                        selection,
                        hovered,
                    );
                    list_item_rects(item, item_rect.rect)
                })
            })
            .collect();

        AssetBrowserLayoutResult {
            view_mode: AssetBrowserViewMode::List,
            columns: 1,
            content_size: Size::new(bounds.width, window.content_extent),
            visible_range: window.visible_range,
            materialized_range: window.materialized_range,
            max_scroll_offset: window.max_scroll_offset,
            scroll_offset: window.clamped_scroll_offset,
            items,
        }
    }
}

/// Resolved asset browser layout output.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBrowserLayoutResult {
    /// View mode used for this result.
    pub view_mode: AssetBrowserViewMode,
    /// Resolved grid column count, or `1` for list mode.
    pub columns: usize,
    /// Total virtual content size.
    pub content_size: Size,
    /// Strict visible item range before overscan.
    pub visible_range: Range<usize>,
    /// Overscanned item range to materialize.
    pub materialized_range: Range<usize>,
    /// Maximum valid scroll offset.
    pub max_scroll_offset: f32,
    /// Sanitized and clamped scroll offset.
    pub scroll_offset: f32,
    /// Materialized item rectangles and metadata.
    pub items: Vec<AssetBrowserItemRect>,
}

impl AssetBrowserLayoutResult {
    fn empty(view_mode: AssetBrowserViewMode, bounds: Rect) -> Self {
        Self {
            view_mode,
            columns: 1,
            content_size: Size::new(finite_non_negative(bounds.width), 0.0),
            visible_range: 0..0,
            materialized_range: 0..0,
            max_scroll_offset: 0.0,
            scroll_offset: 0.0,
            items: Vec::new(),
        }
    }

    /// Returns materialized item IDs in presentation order.
    #[must_use]
    pub fn materialized_item_ids(&self) -> Vec<ItemId> {
        self.items.iter().map(|item| item.item.id).collect()
    }

    /// Resolves a deterministic item or empty-space drop target.
    #[must_use]
    pub fn drop_target_at(
        &self,
        bounds: Rect,
        point: Point,
        source: &CollectionDragSource,
    ) -> Option<AssetBrowserDropTarget> {
        let bounds = finite_rect(bounds);
        if !bounds.contains_point(point) {
            return None;
        }

        if let Some(item) = self
            .items
            .iter()
            .find(|item| item.rect.contains_point(point))
        {
            return item.drop_target(source);
        }

        Some(AssetBrowserDropTarget {
            source: source.clone(),
            kind: AssetBrowserDropTargetKind::EmptySpace {
                index: self.visible_range.end,
            },
        })
    }

    /// Resolves an item or background context target at a point.
    #[must_use]
    pub fn context_target_at(
        &self,
        bounds: Rect,
        point: Point,
        selection: &Selection,
    ) -> Option<CollectionContextTarget> {
        let bounds = finite_rect(bounds);
        if !bounds.contains_point(point) {
            return None;
        }

        if let Some(item) = self
            .items
            .iter()
            .find(|item| item.rect.contains_point(point))
        {
            return item.item.context_target(selection);
        }

        Some(CollectionContextTarget::background())
    }
}

/// Builds semantic collection and item nodes for a resolved asset browser.
#[must_use]
pub fn asset_browser_semantics(
    id: WidgetId,
    bounds: Rect,
    result: &AssetBrowserLayoutResult,
    label: impl Into<String>,
) -> Vec<SemanticNode> {
    let bounds = finite_rect(bounds);
    let visible_items = result
        .items
        .iter()
        .filter(|item| {
            item.rect
                .intersection(bounds)
                .is_some_and(|rect| rect.width > 0.0 && rect.height > 0.0)
        })
        .collect::<Vec<_>>();
    let children = visible_items
        .iter()
        .map(|item| asset_browser_item_widget_id(id, item.item.id))
        .collect::<Vec<_>>();
    let role = match result.view_mode {
        AssetBrowserViewMode::Grid => SemanticRole::Grid,
        AssetBrowserViewMode::List => SemanticRole::List,
    };
    let mut semantics = vec![
        SemanticNode::new(id, role, bounds)
            .with_label(label)
            .with_children(children),
    ];
    semantics[0].state.value = Some(SemanticValue::Text(format!(
        "{} visible items",
        visible_items.len()
    )));

    semantics.extend(visible_items.into_iter().map(|item| {
        let mut node = SemanticNode::new(
            asset_browser_item_widget_id(id, item.item.id),
            SemanticRole::ListItem,
            item.rect,
        )
        .with_label(item.item.name.clone())
        .focusable(!item.item.state.disabled);
        node.description = Some(item.item.kind.clone());
        node.state.disabled = item.item.state.disabled;
        node.state.selected = item.item.state.selected;
        node.state.value = Some(SemanticValue::Text(item.item.name.clone()));
        if !item.item.state.disabled {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Invoke,
                "Select asset",
            ));
        }
        if !item.item.state.disabled && !item.item.state.read_only && item.item.state.renamable {
            node.actions.push(SemanticAction::new(
                SemanticActionKind::Custom("rename".to_owned()),
                "Rename asset",
            ));
        }
        node
    }));

    semantics
}

/// Derives a stable semantic widget ID for an asset browser item.
#[must_use]
pub fn asset_browser_item_widget_id(root: WidgetId, item: ItemId) -> WidgetId {
    root.child(("asset-browser-item", item.raw()))
}

fn grid_item_rects(item: AssetBrowserResolvedItem, rect: Rect) -> AssetBrowserItemRect {
    let rect = finite_rect(rect);
    let padding = 8.0_f32.min(rect.width * 0.25).min(rect.height * 0.25);
    let preview_height = (rect.height * 0.54).max(0.0);
    let preview_rect = finite_rect(Rect::new(
        rect.x + padding,
        rect.y + padding,
        (rect.width - padding * 2.0).max(0.0),
        (preview_height - padding).max(0.0),
    ));
    let name_rect = finite_rect(Rect::new(
        rect.x + padding,
        rect.y + preview_height + padding,
        (rect.width - padding * 2.0).max(0.0),
        16.0_f32.min(rect.height),
    ));
    let kind_rect = finite_rect(Rect::new(
        rect.x + padding,
        finite_sum(name_rect.y, name_rect.height),
        name_rect.width,
        14.0_f32.min(rect.height),
    ));

    AssetBrowserItemRect {
        item,
        rect,
        preview_rect,
        name_rect,
        kind_rect,
    }
}

fn list_item_rects(item: AssetBrowserResolvedItem, rect: Rect) -> AssetBrowserItemRect {
    let rect = finite_rect(rect);
    let padding = 4.0_f32.min(rect.height * 0.25);
    let preview_size = (rect.height - padding * 2.0).max(0.0);
    let preview_rect = finite_rect(Rect::new(
        rect.x + padding,
        rect.y + padding,
        preview_size,
        preview_size,
    ));
    let text_x = finite_sum(preview_rect.x, preview_rect.width + 8.0);
    let text_width = (rect.max_x() - text_x).max(0.0);
    let name_rect = finite_rect(Rect::new(text_x, rect.y + padding, text_width, 16.0));
    let kind_rect = finite_rect(Rect::new(
        text_x,
        finite_sum(name_rect.y, name_rect.height),
        text_width,
        (rect.height - padding - name_rect.height).max(0.0),
    ));

    AssetBrowserItemRect {
        item,
        rect,
        preview_rect,
        name_rect,
        kind_rect,
    }
}

fn fallback_label(kind: &str) -> String {
    kind.chars()
        .filter(char::is_ascii_alphanumeric)
        .take(3)
        .collect::<String>()
        .to_ascii_uppercase()
}

fn row_count(items: usize, columns: usize) -> usize {
    if items == 0 {
        0
    } else {
        items.saturating_add(columns.saturating_sub(1)) / columns.max(1)
    }
}

fn item_range_for_rows(rows: &Range<usize>, columns: usize, count: usize) -> Range<usize> {
    let columns = columns.max(1);
    let start = rows.start.saturating_mul(columns).min(count);
    let end = rows.end.saturating_mul(columns).min(count).max(start);
    start..end
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
