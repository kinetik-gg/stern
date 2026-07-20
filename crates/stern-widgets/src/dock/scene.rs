//! Public, deterministic presentation contract for painted dock scenes.

use stern_core::{Axis, PointerOrder, PointerTarget, PointerTargetPlan, Rect, WidgetId};

use super::{
    Dock, DockChromeStyle, DockDropTarget, DockPlacement, DockSplitPath, FrameId, PanelId,
    frame_tabs, solve_dock_scene_geometry,
};

const DEFAULT_TAB_HEIGHT: f32 = 28.0;
const PREFERRED_TAB_WIDTH: f32 = 160.0;
const TAB_CLOSE_WIDTH: f32 = 22.0;
const DROP_PREVIEW_EDGE_FRACTION: f32 = 0.35;
const DROP_PREVIEW_INSET_FRACTION: f32 = 0.12;

/// Caller-owned configuration for one prepared dock scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockSceneConfig {
    /// Stable root used to derive every scene widget ID.
    pub root: WidgetId,
    /// Logical bounds allocated to the dock.
    pub bounds: Rect,
    /// Logical height reserved for each frame tab strip.
    pub tab_height: f32,
    /// Existing data-only splitter hit style.
    pub chrome_style: DockChromeStyle,
    /// Whether future controller targets are disabled.
    pub disabled: bool,
    /// Optional caller-resolved target painted as a drop preview.
    pub drop_preview: Option<DockDropTarget>,
}

impl DockSceneConfig {
    /// Creates an enabled dock scene using the default tab and splitter metrics.
    #[must_use]
    pub fn new(root: WidgetId, bounds: Rect) -> Self {
        Self {
            root,
            bounds,
            tab_height: DEFAULT_TAB_HEIGHT,
            chrome_style: DockChromeStyle::default(),
            disabled: false,
            drop_preview: None,
        }
    }

    /// Sets the logical frame tab-strip height.
    #[must_use]
    pub const fn with_tab_height(mut self, height: f32) -> Self {
        self.tab_height = height;
        self
    }

    /// Sets the existing data-only dock chrome style.
    #[must_use]
    pub const fn with_chrome_style(mut self, style: DockChromeStyle) -> Self {
        self.chrome_style = style;
        self
    }

    /// Sets whether future controller targets are disabled.
    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    /// Sets an optional caller-resolved drop preview.
    #[must_use]
    pub const fn with_drop_preview(mut self, preview: Option<DockDropTarget>) -> Self {
        self.drop_preview = preview;
        self
    }
}

/// One prepared frame tab.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSceneTab {
    /// Stable tab widget ID derived from the panel identity.
    pub id: WidgetId,
    /// Stable close-affordance ID, whether or not it is currently visible.
    pub close_id: WidgetId,
    /// Stable panel identity.
    pub panel: PanelId,
    /// Display label copied from the panel model.
    pub title: String,
    /// Clipped tab bounds.
    pub rect: Rect,
    /// Visible close-affordance bounds.
    pub close_rect: Option<Rect>,
    /// Whether this is the frame's active panel tab.
    pub selected: bool,
    /// Whether the existing panel policy exposes close.
    pub close_visible: bool,
    /// Whether the existing model exposes tab dragging.
    pub draggable: bool,
}

/// One prepared active panel body.
#[derive(Debug, Clone, PartialEq)]
pub struct DockScenePanel {
    /// Stable panel widget ID derived only from panel identity.
    pub id: WidgetId,
    /// Owning frame.
    pub frame: FrameId,
    /// Stable panel identity.
    pub panel: PanelId,
    /// Display title copied from the panel model.
    pub title: String,
    /// Exact clipped content body bounds.
    pub rect: Rect,
}

/// One prepared frame surface.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSceneFrame {
    /// Stable frame widget ID.
    pub id: WidgetId,
    /// Stable tab-list widget ID.
    pub tab_list_id: WidgetId,
    /// Frame model identity.
    pub frame: FrameId,
    /// Full frame bounds.
    pub rect: Rect,
    /// Tab-list bounds.
    pub tab_list_rect: Rect,
    /// Tabs in model order.
    pub tabs: Vec<DockSceneTab>,
    /// Active passive panel body, when one exists and has positive area.
    pub panel: Option<DockScenePanel>,
    /// Whether this is the dock's active frame.
    pub active: bool,
}

/// One prepared splitter surface.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSceneSplitter {
    /// Stable splitter widget ID.
    pub id: WidgetId,
    /// Existing split-tree address.
    pub path: DockSplitPath,
    /// Split orientation.
    pub axis: Axis,
    /// Effective hit bounds clipped to the dock.
    pub rect: Rect,
}

impl DockSceneSplitter {
    /// Returns the one-unit neutral divider centered inside the hit bounds.
    #[must_use]
    pub fn divider_rect(&self) -> Rect {
        const DIVIDER_THICKNESS: f32 = 1.0;

        match self.axis {
            Axis::Horizontal => {
                let width = DIVIDER_THICKNESS.min(self.rect.width);
                Rect::new(
                    self.rect.x + (self.rect.width - width) * 0.5,
                    self.rect.y,
                    width,
                    self.rect.height,
                )
            }
            Axis::Vertical => {
                let height = DIVIDER_THICKNESS.min(self.rect.height);
                Rect::new(
                    self.rect.x,
                    self.rect.y + (self.rect.height - height) * 0.5,
                    self.rect.width,
                    height,
                )
            }
        }
    }
}

/// Visual kind for a prepared drop preview.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockScenePreviewKind {
    /// Center tab merge.
    Merge,
    /// Edge split insertion.
    Split(DockPlacement),
}

/// One prepared drop-preview surface.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DockScenePreview {
    /// Stable preview widget ID.
    pub id: WidgetId,
    /// Target frame.
    pub frame: FrameId,
    /// Preview bounds.
    pub rect: Rect,
    /// Merge or edge-split visual kind.
    pub kind: DockScenePreviewKind,
}

/// Immutable prepared geometry for one dock frame.
#[derive(Debug, Clone, PartialEq)]
pub struct DockSceneLayout {
    /// Sanitized dock bounds. Invalid input becomes [`Rect::ZERO`].
    pub bounds: Rect,
    /// Frames in deterministic dock-tree order.
    pub frames: Vec<DockSceneFrame>,
    /// Splitters in deterministic split-tree order.
    pub splitters: Vec<DockSceneSplitter>,
    /// Optional resolved drop preview.
    pub preview: Option<DockScenePreview>,
}

/// Prepared, immutable public dock scene.
#[derive(Debug, Clone, PartialEq)]
pub struct DockScene {
    config: DockSceneConfig,
    layout: DockSceneLayout,
}

impl DockScene {
    /// Prepares a frame-local scene from caller-owned dock state.
    #[must_use]
    pub fn new(config: DockSceneConfig, dock: &Dock) -> Self {
        let layout = prepare_layout(config, dock);
        Self { config, layout }
    }

    /// Returns the caller-owned scene configuration.
    #[must_use]
    pub const fn config(&self) -> DockSceneConfig {
        self.config
    }

    /// Returns immutable prepared frame-local geometry.
    #[must_use]
    pub const fn layout(&self) -> &DockSceneLayout {
        &self.layout
    }

    /// Returns the stable Dock semantic/widget root.
    #[must_use]
    pub const fn root_widget_id(&self) -> WidgetId {
        self.config.root
    }

    /// Returns the stable widget ID for a frame.
    #[must_use]
    pub fn frame_widget_id(&self, frame: FrameId) -> WidgetId {
        frame_widget_id(self.config.root, frame)
    }

    /// Returns the stable widget ID for a frame tab list.
    #[must_use]
    pub fn tab_list_widget_id(&self, frame: FrameId) -> WidgetId {
        tab_list_widget_id(self.config.root, frame)
    }

    /// Returns the stable widget ID for a panel tab.
    #[must_use]
    pub fn tab_widget_id(&self, panel: PanelId) -> WidgetId {
        tab_widget_id(self.config.root, panel)
    }

    /// Returns the stable widget ID for a tab close affordance.
    #[must_use]
    pub fn tab_close_widget_id(&self, panel: PanelId) -> WidgetId {
        tab_close_widget_id(self.config.root, panel)
    }

    /// Returns the stable widget ID for a passive panel body.
    #[must_use]
    pub fn panel_widget_id(&self, panel: PanelId) -> WidgetId {
        panel_widget_id(self.config.root, panel)
    }

    /// Returns the stable widget ID for a split-tree path.
    #[must_use]
    pub fn splitter_widget_id(&self, path: &DockSplitPath) -> WidgetId {
        splitter_widget_id(self.config.root, path)
    }

    /// Returns the stable drop-preview widget ID.
    #[must_use]
    pub fn preview_widget_id(&self) -> WidgetId {
        self.config.root.child("drop-preview")
    }

    /// Adds the dock blocker and stable future-controller targets to one plan.
    ///
    /// Use [`Self::declare_pointer_targets_with_content`] when active panel
    /// bodies contain interactive targets that must remain below frame chrome.
    /// The returned order is suitable for later overlays above the whole dock.
    /// This packet does not evaluate controller mutations.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        self.declare_pointer_targets_with_content(plan, first_order, |_, order| order)
    }

    /// Adds dock targets with caller-declared panel content below frame chrome.
    ///
    /// The callback receives the first unused order after the root blocker and
    /// frame targets. It must return the first unused order after declaring all
    /// interactive panel content. Tabs, close affordances, and splitters are
    /// then declared above that content, matching their paint order.
    pub fn declare_pointer_targets_with_content(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
        panel_content: impl FnOnce(&mut PointerTargetPlan, PointerOrder) -> PointerOrder,
    ) -> PointerOrder {
        if !valid_rect(self.layout.bounds) {
            return first_order;
        }

        let enabled = !self.config.disabled;
        let mut ordinal = first_order.raw();
        plan.blocker(self.layout.bounds, take_order(&mut ordinal));

        for frame in &self.layout.frames {
            plan.target(
                PointerTarget::new(frame.id, frame.rect, take_order(&mut ordinal))
                    .drop_owner(frame.id)
                    .enabled(enabled),
            );
        }

        ordinal = ordinal.max(panel_content(plan, PointerOrder::new(ordinal)).raw());

        for frame in &self.layout.frames {
            for tab in &frame.tabs {
                let mut target = PointerTarget::new(tab.id, tab.rect, take_order(&mut ordinal))
                    .drop_owner(tab.id.child("dock-drop"));
                if tab.draggable {
                    target = target.domain_drag_source();
                }
                plan.target(target.enabled(enabled));
                if let Some(close_rect) = tab.close_rect {
                    plan.target(
                        PointerTarget::new(tab.close_id, close_rect, take_order(&mut ordinal))
                            .drop_owner(tab.close_id.child("dock-drop"))
                            .enabled(enabled && tab.close_visible),
                    );
                }
            }
        }

        for splitter in &self.layout.splitters {
            plan.target(
                PointerTarget::new(splitter.id, splitter.rect, take_order(&mut ordinal))
                    .domain_drag_source()
                    .enabled(enabled),
            );
        }

        PointerOrder::new(ordinal)
    }
}

fn prepare_layout(config: DockSceneConfig, dock: &Dock) -> DockSceneLayout {
    let Some(bounds) = valid_rect(config.bounds).then_some(config.bounds) else {
        return DockSceneLayout {
            bounds: Rect::ZERO,
            frames: Vec::new(),
            splitters: Vec::new(),
            preview: None,
        };
    };

    let tab_height = sanitize_tab_height(config.tab_height);
    let (frame_layouts, solved_splitters) =
        solve_dock_scene_geometry(dock, bounds, config.chrome_style);
    let frames = frame_layouts
        .into_iter()
        .filter_map(|layout| {
            let frame = dock.frame(layout.frame)?;
            let rect = layout.rect.intersection(bounds)?;
            let tab_list_height = tab_height.min(rect.height);
            let tab_list_rect = Rect::new(rect.x, rect.y, rect.width, tab_list_height);
            let panel_rect = Rect::new(
                rect.x,
                rect.y + tab_list_height,
                rect.width,
                (rect.height - tab_list_height).max(0.0),
            );
            let tabs = prepare_tabs(config.root, frame_tabs(frame), tab_list_rect);
            let panel = frame.active_panel().and_then(|panel| {
                valid_rect(panel_rect).then(|| DockScenePanel {
                    id: panel_widget_id(config.root, panel.id),
                    frame: frame.id,
                    panel: panel.id,
                    title: panel.title.clone(),
                    rect: panel_rect,
                })
            });
            Some(DockSceneFrame {
                id: frame_widget_id(config.root, frame.id),
                tab_list_id: tab_list_widget_id(config.root, frame.id),
                frame: frame.id,
                rect,
                tab_list_rect,
                tabs,
                panel,
                active: dock.active_frame() == Some(frame.id),
            })
        })
        .collect::<Vec<_>>();

    let splitters = solved_splitters
        .into_iter()
        .filter_map(|splitter| {
            let rect = splitter.rect.intersection(bounds)?;
            Some(DockSceneSplitter {
                id: splitter_widget_id(config.root, &splitter.path),
                path: splitter.path,
                axis: splitter.axis,
                rect,
            })
        })
        .collect();

    let preview = config
        .drop_preview
        .and_then(|target| prepare_preview(config.root, &frames, target));

    DockSceneLayout {
        bounds,
        frames,
        splitters,
        preview,
    }
}

fn prepare_tabs(
    root: WidgetId,
    tabs: Vec<super::FrameTab>,
    tab_list_rect: Rect,
) -> Vec<DockSceneTab> {
    if tabs.is_empty() || !valid_rect(tab_list_rect) {
        return Vec::new();
    }

    let tab_count = u16::try_from(tabs.len()).unwrap_or(u16::MAX);
    let width = (tab_list_rect.width / f32::from(tab_count)).min(PREFERRED_TAB_WIDTH);
    let mut x = tab_list_rect.x;
    tabs.into_iter()
        .map(move |tab| {
            let rect = Rect::new(x, tab_list_rect.y, width, tab_list_rect.height);
            x += width;
            let close_rect = (tab.close_visible && width >= TAB_CLOSE_WIDTH * 2.0).then(|| {
                Rect::new(
                    rect.max_x() - TAB_CLOSE_WIDTH,
                    rect.y,
                    TAB_CLOSE_WIDTH,
                    rect.height,
                )
            });
            DockSceneTab {
                id: tab_widget_id(root, tab.panel),
                close_id: tab_close_widget_id(root, tab.panel),
                panel: tab.panel,
                title: tab.title,
                rect,
                close_rect,
                selected: tab.active,
                close_visible: tab.close_visible,
                draggable: tab.draggable,
            }
        })
        .collect()
}

fn prepare_preview(
    root: WidgetId,
    frames: &[DockSceneFrame],
    target: DockDropTarget,
) -> Option<DockScenePreview> {
    let (frame, kind) = match target {
        DockDropTarget::Tab { frame } => (frame, DockScenePreviewKind::Merge),
        DockDropTarget::Split {
            frame, placement, ..
        } => (frame, DockScenePreviewKind::Split(placement)),
    };
    let target_rect = frames.iter().find(|item| item.frame == frame)?.rect;
    let rect = match kind {
        DockScenePreviewKind::Merge => {
            let inset_x = target_rect.width * DROP_PREVIEW_INSET_FRACTION;
            let inset_y = target_rect.height * DROP_PREVIEW_INSET_FRACTION;
            Rect::new(
                target_rect.x + inset_x,
                target_rect.y + inset_y,
                (target_rect.width - inset_x * 2.0).max(0.0),
                (target_rect.height - inset_y * 2.0).max(0.0),
            )
        }
        DockScenePreviewKind::Split(DockPlacement::Left) => Rect::new(
            target_rect.x,
            target_rect.y,
            target_rect.width * DROP_PREVIEW_EDGE_FRACTION,
            target_rect.height,
        ),
        DockScenePreviewKind::Split(DockPlacement::Right) => {
            let width = target_rect.width * DROP_PREVIEW_EDGE_FRACTION;
            Rect::new(
                target_rect.max_x() - width,
                target_rect.y,
                width,
                target_rect.height,
            )
        }
        DockScenePreviewKind::Split(DockPlacement::Top) => Rect::new(
            target_rect.x,
            target_rect.y,
            target_rect.width,
            target_rect.height * DROP_PREVIEW_EDGE_FRACTION,
        ),
        DockScenePreviewKind::Split(DockPlacement::Bottom) => {
            let height = target_rect.height * DROP_PREVIEW_EDGE_FRACTION;
            Rect::new(
                target_rect.x,
                target_rect.max_y() - height,
                target_rect.width,
                height,
            )
        }
    };
    valid_rect(rect).then(|| DockScenePreview {
        id: root.child("drop-preview"),
        frame,
        rect,
        kind,
    })
}

fn frame_widget_id(root: WidgetId, frame: FrameId) -> WidgetId {
    root.child(("frame", frame.raw()))
}

fn tab_list_widget_id(root: WidgetId, frame: FrameId) -> WidgetId {
    root.child(("tab-list", frame.raw()))
}

fn tab_widget_id(root: WidgetId, panel: PanelId) -> WidgetId {
    root.child(("tab", panel.raw()))
}

fn tab_close_widget_id(root: WidgetId, panel: PanelId) -> WidgetId {
    tab_widget_id(root, panel).child("close")
}

fn panel_widget_id(root: WidgetId, panel: PanelId) -> WidgetId {
    root.child(("panel", panel.raw()))
}

fn splitter_widget_id(root: WidgetId, path: &DockSplitPath) -> WidgetId {
    root.child(("splitter", path))
}

fn sanitize_tab_height(height: f32) -> f32 {
    if height.is_finite() && height > 0.0 {
        height
    } else {
        DEFAULT_TAB_HEIGHT
    }
}

fn valid_rect(rect: Rect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width > 0.0
        && rect.height > 0.0
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
