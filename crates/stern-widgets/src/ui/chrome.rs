mod system_feedback;

use stern_core::{
    Brush, ClipId, ComponentState, Point, Primitive, Rect, RectPrimitive, RepaintRequest,
    SemanticAction, SemanticActionKind, SemanticNode, SemanticRole, Stroke, TextPrimitive,
    TextRole,
};
use stern_text::{TextLayoutKey, TextOverflow, TextStyle};

use super::{Ui, response_activated};
use crate::chrome::{
    ChromeScene, ChromeSceneIntent, ChromeSceneOutput, ChromeSceneRow, ChromeSceneRowKind,
    ChromeSurfaceKind, WindowSystemMenuTrigger,
};
use crate::components::{
    ButtonFocusPlacement, TabFocusPlacement, button_surface_primitives, tab_surface_primitives,
};
use crate::{
    icon_button_with_label as fallback_icon_button_with_label,
    icon_button_with_library as icon_button_with_library_widget,
};

impl Ui<'_> {
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
        let icons = self.icons;
        let (input, memory) = self.runtime.input_and_memory_mut();
        let output = if let Some(icons) = icons {
            icon_button_with_library_widget(
                trigger.widget_id(),
                trigger.titlebar_rect(),
                trigger.icon(),
                "Open window system menu",
                icons,
                input,
                memory,
                theme,
                false,
            )
        } else {
            fallback_icon_button_with_label(
                trigger.widget_id(),
                trigger.titlebar_rect(),
                trigger.icon(),
                "Open window system menu",
                input,
                memory,
                theme,
                false,
            )
        };
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
        let mut primitive = Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(
                row.rect.x + self.theme.controls.padding_x,
                row.rect.y + extra + font.size,
            ),
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
            let padding_x = self.theme.controls.padding_x;
            let raw_span = row.rect.width - padding_x * 2.0_f32;
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
