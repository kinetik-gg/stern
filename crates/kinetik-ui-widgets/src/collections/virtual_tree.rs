use kinetik_ui_core::{
    PointerOrder, PointerTarget, PointerTargetPlan, Rect, Response, ScrollResponse, Size,
    Transform, Vec2, WidgetId,
};

use super::{
    CollectionCursorTarget, CollectionProjection, ItemId, TreeExpansion, TreeLayout, TreeModel,
    TreeRow, VirtualWindow,
};

/// Selection behavior used by a public virtual tree.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum VirtualTreeSelectionMode {
    /// Plain selection only; modifier keys do not retain other rows.
    #[default]
    Single,
    /// Control/Super toggles and Shift extends from the retained anchor.
    Multiple,
}

/// Configuration for one fixed-height virtual tree.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTreeConfig {
    /// Visible tree viewport in logical coordinates.
    pub bounds: Rect,
    /// Fixed-height row and indentation layout.
    pub layout: TreeLayout,
    /// Extra rows materialized before and after the strict visible range.
    pub overscan: usize,
    /// Accessible tree name.
    pub label: String,
    /// Whether scrolling and row interaction are disabled.
    pub disabled: bool,
    /// Selection behavior for pointer and keyboard movement.
    pub selection_mode: VirtualTreeSelectionMode,
}

impl VirtualTreeConfig {
    /// Creates an enabled single-selection tree with one overscan row.
    #[must_use]
    pub fn new(bounds: Rect, row_height: f32, indent_width: f32) -> Self {
        Self {
            bounds,
            layout: TreeLayout::new(row_height, indent_width),
            overscan: 1,
            label: "Tree".to_owned(),
            disabled: false,
            selection_mode: VirtualTreeSelectionMode::Single,
        }
    }

    /// Sets the accessible tree name.
    #[must_use]
    pub fn label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    /// Sets the number of rows materialized around the strict visible range.
    #[must_use]
    pub const fn overscan(mut self, overscan: usize) -> Self {
        self.overscan = overscan;
        self
    }

    /// Sets whether tree interaction is disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets pointer and keyboard selection behavior.
    #[must_use]
    pub const fn selection_mode(mut self, selection_mode: VirtualTreeSelectionMode) -> Self {
        self.selection_mode = selection_mode;
        self
    }
}

/// Presentation returned by the callback for one materialized tree row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VirtualTreeRow {
    /// Visible and accessible row label.
    pub label: String,
}

impl VirtualTreeRow {
    /// Creates a materialized row presentation.
    #[must_use]
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
        }
    }
}

/// Interaction response for one materialized virtual-tree item.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VirtualTreeItemResponse {
    /// Flattened visible tree-row metadata.
    pub row: TreeRow,
    /// Shared interaction response for the row body.
    pub response: Response,
    /// Disclosure response when the row has children.
    pub disclosure_response: Option<Response>,
}

/// Output from one [`crate::Ui::virtual_tree`] evaluation.
#[derive(Debug, Clone, PartialEq)]
pub struct VirtualTreeOutput {
    /// Scroll behavior result. Geometry for this frame remains frozen to
    /// [`Self::window`]; an applied wheel delta affects the next frame.
    pub scroll: ScrollResponse,
    /// Prepared virtual window shared by input, painting, and semantics.
    pub window: VirtualWindow,
    /// Item activated by double-click, Enter, or Space.
    pub activated: Option<ItemId>,
    /// Whether caller-owned selection changed this frame.
    pub selection_changed: bool,
    /// Whether caller-owned expansion changed this frame.
    pub expansion_changed: bool,
    /// Last item whose disclosure state changed this frame.
    pub toggled: Option<ItemId>,
    /// Last cursor target produced by reconciliation, navigation, or clicking.
    pub cursor_target: Option<CollectionCursorTarget>,
    /// Responses for materialized rows in flattened visible order.
    pub responses: Vec<VirtualTreeItemResponse>,
}

/// Prepared fixed-height virtual-tree frame.
///
/// Prepare this snapshot before resolving the frame pointer plan, then use the
/// same snapshot for pointer declaration and [`crate::Ui::virtual_tree`].
#[derive(Debug)]
pub struct VirtualTree<'a> {
    root: WidgetId,
    config: VirtualTreeConfig,
    model: &'a TreeModel,
    projection: CollectionProjection,
    window: VirtualWindow,
    rows: Vec<VirtualTreeProjectedRow>,
    content_height: f32,
}

impl<'a> VirtualTree<'a> {
    pub(crate) fn prepare(
        root: WidgetId,
        config: VirtualTreeConfig,
        model: &'a TreeModel,
        expansion: &TreeExpansion,
        retained_scroll_offset: f32,
    ) -> Option<Self> {
        valid_viewport(config.bounds)?;
        let row_height = config.layout.effective_row_height()?;
        model.validate().ok()?;

        let projection = CollectionProjection::from_source_ids(&model.visible_item_ids(expansion));
        let total_rows = projection.len();
        let window = config.layout.virtual_window(
            total_rows,
            retained_scroll_offset,
            config.bounds.height,
            config.overscan,
        );
        let visible_rows =
            model.visible_rows_in_range(expansion, window.materialized_range.clone());
        let rows = config
            .layout
            .visible_row_rects_in_range_content(
                config.bounds,
                total_rows,
                &visible_rows,
                window.clamped_scroll_offset,
                config.overscan,
            )
            .into_iter()
            .map(|row| {
                let disclosure_width = row.content_rect.width.min(row_height).max(0.0);
                let disclosure_rect = Rect::new(
                    row.content_rect.x,
                    row.rect.y,
                    disclosure_width,
                    row.rect.height,
                );
                VirtualTreeProjectedRow {
                    id: row_widget_id(root, row.row.id),
                    disclosure_id: disclosure_widget_id(root, row.row.id),
                    row: row.row,
                    rect: row.rect,
                    disclosure_rect,
                }
            })
            .collect();
        let content_height = config.layout.content_height(total_rows);

        Some(Self {
            root,
            config,
            model,
            projection,
            window,
            rows,
            content_height,
        })
    }

    /// Returns the stable widget ID for the tree surface and scroll owner.
    #[must_use]
    pub const fn widget_id(&self) -> WidgetId {
        self.root
    }

    /// Returns the stable widget ID for one tree row.
    #[must_use]
    pub fn row_widget_id(&self, id: ItemId) -> WidgetId {
        row_widget_id(self.root, id)
    }

    /// Returns the stable widget ID for one row disclosure affordance.
    #[must_use]
    pub fn disclosure_widget_id(&self, id: ItemId) -> WidgetId {
        disclosure_widget_id(self.root, id)
    }

    /// Returns the fixed virtual window used by pointer, paint, and semantics.
    #[must_use]
    pub const fn window(&self) -> &VirtualWindow {
        &self.window
    }

    /// Adds the viewport blocker, wheel owner, clipped rows, and disclosure
    /// targets to one caller-owned frame pointer plan.
    ///
    /// The returned order is the first unused ordinal after this tree.
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
        plan.with_clip(self.config.bounds, |plan| {
            plan.with_transform(
                Transform::translation(Vec2::new(0.0, -self.window.clamped_scroll_offset)),
                |plan| {
                    if !self.config.disabled {
                        for row in &self.rows {
                            plan.target(PointerTarget::new(
                                row.id,
                                row.rect,
                                take_order(&mut ordinal),
                            ));
                            if row.row.has_children {
                                plan.target(PointerTarget::new(
                                    row.disclosure_id,
                                    row.disclosure_rect,
                                    take_order(&mut ordinal),
                                ));
                            }
                        }
                    }
                },
            );
        });
        PointerOrder::new(ordinal)
    }

    pub(crate) const fn config(&self) -> &VirtualTreeConfig {
        &self.config
    }

    pub(crate) const fn model(&self) -> &'a TreeModel {
        self.model
    }

    pub(crate) const fn projection(&self) -> &CollectionProjection {
        &self.projection
    }

    pub(crate) fn rows(&self) -> &[VirtualTreeProjectedRow] {
        &self.rows
    }

    pub(crate) fn contains_materialized(&self, id: ItemId) -> bool {
        self.rows.iter().any(|row| row.row.id == id)
    }

    pub(crate) fn row_is_visible(&self, row: &VirtualTreeProjectedRow) -> bool {
        let screen_rect = Rect::new(
            row.rect.x,
            row.rect.y - self.window.clamped_scroll_offset,
            row.rect.width,
            row.rect.height,
        );
        screen_rect.intersection(self.config.bounds).is_some()
    }

    pub(crate) fn content_size(&self) -> Size {
        Size::new(
            self.config.bounds.width,
            self.content_height.max(self.config.bounds.height),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct VirtualTreeProjectedRow {
    pub(crate) id: WidgetId,
    pub(crate) disclosure_id: WidgetId,
    pub(crate) row: TreeRow,
    pub(crate) rect: Rect,
    pub(crate) disclosure_rect: Rect,
}

fn row_widget_id(root: WidgetId, id: ItemId) -> WidgetId {
    root.child(("virtual-tree-row", id.raw()))
}

fn disclosure_widget_id(root: WidgetId, id: ItemId) -> WidgetId {
    root.child(("virtual-tree-disclosure", id.raw()))
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

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
