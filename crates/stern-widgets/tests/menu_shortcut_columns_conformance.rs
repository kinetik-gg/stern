//! Deterministic menu shortcut-column presentation conformance.

use std::{cell::RefCell, time::Duration};

use stern_core::{
    ActionContext, ActionDescriptor, ActionSource, Brush, ComponentState, FrameContext, Key,
    KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, Primitive, Rect, SemanticActionKind, Shortcut,
    ShortcutLabelLocalizer, ShortcutLabelToken, ShortcutModifier, ShortcutPlatform, Size,
    TextPrimitive, TimeInfo, UiInput, UiMemory, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::{
    CommandPaletteOverlay, DropdownItem, DropdownItemId, DropdownModel, DropdownOverlay, Menu,
    MenuItem, MenuOverlay, ModalAction, ModalActionRole, ModalDialog, ModalDialogOverlay,
    OverlayDismissal, OverlayEntry, OverlayId, OverlayKind, OverlayScene, OverlaySceneIntent,
    OverlaySceneMetrics, OverlaySceneOutput, OverlaySceneSurface, Ui,
};

const SURFACE_RECT: Rect = Rect::new(20.0, 20.0, 280.0, 184.0);

#[derive(Debug, Clone, PartialEq, Eq)]
enum Callback {
    Token(ShortcutPlatform, String),
    Separator(ShortcutPlatform),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Failure {
    None,
    RejectAlt,
    EmptyKey,
}

struct RecordingLocalizer {
    separator: String,
    failure: Failure,
    callbacks: RefCell<Vec<Callback>>,
}

impl RecordingLocalizer {
    fn new(separator: &str, failure: Failure) -> Self {
        Self {
            separator: separator.to_owned(),
            failure,
            callbacks: RefCell::new(Vec::new()),
        }
    }

    fn callbacks(&self) -> Vec<Callback> {
        self.callbacks.borrow().clone()
    }
}

impl ShortcutLabelLocalizer for RecordingLocalizer {
    fn token_label(
        &self,
        platform: ShortcutPlatform,
        token: ShortcutLabelToken<'_>,
    ) -> Option<String> {
        let (identity, label) = match token {
            ShortcutLabelToken::Modifier(modifier) => (
                format!("modifier:{modifier:?}"),
                match modifier {
                    ShortcutModifier::Control => "control-label-that-is-intentionally-long",
                    ShortcutModifier::Alt => "alternate-label-that-is-intentionally-long",
                    ShortcutModifier::Shift => "shift-label-that-is-intentionally-long",
                    ShortcutModifier::Super => "super-label-that-is-intentionally-long",
                }
                .to_owned(),
            ),
            ShortcutLabelToken::LogicalKey(key) => {
                (format!("logical:{key:?}"), format!("logical-key:{key:?}"))
            }
            ShortcutLabelToken::PhysicalKey(key) => {
                (format!("physical:{key:?}"), format!("physical-key:{key:?}"))
            }
        };
        self.callbacks
            .borrow_mut()
            .push(Callback::Token(platform, identity));

        if self.failure == Failure::RejectAlt
            && matches!(token, ShortcutLabelToken::Modifier(ShortcutModifier::Alt))
        {
            return None;
        }
        if self.failure == Failure::EmptyKey
            && matches!(
                token,
                ShortcutLabelToken::LogicalKey(_) | ShortcutLabelToken::PhysicalKey(_)
            )
        {
            return Some(String::new());
        }
        Some(label)
    }

    fn separator(&self, platform: ShortcutPlatform) -> &str {
        self.callbacks
            .borrow_mut()
            .push(Callback::Separator(platform));
        &self.separator
    }
}

fn shortcut(key: &str) -> Shortcut {
    Shortcut::new(
        Modifiers::new(true, true, true, false),
        Key::Character(key.to_owned()),
    )
}

fn action_with_shortcut(id: &str, label: &str, shortcut: Shortcut) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(id, label);
    action.shortcut = Some(shortcut);
    action
}

fn checked_submenu_fixture() -> (Menu, ActionDescriptor) {
    let mut trigger =
        action_with_shortcut("menu.checked-submenu", "Checked submenu", shortcut("s"));
    trigger.state.checked = Some(true);
    let expected = trigger.clone();
    let mut menu = Menu::new();
    menu.push_submenu(
        trigger,
        Menu::from_actions([ActionDescriptor::new("submenu.child", "Child")]),
    );
    (menu, expected)
}

fn only_menu_descriptor(scene: &OverlayScene) -> &ActionDescriptor {
    let Some(OverlaySceneSurface::Menu { overlay, .. }) = scene.surfaces().first() else {
        panic!("menu surface");
    };
    let Some(MenuItem::Action(action)) = overlay.visible_items_iter().next() else {
        panic!("menu action");
    };
    action
}

fn menu_scene(rect: Rect, menu: Menu) -> OverlayScene {
    let mut scene = OverlayScene::new();
    scene.push(OverlaySceneSurface::menu(
        "Commands",
        MenuOverlay::new(
            OverlayEntry::new(OverlayId::from_raw(41), OverlayKind::Menu, rect),
            menu,
            ActionSource::Menu,
            ActionContext::Frame(WidgetId::from_key("document:alpha")),
        ),
    ));
    scene
}

fn frame_context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(640.0, 480.0),
            PhysicalSize::new(640, 480),
            stern_core::ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(400), Duration::from_millis(16), 1),
    )
}

fn run_presented(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
    platform: ShortcutPlatform,
    localizer: &dyn ShortcutLabelLocalizer,
) -> (OverlaySceneOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(frame_context(input), memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let output = ui.overlay_scene_with_menu_presentation(scene, platform, localizer);
    (output, ui.finish_output())
}

fn run_legacy(
    scene: &mut OverlayScene,
    memory: &mut UiMemory,
    input: UiInput,
) -> (OverlaySceneOutput, stern_core::FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(frame_context(input), memory, &theme);
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid pointer plan");
    let output = ui.overlay_scene(scene);
    (output, ui.finish_output())
}

fn text_primitives(frame: &stern_core::FrameOutput) -> Vec<&TextPrimitive> {
    frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text),
            _ => None,
        })
        .collect()
}

fn text_contents(frame: &stern_core::FrameOutput) -> Vec<&str> {
    text_primitives(frame)
        .into_iter()
        .map(|text| text.text.as_str())
        .collect()
}

fn declaration_shape(source: &str, start: &str, end: &str) -> String {
    let source = source.replace("\r\n", "\n");
    let (_, tail) = source.split_once(start).expect("declaration start");
    let (body, _) = tail.split_once(end).expect("declaration end");
    format!("{start}{body}")
        .lines()
        .map(str::trim)
        .filter(|line| !line.starts_with("///"))
        .collect()
}

fn mixed_menu() -> (Menu, Vec<Shortcut>) {
    let primary = shortcut("p");
    let icon = shortcut("i");
    let submenu = shortcut("m");
    let disabled = shortcut("d");
    let hidden = shortcut("h");

    let mut menu = Menu::new();
    menu.push(MenuItem::Action(action_with_shortcut(
        "menu.primary",
        "A label long enough to cross every trailing column without clipping",
        primary.clone(),
    )));
    menu.push(MenuItem::Action(ActionDescriptor::new(
        "menu.no-shortcut",
        "No shortcut",
    )));
    let mut icon_action = action_with_shortcut("menu.icon", "Icon action", icon.clone());
    icon_action.icon = Some(stern_icons_phosphor::regular::FLOPPY_DISK.into());
    icon_action.state.checked = Some(false);
    menu.push(MenuItem::Action(icon_action));
    menu.push_submenu(
        action_with_shortcut("menu.submenu", "Submenu", submenu.clone()),
        Menu::from_actions([ActionDescriptor::new("submenu.child", "Child")]),
    );
    menu.push(MenuItem::Label("Section label".to_owned()));
    menu.push(MenuItem::Separator);
    let mut hidden_action = action_with_shortcut("menu.hidden", "Hidden", hidden);
    hidden_action.state.visible = false;
    hidden_action.state.checked = Some(true);
    menu.push(MenuItem::Action(hidden_action));
    let mut disabled_action = action_with_shortcut("menu.disabled", "Disabled", disabled.clone());
    disabled_action.state.enabled = false;
    disabled_action.state.checked = Some(true);
    menu.push(MenuItem::Action(disabled_action));

    (menu, vec![primary, icon, submenu, disabled])
}

fn pointer_input(position: Point, pressed: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(position),
            primary: if pressed {
                PointerButtonState::new(true, true, false)
            } else {
                PointerButtonState::new(false, false, true)
            },
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_sequence(keys: &[Key]) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            events: keys
                .iter()
                .cloned()
                .map(|key| KeyEvent::new(key, KeyState::Pressed, Modifiers::default(), false))
                .collect(),
            ..KeyboardInput::default()
        },
        ..UiInput::default()
    }
}

#[test]
#[allow(clippy::float_cmp, clippy::too_many_lines)]
fn wide_mixed_menu_emits_stable_clipped_columns_and_decorative_semantics() {
    let (menu, eligible_shortcuts) = mixed_menu();
    let mut scene = menu_scene(SURFACE_RECT, menu);
    let original_scene = scene.clone();
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (output, frame) = run_presented(
        &mut scene,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::Windows,
        &localizer,
    );
    assert_eq!(
        scene, original_scene,
        "presentation does not mutate descriptors"
    );
    assert!(output.intents.is_empty());
    assert_eq!(
        output
            .responses
            .iter()
            .map(|response| response.rect)
            .collect::<Vec<_>>(),
        [24.0, 52.0, 80.0, 108.0].map(|y| Rect::new(24.0, y, 272.0, 28.0))
    );
    let texts = text_primitives(&frame);
    assert_eq!(texts.len(), 11);
    let label_texts = texts
        .iter()
        .filter(|text| text.origin.x == 80.0)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(label_texts.len(), 6);
    assert_eq!(
        label_texts
            .iter()
            .map(|text| text.text.as_str())
            .collect::<Vec<_>>(),
        [
            "A label long enough to cross every trailing column without clipping",
            "No shortcut",
            "Icon action",
            "Submenu",
            "Section label",
            "Disabled",
        ]
    );
    let shortcut_texts = texts
        .iter()
        .filter(|text| text.origin.x == 152.0)
        .copied()
        .collect::<Vec<_>>();
    assert_eq!(shortcut_texts.len(), 4);
    assert!(shortcut_texts.iter().all(|text| text.text.contains("::")));
    assert_eq!(
        texts
            .iter()
            .filter(|text| text.text == "›" && text.origin.x == 272.0)
            .count(),
        1
    );
    assert!(texts.iter().all(|text| text.text != "symbolic-save-icon"));
    assert!(texts.iter().all(|text| text.text != "Hidden"));
    let check_lines = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(check_lines.len(), 2, "only the visible checked row paints");
    assert_eq!(
        check_lines
            .iter()
            .map(|line| (line.from, line.to))
            .collect::<Vec<_>>(),
        [
            (Point::new(35.0, 186.0), Point::new(38.5, 189.0)),
            (Point::new(38.5, 189.0), Point::new(45.0, 182.0)),
        ]
    );
    let disabled_foreground = default_dark_theme()
        .row(ComponentState {
            disabled: true,
            ..ComponentState::default()
        })
        .foreground;
    assert!(
        check_lines
            .iter()
            .all(|line| line.stroke.brush == Brush::Solid(disabled_foreground))
    );
    let begins = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::ClipBegin { rect, .. } => Some(*rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        begins.iter().filter(|rect| **rect == SURFACE_RECT).count(),
        1
    );
    assert_eq!(
        begins
            .iter()
            .filter(|rect| rect.x == 80.0 && rect.width == 40.0)
            .count(),
        6
    );
    assert_eq!(
        begins
            .iter()
            .filter(|rect| rect.x == 152.0 && rect.width == 112.0)
            .count(),
        4
    );
    let mut clip_stack = Vec::new();
    for primitive in &frame.primitives {
        match primitive {
            Primitive::ClipBegin { id, rect } => {
                if !clip_stack.is_empty() {
                    assert_ne!(
                        *rect, SURFACE_RECT,
                        "row clips stay inside the surface clip"
                    );
                }
                clip_stack.push(*id);
            }
            Primitive::ClipEnd { id } => assert_eq!(clip_stack.pop(), Some(*id)),
            _ => {}
        }
    }
    assert!(clip_stack.is_empty());
    let surface = frame
        .semantics
        .get(WidgetId::from_raw(41))
        .expect("menu surface semantics");
    assert_eq!(surface.children.len(), 7);
    for child in &surface.children {
        let node = frame.semantics.get(*child).expect("row semantics");
        assert!(node.state.value.is_none());
        assert!(node.description.is_none());
        let label = node.label.as_deref().expect("row label");
        assert!(!label.contains("::"));
        assert!(!label.contains('›'));
        assert!(!label.contains("symbolic-save-icon"));
    }
    let icon_id = WidgetId::from_raw(41)
        .child("overlay-scene")
        .child(("overlay-action", "menu.icon"));
    assert_eq!(
        frame
            .semantics
            .get(icon_id)
            .expect("icon action")
            .label
            .as_deref(),
        Some("Icon action")
    );
    let disabled_id = WidgetId::from_raw(41)
        .child("overlay-scene")
        .child(("overlay-action", "menu.disabled"));
    let disabled_node = frame.semantics.get(disabled_id).expect("disabled action");
    assert!(disabled_node.state.disabled);
    assert_eq!(disabled_node.state.checked, Some(true));
    assert!(disabled_node.actions.is_empty());

    let separator = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Rect(rect)
                if rect.rect.x == 24.0
                    && rect.rect.width == 272.0
                    && rect.rect.y >= 164.0
                    && rect.rect.y < 172.0 =>
            {
                Some(rect.rect)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(separator.len(), 1);
    let mut expected_callbacks = Vec::new();
    for shortcut in &eligible_shortcuts {
        let direct = RecordingLocalizer::new("::", Failure::None);
        assert!(
            shortcut
                .localized_label(ShortcutPlatform::Windows, &direct)
                .is_some()
        );
        expected_callbacks.extend(direct.callbacks());
    }
    assert_eq!(localizer.callbacks(), expected_callbacks);
    let repeat_localizer = RecordingLocalizer::new("::", Failure::None);
    let (_, repeated_frame) = run_presented(
        &mut scene,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::Windows,
        &repeat_localizer,
    );
    assert_eq!(frame.primitives, repeated_frame.primitives);
    assert_eq!(frame.semantics, repeated_frame.semantics);
}

#[test]
fn widget_uses_each_explicit_platform_and_caller_owned_localizer_policy_verbatim() {
    let shortcut = Shortcut::new(
        Modifiers::new(true, true, false, true),
        Key::Character("k".to_owned()),
    );
    for platform in [
        ShortcutPlatform::Windows,
        ShortcutPlatform::MacOs,
        ShortcutPlatform::Linux,
    ] {
        let direct = RecordingLocalizer::new(" / ", Failure::None);
        let expected = shortcut
            .localized_label(platform, &direct)
            .expect("direct core label");
        let expected_callbacks = direct.callbacks();

        let mut scene = menu_scene(
            Rect::new(20.0, 20.0, 280.0, 40.0),
            Menu::from_actions([action_with_shortcut(
                "menu.platform",
                "Platform",
                shortcut.clone(),
            )]),
        );
        let widget = RecordingLocalizer::new(" / ", Failure::None);
        let (_, frame) = run_presented(
            &mut scene,
            &mut UiMemory::new(),
            UiInput::default(),
            platform,
            &widget,
        );
        assert!(
            text_primitives(&frame)
                .iter()
                .any(|text| text.text == expected)
        );
        assert_eq!(widget.callbacks(), expected_callbacks);
        assert_eq!(
            widget
                .callbacks()
                .iter()
                .filter(|callback| matches!(callback, Callback::Separator(_)))
                .count(),
            1
        );
    }
}

#[test]
fn rejected_or_empty_tokens_fail_closed_without_separator_or_routing_changes() {
    for failure in [Failure::RejectAlt, Failure::EmptyKey] {
        let shortcut = shortcut("x");
        let direct = RecordingLocalizer::new(" + ", failure);
        assert_eq!(
            shortcut.localized_label(ShortcutPlatform::Windows, &direct),
            None
        );
        let expected_callbacks = direct.callbacks();
        assert!(
            expected_callbacks
                .iter()
                .all(|callback| !matches!(callback, Callback::Separator(_)))
        );

        let mut scene = menu_scene(
            Rect::new(20.0, 20.0, 280.0, 40.0),
            Menu::from_actions([action_with_shortcut(
                "menu.rejected",
                "Rejected shortcut",
                shortcut,
            )]),
        );
        let before = scene.clone();
        let widget = RecordingLocalizer::new(" + ", failure);
        let (output, frame) = run_presented(
            &mut scene,
            &mut UiMemory::new(),
            UiInput::default(),
            ShortcutPlatform::Windows,
            &widget,
        );
        assert_eq!(widget.callbacks(), expected_callbacks);
        assert_eq!(scene, before);
        assert_eq!(output.responses.len(), 1);
        assert_eq!(output.responses[0].rect, Rect::new(24.0, 24.0, 272.0, 28.0));
        assert_eq!(text_contents(&frame), ["Rejected shortcut"]);
        let row_id = WidgetId::from_raw(41)
            .child("overlay-scene")
            .child(("overlay-action", "menu.rejected"));
        let semantics = frame.semantics.get(row_id).expect("row semantics");
        assert_eq!(semantics.label.as_deref(), Some("Rejected shortcut"));
        assert!(semantics.state.value.is_none());
        assert_eq!(
            frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::ClipBegin { .. }))
                .count(),
            2,
            "surface and label clips only"
        );
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn narrow_rows_and_non_menu_surfaces_remain_value_equal_to_legacy_presentation() {
    let (narrow_menu, _) = checked_submenu_fixture();
    let mut legacy_scene = menu_scene(Rect::new(20.0, 20.0, 272.0, 40.0), narrow_menu.clone());
    let mut presented_scene = legacy_scene.clone();
    let (legacy_output, legacy_frame) =
        run_legacy(&mut legacy_scene, &mut UiMemory::new(), UiInput::default());
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (presented_output, presented_frame) = run_presented(
        &mut presented_scene,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::Linux,
        &localizer,
    );
    assert!(localizer.callbacks().is_empty());
    assert_eq!(presented_output, legacy_output);
    assert_eq!(presented_frame, legacy_frame);
    assert_eq!(text_contents(&presented_frame), ["Checked submenu"]);
    let below = f32::from_bits(272.0_f32.to_bits() - 1);
    for width in [below, 271.0, 120.0] {
        let metrics = OverlaySceneMetrics {
            inset: 0.0,
            ..OverlaySceneMetrics::default()
        };
        let mut presented_scene = OverlayScene::with_metrics(metrics);
        presented_scene.push(OverlaySceneSurface::menu(
            "Narrow",
            MenuOverlay::new(
                OverlayEntry::new(
                    OverlayId::from_raw(42),
                    OverlayKind::Menu,
                    Rect::new(7.0, 11.0, width, 28.0),
                ),
                narrow_menu.clone(),
                ActionSource::Menu,
                ActionContext::Global,
            ),
        ));
        let mut legacy_scene = presented_scene.clone();
        let (legacy_output, legacy_frame) =
            run_legacy(&mut legacy_scene, &mut UiMemory::new(), UiInput::default());
        let localizer = RecordingLocalizer::new("::", Failure::None);
        let (presented_output, presented_frame) = run_presented(
            &mut presented_scene,
            &mut UiMemory::new(),
            UiInput::default(),
            ShortcutPlatform::Linux,
            &localizer,
        );
        assert!(localizer.callbacks().is_empty());
        assert_eq!(presented_output, legacy_output);
        assert_eq!(presented_frame, legacy_frame);
        assert_eq!(text_contents(&presented_frame), ["Checked submenu"]);
        assert_eq!(
            presented_frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::ClipBegin { .. }))
                .count(),
            1
        );
    }

    let mut non_menu = OverlayScene::new();
    non_menu.push(OverlaySceneSurface::dropdown(
        "Dropdown",
        DropdownOverlay::new(
            OverlayEntry::new(
                OverlayId::from_raw(50),
                OverlayKind::Dropdown,
                Rect::new(10.0, 10.0, 180.0, 40.0),
            ),
            WidgetId::from_key("dropdown-trigger"),
            DropdownModel::from_items([DropdownItem::new(DropdownItemId::from_raw(1), "Choice")]),
        ),
    ));
    non_menu.push(OverlaySceneSurface::command_palette(
        "Palette",
        CommandPaletteOverlay::from_actions(
            OverlayEntry::new(
                OverlayId::from_raw(51),
                OverlayKind::CommandPalette,
                Rect::new(200.0, 10.0, 200.0, 70.0),
            ),
            &[action_with_shortcut(
                "palette.action",
                "Palette",
                shortcut("n"),
            )],
            ActionContext::Global,
        ),
    ));
    let modal =
        ModalDialog::new(WidgetId::from_key("modal"), "Modal").with_actions([ModalAction::new(
            ActionDescriptor::new("modal.action", "Confirm"),
            ModalActionRole::Primary,
        )]);
    non_menu.push(OverlaySceneSurface::modal(ModalDialogOverlay::placed(
        OverlayId::from_raw(52),
        Rect::new(10.0, 100.0, 180.0, 70.0),
        modal,
        OverlayDismissal::Escape,
        ActionContext::Global,
    )));
    non_menu.push(OverlaySceneSurface::passive(
        OverlayEntry::new(
            OverlayId::from_raw(53),
            OverlayKind::Popover,
            Rect::new(200.0, 100.0, 180.0, 40.0),
        ),
        "Passive",
        "Passive text",
    ));
    let mut legacy_non_menu = non_menu.clone();
    let (legacy_output, legacy_frame) = run_legacy(
        &mut legacy_non_menu,
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (presented_output, presented_frame) = run_presented(
        &mut non_menu,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::MacOs,
        &localizer,
    );
    assert!(localizer.callbacks().is_empty());
    assert_eq!(presented_output, legacy_output);
    assert_eq!(presented_frame, legacy_frame);
}

#[test]
#[allow(clippy::too_many_lines)]
fn presentation_preserves_full_row_pointer_keyboard_and_fifo_action_routing() {
    let (menu, expected_descriptor) = checked_submenu_fixture();
    let mut legacy_scene = menu_scene(SURFACE_RECT, menu);
    let mut presented_scene = legacy_scene.clone();
    let mut legacy_memory = UiMemory::new();
    let mut presented_memory = UiMemory::new();
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let expected_shortcut = expected_descriptor
        .shortcut
        .as_ref()
        .expect("submenu shortcut")
        .localized_label(
            ShortcutPlatform::Windows,
            &RecordingLocalizer::new("::", Failure::None),
        )
        .expect("localized submenu shortcut");
    let row_id = WidgetId::from_raw(41)
        .child("overlay-scene")
        .child(("overlay-action", "menu.checked-submenu"));
    for pressed in [true, false] {
        let input = pointer_input(Point::new(280.0, 30.0), pressed);
        let (legacy_output, legacy_frame) =
            run_legacy(&mut legacy_scene, &mut legacy_memory, input.clone());
        let (presented_output, presented_frame) = run_presented(
            &mut presented_scene,
            &mut presented_memory,
            input,
            ShortcutPlatform::Windows,
            &localizer,
        );
        assert_eq!(presented_output.intents, legacy_output.intents);
        assert_eq!(presented_output.responses, legacy_output.responses);
        assert_eq!(presented_output.responses.len(), 1);
        assert_eq!(
            presented_output.responses[0].rect,
            Rect::new(24.0, 24.0, 272.0, 28.0)
        );
        assert!(legacy_frame.actions.is_empty());
        assert!(presented_frame.actions.is_empty());
        assert_eq!(only_menu_descriptor(&legacy_scene), &expected_descriptor);
        assert_eq!(only_menu_descriptor(&presented_scene), &expected_descriptor);
        let legacy_surface = legacy_frame
            .semantics
            .get(WidgetId::from_raw(41))
            .expect("legacy surface semantics");
        let presented_surface = presented_frame
            .semantics
            .get(WidgetId::from_raw(41))
            .expect("presented surface semantics");
        assert_eq!(legacy_surface.children, vec![row_id]);
        assert_eq!(presented_surface.children, legacy_surface.children);
        let legacy_node = legacy_frame
            .semantics
            .get(row_id)
            .expect("legacy row semantics");
        let presented_node = presented_frame
            .semantics
            .get(row_id)
            .expect("presented row semantics");
        assert_eq!(presented_node, legacy_node);
        assert_eq!(presented_node.state.checked, Some(true));
        assert_eq!(presented_node.state.expanded, Some(false));
        assert!(presented_node.focusable);
        assert!(
            presented_node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Open)
        );
        let focus_order = vec![WidgetId::from_raw(41), row_id];
        assert_eq!(legacy_frame.semantics.focus_order(), focus_order);
        assert_eq!(presented_frame.semantics.focus_order(), focus_order);
        assert_eq!(presented_node.label.as_deref(), Some("Checked submenu"));
        assert!(presented_node.description.is_none());
        assert!(presented_node.state.value.is_none());
        let check_lines = presented_frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Line(line) => Some(line),
                _ => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(check_lines.len(), 2);
        assert!(check_lines.iter().all(|line| {
            [line.from, line.to]
                .iter()
                .all(|point| (32.0..=48.0).contains(&point.x) && (24.0..=52.0).contains(&point.y))
        }));
        assert!(
            presented_node
                .actions
                .iter()
                .all(|action| action.kind != SemanticActionKind::Invoke)
        );
        for node in [legacy_node, presented_node] {
            assert!(
                !node
                    .label
                    .as_deref()
                    .unwrap_or_default()
                    .contains(&expected_shortcut)
            );
            assert!(!node.label.as_deref().unwrap_or_default().contains('›'));
            assert!(node.actions.iter().all(|action| {
                !action.label.contains(&expected_shortcut) && !action.label.contains('›')
            }));
        }
        if !pressed {
            let [OverlaySceneIntent::OpenSubmenu(intent)] = presented_output.intents.as_slice()
            else {
                panic!("one submenu intent");
            };
            assert_eq!(intent.parent_overlay, OverlayId::from_raw(41));
            assert_eq!(intent.trigger_action.as_str(), "menu.checked-submenu");
            assert_eq!(intent.visible_index, 0);
            assert_eq!(intent.source, ActionSource::Menu);
            assert_eq!(
                intent.context,
                ActionContext::Frame(WidgetId::from_key("document:alpha"))
            );
            assert_eq!(text_contents(&legacy_frame), ["Checked submenu"]);
            let presented_texts = text_primitives(&presented_frame);
            assert_eq!(
                text_contents(&presented_frame),
                ["Checked submenu", expected_shortcut.as_str(), "›"]
            );
            assert_eq!(
                presented_texts
                    .iter()
                    .filter(|text| text.text == "›")
                    .count(),
                1
            );
        }
    }

    let first = action_with_shortcut("queue.first", "First", shortcut("1"));
    let second = action_with_shortcut("queue.second", "Second", shortcut("2"));
    let mut legacy_scene = menu_scene(
        Rect::new(20.0, 20.0, 280.0, 68.0),
        Menu::from_actions([first, second]),
    );
    let mut presented_scene = legacy_scene.clone();
    let input = key_sequence(&[Key::ArrowDown, Key::Enter, Key::ArrowDown, Key::Enter]);
    let (legacy_output, legacy_frame) =
        run_legacy(&mut legacy_scene, &mut UiMemory::new(), input.clone());
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (presented_output, mut presented_frame) = run_presented(
        &mut presented_scene,
        &mut UiMemory::new(),
        input,
        ShortcutPlatform::Windows,
        &localizer,
    );
    assert_eq!(presented_output.intents, legacy_output.intents);
    assert_eq!(presented_frame.actions, legacy_frame.actions);
    assert_eq!(
        [
            presented_frame
                .actions
                .pop_front()
                .expect("first action")
                .action_id
                .as_str()
                .to_owned(),
            presented_frame
                .actions
                .pop_front()
                .expect("second action")
                .action_id
                .as_str()
                .to_owned(),
        ],
        ["queue.first", "queue.second"]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn destructive_menu_rows_preserve_geometry_paint_precedence_and_invocation() {
    let neutral = action_with_shortcut("menu.neutral", "Neutral", shortcut("n"));
    let mut destructive = action_with_shortcut("menu.destructive", "Destructive", shortcut("d"));
    destructive.destructive = true;
    let mut submenu = action_with_shortcut(
        "menu.destructive-submenu",
        "Destructive submenu",
        shortcut("s"),
    );
    submenu.destructive = true;
    submenu.state.checked = Some(true);
    let mut disabled = action_with_shortcut("menu.disabled", "Disabled destructive", shortcut("x"));
    disabled.destructive = true;
    disabled.state.enabled = false;
    let mut terminal = ActionDescriptor::new("menu.delete", "Delete permanently");
    terminal.destructive = true;
    let mut menu = Menu::new();
    menu.push(MenuItem::Action(neutral));
    menu.push(MenuItem::Action(destructive));
    menu.push_submenu(
        submenu,
        Menu::from_actions([ActionDescriptor::new("submenu.child", "Child")]),
    );
    menu.push(MenuItem::Action(disabled));
    menu.push(MenuItem::Action(terminal));
    let mut scene = menu_scene(SURFACE_RECT, menu);
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (output, frame) = run_presented(
        &mut scene,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::Windows,
        &localizer,
    );
    assert!(output.intents.is_empty());

    let root = WidgetId::from_raw(41).child("overlay-scene");
    let row_ids = [
        "menu.neutral",
        "menu.destructive",
        "menu.destructive-submenu",
        "menu.disabled",
        "menu.delete",
    ]
    .map(|id| root.child(("overlay-action", id)));
    assert_eq!(
        row_ids.map(|id| frame.semantics.get(id).expect("menu row semantics").bounds),
        [24.0, 52.0, 80.0, 108.0, 136.0].map(|y| Rect::new(24.0, y, 272.0, 28.0))
    );

    let theme = default_dark_theme();
    let danger = Brush::Solid(theme.colors.status.danger.foreground);
    let disabled = Brush::Solid(
        theme
            .row(ComponentState {
                disabled: true,
                ..ComponentState::default()
            })
            .foreground,
    );
    let text_brush = |content: &str| {
        text_primitives(&frame)
            .into_iter()
            .find(|text| text.text == content)
            .unwrap_or_else(|| panic!("missing text {content}"))
            .brush
    };
    assert_ne!(text_brush("Neutral"), danger);
    for content in [
        "Destructive",
        "control-label-that-is-intentionally-long::alternate-label-that-is-intentionally-long::shift-label-that-is-intentionally-long::logical-key:Character(\"d\")",
        "Destructive submenu",
        "control-label-that-is-intentionally-long::alternate-label-that-is-intentionally-long::shift-label-that-is-intentionally-long::logical-key:Character(\"s\")",
        "›",
        "Delete permanently",
    ] {
        assert_eq!(
            text_brush(content),
            danger,
            "danger foreground for {content}"
        );
    }
    assert_eq!(text_brush("Disabled destructive"), disabled);
    assert_eq!(
        text_brush(
            "control-label-that-is-intentionally-long::alternate-label-that-is-intentionally-long::shift-label-that-is-intentionally-long::logical-key:Character(\"x\")"
        ),
        disabled
    );
    let check_lines = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Line(line) => Some(line),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(check_lines.len(), 2);
    assert!(check_lines.iter().all(|line| line.stroke.brush == danger));

    let mut memory = UiMemory::new();
    let (pressed, pressed_frame) = run_presented(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(280.0, 140.0), true),
        ShortcutPlatform::Windows,
        &localizer,
    );
    assert!(pressed.intents.is_empty() && pressed_frame.actions.is_empty());
    let (released, mut released_frame) = run_presented(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(280.0, 140.0), false),
        ShortcutPlatform::Windows,
        &localizer,
    );
    let [OverlaySceneIntent::Action(invocation)] = released.intents.as_slice() else {
        panic!("one terminal action invocation");
    };
    assert_eq!(invocation.action_id.as_str(), "menu.delete");
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(
        invocation.context,
        ActionContext::Frame(WidgetId::from_key("document:alpha"))
    );
    assert_eq!(released_frame.actions.len(), 1);
    let queued = released_frame.actions.pop_front();
    assert_eq!(queued.as_ref(), Some(invocation));
}

#[test]
#[allow(clippy::float_cmp, clippy::too_many_lines)]
fn mixed_and_checked_menu_rows_preserve_geometry_semantics_and_routing() {
    let mut mixed = ActionDescriptor::new("menu.mixed", "Mixed");
    mixed.state.checked = Some(true);
    mixed.state.mixed = true;
    let mut checked = ActionDescriptor::new("menu.checked", "Checked");
    checked.state.checked = Some(true);
    let mut scene = menu_scene(
        Rect::new(20.0, 20.0, 280.0, 68.0),
        Menu::from_actions([mixed, checked]),
    );
    let localizer = RecordingLocalizer::new("::", Failure::None);
    let (output, frame) = run_presented(
        &mut scene,
        &mut UiMemory::new(),
        UiInput::default(),
        ShortcutPlatform::Windows,
        &localizer,
    );

    assert!(output.intents.is_empty());
    assert_eq!(
        output
            .responses
            .iter()
            .map(|response| response.rect)
            .collect::<Vec<_>>(),
        [
            Rect::new(24.0, 24.0, 272.0, 28.0),
            Rect::new(24.0, 52.0, 272.0, 28.0),
        ]
    );
    let row_id = |action: &'static str| {
        WidgetId::from_raw(41)
            .child("overlay-scene")
            .child(("overlay-action", action))
    };
    let mixed_node = frame
        .semantics
        .get(row_id("menu.mixed"))
        .expect("mixed row semantics");
    let checked_node = frame
        .semantics
        .get(row_id("menu.checked"))
        .expect("checked row semantics");
    assert_eq!(mixed_node.bounds.width, checked_node.bounds.width);
    assert_eq!(mixed_node.bounds.height, checked_node.bounds.height);
    assert_eq!(mixed_node.state.checked, Some(true));
    assert!(mixed_node.state.mixed);
    assert_eq!(checked_node.state.checked, Some(true));
    assert!(!checked_node.state.mixed);

    let marks = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Line(line) => Some((line.from, line.to)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        marks,
        [
            (Point::new(35.0, 38.0), Point::new(45.0, 38.0)),
            (Point::new(35.0, 66.0), Point::new(38.5, 69.0)),
            (Point::new(38.5, 69.0), Point::new(45.0, 62.0)),
        ]
    );

    let mut memory = UiMemory::new();
    let (pressed, pressed_frame) = run_presented(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(160.0, 38.0), true),
        ShortcutPlatform::Windows,
        &localizer,
    );
    assert!(pressed.intents.is_empty());
    assert!(pressed_frame.actions.is_empty());
    let (released, mut released_frame) = run_presented(
        &mut scene,
        &mut memory,
        pointer_input(Point::new(160.0, 38.0), false),
        ShortcutPlatform::Windows,
        &localizer,
    );
    let [OverlaySceneIntent::Action(invocation)] = released.intents.as_slice() else {
        panic!("one unchanged menu invocation");
    };
    assert_eq!(invocation.action_id.as_str(), "menu.mixed");
    assert_eq!(invocation.source, ActionSource::Menu);
    assert_eq!(
        invocation.context,
        ActionContext::Frame(WidgetId::from_key("document:alpha"))
    );
    assert_eq!(released_frame.actions.len(), 1);
    assert_eq!(
        released_frame.actions.pop_front().as_ref(),
        Some(invocation)
    );
}

#[test]
fn widget_source_uses_the_core_policy_without_public_shape_or_naming_duplication() {
    let ui = include_str!("../src/ui/overlays.rs");
    assert_eq!(
        ui.matches("pub fn overlay_scene_with_menu_presentation(")
            .count(),
        1
    );
    assert_eq!(ui.matches(".localized_label(").count(), 1);
    for duplicated_policy in [
        "ShortcutModifier::",
        "\"Ctrl\"",
        "\"Command\"",
        "\"Option\"",
        "\"Super\"",
    ] {
        assert!(!ui.contains(duplicated_policy));
    }

    let menu = include_str!("../src/overlays/menu.rs");
    let scene = include_str!("../src/overlays/scene.rs");
    for stored_policy in [
        "ShortcutPlatform",
        "ShortcutLabelLocalizer",
        "shortcut_label",
    ] {
        assert!(!menu.contains(stored_policy));
        assert!(!scene.contains(stored_policy));
    }
    assert!(scene.contains("pub(crate) menu_columns: bool"));
    assert!(scene.contains("pub(crate) shortcut: Option<Shortcut>"));

    assert_eq!(
        declaration_shape(
            menu,
            "pub enum MenuItem {",
            "#[derive(Debug, Clone, PartialEq, Eq)]\nstruct MenuEntry {"
        ),
        "pub enum MenuItem {Label(String),Separator,Action(ActionDescriptor),}"
    );
    assert_eq!(
        declaration_shape(menu, "pub struct Menu {", "impl Menu {"),
        "pub struct Menu {entries: Vec<MenuEntry>,highlighted: Option<ActionId>,}"
    );
    assert_eq!(
        declaration_shape(menu, "pub struct MenuOverlay {", "impl MenuOverlay {"),
        "pub struct MenuOverlay {pub entry: OverlayEntry,pub menu: Menu,pub source: ActionSource,pub context: ActionContext,}"
    );
    assert_eq!(
        declaration_shape(
            scene,
            "pub struct OverlaySceneMetrics {",
            "impl Default for OverlaySceneMetrics {"
        ),
        "pub struct OverlaySceneMetrics {pub inset: f32,pub row_height: f32,pub separator_height: f32,}"
    );
    assert_eq!(
        declaration_shape(scene, "pub struct OverlayScene {", "impl OverlayScene {"),
        "pub struct OverlayScene {surfaces: Vec<OverlaySceneSurface>,metrics: OverlaySceneMetrics,}"
    );
}
