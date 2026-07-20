mod system_feedback;

use stern_core::{
    Brush, ClipId, ComponentState, IconPrimitive, Key, KeyEvent, KeyState, Point, Primitive, Rect,
    RectPrimitive, RepaintRequest, SemanticAction, SemanticActionKind, SemanticNode, SemanticRole,
    Size, SpacingRole, Stroke, TextPrimitive, TextRole, fit_box,
};
use stern_text::{TextLayoutKey, TextOverflow, TextStyle};

use super::{Ui, response_activated};
use crate::chrome::{
    ApplicationBar, ApplicationBarIntent, ApplicationBarOutput, ApplicationBarRow,
    ApplicationBarRowKind, ChromeScene, ChromeSceneIntent, ChromeSceneOutput, ChromeSceneRow,
    ChromeSceneRowKind, ChromeSurfaceKind, PreparedApplicationBar, WindowSystemMenuTrigger,
};
use crate::components::{
    ButtonFocusPlacement, TabFocusPlacement, button_surface_primitives, tab_surface_primitives,
};
use crate::icon_button;

impl Ui<'_> {
    /// Paints and evaluates one public application menu/workspace composition.
    ///
    /// Use the same [`PreparedApplicationBar`] for the pointer prepass and this
    /// evaluation so hit, paint, response, output, and semantic geometry agree.
    #[allow(clippy::too_many_lines)]
    pub fn application_bar(
        &mut self,
        bar: &mut ApplicationBar,
        layout: &PreparedApplicationBar,
    ) -> ApplicationBarOutput {
        if !layout.matches(bar, self.theme()) {
            return ApplicationBarOutput::default();
        }
        let mut output = ApplicationBarOutput {
            drag_safe_regions: layout.drag_safe.into_iter().collect(),
            ..ApplicationBarOutput::default()
        };
        let root = bar.config.root;
        self.register_id(root);
        self.paint_application_bar_surface(layout.bounds);
        let composite_children = layout
            .menu_bounds
            .zip(layout.workspace_bounds)
            .map_or_else(Vec::new, |_| {
                vec![layout.menu_composite, layout.workspace_composite]
            });
        self.push_semantic_node(
            SemanticNode::new(
                root,
                SemanticRole::Custom("application-bar".to_owned()),
                layout.bounds,
            )
            .with_label("Application bar")
            .with_children(composite_children),
        );
        let (Some(menu_bounds), Some(workspace_bounds)) =
            (layout.menu_bounds, layout.workspace_bounds)
        else {
            return output;
        };
        reconcile_workspace_focus(self, bar, layout);
        let events = self.input().keyboard.events.clone();
        let menu_owned = handle_menu_keyboard(self, bar, layout, &events, &mut output);
        if !menu_owned {
            handle_workspace_keyboard(self, bar, layout, &events);
        }

        self.register_id(layout.menu_composite);
        self.register_id(layout.workspace_composite);
        self.push_semantic_node(
            SemanticNode::new(
                layout.menu_composite,
                SemanticRole::Custom("menu-bar".to_owned()),
                menu_bounds,
            )
            .with_label("Application menu")
            .with_children(layout.menu_rows.iter().map(|row| row.id)),
        );
        self.push_semantic_node(
            SemanticNode::new(
                layout.workspace_composite,
                SemanticRole::TabList,
                workspace_bounds,
            )
            .with_label("Workspaces")
            .with_children(layout.workspace_rows.iter().map(|row| row.id)),
        );

        let clip = ClipId::from_raw(root.child("application-bar-clip").raw());
        self.primitive(Primitive::ClipBegin {
            id: clip,
            rect: layout.bounds,
        });
        let mut evaluated = Vec::new();
        for row in layout.menu_rows.iter().chain(&layout.workspace_rows) {
            self.register_id(row.id);
            let mut response = self.pressable_with_id(row.id, row.rect, !row.enabled);
            let selected = match row.kind {
                ApplicationBarRowKind::Menu(id) => bar.menu_bar.active_id() == Some(id),
                ApplicationBarRowKind::Workspace(target) => {
                    let selected = bar.workspaces[target.index].active;
                    if response.state.focused && row.enabled {
                        bar.workspace_focus = Some(target.id);
                    }
                    selected
                }
            };
            response.state.selected = selected;
            if response_activated(&response) {
                match row.kind {
                    ApplicationBarRowKind::Menu(menu) => {
                        let was_active = bar.menu_bar.active_id() == Some(menu);
                        if bar.menu_bar.toggle(menu) {
                            let intent = if was_active {
                                ApplicationBarIntent::DismissMenu { menu }
                            } else {
                                ApplicationBarIntent::OpenMenu {
                                    menu,
                                    anchor: row.rect,
                                }
                            };
                            output.intents.push(intent);
                        }
                    }
                    ApplicationBarRowKind::Workspace(target) if row.enabled => output
                        .intents
                        .push(ApplicationBarIntent::ActivateWorkspace(target)),
                    ApplicationBarRowKind::Workspace(_) => {}
                }
            } else if let ApplicationBarRowKind::Menu(menu) = row.kind
                && response.state.hovered
                && bar.menu_bar.active_id().is_some()
                && bar.menu_bar.active_id() != Some(menu)
                && bar.menu_bar.hover_open(menu)
            {
                output.intents.push(ApplicationBarIntent::OpenMenu {
                    menu,
                    anchor: row.rect,
                });
            }
            evaluated.push((row, response, selected));
            output.responses.push(response);
        }
        let menu_roving = layout
            .menu_rows
            .iter()
            .find(|row| self.memory().focused() == Some(row.id))
            .or_else(|| {
                bar.menu_bar
                    .active_id()
                    .and_then(|active| menu_row(layout, active))
            })
            .or_else(|| layout.menu_rows.first())
            .map(|row| row.id);
        for (row, response, selected) in evaluated {
            self.paint_application_bar_row(row, response, selected);
            self.push_semantic_node(application_bar_row_semantics(
                bar,
                row,
                response,
                selected,
                menu_roving,
            ));
        }
        self.primitive(Primitive::ClipEnd { id: clip });
        output
    }

    /// Paints and evaluates one platform-owned window system-menu trigger.
    ///
    /// Call [`WindowSystemMenuTrigger::declare_pointer_target`] before lower
    /// titlebar targets so explicit paint order can keep this trigger on top.
    pub fn window_system_menu_trigger(
        &mut self,
        trigger: &WindowSystemMenuTrigger,
    ) -> Option<stern_core::Response> {
        if !trigger.is_valid() {
            return None;
        }

        self.register_id(trigger.widget_id());
        let theme = self.theme;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = icon_button(
            trigger.widget_id(),
            trigger.titlebar_rect(),
            trigger.icon(),
            "Open window system menu",
            input,
            memory,
            theme,
            false,
        );
        let response = self.push_interactive(output);
        if response_activated(&response) {
            let requested = self
                .runtime
                .request_window_system_menu(trigger.request_position());
            debug_assert!(requested, "validated system-menu position");
        }
        Some(response)
    }

    /// Paints and evaluates one public editor-chrome scene.
    ///
    /// Call [`ChromeScene::declare_pointer_targets`] from the closure passed to
    /// [`Self::resolve_pointer_targets`] before evaluating lower UI and this
    /// scene. Toolbar actions are also appended to the frame action queue.
    pub fn chrome_scene(&mut self, scene: &ChromeScene<'_>) -> ChromeSceneOutput {
        let mut output = ChromeSceneOutput::default();
        for surface in scene.layout().surfaces {
            self.register_id(surface.id);
            self.paint_chrome_surface(surface.kind, surface.rect);
            let children = surface.rows.iter().map(|row| row.id).collect::<Vec<_>>();
            self.push_semantic_node(chrome_surface_semantics(
                surface.id,
                surface.kind,
                surface.rect,
                children,
            ));

            let clip = ClipId::from_raw(surface.id.child("clip").raw());
            self.primitive(Primitive::ClipBegin {
                id: clip,
                rect: surface.rect,
            });
            for row in surface.rows {
                self.register_id(row.id);
                let response = row.interactive().then(|| {
                    let response = self.pressable_with_id(row.id, row.rect, !row.enabled);
                    if response.clicked || response.state.pressed {
                        self.request_repaint(RepaintRequest::NextFrame);
                    }
                    output.responses.push(response);
                    response
                });

                self.paint_chrome_row(&row, response.as_ref());
                self.push_semantic_node(chrome_row_semantics(&row, response.as_ref()));

                if response.is_some_and(|response| response.clicked)
                    && let Some(intent) = row.intent()
                {
                    if let ChromeSceneIntent::Action(invocation) = &intent {
                        self.push_action(invocation.clone());
                    }
                    self.request_repaint(RepaintRequest::NextFrame);
                    output.intents.push(intent);
                }
            }
            self.primitive(Primitive::ClipEnd { id: clip });
        }
        output
    }

    fn paint_chrome_surface(&mut self, kind: ChromeSurfaceKind, rect: Rect) {
        let fill = match kind {
            ChromeSurfaceKind::TabStrip => self.theme.colors.surface.sunken,
            ChromeSurfaceKind::MenuBar
            | ChromeSurfaceKind::Toolbar
            | ChromeSurfaceKind::StatusBar => self.theme.colors.surface.panel,
        };
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(fill)),
            stroke: Some(Stroke::new(
                self.theme.strokes.hairline,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_application_bar_surface(&mut self, rect: Rect) {
        self.primitive(Primitive::Rect(RectPrimitive {
            rect,
            fill: Some(Brush::Solid(self.theme.colors.surface.application)),
            stroke: Some(Stroke::new(
                self.theme.strokes.hairline,
                Brush::Solid(self.theme.colors.border.subtle),
            )),
            radius: self.theme.radii.none,
        }));
    }

    fn paint_application_bar_row(
        &mut self,
        row: &ApplicationBarRow,
        response: stern_core::Response,
        selected: bool,
    ) {
        let state = ComponentState {
            hovered: response.state.hovered,
            pressed: response.state.pressed,
            focused: response.state.focused,
            disabled: !row.enabled,
            selected,
        };
        let foreground = match row.kind {
            ApplicationBarRowKind::Menu(_) => {
                let recipe = self.theme.button(state);
                self.extend(button_surface_primitives(
                    self.theme,
                    &recipe,
                    state,
                    row.rect,
                    recipe.radius,
                    ButtonFocusPlacement::Inward,
                ));
                recipe.foreground
            }
            ApplicationBarRowKind::Workspace(_) => {
                let recipe = self.theme.tab(state);
                self.extend(tab_surface_primitives(
                    self.theme,
                    &recipe,
                    state,
                    row.rect,
                    recipe.radius,
                    TabFocusPlacement::Inward,
                ));
                recipe.foreground
            }
        };
        let font = self.theme.font(TextRole::Label);
        self.primitive(Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                row.rect.x + self.theme.controls.padding_x,
                row.rect.y + (row.rect.height - font.line_height).max(0.0) * 0.5 + font.size,
            ),
            text: row.label.clone(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(foreground),
        }));
    }

    fn paint_chrome_row(&mut self, row: &ChromeSceneRow, response: Option<&stern_core::Response>) {
        let state = ComponentState {
            hovered: response.is_some_and(|response| response.state.hovered),
            pressed: response.is_some_and(|response| response.state.pressed),
            focused: response.is_some_and(|response| response.state.focused),
            disabled: !row.enabled,
            selected: row.selected,
        };
        let foreground = match row.kind {
            ChromeSceneRowKind::Status => self.theme.label(TextRole::Label, true).foreground,
            ChromeSceneRowKind::Tab { .. } => {
                let recipe = self.theme.tab(state);
                for primitive in tab_surface_primitives(
                    self.theme,
                    &recipe,
                    state,
                    row.rect,
                    recipe.radius,
                    TabFocusPlacement::Inward,
                ) {
                    self.primitive(primitive);
                }
                recipe.foreground
            }
            ChromeSceneRowKind::Menu
            | ChromeSceneRowKind::Toolbar
            | ChromeSceneRowKind::TabClose
            | ChromeSceneRowKind::Overflow => {
                let recipe = self.theme.button(state);
                for primitive in button_surface_primitives(
                    self.theme,
                    &recipe,
                    state,
                    row.rect,
                    recipe.radius,
                    ButtonFocusPlacement::Inward,
                ) {
                    self.primitive(primitive);
                }
                recipe.foreground
            }
        };

        let font = self.theme.font(TextRole::Label);
        let extra = (row.rect.height - font.line_height).max(0.0) * 0.5;
        let text = match row.kind {
            ChromeSceneRowKind::TabClose => "×",
            ChromeSceneRowKind::Overflow => "…",
            ChromeSceneRowKind::Menu
            | ChromeSceneRowKind::Toolbar
            | ChromeSceneRowKind::Tab { .. }
            | ChromeSceneRowKind::Status => &row.label,
        };
        let padding_x = self.theme.controls.padding_x;
        let text_x = self.paint_chrome_toolbar_icon(row, foreground, padding_x);
        let mut primitive = Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(text_x, row.rect.y + extra + font.size),
            text: text.to_owned(),
            family: font.family.to_owned(),
            size: font.size,
            line_height: font.line_height,
            brush: Brush::Solid(foreground),
        });
        if row.kind == ChromeSceneRowKind::Toolbar
            && let Some(text_layouts) = self.text_layouts.as_deref_mut()
            && let Primitive::Text(text) = &mut primitive
        {
            let raw_span = row.rect.max_x() - padding_x - text_x;
            let label_width = raw_span.max(0.0_f32);
            text.layout = text_layouts.try_layout_id(
                TextLayoutKey::new(
                    text.text.clone(),
                    TextStyle::new(text.family.clone(), text.size, text.line_height),
                    label_width,
                    false,
                )
                .with_overflow(TextOverflow::EndEllipsis),
            );
        }
        self.primitive(primitive);
    }

    fn paint_chrome_toolbar_icon(
        &mut self,
        row: &ChromeSceneRow,
        foreground: stern_core::Color,
        padding_x: f32,
    ) -> f32 {
        let Some(icon) = (row.kind == ChromeSceneRowKind::Toolbar)
            .then_some(row.icon)
            .flatten()
        else {
            return row.rect.x + padding_x;
        };
        let icon_size = self
            .theme
            .sizes
            .icon
            .md
            .min((row.rect.width - padding_x * 2.0).max(0.0))
            .min(row.rect.height.max(0.0));
        let icon_rect = fit_box(
            Rect::new(
                row.rect.x + padding_x,
                row.rect.y,
                icon_size,
                row.rect.height,
            ),
            Size::new(icon_size, icon_size),
            stern_core::Alignment::Center,
            stern_core::Alignment::Center,
        );
        self.primitive(Primitive::Icon(IconPrimitive::new(
            icon, icon_rect, foreground,
        )));
        icon_rect.max_x() + self.theme.spacing.resolve(SpacingRole::IconLabelGap)
    }
}

fn chrome_surface_semantics(
    id: stern_core::WidgetId,
    kind: ChromeSurfaceKind,
    rect: Rect,
    children: Vec<stern_core::WidgetId>,
) -> SemanticNode {
    let (role, label) = match kind {
        ChromeSurfaceKind::MenuBar => (
            SemanticRole::Custom("menu-bar".to_owned()),
            "Application menu",
        ),
        ChromeSurfaceKind::Toolbar => (
            SemanticRole::Custom("toolbar".to_owned()),
            "Application toolbar",
        ),
        ChromeSurfaceKind::TabStrip => (SemanticRole::TabList, "Document tabs"),
        ChromeSurfaceKind::StatusBar => (
            SemanticRole::Custom("status-bar".to_owned()),
            "Application status",
        ),
    };
    SemanticNode::new(id, role, rect)
        .with_label(label)
        .with_children(children)
}

fn chrome_row_semantics(
    row: &ChromeSceneRow,
    response: Option<&stern_core::Response>,
) -> SemanticNode {
    let mut node = SemanticNode::new(row.id, row.role.clone(), row.rect).with_label(&row.label);
    node.state.disabled = row.interactive() && !row.enabled;
    node.state.selected = row.selected;
    node.state.checked = row.checked;
    node.state.focused = response.is_some_and(|response| response.state.focused);
    node.state.pressed = response.is_some_and(|response| response.state.pressed);
    if row.kind == ChromeSceneRowKind::Menu {
        node.state.expanded = Some(row.selected);
    }
    if row.actionable() {
        node = node.focusable(true);
        let kind = if matches!(
            row.kind,
            ChromeSceneRowKind::Menu | ChromeSceneRowKind::Overflow
        ) {
            SemanticActionKind::Open
        } else {
            SemanticActionKind::Invoke
        };
        node.actions.push(SemanticAction {
            kind,
            label: if matches!(
                row.kind,
                ChromeSceneRowKind::Menu | ChromeSceneRowKind::Overflow
            ) {
                "Open".to_owned()
            } else {
                "Invoke".to_owned()
            },
            action_id: row.action_id.clone(),
        });
    }
    node
}

fn reconcile_workspace_focus(
    ui: &mut Ui<'_>,
    bar: &mut ApplicationBar,
    layout: &PreparedApplicationBar,
) {
    let focused = ui.memory().focused();
    let focused_row = layout
        .workspace_rows
        .iter()
        .find(|row| focused == Some(row.id));
    if let Some(ApplicationBarRow {
        enabled: true,
        kind: ApplicationBarRowKind::Workspace(target),
        ..
    }) = focused_row
    {
        bar.workspace_focus = Some(target.id);
        return;
    }
    let retained_owned = focused_row.is_some()
        || bar
            .workspace_focus
            .is_some_and(|id| focused == Some(bar.workspace_widget_id(id)));
    let retained_valid = bar.workspace_focus.is_some_and(|id| {
        layout.workspace_rows.iter().any(|row| {
            row.enabled
                && matches!(row.kind, ApplicationBarRowKind::Workspace(target) if target.id == id)
        })
    });
    if !retained_valid {
        bar.workspace_focus = workspace_fallback(bar, layout);
        if retained_owned {
            if let Some(id) = bar.workspace_focus {
                ui.runtime.memory_mut().focus(bar.workspace_widget_id(id));
            } else {
                ui.runtime.memory_mut().clear_focus();
            }
            ui.request_repaint(RepaintRequest::NextFrame);
        }
    }
}

fn workspace_fallback(
    bar: &ApplicationBar,
    layout: &PreparedApplicationBar,
) -> Option<crate::chrome::WorkspaceTabId> {
    layout
        .workspace_rows
        .iter()
        .find(|row| {
            row.enabled
                && matches!(row.kind, ApplicationBarRowKind::Workspace(target) if bar.workspaces[target.index].active)
        })
        .or_else(|| layout.workspace_rows.iter().find(|row| row.enabled))
        .and_then(|row| match row.kind {
            ApplicationBarRowKind::Workspace(target) => Some(target.id),
            ApplicationBarRowKind::Menu(_) => None,
        })
}

fn handle_menu_keyboard(
    ui: &mut Ui<'_>,
    bar: &mut ApplicationBar,
    layout: &PreparedApplicationBar,
    events: &[KeyEvent],
    output: &mut ApplicationBarOutput,
) -> bool {
    if layout.menu_rows.is_empty() {
        return false;
    }
    let mut handled = false;
    for event in events {
        if let Some(menu) = bar.menu_bar.open_platform_entry(event)
            && let Some(row) = menu_row(layout, menu)
        {
            ui.runtime.memory_mut().focus(row.id);
            output.intents.push(ApplicationBarIntent::OpenMenu {
                menu,
                anchor: row.rect,
            });
            handled = true;
            continue;
        }
        let Some(active) = bar.menu_bar.active_id() else {
            continue;
        };
        if event.state != KeyState::Pressed || !event.modifiers.is_empty() {
            continue;
        }
        if event.key == Key::Escape && !event.repeat {
            bar.menu_bar.close();
            if let Some(row) = menu_row(layout, active) {
                ui.runtime.memory_mut().focus(row.id);
            }
            output
                .intents
                .push(ApplicationBarIntent::DismissMenu { menu: active });
            handled = true;
            continue;
        }
        let moved = match event.key {
            Key::ArrowLeft => bar.menu_bar.move_previous(),
            Key::ArrowRight => bar.menu_bar.move_next(),
            _ => None,
        };
        if let Some(menu) = moved
            && let Some(row) = menu_row(layout, menu)
        {
            ui.runtime.memory_mut().focus(row.id);
            output.intents.push(ApplicationBarIntent::OpenMenu {
                menu,
                anchor: row.rect,
            });
            handled = true;
        }
    }
    handled || bar.menu_bar.active_id().is_some()
}

fn handle_workspace_keyboard(
    ui: &mut Ui<'_>,
    bar: &mut ApplicationBar,
    layout: &PreparedApplicationBar,
    events: &[KeyEvent],
) {
    let Some(mut index) = layout
        .workspace_rows
        .iter()
        .position(|row| row.enabled && ui.memory().focused() == Some(row.id))
    else {
        return;
    };
    let enabled = layout
        .workspace_rows
        .iter()
        .enumerate()
        .filter_map(|(index, row)| row.enabled.then_some(index))
        .collect::<Vec<_>>();
    for event in events {
        if event.state != KeyState::Pressed || !event.modifiers.is_empty() {
            continue;
        }
        let position = enabled
            .iter()
            .position(|candidate| *candidate == index)
            .unwrap_or(0);
        let next = match event.key {
            Key::ArrowLeft => Some(enabled[(position + enabled.len() - 1) % enabled.len()]),
            Key::ArrowRight => Some(enabled[(position + 1) % enabled.len()]),
            Key::Home => enabled.first().copied(),
            Key::End => enabled.last().copied(),
            _ => None,
        };
        if let Some(next) = next {
            index = next;
            let row = &layout.workspace_rows[index];
            let ApplicationBarRowKind::Workspace(target) = row.kind else {
                continue;
            };
            bar.workspace_focus = Some(target.id);
            ui.runtime.memory_mut().focus(row.id);
            ui.request_repaint(RepaintRequest::NextFrame);
        }
    }
}

fn menu_row(
    layout: &PreparedApplicationBar,
    id: crate::chrome::MenuBarMenuId,
) -> Option<&ApplicationBarRow> {
    layout
        .menu_rows
        .iter()
        .find(|row| matches!(row.kind, ApplicationBarRowKind::Menu(menu) if menu == id))
}

fn application_bar_row_semantics(
    bar: &ApplicationBar,
    row: &ApplicationBarRow,
    response: stern_core::Response,
    selected: bool,
    menu_roving: Option<stern_core::WidgetId>,
) -> SemanticNode {
    let (role, action, roving) = match row.kind {
        ApplicationBarRowKind::Menu(_menu) => (
            SemanticRole::MenuItem,
            SemanticActionKind::Open,
            menu_roving,
        ),
        ApplicationBarRowKind::Workspace(_) => (
            SemanticRole::Tab,
            SemanticActionKind::Invoke,
            bar.workspace_focus.map(|id| bar.workspace_widget_id(id)),
        ),
    };
    let mut node = SemanticNode::new(row.id, role, row.rect)
        .with_label(&row.label)
        .focusable(row.enabled && roving == Some(row.id));
    node.state.disabled = !row.enabled;
    node.state.selected = selected;
    node.state.focused = response.state.focused;
    node.state.pressed = response.state.pressed;
    if let ApplicationBarRowKind::Menu(menu) = row.kind {
        node.state.expanded = Some(bar.menu_bar.active_id() == Some(menu));
    }
    if row.enabled {
        node.actions.push(SemanticAction {
            kind: action.clone(),
            label: if action == SemanticActionKind::Open {
                "Open"
            } else {
                "Invoke"
            }
            .to_owned(),
            action_id: None,
        });
    }
    node
}
