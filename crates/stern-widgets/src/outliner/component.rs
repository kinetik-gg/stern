//! Prepared public outliner composition contracts.

use stern_core::{
    PointerOrder, PointerTarget, PointerTargetPlan, Rect, Response, ScrollResponse, Size, WidgetId,
};
use stern_text::TextEditState;

use crate::{
    CollectionContextActionRequest, CollectionContextTarget, CollectionCursor,
    CollectionCursorTarget, CollectionDragSource, CollectionProjection, InlineEditDraftPolicy,
    InlineEditFocusLossPolicy, InlineEditRequest, InlineEditSession, ItemId, OutlinerDropTarget,
    OutlinerLayout, OutlinerLockToggleRequest, OutlinerModel, OutlinerRowZones,
    OutlinerVisibilityToggleRequest, OverlayScene, Selection, TreeExpansion, VirtualWindow,
    inline_edit_widget_id, outliner_row_widget_id,
};

/// Pointer and keyboard selection behavior for the public outliner.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum OutlinerSelectionMode {
    /// Plain selection replaces the current selection.
    #[default]
    Single,
    /// Control/Super toggles and Shift extends the current selection.
    Multiple,
}

/// Context-menu geometry owned by the reusable outliner component.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinerContextMenuConfig {
    /// Preferred logical menu size before viewport fitting.
    pub size: Size,
    /// Gap between the contextual anchor and menu.
    pub offset: f32,
}

impl Default for OutlinerContextMenuConfig {
    fn default() -> Self {
        Self {
            size: Size::new(220.0, 196.0),
            offset: 4.0,
        }
    }
}

/// Configuration for one prepared, fixed-height outliner frame.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinerConfig {
    /// Visible outliner viewport in logical coordinates.
    pub bounds: Rect,
    /// Existing row, indentation, and affordance layout.
    pub layout: OutlinerLayout,
    /// Rows materialized before and after the strict visible window.
    pub overscan: usize,
    /// Accessible collection label.
    pub label: String,
    /// Whether all outliner interaction is disabled.
    pub disabled: bool,
    /// Pointer and keyboard selection policy.
    pub selection_mode: OutlinerSelectionMode,
    /// Focus-loss behavior for inline rename.
    pub rename_focus_loss: InlineEditFocusLossPolicy,
    /// Empty and unchanged rename behavior.
    pub rename_draft_policy: InlineEditDraftPolicy,
    /// Context-menu placement metrics.
    pub context_menu: OutlinerContextMenuConfig,
}

impl OutlinerConfig {
    /// Creates an enabled fixed-height outliner with one overscan row.
    #[must_use]
    pub fn new(bounds: Rect, row_height: f32, indent_width: f32) -> Self {
        Self {
            bounds,
            layout: OutlinerLayout::new(row_height, indent_width),
            overscan: 1,
            label: "Outliner".to_owned(),
            disabled: false,
            selection_mode: OutlinerSelectionMode::Single,
            rename_focus_loss: InlineEditFocusLossPolicy::Commit,
            rename_draft_policy: InlineEditDraftPolicy::new(
                crate::InlineEditDraftDisposition::Cancel,
                crate::InlineEditDraftDisposition::Cancel,
            ),
            context_menu: OutlinerContextMenuConfig::default(),
        }
    }

    /// Sets the accessible outliner label.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets materialized overscan rows.
    #[must_use]
    pub const fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Sets whether all outliner interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets pointer and keyboard selection behavior.
    #[must_use]
    pub const fn selection_mode(mut self, mode: OutlinerSelectionMode) -> Self {
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
    pub const fn context_menu(mut self, context_menu: OutlinerContextMenuConfig) -> Self {
        self.context_menu = context_menu;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutlinerEditState {
    pub session: InlineEditSession,
    pub text: TextEditState,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutlinerDragState {
    pub widget: WidgetId,
    pub source: CollectionDragSource,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OutlinerContextState {
    pub target: CollectionContextTarget,
    pub trigger: WidgetId,
    pub scene: OverlayScene,
}

/// Retained interaction state for one public outliner.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OutlinerState {
    /// Stable cursor used for keyboard movement and focus repair.
    pub cursor: CollectionCursor,
    /// Caller-observable stable-ID selection.
    pub selection: Selection,
    /// Caller-observable hierarchy expansion state.
    pub expansion: TreeExpansion,
    pub(crate) edit: Option<OutlinerEditState>,
    pub(crate) drag: Option<OutlinerDragState>,
    pub(crate) context: Option<OutlinerContextState>,
}

impl OutlinerState {
    /// Creates empty outliner interaction state.
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

    /// Returns the current context-menu target.
    #[must_use]
    pub fn context_target(&self) -> Option<&CollectionContextTarget> {
        self.context.as_ref().map(|context| &context.target)
    }

    /// Returns whether a hierarchy drag is retained.
    #[must_use]
    pub const fn dragging(&self) -> bool {
        self.drag.is_some()
    }

    pub(crate) fn begin_rename(
        &mut self,
        begin: crate::InlineEditBeginRequest,
        config: &OutlinerConfig,
    ) -> InlineEditRequest {
        let text = TextEditState::new(begin.initial_text.clone());
        let session = InlineEditSession::new(
            begin.clone(),
            config.rename_focus_loss,
            config.rename_draft_policy,
        );
        self.edit = Some(OutlinerEditState { session, text });
        InlineEditRequest::Begin(begin)
    }

    pub(crate) fn clear_rename(&mut self) {
        self.edit = None;
    }
}

/// Typed application-owned request emitted by [`crate::Ui::outliner`].
#[derive(Debug, Clone, PartialEq)]
pub enum OutlinerRequest {
    /// Begin, draft, commit, or cancel an inline rename.
    Rename(InlineEditRequest),
    /// Toggle the current app-owned visibility state.
    Visibility(OutlinerVisibilityToggleRequest),
    /// Toggle the current app-owned lock state.
    Lock(OutlinerLockToggleRequest),
    /// Reparent or reorder stable item identities.
    Drop(OutlinerDropTarget),
    /// Invoke an application-owned action against a captured context target.
    Context(CollectionContextActionRequest),
}

/// Interaction responses emitted for one materialized outliner row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OutlinerRowResponse {
    /// Stable row identity.
    pub item: ItemId,
    /// Row label/body response.
    pub row: Response,
    /// Disclosure affordance response, when available.
    pub disclosure: Option<Response>,
    /// Visibility affordance response, when available.
    pub visibility: Option<Response>,
    /// Lock affordance response, when available.
    pub lock: Option<Response>,
}

/// Output from one [`crate::Ui::outliner`] evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinerOutput {
    /// Scroll response; applied wheel movement affects the next prepared frame.
    pub scroll: ScrollResponse,
    /// Frozen virtual window shared by input, paint, and semantics.
    pub window: VirtualWindow,
    /// Item activated by Enter or a plain double-click.
    pub activated: Option<ItemId>,
    /// Whether selection changed this frame.
    pub selection_changed: bool,
    /// Whether expansion changed this frame.
    pub expansion_changed: bool,
    /// Context target whose real menu opened this frame.
    pub context_opened: Option<CollectionContextTarget>,
    /// Current cycle-safe drop preview.
    pub drop_preview: Option<OutlinerDropTarget>,
    /// Ordered application-owned requests.
    pub requests: Vec<OutlinerRequest>,
    /// Materialized row responses.
    pub responses: Vec<OutlinerRowResponse>,
}

/// Immutable prepared geometry for one public outliner frame.
#[derive(Debug, Clone, PartialEq)]
pub struct OutlinerScene<'a> {
    root: WidgetId,
    config: OutlinerConfig,
    model: &'a OutlinerModel,
    projection: CollectionProjection,
    window: VirtualWindow,
    rows: Vec<OutlinerRowZones>,
    content_height: f32,
    context_scene: Option<OverlayScene>,
}

impl<'a> OutlinerScene<'a> {
    pub(crate) fn prepare(
        root: WidgetId,
        config: OutlinerConfig,
        model: &'a OutlinerModel,
        state: &OutlinerState,
        retained_scroll_offset: f32,
    ) -> Option<Self> {
        valid_bounds(config.bounds)?;
        config.layout.tree.effective_row_height()?;
        let tree = model.tree_model();
        tree.validate().ok()?;

        let projection =
            CollectionProjection::from_source_ids(&tree.visible_item_ids(&state.expansion));
        let window = config.layout.tree.virtual_window(
            projection.len(),
            retained_scroll_offset,
            config.bounds.height,
            config.overscan,
        );
        let rows = config.layout.visible_model_row_zones(
            config.bounds,
            model,
            &state.expansion,
            window.clamped_scroll_offset,
            config.overscan,
        );
        let content_height = config.layout.tree.content_height(projection.len());
        let context_scene = state
            .context
            .as_ref()
            .filter(|context| {
                context.target.target_ids().into_iter().all(|item| {
                    projection.projected_index(item).is_some()
                        && model
                            .item_by_id(item)
                            .is_some_and(|item| item.flags.can_request_selection())
                })
            })
            .map(|context| context.scene.clone());

        Some(Self {
            root,
            config,
            model,
            projection,
            window,
            rows,
            content_height,
            context_scene,
        })
    }

    /// Returns the stable root and scroll-owner ID.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.root
    }

    /// Returns the frozen virtual window.
    #[must_use]
    pub const fn window(&self) -> &VirtualWindow {
        &self.window
    }

    /// Returns materialized row geometry in flattened visible order.
    #[must_use]
    pub fn rows(&self) -> &[OutlinerRowZones] {
        &self.rows
    }

    /// Returns the stable widget ID for one row.
    #[must_use]
    pub fn row_widget_id(&self, item: ItemId) -> WidgetId {
        outliner_row_widget_id(self.root, item)
    }

    /// Returns the stable text owner for one inline rename.
    #[must_use]
    pub fn rename_widget_id(&self, item: ItemId) -> WidgetId {
        inline_edit_widget_id(self.root, item)
    }

    /// Adds the blocker, wheel owner, row sub-targets, drop destinations, and
    /// retained context overlay to the caller's single frame pointer plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
        state: &OutlinerState,
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
                plan.target(PointerTarget::new(
                    background_widget_id(self.root),
                    self.config.bounds,
                    take_order(&mut ordinal),
                ));
                for zones in &self.rows {
                    let row_id = self.row_widget_id(zones.row.id);
                    let drop_id = drop_widget_id(row_id);
                    let editing = state.rename_target() == Some(zones.row.id)
                        && zones.row.flags.can_request_rename();
                    plan.target(
                        PointerTarget::new(row_id, zones.rect, take_order(&mut ordinal))
                            .drop_owner(drop_id)
                            .domain_drag_source()
                            .enabled(!zones.row.flags.disabled && !editing),
                    );
                    if zones.row.has_children && !zones.row.flags.disabled {
                        plan.target(PointerTarget::new(
                            disclosure_widget_id(row_id),
                            zones.disclosure_rect,
                            take_order(&mut ordinal),
                        ));
                    }
                    if zones.row.flags.can_request_visibility_toggle() {
                        plan.target(PointerTarget::new(
                            visibility_widget_id(row_id),
                            zones.visibility_toggle_rect,
                            take_order(&mut ordinal),
                        ));
                    }
                    if zones.row.flags.can_request_lock_toggle() {
                        plan.target(PointerTarget::new(
                            lock_widget_id(row_id),
                            zones.lock_toggle_rect,
                            take_order(&mut ordinal),
                        ));
                    }
                    if editing {
                        plan.target(PointerTarget::new(
                            self.rename_widget_id(zones.row.id),
                            zones.label_rect,
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

    pub(crate) const fn config(&self) -> &OutlinerConfig {
        &self.config
    }

    pub(crate) const fn model(&self) -> &'a OutlinerModel {
        self.model
    }

    pub(crate) const fn projection(&self) -> &CollectionProjection {
        &self.projection
    }

    pub(crate) fn content_size(&self) -> Size {
        Size::new(self.config.bounds.width, self.content_height)
    }

    pub(crate) const fn has_prepared_context(&self) -> bool {
        self.context_scene.is_some()
    }

    pub(crate) fn row(&self, item: ItemId) -> Option<&OutlinerRowZones> {
        self.rows.iter().find(|row| row.row.id == item)
    }

    pub(crate) fn strict_rows(&self) -> impl Iterator<Item = &OutlinerRowZones> {
        self.rows.iter().filter(|row| {
            row.rect
                .intersection(self.config.bounds)
                .is_some_and(|rect| rect.width > 0.0 && rect.height > 0.0)
        })
    }

    pub(crate) fn page_rows(&self) -> usize {
        let row_height = self
            .config
            .layout
            .tree
            .effective_row_height()
            .expect("prepared outliner has valid row height");
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let rows = (self.config.bounds.height / row_height).floor() as usize;
        rows.max(1)
    }

    pub(crate) fn reveal_scroll_offset(&self, target: CollectionCursorTarget) -> f32 {
        let row_height = self
            .config
            .layout
            .tree
            .effective_row_height()
            .expect("prepared outliner has valid row height");
        #[allow(clippy::cast_precision_loss)]
        let top = target.projected_index as f32 * row_height;
        let bottom = top + row_height;
        let current = self.window.clamped_scroll_offset;
        let desired = if top < current {
            top
        } else if bottom > current + self.config.bounds.height {
            bottom - self.config.bounds.height
        } else {
            current
        };
        self.config.layout.tree.clamp_scroll_offset(
            self.projection.len(),
            self.config.bounds.height,
            desired,
        )
    }

    pub(crate) fn resolve_drop(
        &self,
        zones: &OutlinerRowZones,
        point: stern_core::Point,
        source: &CollectionDragSource,
    ) -> Option<OutlinerDropTarget> {
        let target = zones.drop_target(point, source)?;
        (!self.target_descends_from_source(target.target, source)).then_some(target)
    }

    fn target_descends_from_source(&self, target: ItemId, source: &CollectionDragSource) -> bool {
        let mut current = Some(target);
        while let Some(item) = current {
            if source.contains(item) {
                return true;
            }
            current = self.model.item_by_id(item).and_then(|item| item.parent);
        }
        false
    }
}

pub(crate) fn background_widget_id(root: WidgetId) -> WidgetId {
    root.child("background")
}

pub(crate) fn disclosure_widget_id(row: WidgetId) -> WidgetId {
    row.child("disclosure")
}

pub(crate) fn visibility_widget_id(row: WidgetId) -> WidgetId {
    row.child("visibility")
}

pub(crate) fn lock_widget_id(row: WidgetId) -> WidgetId {
    row.child("lock")
}

pub(crate) fn drop_widget_id(row: WidgetId) -> WidgetId {
    row.child("drop")
}

pub(crate) fn context_overlay_id(root: WidgetId) -> crate::OverlayId {
    crate::OverlayId::from_raw(root.child("context-menu").raw())
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

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal
        .checked_add(1)
        .expect("outliner pointer order exhausted");
    order
}
