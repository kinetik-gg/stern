//! Public, deterministic layout contract for painted editor chrome.

use std::collections::HashMap;

use kinetik_ui_core::{
    ActionContext, ActionId, ActionInvocation, PointerOrder, PointerTarget, PointerTargetPlan,
    Rect, Response, SemanticRole, WidgetId,
};

use crate::{PanelId, TabStripTarget};

use super::{
    ChromeOverflowItem, MenuBar, MenuBarMenuId, StatusBar, StatusItemId, TabStrip, Toolbar,
    ToolbarGroupId, project_chrome_overflow,
};

/// Stable identity for one item presented by a chrome scene.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChromeSceneItemKey {
    /// Top-level menu heading.
    Menu(MenuBarMenuId),
    /// Action-backed toolbar item, scoped by its group.
    Toolbar {
        /// Stable toolbar group identity.
        group: ToolbarGroupId,
        /// Stable application action identity.
        action: ActionId,
    },
    /// Frame tab header.
    Tab(PanelId),
    /// Passive status-bar item.
    Status(StatusItemId),
}

/// One of the four public editor-chrome surfaces.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ChromeSurfaceKind {
    /// Top-level application menu headings.
    MenuBar,
    /// Action-backed tool controls.
    Toolbar,
    /// Frame tab headers.
    TabStrip,
    /// Passive application status.
    StatusBar,
}

/// Caller-owned geometry and action context for one chrome scene.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeSceneConfig {
    /// Stable root used to derive every surface and item widget ID.
    pub root: WidgetId,
    /// Menu-bar bounds.
    pub menu_bar_rect: Rect,
    /// Toolbar bounds.
    pub toolbar_rect: Rect,
    /// Tab-strip bounds.
    pub tab_strip_rect: Rect,
    /// Status-bar bounds.
    pub status_bar_rect: Rect,
    /// Context captured by toolbar action invocations.
    pub toolbar_context: ActionContext,
    /// Desired widths keyed by stable model identity.
    pub widths: HashMap<ChromeSceneItemKey, f32>,
    /// Width reserved for a compact overflow trigger.
    pub overflow_trigger_width: f32,
    /// Width reserved for a visible tab close affordance.
    pub tab_close_width: f32,
}

impl ChromeSceneConfig {
    /// Creates a scene config with caller-owned surface bounds.
    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub fn new(
        root: WidgetId,
        menu_bar_rect: Rect,
        toolbar_rect: Rect,
        tab_strip_rect: Rect,
        status_bar_rect: Rect,
        toolbar_context: ActionContext,
    ) -> Self {
        Self {
            root,
            menu_bar_rect,
            toolbar_rect,
            tab_strip_rect,
            status_bar_rect,
            toolbar_context,
            widths: HashMap::new(),
            overflow_trigger_width: 28.0,
            tab_close_width: 20.0,
        }
    }

    /// Adds or replaces one stable item's desired width.
    #[must_use]
    pub fn with_width(mut self, key: ChromeSceneItemKey, width: f32) -> Self {
        self.widths.insert(key, width);
        self
    }

    /// Replaces all desired item widths.
    #[must_use]
    pub fn with_widths(
        mut self,
        widths: impl IntoIterator<Item = (ChromeSceneItemKey, f32)>,
    ) -> Self {
        self.widths = widths.into_iter().collect();
        self
    }

    /// Sets the compact overflow-trigger width.
    #[must_use]
    pub const fn with_overflow_trigger_width(mut self, width: f32) -> Self {
        self.overflow_trigger_width = width;
        self
    }

    /// Sets the visible tab close-affordance width.
    #[must_use]
    pub const fn with_tab_close_width(mut self, width: f32) -> Self {
        self.tab_close_width = width;
        self
    }
}

/// Borrowed public chrome models evaluated as one frame-local scene.
#[derive(Debug)]
pub struct ChromeScene<'a> {
    /// Caller-owned geometry, widths, and stable root.
    pub config: ChromeSceneConfig,
    /// Top-level menu-bar model.
    pub menu_bar: &'a MenuBar,
    /// Action-backed toolbar model.
    pub toolbar: &'a Toolbar,
    /// Frame tab-strip model.
    pub tab_strip: &'a TabStrip,
    /// Passive status-bar model.
    pub status_bar: &'a StatusBar,
}

impl<'a> ChromeScene<'a> {
    /// Creates a borrowed scene over existing chrome models.
    #[must_use]
    pub const fn new(
        config: ChromeSceneConfig,
        menu_bar: &'a MenuBar,
        toolbar: &'a Toolbar,
        tab_strip: &'a TabStrip,
        status_bar: &'a StatusBar,
    ) -> Self {
        Self {
            config,
            menu_bar,
            toolbar,
            tab_strip,
            status_bar,
        }
    }

    /// Returns the stable widget ID for one chrome surface.
    #[must_use]
    pub fn surface_widget_id(&self, kind: ChromeSurfaceKind) -> WidgetId {
        surface_widget_id(self.config.root, kind)
    }

    /// Returns the stable widget ID for one model item.
    #[must_use]
    pub fn item_widget_id(&self, key: &ChromeSceneItemKey) -> WidgetId {
        item_widget_id(self.config.root, key)
    }

    /// Returns the stable widget ID for one tab close affordance.
    #[must_use]
    pub fn tab_close_widget_id(&self, panel: PanelId) -> WidgetId {
        item_widget_id(self.config.root, &ChromeSceneItemKey::Tab(panel)).child("close")
    }

    /// Returns the stable widget ID for one surface's overflow trigger.
    #[must_use]
    pub fn overflow_widget_id(&self, kind: ChromeSurfaceKind) -> WidgetId {
        overflow_widget_id(self.config.root, kind)
    }

    /// Adds surface blockers and enabled chrome targets to one caller-owned plan.
    ///
    /// `first_order` must be ordered relative to the rest of the frame. The
    /// returned order is the first unused ordinal after this scene.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let mut ordinal = first_order.raw();
        for surface in self.layout().surfaces {
            plan.blocker(surface.rect, take_order(&mut ordinal));
            plan.with_clip(surface.rect, |plan| {
                for row in surface.rows {
                    if row.actionable() {
                        plan.target(PointerTarget::new(
                            row.id,
                            row.rect,
                            take_order(&mut ordinal),
                        ));
                    }
                }
            });
        }
        PointerOrder::new(ordinal)
    }

    pub(crate) fn layout(&self) -> ChromeSceneLayout {
        let mut surfaces = Vec::new();
        if let Some(surface) = self.menu_bar_layout() {
            surfaces.push(surface);
        }
        if let Some(surface) = self.toolbar_layout() {
            surfaces.push(surface);
        }
        if let Some(surface) = self.tab_strip_layout() {
            surfaces.push(surface);
        }
        if let Some(surface) = self.status_bar_layout() {
            surfaces.push(surface);
        }
        ChromeSceneLayout { surfaces }
    }

    fn menu_bar_layout(&self) -> Option<ChromeSurfaceLayout> {
        let kind = ChromeSurfaceKind::MenuBar;
        let rect = valid_surface_rect(self.config.menu_bar_rect)?;
        let items = self
            .menu_bar
            .menus()
            .iter()
            .filter(|menu| menu.has_visible_items())
            .map(|menu| {
                let key = ChromeSceneItemKey::Menu(menu.id);
                ChromeSourceItem {
                    width: self.width(&key),
                    key,
                    label: menu.title.clone(),
                    role: SemanticRole::MenuItem,
                    enabled: true,
                    selected: self.menu_bar.active_id() == Some(menu.id),
                    checked: None,
                    behavior: Some(ChromeRowBehavior::OpenMenu(menu.id)),
                    row_kind: ChromeSceneRowKind::Menu,
                }
            })
            .collect::<Vec<_>>();
        Some(self.project_surface(kind, rect, &items))
    }

    fn toolbar_layout(&self) -> Option<ChromeSurfaceLayout> {
        let kind = ChromeSurfaceKind::Toolbar;
        let rect = valid_surface_rect(self.config.toolbar_rect)?;
        let mut items = Vec::new();
        for group in self.toolbar.visible_groups_iter() {
            for item in group.visible_items_iter() {
                let key = ChromeSceneItemKey::Toolbar {
                    group: group.id,
                    action: item.action_id().clone(),
                };
                items.push(ChromeSourceItem {
                    width: self.width(&key),
                    key,
                    label: item.label().to_owned(),
                    role: if item.icon().is_some() {
                        SemanticRole::IconButton
                    } else {
                        SemanticRole::Button
                    },
                    enabled: item.enabled(),
                    selected: item.selected(),
                    checked: item.checked(),
                    behavior: item
                        .invocation(self.config.toolbar_context.clone())
                        .map(ChromeRowBehavior::Action),
                    row_kind: ChromeSceneRowKind::Toolbar,
                });
            }
        }
        Some(self.project_surface(kind, rect, &items))
    }

    fn tab_strip_layout(&self) -> Option<ChromeSurfaceLayout> {
        let kind = ChromeSurfaceKind::TabStrip;
        let rect = valid_surface_rect(self.config.tab_strip_rect)?;
        let items = self
            .tab_strip
            .tabs()
            .iter()
            .enumerate()
            .map(|(index, tab)| {
                let key = ChromeSceneItemKey::Tab(tab.panel);
                ChromeSourceItem {
                    width: self.width(&key),
                    key,
                    label: tab.title.clone(),
                    role: SemanticRole::Tab,
                    enabled: true,
                    selected: tab.active,
                    checked: None,
                    behavior: Some(ChromeRowBehavior::ActivateTab(TabStripTarget::new(
                        tab.panel, index,
                    ))),
                    row_kind: ChromeSceneRowKind::Tab {
                        close: tab
                            .close_visible
                            .then_some(TabStripTarget::new(tab.panel, index)),
                    },
                }
            })
            .collect::<Vec<_>>();
        Some(self.project_surface(kind, rect, &items))
    }

    fn status_bar_layout(&self) -> Option<ChromeSurfaceLayout> {
        let kind = ChromeSurfaceKind::StatusBar;
        let rect = valid_surface_rect(self.config.status_bar_rect)?;
        let items = self
            .status_bar
            .visible_items_iter()
            .map(|item| {
                let key = ChromeSceneItemKey::Status(item.id);
                ChromeSourceItem {
                    width: self.width(&key),
                    key,
                    label: item.text.clone(),
                    role: SemanticRole::Label,
                    enabled: false,
                    selected: false,
                    checked: None,
                    behavior: None,
                    row_kind: ChromeSceneRowKind::Status,
                }
            })
            .collect::<Vec<_>>();
        Some(self.project_surface(kind, rect, &items))
    }

    fn project_surface(
        &self,
        kind: ChromeSurfaceKind,
        rect: Rect,
        items: &[ChromeSourceItem],
    ) -> ChromeSurfaceLayout {
        let projection = project_chrome_overflow(
            items
                .iter()
                .map(|item| ChromeOverflowItem::new(item.key.clone(), item.width)),
            rect.width,
            self.config.overflow_trigger_width,
        );
        let mut rows = Vec::new();
        for placement in projection.visible() {
            let Some(source) = items.iter().find(|item| item.key == placement.key) else {
                continue;
            };
            let item_rect = Rect::new(rect.x + placement.x, rect.y, placement.width, rect.height);
            if item_rect.is_empty() {
                continue;
            }
            match source.row_kind {
                ChromeSceneRowKind::Tab { close } => {
                    self.push_tab_rows(&mut rows, source, item_rect, close);
                }
                _ => rows.push(source.row(self.config.root, item_rect)),
            }
        }
        if let Some(trigger) = projection.trigger() {
            let trigger_rect = Rect::new(rect.x + trigger.x, rect.y, trigger.width, rect.height);
            if !trigger_rect.is_empty() {
                let request = ChromeOverflowRequest {
                    surface: kind,
                    trigger_rect,
                    items: projection.overflowed().to_vec(),
                };
                rows.push(ChromeSceneRow {
                    id: overflow_widget_id(self.config.root, kind),
                    rect: trigger_rect,
                    label: "More".to_owned(),
                    role: SemanticRole::Button,
                    enabled: true,
                    selected: false,
                    checked: None,
                    action_id: None,
                    behavior: Some(ChromeRowBehavior::OpenOverflow(request)),
                    kind: ChromeSceneRowKind::Overflow,
                });
            }
        }
        ChromeSurfaceLayout {
            id: surface_widget_id(self.config.root, kind),
            kind,
            rect,
            rows,
        }
    }

    fn push_tab_rows(
        &self,
        rows: &mut Vec<ChromeSceneRow>,
        source: &ChromeSourceItem,
        rect: Rect,
        close: Option<TabStripTarget>,
    ) {
        let close_width = close.map_or(0.0, |_| {
            finite_non_negative(self.config.tab_close_width).min(rect.width)
        });
        let body_rect = Rect::new(rect.x, rect.y, rect.width - close_width, rect.height);
        if !body_rect.is_empty() {
            rows.push(source.row(self.config.root, body_rect));
        }
        if let Some(target) = close {
            let close_rect =
                Rect::new(rect.max_x() - close_width, rect.y, close_width, rect.height);
            if !close_rect.is_empty() {
                rows.push(ChromeSceneRow {
                    id: item_widget_id(self.config.root, &source.key).child("close"),
                    rect: close_rect,
                    label: format!("Close {}", source.label),
                    role: SemanticRole::Button,
                    enabled: true,
                    selected: false,
                    checked: None,
                    action_id: None,
                    behavior: Some(ChromeRowBehavior::CloseTab(target)),
                    kind: ChromeSceneRowKind::TabClose,
                });
            }
        }
    }

    fn width(&self, key: &ChromeSceneItemKey) -> f32 {
        self.config.widths.get(key).copied().unwrap_or(0.0)
    }
}

/// Typed request to open one compact chrome overflow surface.
#[derive(Debug, Clone, PartialEq)]
pub struct ChromeOverflowRequest {
    /// Surface that owns the overflow trigger.
    pub surface: ChromeSurfaceKind,
    /// Painted trigger bounds suitable for overlay anchoring.
    pub trigger_rect: Rect,
    /// Overflowed stable keys in their source presentation order.
    pub items: Vec<ChromeSceneItemKey>,
}

/// Application-owned intent emitted by a painted chrome scene.
#[derive(Debug, Clone, PartialEq)]
pub enum ChromeSceneIntent {
    /// A top-level menu heading requested its menu surface.
    OpenMenu {
        /// Stable menu identity.
        menu: MenuBarMenuId,
        /// Painted heading bounds suitable for overlay anchoring.
        anchor: Rect,
    },
    /// An enabled toolbar action was invoked.
    Action(ActionInvocation),
    /// A frame tab requested activation.
    ActivateTab(TabStripTarget),
    /// A frame tab requested closure.
    CloseTab(TabStripTarget),
    /// One compact surface requested an overflow overlay.
    OpenOverflow(ChromeOverflowRequest),
}

/// Per-frame result of evaluating a chrome scene.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct ChromeSceneOutput {
    /// Interactive row responses in painted order.
    pub responses: Vec<Response>,
    /// Application-owned intents in event order.
    pub intents: Vec<ChromeSceneIntent>,
}

#[derive(Debug, Clone, PartialEq)]
struct ChromeSourceItem {
    key: ChromeSceneItemKey,
    width: f32,
    label: String,
    role: SemanticRole,
    enabled: bool,
    selected: bool,
    checked: Option<bool>,
    behavior: Option<ChromeRowBehavior>,
    row_kind: ChromeSceneRowKind,
}

impl ChromeSourceItem {
    fn row(&self, root: WidgetId, rect: Rect) -> ChromeSceneRow {
        let action_id = match &self.key {
            ChromeSceneItemKey::Toolbar { action, .. } => Some(action.clone()),
            ChromeSceneItemKey::Menu(_)
            | ChromeSceneItemKey::Tab(_)
            | ChromeSceneItemKey::Status(_) => None,
        };
        ChromeSceneRow {
            id: item_widget_id(root, &self.key),
            rect,
            label: self.label.clone(),
            role: self.role.clone(),
            enabled: self.enabled,
            selected: self.selected,
            checked: self.checked,
            action_id,
            behavior: self.behavior.clone(),
            kind: self.row_kind,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChromeSceneLayout {
    pub(crate) surfaces: Vec<ChromeSurfaceLayout>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChromeSurfaceLayout {
    pub(crate) id: WidgetId,
    pub(crate) kind: ChromeSurfaceKind,
    pub(crate) rect: Rect,
    pub(crate) rows: Vec<ChromeSceneRow>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ChromeSceneRow {
    pub(crate) id: WidgetId,
    pub(crate) rect: Rect,
    pub(crate) label: String,
    pub(crate) role: SemanticRole,
    pub(crate) enabled: bool,
    pub(crate) selected: bool,
    pub(crate) checked: Option<bool>,
    pub(crate) action_id: Option<ActionId>,
    behavior: Option<ChromeRowBehavior>,
    pub(crate) kind: ChromeSceneRowKind,
}

impl ChromeSceneRow {
    pub(crate) fn interactive(&self) -> bool {
        !matches!(self.kind, ChromeSceneRowKind::Status)
    }

    pub(crate) fn actionable(&self) -> bool {
        self.interactive()
            && self.enabled
            && self.behavior.is_some()
            && !self.rect.is_empty()
            && rect_is_finite(self.rect)
    }

    pub(crate) fn intent(&self) -> Option<ChromeSceneIntent> {
        match self.behavior.clone()? {
            ChromeRowBehavior::OpenMenu(menu) => Some(ChromeSceneIntent::OpenMenu {
                menu,
                anchor: self.rect,
            }),
            ChromeRowBehavior::Action(invocation) => Some(ChromeSceneIntent::Action(invocation)),
            ChromeRowBehavior::ActivateTab(target) => Some(ChromeSceneIntent::ActivateTab(target)),
            ChromeRowBehavior::CloseTab(target) => Some(ChromeSceneIntent::CloseTab(target)),
            ChromeRowBehavior::OpenOverflow(request) => {
                Some(ChromeSceneIntent::OpenOverflow(request))
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum ChromeRowBehavior {
    OpenMenu(MenuBarMenuId),
    Action(ActionInvocation),
    ActivateTab(TabStripTarget),
    CloseTab(TabStripTarget),
    OpenOverflow(ChromeOverflowRequest),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ChromeSceneRowKind {
    Menu,
    Toolbar,
    Tab { close: Option<TabStripTarget> },
    TabClose,
    Status,
    Overflow,
}

fn surface_widget_id(root: WidgetId, kind: ChromeSurfaceKind) -> WidgetId {
    root.child(("chrome-surface", kind))
}

fn item_widget_id(root: WidgetId, key: &ChromeSceneItemKey) -> WidgetId {
    match key {
        ChromeSceneItemKey::Menu(menu) => {
            surface_widget_id(root, ChromeSurfaceKind::MenuBar).child(("menu", menu.raw()))
        }
        ChromeSceneItemKey::Toolbar { group, action } => {
            surface_widget_id(root, ChromeSurfaceKind::Toolbar)
                .child(("group", group.raw()))
                .child(("action", action.as_str()))
        }
        ChromeSceneItemKey::Tab(panel) => {
            surface_widget_id(root, ChromeSurfaceKind::TabStrip).child(("tab", panel.raw()))
        }
        ChromeSceneItemKey::Status(status) => {
            surface_widget_id(root, ChromeSurfaceKind::StatusBar).child(("status", status.raw()))
        }
    }
}

fn overflow_widget_id(root: WidgetId, kind: ChromeSurfaceKind) -> WidgetId {
    surface_widget_id(root, kind).child("overflow")
}

fn valid_surface_rect(rect: Rect) -> Option<Rect> {
    (!rect.is_empty()
        && rect_is_finite(rect)
        && rect.max_x().is_finite()
        && rect.max_y().is_finite())
    .then_some(rect)
}

fn rect_is_finite(rect: Rect) -> bool {
    rect.x.is_finite() && rect.y.is_finite() && rect.width.is_finite() && rect.height.is_finite()
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
