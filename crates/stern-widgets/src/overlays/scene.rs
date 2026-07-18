use stern_core::{
    ActionId, ActionInvocation, PointerOrder, PointerTarget, PointerTargetPlan, Rect, Response,
    SemanticRole, Shortcut, WidgetId,
};

use super::{
    CommandPaletteOverlay, DropdownItemId, DropdownNavigationIntent, DropdownOverlay, MenuItem,
    MenuNavigationIntent, MenuOverlay, MenuSubmenuOpenIntent, ModalDialogOverlay, OverlayEntry,
    OverlayId, OverlayNavigationInput, OverlayStack, TypeaheadBuffer,
    stack::descendant_overlay_ids,
};

/// Dense, deterministic geometry used by the public overlay scene.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct OverlaySceneMetrics {
    /// Inset between an overlay surface and its rows.
    pub inset: f32,
    /// Height of ordinary rows.
    pub row_height: f32,
    /// Height reserved for separator rows.
    pub separator_height: f32,
}

impl Default for OverlaySceneMetrics {
    fn default() -> Self {
        Self {
            inset: 4.0,
            row_height: 28.0,
            separator_height: 8.0,
        }
    }
}

/// One typed surface in an [`OverlayScene`].
#[derive(Debug, Clone, PartialEq)]
pub enum OverlaySceneSurface {
    /// Action-backed menu or context-menu surface.
    Menu {
        /// Accessible surface label.
        label: String,
        /// Existing menu overlay model.
        overlay: MenuOverlay,
        /// Retained typeahead state for this surface.
        typeahead: TypeaheadBuffer,
    },
    /// Select-like dropdown surface.
    Dropdown {
        /// Accessible surface label.
        label: String,
        /// Existing dropdown overlay model.
        overlay: DropdownOverlay,
        /// Retained typeahead state for this surface.
        typeahead: TypeaheadBuffer,
    },
    /// Filtered command-palette surface.
    CommandPalette {
        /// Accessible surface label.
        label: String,
        /// Existing command-palette overlay model.
        overlay: CommandPaletteOverlay,
    },
    /// Action-backed modal dialog surface.
    Modal {
        /// Existing modal dialog overlay model.
        overlay: ModalDialogOverlay,
    },
    /// Passive popover, tooltip, or drag-preview text surface.
    Passive {
        /// Existing overlay stack entry.
        entry: OverlayEntry,
        /// Accessible surface label.
        label: String,
        /// Plain presentation text.
        text: String,
    },
}

impl OverlaySceneSurface {
    /// Creates a menu or context-menu surface.
    #[must_use]
    pub fn menu(label: impl Into<String>, overlay: MenuOverlay) -> Self {
        Self::Menu {
            label: label.into(),
            overlay,
            typeahead: TypeaheadBuffer::default(),
        }
    }

    /// Creates a dropdown surface.
    #[must_use]
    pub fn dropdown(label: impl Into<String>, overlay: DropdownOverlay) -> Self {
        Self::Dropdown {
            label: label.into(),
            overlay,
            typeahead: TypeaheadBuffer::default(),
        }
    }

    /// Creates a command-palette surface.
    #[must_use]
    pub fn command_palette(label: impl Into<String>, overlay: CommandPaletteOverlay) -> Self {
        Self::CommandPalette {
            label: label.into(),
            overlay,
        }
    }

    /// Creates a modal dialog surface.
    #[must_use]
    pub const fn modal(overlay: ModalDialogOverlay) -> Self {
        Self::Modal { overlay }
    }

    /// Creates a passive popover, tooltip, or drag-preview surface.
    #[must_use]
    pub fn passive(entry: OverlayEntry, label: impl Into<String>, text: impl Into<String>) -> Self {
        Self::Passive {
            entry,
            label: label.into(),
            text: text.into(),
        }
    }

    /// Returns the stack entry that owns this surface.
    #[must_use]
    pub const fn entry(&self) -> &OverlayEntry {
        match self {
            Self::Menu { overlay, .. } => &overlay.entry,
            Self::Dropdown { overlay, .. } => &overlay.entry,
            Self::CommandPalette { overlay, .. } => &overlay.entry,
            Self::Modal { overlay } => &overlay.entry,
            Self::Passive { entry, .. } => entry,
        }
    }

    /// Returns the accessible surface label.
    #[must_use]
    pub fn label(&self) -> &str {
        match self {
            Self::Menu { label, .. }
            | Self::Dropdown { label, .. }
            | Self::CommandPalette { label, .. }
            | Self::Passive { label, .. } => label,
            Self::Modal { overlay } => &overlay.dialog.title,
        }
    }
}

/// Bottom-to-top collection of public overlay surfaces.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlayScene {
    surfaces: Vec<OverlaySceneSurface>,
    metrics: OverlaySceneMetrics,
}

impl OverlayScene {
    /// Creates an empty scene with dense default metrics.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Creates an empty scene with caller-provided metrics.
    #[must_use]
    pub const fn with_metrics(metrics: OverlaySceneMetrics) -> Self {
        Self {
            surfaces: Vec::new(),
            metrics,
        }
    }

    /// Adds a surface at the top of the scene.
    pub fn push(&mut self, surface: OverlaySceneSurface) {
        let id = surface.entry().id;
        if self
            .surfaces
            .iter()
            .any(|candidate| candidate.entry().id == id)
        {
            let closing =
                descendant_overlay_ids(self.surfaces.iter().map(OverlaySceneSurface::entry), id);
            self.surfaces
                .retain(|candidate| !closing.contains(&candidate.entry().id));
        }
        self.surfaces.push(surface);
    }

    /// Returns scene surfaces in bottom-to-top order.
    #[must_use]
    pub fn surfaces(&self) -> &[OverlaySceneSurface] {
        &self.surfaces
    }

    /// Returns mutable scene surfaces in bottom-to-top order.
    #[must_use]
    pub fn surfaces_mut(&mut self) -> &mut [OverlaySceneSurface] {
        &mut self.surfaces
    }

    /// Returns the row-layout metrics used by painting and pointer declaration.
    #[must_use]
    pub const fn metrics(&self) -> OverlaySceneMetrics {
        self.metrics
    }

    /// Adds scene blockers, capture barriers, and enabled row targets to one caller-owned plan.
    ///
    /// `first_order` must be above lower application UI. The returned order is the first unused
    /// ordinal after this scene, allowing the caller to continue the same plan.
    pub fn declare_pointer_targets(
        &self,
        plan: &mut PointerTargetPlan,
        first_order: PointerOrder,
    ) -> PointerOrder {
        let mut ordinal = first_order.raw();
        for surface_index in 0..self.surfaces.len() {
            let entry = self.surfaces[surface_index].entry();
            if entry.captures_lower_layers() || entry.dismissal.closes_on_outside_click() {
                plan.capture_lower_layers(take_order(&mut ordinal));
            }
            plan.blocker(entry.rect, take_order(&mut ordinal));
            plan.with_clip(entry.rect, |plan| {
                for row in self.rows(surface_index) {
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

    #[allow(clippy::too_many_lines)]
    pub(crate) fn rows(&self, surface_index: usize) -> Vec<OverlaySceneRow> {
        let Some(surface) = self.surfaces.get(surface_index) else {
            return Vec::new();
        };
        let mut layout = RowLayout::new(surface.entry().rect, self.metrics);
        let root = surface_widget_id(surface.entry().id);
        let mut rows = Vec::new();

        match surface {
            OverlaySceneSurface::Menu { overlay, .. } => {
                let mut label_occurrences = std::collections::HashMap::<&str, usize>::new();
                let mut separator_occurrence = 0_usize;
                for (visible_index, item) in overlay.visible_items_iter().enumerate() {
                    match item {
                        MenuItem::Action(action) => {
                            let has_submenu =
                                overlay.menu.submenu_for_visible(visible_index).is_some();
                            rows.push(OverlaySceneRow {
                                id: root.child(("overlay-action", action.id.as_str())),
                                rect: layout.next(false),
                                label: action.label.clone(),
                                role: SemanticRole::MenuItem,
                                enabled: action.can_invoke(),
                                selected: overlay.menu.highlighted_action_id() == Some(&action.id),
                                checked: action.state.checked,
                                expanded: has_submenu.then_some(false),
                                action_id: Some(action.id.clone()),
                                menu_columns: true,
                                shortcut: action.shortcut.clone(),
                                kind: OverlaySceneRowKind::Action,
                                behavior: OverlaySceneRowBehavior::Menu {
                                    visible_index,
                                    trigger_action: action.id.clone(),
                                    submenu: has_submenu,
                                },
                            });
                        }
                        MenuItem::Label(label) => {
                            let occurrence = label_occurrences.entry(label.as_str()).or_default();
                            rows.push(OverlaySceneRow::menu_label(
                                root.child(("overlay-label", label.as_str(), *occurrence)),
                                layout.next(false),
                                label.clone(),
                                SemanticRole::Label,
                            ));
                            *occurrence += 1;
                        }
                        MenuItem::Separator => {
                            rows.push(OverlaySceneRow::separator(
                                root.child(("overlay-separator", separator_occurrence)),
                                layout.next(true),
                            ));
                            separator_occurrence += 1;
                        }
                    }
                }
            }
            OverlaySceneSurface::Dropdown { overlay, .. } => {
                for item in overlay.model.items() {
                    rows.push(OverlaySceneRow {
                        id: root.child(("dropdown-item", item.id.raw())),
                        rect: layout.next(false),
                        label: item.label.clone(),
                        role: SemanticRole::MenuItem,
                        enabled: item.enabled,
                        selected: overlay.model.highlighted_id() == Some(item.id)
                            || overlay.model.selected_id() == Some(item.id),
                        checked: Some(overlay.model.selected_id() == Some(item.id)),
                        expanded: None,
                        action_id: None,
                        menu_columns: false,
                        shortcut: None,
                        kind: OverlaySceneRowKind::Action,
                        behavior: OverlaySceneRowBehavior::Dropdown { item_id: item.id },
                    });
                }
            }
            OverlaySceneSurface::CommandPalette { overlay, .. } => {
                rows.push(OverlaySceneRow::passive(
                    root.child("command-query"),
                    layout.next(false),
                    format!("> {}", overlay.palette.query),
                    SemanticRole::SearchField,
                ));
                for (visible_index, entry) in overlay.matches_iter().enumerate() {
                    rows.push(OverlaySceneRow {
                        id: root.child(("command", entry.action_id.as_str())),
                        rect: layout.next(false),
                        label: entry.label.clone(),
                        role: SemanticRole::MenuItem,
                        enabled: entry.enabled,
                        selected: overlay.palette.selected == visible_index,
                        checked: entry.checked,
                        expanded: None,
                        action_id: Some(entry.action_id.clone()),
                        menu_columns: false,
                        shortcut: None,
                        kind: OverlaySceneRowKind::Action,
                        behavior: OverlaySceneRowBehavior::Command {
                            action_id: entry.action_id.clone(),
                        },
                    });
                }
            }
            OverlaySceneSurface::Modal { overlay } => {
                rows.push(OverlaySceneRow::passive(
                    root.child("modal-title"),
                    layout.next(false),
                    overlay.dialog.title.clone(),
                    SemanticRole::Label,
                ));
                if let Some(body) = &overlay.dialog.body {
                    rows.push(OverlaySceneRow::passive(
                        root.child("modal-body"),
                        layout.next(false),
                        body.text.clone(),
                        SemanticRole::Label,
                    ));
                }
                for (visible_index, action) in overlay.visible_actions_iter().enumerate() {
                    rows.push(OverlaySceneRow {
                        id: root.child(("modal-action", action.action_id().as_str())),
                        rect: layout.next(false),
                        label: action.action.label.clone(),
                        role: SemanticRole::Button,
                        enabled: action.can_invoke(),
                        selected: false,
                        checked: action.action.state.checked,
                        expanded: None,
                        action_id: Some(action.action.id.clone()),
                        menu_columns: false,
                        shortcut: None,
                        kind: OverlaySceneRowKind::Action,
                        behavior: OverlaySceneRowBehavior::Modal { visible_index },
                    });
                }
            }
            OverlaySceneSurface::Passive { text, .. } => rows.push(OverlaySceneRow::passive(
                root.child("passive-content"),
                layout.next(false),
                text.clone(),
                SemanticRole::Label,
            )),
        }

        rows.retain(|row| {
            row.rect.width.is_finite()
                && row.rect.height.is_finite()
                && row.rect.width > 0.0
                && row.rect.height > 0.0
        });
        rows
    }

    pub(crate) fn top_keyboard_surface(&self) -> Option<usize> {
        self.surfaces
            .iter()
            .rposition(|surface| surface.entry().receives_focus())
    }

    pub(crate) fn clear_command_palette_query(&mut self, surface_index: usize) -> bool {
        let Some(OverlaySceneSurface::CommandPalette { overlay, .. }) =
            self.surfaces.get_mut(surface_index)
        else {
            return false;
        };
        if overlay.palette.query.is_empty() {
            return false;
        }

        overlay.palette.query.clear();
        overlay.palette.clamp_selection();
        true
    }

    pub(crate) fn navigate(
        &mut self,
        surface_index: usize,
        input: OverlayNavigationInput,
    ) -> OverlaySceneNavigation {
        let Some(surface) = self.surfaces.get_mut(surface_index) else {
            return OverlaySceneNavigation::default();
        };
        match surface {
            OverlaySceneSurface::Menu { overlay, .. } => match overlay.navigate(input) {
                Some(MenuNavigationIntent::Highlighted { .. }) => OverlaySceneNavigation::changed(),
                Some(MenuNavigationIntent::Invoke(invocation)) => {
                    OverlaySceneNavigation::intent(OverlaySceneIntent::Action(invocation))
                }
                Some(MenuNavigationIntent::OpenSubmenu(intent)) => {
                    OverlaySceneNavigation::intent(OverlaySceneIntent::OpenSubmenu(intent))
                }
                Some(MenuNavigationIntent::Close { overlay_id }) => OverlaySceneNavigation::intent(
                    OverlaySceneIntent::Dismiss(OverlaySceneDismissRequest {
                        overlay_id,
                        reason: OverlaySceneDismissReason::Escape,
                        focus_return: None,
                    }),
                ),
                None => OverlaySceneNavigation::default(),
            },
            OverlaySceneSurface::Dropdown { overlay, .. } => match overlay.navigate(input) {
                Some(DropdownNavigationIntent::Highlighted(_)) => OverlaySceneNavigation::changed(),
                Some(DropdownNavigationIntent::Select(item_id)) => OverlaySceneNavigation::intent(
                    OverlaySceneIntent::SelectDropdown(OverlaySceneDropdownSelection {
                        overlay_id: overlay.entry.id,
                        item_id,
                        focus_return: overlay.trigger_id,
                    }),
                ),
                Some(DropdownNavigationIntent::Close {
                    overlay_id,
                    focus_return,
                }) => OverlaySceneNavigation::intent(OverlaySceneIntent::Dismiss(
                    OverlaySceneDismissRequest {
                        overlay_id,
                        reason: OverlaySceneDismissReason::Escape,
                        focus_return: Some(focus_return),
                    },
                )),
                None => OverlaySceneNavigation::default(),
            },
            OverlaySceneSurface::CommandPalette { overlay, .. } => {
                let before = overlay.palette.selected;
                match input {
                    OverlayNavigationInput::Previous => overlay.palette.move_selection(-1),
                    OverlayNavigationInput::Next => overlay.palette.move_selection(1),
                    OverlayNavigationInput::First => overlay.palette.selected = 0,
                    OverlayNavigationInput::Last => {
                        overlay.palette.selected = overlay.palette.match_count().saturating_sub(1);
                    }
                    OverlayNavigationInput::Activate => {
                        return overlay.invocation_for_selected().map_or_else(
                            OverlaySceneNavigation::default,
                            |invocation| {
                                OverlaySceneNavigation::intent(OverlaySceneIntent::Action(
                                    invocation,
                                ))
                            },
                        );
                    }
                    OverlayNavigationInput::Escape => return OverlaySceneNavigation::default(),
                }
                OverlaySceneNavigation {
                    changed: overlay.palette.selected != before,
                    intent: None,
                }
            }
            OverlaySceneSurface::Modal { .. } | OverlaySceneSurface::Passive { .. } => {
                OverlaySceneNavigation::default()
            }
        }
    }

    pub(crate) fn typeahead(&mut self, surface_index: usize, text: &str, now_millis: u64) -> bool {
        match self.surfaces.get_mut(surface_index) {
            Some(OverlaySceneSurface::Menu {
                overlay, typeahead, ..
            }) => overlay
                .menu
                .typeahead(typeahead, text, now_millis)
                .is_some(),
            Some(OverlaySceneSurface::Dropdown {
                overlay, typeahead, ..
            }) => overlay
                .model
                .typeahead(typeahead, text, now_millis)
                .is_some(),
            Some(
                OverlaySceneSurface::CommandPalette { .. }
                | OverlaySceneSurface::Modal { .. }
                | OverlaySceneSurface::Passive { .. },
            )
            | None => false,
        }
    }

    pub(crate) fn activate_row(
        &mut self,
        surface_index: usize,
        row: &OverlaySceneRow,
    ) -> Option<OverlaySceneIntent> {
        let surface = self.surfaces.get_mut(surface_index)?;
        match (&row.behavior, surface) {
            (
                OverlaySceneRowBehavior::Menu {
                    visible_index,
                    trigger_action,
                    submenu,
                },
                OverlaySceneSurface::Menu { overlay, .. },
            ) => {
                if *submenu {
                    Some(OverlaySceneIntent::OpenSubmenu(MenuSubmenuOpenIntent {
                        parent_overlay: overlay.entry.id,
                        trigger_action: trigger_action.clone(),
                        visible_index: *visible_index,
                        source: overlay.source,
                        context: overlay.context.clone(),
                    }))
                } else {
                    overlay
                        .invocation_for_visible(*visible_index)
                        .map(OverlaySceneIntent::Action)
                }
            }
            (
                OverlaySceneRowBehavior::Dropdown { item_id },
                OverlaySceneSurface::Dropdown { overlay, .. },
            ) if row.enabled => Some(OverlaySceneIntent::SelectDropdown(
                OverlaySceneDropdownSelection {
                    overlay_id: overlay.entry.id,
                    item_id: *item_id,
                    focus_return: overlay.trigger_id,
                },
            )),
            (
                OverlaySceneRowBehavior::Command { action_id },
                OverlaySceneSurface::CommandPalette { overlay, .. },
            ) if row.enabled => Some(OverlaySceneIntent::Action(ActionInvocation::new(
                action_id.clone(),
                stern_core::ActionSource::CommandPalette,
                overlay.context.clone(),
            ))),
            (
                OverlaySceneRowBehavior::Modal { visible_index },
                OverlaySceneSurface::Modal { overlay },
            ) => overlay
                .invocation_for_visible(*visible_index)
                .map(OverlaySceneIntent::Action),
            _ => None,
        }
    }

    pub(crate) fn dismissal_request(
        &self,
        outside_activation: Option<stern_core::Point>,
        escape_pressed: bool,
    ) -> Option<OverlaySceneDismissRequest> {
        let mut stack = OverlayStack::new();
        for surface in &self.surfaces {
            stack.open(surface.entry().clone());
        }
        let overlay_id = stack
            .dismissal_requests(outside_activation, escape_pressed)
            .into_iter()
            .next()?;
        let surface = self
            .surfaces
            .iter()
            .find(|surface| surface.entry().id == overlay_id)?;
        let reason = if outside_activation.is_some_and(|point| {
            stack
                .outside_click_close_requests(point)
                .contains(&overlay_id)
        }) {
            OverlaySceneDismissReason::OutsideClick
        } else {
            OverlaySceneDismissReason::Escape
        };
        let focus_return = match surface {
            OverlaySceneSurface::Dropdown { overlay, .. } => Some(overlay.trigger_id),
            OverlaySceneSurface::Modal { overlay } => overlay.dialog.focus.return_focus(),
            _ => None,
        };
        Some(OverlaySceneDismissRequest {
            overlay_id,
            reason,
            focus_return,
        })
    }
}

/// Reason a public overlay scene requested dismissal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OverlaySceneDismissReason {
    /// Primary pointer activation occurred outside a dismissible surface.
    OutsideClick,
    /// Escape requested dismissal.
    Escape,
}

/// Pure close request emitted by an [`OverlayScene`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlaySceneDismissRequest {
    /// Topmost overlay requested for close.
    pub overlay_id: OverlayId,
    /// Input reason for the request.
    pub reason: OverlaySceneDismissReason,
    /// Widget that should regain focus, when known.
    pub focus_return: Option<WidgetId>,
}

/// Pure dropdown selection emitted by an [`OverlayScene`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OverlaySceneDropdownSelection {
    /// Dropdown overlay that owns the item.
    pub overlay_id: OverlayId,
    /// Stable selected item identity.
    pub item_id: DropdownItemId,
    /// Trigger that should regain focus after the application closes the dropdown.
    pub focus_return: WidgetId,
}

/// Application-facing result emitted while evaluating an overlay scene.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OverlaySceneIntent {
    /// Application action invocation; [`crate::Ui`] also queues it in the frame output.
    Action(ActionInvocation),
    /// Nested menu opening request.
    OpenSubmenu(MenuSubmenuOpenIntent),
    /// Dropdown selection request.
    SelectDropdown(OverlaySceneDropdownSelection),
    /// Overlay dismissal request.
    Dismiss(OverlaySceneDismissRequest),
}

/// Frame output returned by [`crate::Ui::overlay_scene`].
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OverlaySceneOutput {
    /// Action and lifecycle intents in deterministic evaluation order.
    pub intents: Vec<OverlaySceneIntent>,
    /// Responses for enabled interactive rows in paint order.
    pub responses: Vec<Response>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum OverlaySceneRowKind {
    Action,
    Passive,
    Separator,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum OverlaySceneRowBehavior {
    None,
    Menu {
        visible_index: usize,
        trigger_action: ActionId,
        submenu: bool,
    },
    Dropdown {
        item_id: DropdownItemId,
    },
    Command {
        action_id: ActionId,
    },
    Modal {
        visible_index: usize,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OverlaySceneRow {
    pub(crate) id: WidgetId,
    pub(crate) rect: Rect,
    pub(crate) label: String,
    pub(crate) role: SemanticRole,
    pub(crate) enabled: bool,
    pub(crate) selected: bool,
    pub(crate) checked: Option<bool>,
    pub(crate) expanded: Option<bool>,
    pub(crate) action_id: Option<ActionId>,
    pub(crate) menu_columns: bool,
    pub(crate) shortcut: Option<Shortcut>,
    pub(crate) kind: OverlaySceneRowKind,
    pub(crate) behavior: OverlaySceneRowBehavior,
}

impl OverlaySceneRow {
    fn passive(id: WidgetId, rect: Rect, label: String, role: SemanticRole) -> Self {
        Self {
            id,
            rect,
            label,
            role,
            enabled: false,
            selected: false,
            checked: None,
            expanded: None,
            action_id: None,
            menu_columns: false,
            shortcut: None,
            kind: OverlaySceneRowKind::Passive,
            behavior: OverlaySceneRowBehavior::None,
        }
    }

    fn menu_label(id: WidgetId, rect: Rect, label: String, role: SemanticRole) -> Self {
        Self {
            menu_columns: true,
            ..Self::passive(id, rect, label, role)
        }
    }

    fn separator(id: WidgetId, rect: Rect) -> Self {
        Self {
            id,
            rect,
            label: "Separator".to_owned(),
            role: SemanticRole::Custom("separator".to_owned()),
            enabled: false,
            selected: false,
            checked: None,
            expanded: None,
            action_id: None,
            menu_columns: false,
            shortcut: None,
            kind: OverlaySceneRowKind::Separator,
            behavior: OverlaySceneRowBehavior::None,
        }
    }

    pub(crate) fn actionable(&self) -> bool {
        self.enabled
            && self.rect.width.is_finite()
            && self.rect.height.is_finite()
            && self.rect.width > 0.0
            && self.rect.height > 0.0
            && !matches!(self.behavior, OverlaySceneRowBehavior::None)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct OverlaySceneNavigation {
    pub(crate) changed: bool,
    pub(crate) intent: Option<OverlaySceneIntent>,
}

impl OverlaySceneNavigation {
    fn changed() -> Self {
        Self {
            changed: true,
            intent: None,
        }
    }

    fn intent(intent: OverlaySceneIntent) -> Self {
        Self {
            changed: true,
            intent: Some(intent),
        }
    }
}

struct RowLayout {
    x: f32,
    y: f32,
    width: f32,
    max_y: f32,
    row_height: f32,
    separator_height: f32,
}

impl RowLayout {
    fn new(rect: Rect, metrics: OverlaySceneMetrics) -> Self {
        let inset = finite_non_negative(metrics.inset);
        let y = rect.y + inset;
        Self {
            x: rect.x + inset,
            y,
            width: (rect.width - inset * 2.0).max(0.0),
            max_y: (rect.max_y() - inset).max(y),
            row_height: finite_non_negative(metrics.row_height),
            separator_height: finite_non_negative(metrics.separator_height),
        }
    }

    fn next(&mut self, separator: bool) -> Rect {
        let requested = if separator {
            self.separator_height
        } else {
            self.row_height
        };
        let height = requested.min((self.max_y - self.y).max(0.0));
        let rect = Rect::new(self.x, self.y, self.width, height);
        self.y += height;
        rect
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn surface_widget_id(id: OverlayId) -> WidgetId {
    WidgetId::from_raw(id.raw()).child("overlay-scene")
}

fn take_order(ordinal: &mut u64) -> PointerOrder {
    let order = PointerOrder::new(*ordinal);
    *ordinal = ordinal.saturating_add(1);
    order
}
