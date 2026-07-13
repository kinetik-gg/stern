//! Prepared public asset-browser composition contracts.

use std::ops::Range;

use kinetik_ui_core::{
    PointerOrder, PointerTarget, PointerTargetPlan, Rect, Response, ScrollResponse, Size, WidgetId,
};
use kinetik_ui_text::TextEditState;

use super::{
    AssetBrowserDropTarget, AssetBrowserItem, AssetBrowserItemRect, AssetBrowserLayout,
    AssetBrowserLayoutResult, AssetBrowserModel, AssetBrowserSort, AssetBrowserViewMode,
    asset_browser_item_widget_id,
};
use crate::{
    CollectionContextActionRequest, CollectionContextTarget, CollectionCursor,
    CollectionCursorTarget, CollectionDragSource, CollectionProjection, InlineEditDraftPolicy,
    InlineEditFocusLossPolicy, InlineEditRequest, InlineEditSession, ItemId, OverlayScene,
    Selection, inline_edit_widget_id,
};

/// Pointer and keyboard selection behavior for the public asset browser.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AssetBrowserSelectionMode {
    /// Plain selection replaces the current selection.
    #[default]
    Single,
    /// Control/Super toggles and Shift extends the current selection.
    Multiple,
}

/// Context-menu geometry owned by the reusable asset browser.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetBrowserContextMenuConfig {
    /// Preferred logical menu size before viewport fitting.
    pub size: Size,
    /// Gap between the contextual anchor and menu.
    pub offset: f32,
}

impl Default for AssetBrowserContextMenuConfig {
    fn default() -> Self {
        Self {
            size: Size::new(220.0, 196.0),
            offset: 4.0,
        }
    }
}

/// Configuration for one prepared fixed-size grid or list asset-browser frame.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBrowserConfig {
    /// Visible asset-browser viewport in logical coordinates.
    pub bounds: Rect,
    /// Existing fixed-size grid/list layout contract.
    pub layout: AssetBrowserLayout,
    /// Case-insensitive substring query applied to name, kind, and tags.
    pub query: String,
    /// Optional stable sort applied after filtering.
    pub sort: Option<AssetBrowserSort>,
    /// Accessible collection label.
    pub label: String,
    /// Whether all asset-browser interaction is disabled.
    pub disabled: bool,
    /// Pointer and keyboard selection policy.
    pub selection_mode: AssetBrowserSelectionMode,
    /// Focus-loss behavior for inline rename.
    pub rename_focus_loss: InlineEditFocusLossPolicy,
    /// Empty and unchanged rename behavior.
    pub rename_draft_policy: InlineEditDraftPolicy,
    /// Context-menu placement metrics.
    pub context_menu: AssetBrowserContextMenuConfig,
}

impl AssetBrowserConfig {
    /// Creates an enabled asset browser from an existing fixed-size layout.
    #[must_use]
    pub fn new(bounds: Rect, layout: AssetBrowserLayout) -> Self {
        Self {
            bounds,
            layout,
            query: String::new(),
            sort: None,
            label: "Assets".to_owned(),
            disabled: false,
            selection_mode: AssetBrowserSelectionMode::Single,
            rename_focus_loss: InlineEditFocusLossPolicy::Commit,
            rename_draft_policy: InlineEditDraftPolicy::new(
                crate::InlineEditDraftDisposition::Cancel,
                crate::InlineEditDraftDisposition::Cancel,
            ),
            context_menu: AssetBrowserContextMenuConfig::default(),
        }
    }

    /// Sets the live filter query.
    #[must_use]
    pub fn query(mut self, query: impl Into<String>) -> Self {
        self.query = query.into();
        self
    }

    /// Sets the stable sort contract.
    #[must_use]
    pub const fn sort(mut self, sort: Option<AssetBrowserSort>) -> Self {
        self.sort = sort;
        self
    }

    /// Selects the grid or list recipe without replacing its metrics.
    #[must_use]
    pub const fn view_mode(mut self, view_mode: AssetBrowserViewMode) -> Self {
        self.layout.view_mode = view_mode;
        self
    }

    /// Sets the accessible asset-browser label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets whether all asset-browser interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets pointer and keyboard selection behavior.
    #[must_use]
    pub const fn selection_mode(mut self, mode: AssetBrowserSelectionMode) -> Self {
        self.selection_mode = mode;
        self
    }

    /// Sets inline-rename focus-loss and draft behavior.
    #[must_use]
    pub const fn rename_policy(
        mut self,
        focus_loss: InlineEditFocusLossPolicy,
        draft: InlineEditDraftPolicy,
    ) -> Self {
        self.rename_focus_loss = focus_loss;
        self.rename_draft_policy = draft;
        self
    }

    /// Sets context-menu placement metrics.
    #[must_use]
    pub const fn context_menu(mut self, context_menu: AssetBrowserContextMenuConfig) -> Self {
        self.context_menu = context_menu;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetBrowserEditState {
    pub session: InlineEditSession,
    pub text: TextEditState,
    pub conflict: Option<AssetBrowserRenameConflict>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetBrowserDragState {
    pub widget: WidgetId,
    pub source: CollectionDragSource,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct AssetBrowserContextState {
    pub target: CollectionContextTarget,
    pub trigger: WidgetId,
    pub scene: OverlayScene,
}

/// Retained interaction state for one public asset browser.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct AssetBrowserState {
    /// Stable cursor used for keyboard movement and focus repair.
    pub cursor: CollectionCursor,
    /// Caller-observable stable-ID selection. Filtered IDs remain selected.
    pub selection: Selection,
    pub(crate) edit: Option<AssetBrowserEditState>,
    pub(crate) drag: Option<AssetBrowserDragState>,
    pub(crate) context: Option<AssetBrowserContextState>,
    pub(crate) view_mode: Option<AssetBrowserViewMode>,
}

impl AssetBrowserState {
    /// Creates empty asset-browser interaction state.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the item currently being renamed.
    #[must_use]
    pub fn rename_target(&self) -> Option<ItemId> {
        self.edit.as_ref().map(|edit| edit.session.target)
    }

    /// Returns the current retained rename draft.
    #[must_use]
    pub fn rename_draft(&self) -> Option<&str> {
        self.edit.as_ref().map(|edit| edit.text.text.as_str())
    }

    /// Returns the current caller-reported rename conflict.
    #[must_use]
    pub fn rename_conflict(&self) -> Option<&AssetBrowserRenameConflict> {
        self.edit.as_ref().and_then(|edit| edit.conflict.as_ref())
    }

    /// Returns the selection-aware drag payload currently retained.
    #[must_use]
    pub fn drag_source(&self) -> Option<&CollectionDragSource> {
        self.drag.as_ref().map(|drag| &drag.source)
    }

    /// Returns the current context-menu target.
    #[must_use]
    pub fn context_target(&self) -> Option<&CollectionContextTarget> {
        self.context.as_ref().map(|context| &context.target)
    }

    pub(crate) fn begin_rename(
        &mut self,
        begin: crate::InlineEditBeginRequest,
        config: &AssetBrowserConfig,
    ) -> InlineEditRequest {
        let text = TextEditState::new(begin.initial_text.clone());
        let session = InlineEditSession::new(
            begin.clone(),
            config.rename_focus_loss,
            config.rename_draft_policy,
        );
        self.edit = Some(AssetBrowserEditState {
            session,
            text,
            conflict: None,
        });
        InlineEditRequest::Begin(begin)
    }

    pub(crate) fn clear_rename(&mut self) {
        self.edit = None;
    }
}

/// Caller-supplied rename validation failure retained by the editor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AssetBrowserRenameConflict {
    /// Stable item identity being renamed.
    pub target: ItemId,
    /// Rejected draft text.
    pub draft_text: String,
    /// Caller-owned conflict message.
    pub message: String,
}

/// Typed application-owned request emitted by [`crate::Ui::asset_browser`].
#[derive(Debug, Clone, PartialEq)]
pub enum AssetBrowserRequest {
    /// Begin, draft, commit, or cancel an inline rename.
    Rename(InlineEditRequest),
    /// Preview or open an asset without embedding domain behavior.
    Preview(ItemId),
    /// Drop stable selected asset identities on an item or empty space.
    Drop(AssetBrowserDropTarget),
    /// Invoke an application-owned action against a captured context target.
    Context(CollectionContextActionRequest),
}

/// Interaction response emitted for one materialized asset item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AssetBrowserItemResponse {
    /// Stable item identity.
    pub item: ItemId,
    /// Tile or row response.
    pub response: Response,
}

/// Output from one [`crate::Ui::asset_browser`] evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBrowserOutput {
    /// Scroll response; wheel movement affects the next prepared frame.
    pub scroll: ScrollResponse,
    /// Frozen view mode shared by input, paint, and semantics.
    pub view_mode: AssetBrowserViewMode,
    /// Strict visible projected range.
    pub visible_range: Range<usize>,
    /// Overscanned materialized projected range.
    pub materialized_range: Range<usize>,
    /// Whether selection changed this frame.
    pub selection_changed: bool,
    /// Context target whose real menu opened this frame.
    pub context_opened: Option<CollectionContextTarget>,
    /// Selection-aware drag payload retained after this frame.
    pub drag_payload: Option<CollectionDragSource>,
    /// Current item or empty-space drop preview.
    pub drop_preview: Option<AssetBrowserDropTarget>,
    /// Caller-reported rename conflict that kept editing active.
    pub rename_conflict: Option<AssetBrowserRenameConflict>,
    /// Ordered application-owned requests.
    pub requests: Vec<AssetBrowserRequest>,
    /// Materialized item responses.
    pub responses: Vec<AssetBrowserItemResponse>,
}

/// Immutable prepared geometry for one public asset-browser frame.
#[derive(Debug, Clone, PartialEq)]
pub struct AssetBrowserScene<'a> {
    root: WidgetId,
    config: AssetBrowserConfig,
    model: &'a AssetBrowserModel,
    projection: CollectionProjection,
    layout: AssetBrowserLayoutResult,
    context_scene: Option<OverlayScene>,
}

impl<'a> AssetBrowserScene<'a> {
    pub(crate) fn prepare(
        root: WidgetId,
        config: AssetBrowserConfig,
        model: &'a AssetBrowserModel,
        state: &AssetBrowserState,
        retained_scroll_offset: f32,
    ) -> Option<Self> {
        valid_bounds(config.bounds)?;
        valid_layout(config.layout)?;
        model.validate().ok()?;

        let query = config.query.trim().to_lowercase();
        let projection = model.projected(
            |item| query.is_empty() || item_matches_query(item, &query),
            config.sort,
        );
        let layout = config.layout.resolve_projected(
            config.bounds,
            model,
            &projection,
            retained_scroll_offset,
            &state.selection,
            None,
        );
        let context_scene = state
            .context
            .as_ref()
            .filter(|context| context_target_valid(model, &projection, &context.target))
            .map(|context| context.scene.clone());

        Some(Self {
            root,
            config,
            model,
            projection,
            layout,
            context_scene,
        })
    }

    /// Returns the stable root and scroll-owner ID.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.root
    }

    /// Returns the frozen filtered and sorted projection.
    #[must_use]
    pub const fn projection(&self) -> &CollectionProjection {
        &self.projection
    }

    /// Returns the frozen virtual grid/list geometry.
    #[must_use]
    pub const fn layout(&self) -> &AssetBrowserLayoutResult {
        &self.layout
    }

    /// Returns the stable widget ID for one asset item.
    #[must_use]
    pub fn item_widget_id(&self, item: ItemId) -> WidgetId {
        asset_browser_item_widget_id(self.root, item)
    }

    /// Returns the stable text owner for one inline rename.
    #[must_use]
    pub fn rename_widget_id(&self, item: ItemId) -> WidgetId {
        inline_edit_widget_id(self.root, item)
    }

    /// Adds the blocker, wheel owner, item/drop targets, background drop
    /// destination, and retained context overlay to one frame pointer plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
        state: &AssetBrowserState,
    ) -> PointerOrder {
        let mut ordinal = first_order.raw();
        plan.blocker(self.config.bounds, take_order(&mut ordinal));
        plan.target(PointerTarget::wheel_only(
            self.root,
            self.config.bounds,
            take_order(&mut ordinal),
        ));
        plan.with_clip(self.config.bounds, |plan| {
            if !self.config.disabled {
                plan.target(
                    PointerTarget::new(
                        background_widget_id(self.root),
                        self.config.bounds,
                        take_order(&mut ordinal),
                    )
                    .drop_owner(background_drop_widget_id(self.root)),
                );
                for item in &self.layout.items {
                    let item_id = self.item_widget_id(item.item.id);
                    let editing = state.rename_target() == Some(item.item.id)
                        && !item.item.state.disabled
                        && !item.item.state.read_only
                        && item.item.state.renamable;
                    let target = PointerTarget::new(item_id, item.rect, take_order(&mut ordinal))
                        .drop_owner(drop_widget_id(item_id));
                    let target = if item.item.state.read_only {
                        target
                    } else {
                        target.domain_drag_source()
                    };
                    plan.target(target.enabled(!item.item.state.disabled && !editing));
                    if editing {
                        plan.target(PointerTarget::new(
                            self.rename_widget_id(item.item.id),
                            item.name_rect,
                            take_order(&mut ordinal),
                        ));
                    }
                }
            }
        });
        if !self.config.disabled
            && let Some(context_scene) = &self.context_scene
        {
            return context_scene.declare_pointer_targets(plan, PointerOrder::new(ordinal));
        }
        PointerOrder::new(ordinal)
    }

    pub(crate) const fn config(&self) -> &AssetBrowserConfig {
        &self.config
    }

    pub(crate) const fn model(&self) -> &'a AssetBrowserModel {
        self.model
    }

    pub(crate) const fn has_prepared_context(&self) -> bool {
        self.context_scene.is_some()
    }

    pub(crate) fn context_target_valid(&self, target: &CollectionContextTarget) -> bool {
        context_target_valid(self.model, &self.projection, target)
    }

    pub(crate) fn item(&self, item: ItemId) -> Option<&AssetBrowserItemRect> {
        self.layout.items.iter().find(|entry| entry.item.id == item)
    }

    pub(crate) fn strict_items(&self) -> impl Iterator<Item = &AssetBrowserItemRect> {
        self.layout.items.iter().filter(|item| {
            item.rect
                .intersection(self.config.bounds)
                .is_some_and(|rect| rect.width > 0.0 && rect.height > 0.0)
        })
    }

    pub(crate) fn content_size(&self) -> Size {
        self.layout.content_size
    }

    pub(crate) fn page_items(&self) -> usize {
        self.layout.visible_range.len().max(1)
    }

    pub(crate) fn reveal_scroll_offset(&self, target: CollectionCursorTarget) -> f32 {
        let (top, extent) = match self.config.layout.view_mode {
            AssetBrowserViewMode::Grid => {
                let item_size = self
                    .config
                    .layout
                    .grid
                    .effective_item_size()
                    .expect("prepared asset browser has valid grid items");
                let gap = self.config.layout.grid.effective_gap();
                #[allow(clippy::cast_precision_loss)]
                let top = (target.projected_index / self.layout.columns.max(1)) as f32
                    * (item_size.height + gap);
                (top, item_size.height)
            }
            AssetBrowserViewMode::List => {
                let row_height = self
                    .config
                    .layout
                    .list
                    .effective_row_height()
                    .expect("prepared asset browser has valid list rows");
                #[allow(clippy::cast_precision_loss)]
                let top = target.projected_index as f32 * row_height;
                (top, row_height)
            }
        };
        let bottom = top + extent;
        let current = self.layout.scroll_offset;
        let desired = if top < current {
            top
        } else if bottom > current + self.config.bounds.height {
            bottom - self.config.bounds.height
        } else {
            current
        };
        desired.clamp(0.0, self.layout.max_scroll_offset)
    }

    pub(crate) fn resolve_drop(
        &self,
        point: kinetik_ui_core::Point,
        source: &CollectionDragSource,
    ) -> Option<AssetBrowserDropTarget> {
        self.layout
            .drop_target_at(self.config.bounds, point, source)
    }
}

pub(crate) fn background_widget_id(root: WidgetId) -> WidgetId {
    root.child("background")
}

pub(crate) fn background_drop_widget_id(root: WidgetId) -> WidgetId {
    background_widget_id(root).child("drop")
}

pub(crate) fn drop_widget_id(item: WidgetId) -> WidgetId {
    item.child("drop")
}

pub(crate) fn context_overlay_id(root: WidgetId) -> crate::OverlayId {
    crate::OverlayId::from_raw(root.child("context-menu").raw())
}

fn item_matches_query(item: &AssetBrowserItem, query: &str) -> bool {
    item.name.to_lowercase().contains(query)
        || item.kind.to_lowercase().contains(query)
        || item
            .tags
            .iter()
            .any(|tag| tag.to_lowercase().contains(query))
}

fn context_target_valid(
    model: &AssetBrowserModel,
    projection: &CollectionProjection,
    target: &CollectionContextTarget,
) -> bool {
    target.target_ids().into_iter().all(|item| {
        projection.projected_index(item).is_some()
            && model.item_by_id(item).is_some_and(|item| !item.disabled)
    })
}

fn valid_bounds(bounds: Rect) -> Option<()> {
    (bounds.x.is_finite()
        && bounds.y.is_finite()
        && bounds.width.is_finite()
        && bounds.height.is_finite()
        && bounds.width > 0.0
        && bounds.height > 0.0)
        .then_some(())
}

fn valid_layout(layout: AssetBrowserLayout) -> Option<()> {
    match layout.view_mode {
        AssetBrowserViewMode::Grid => layout.grid.effective_item_size().map(|_| ()),
        AssetBrowserViewMode::List => layout.list.effective_row_height().map(|_| ()),
    }
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal
        .checked_add(1)
        .expect("asset-browser pointer order exhausted");
    order
}
