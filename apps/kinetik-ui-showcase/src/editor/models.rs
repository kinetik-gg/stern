fn menu_header_rects() -> [(EditorMenuKind, &'static str, Rect); 7] {
    let specs = [
        (EditorMenuKind::File, "File", 44.0),
        (EditorMenuKind::Edit, "Edit", 44.0),
        (EditorMenuKind::View, "View", 52.0),
        (EditorMenuKind::Project, "Project", 64.0),
        (EditorMenuKind::Build, "Build", 54.0),
        (EditorMenuKind::Window, "Window", 68.0),
        (EditorMenuKind::Help, "Help", 48.0),
    ];
    let mut x = 126.0;
    specs.map(|(kind, label, width)| {
        let rect = Rect::new(x, 3.0, width, 22.0);
        x += width + 4.0;
        (kind, label, rect)
    })
}

fn menu_bar_rect() -> Rect {
    Rect::new(0.0, 0.0, 760.0, 28.0)
}

fn editor_workspace_rect(theme: &Theme, viewport: Rect) -> Rect {
    let bottom_bar = 24.0;
    let workspace_top = workspace_top(theme);
    Rect::new(
        4.0,
        workspace_top,
        (viewport.width - 8.0).max(1.0),
        (viewport.height - workspace_top - bottom_bar - 4.0).max(1.0),
    )
}

fn menu_anchor(kind: EditorMenuKind) -> Rect {
    menu_header_rects()
        .into_iter()
        .find_map(|(candidate, _, rect)| (candidate == kind).then_some(rect))
        .unwrap_or(Rect::new(126.0, 3.0, 44.0, 22.0))
}

fn menu_size(kind: EditorMenuKind) -> Size {
    match kind {
        EditorMenuKind::File => Size::new(238.0, 188.0),
        EditorMenuKind::Edit => Size::new(226.0, 154.0),
        EditorMenuKind::View => Size::new(224.0, 136.0),
        EditorMenuKind::Project => Size::new(224.0, 106.0),
        EditorMenuKind::Build => Size::new(224.0, 82.0),
        EditorMenuKind::Window => Size::new(232.0, 340.0),
        EditorMenuKind::Help => Size::new(230.0, 88.0),
    }
}

fn menu<const N: usize>(items: [MenuItem; N]) -> Menu {
    let mut menu = Menu::new();
    for item in items {
        menu.push(item);
    }
    menu
}

fn menu_action(
    action_id: &'static str,
    label: &'static str,
    shortcut: Option<Shortcut>,
    checked: Option<bool>,
    enabled: bool,
) -> MenuItem {
    let mut action = ActionDescriptor::new(action_id, label);
    action.shortcut = shortcut;
    action.state.checked = checked;
    action.state.enabled = enabled;
    MenuItem::Action(action)
}

fn toolbar_action(
    action_id: &'static str,
    label: &'static str,
    icon: ToolbarIcon,
    checked: Option<bool>,
    enabled: bool,
) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(action_id, label);
    action.icon = Some(ActionIcon::new(icon.symbol()));
    action.tooltip = Some(label.to_owned());
    action.keywords = vec!["editor".to_owned(), icon.symbol().to_owned()];
    action.state.checked = checked;
    action.state.enabled = enabled;
    action
}

fn modal_action(action_id: &'static str, label: &'static str, enabled: bool) -> ActionDescriptor {
    let mut action = ActionDescriptor::new(action_id, label);
    action.keywords = vec!["editor".to_owned(), "modal".to_owned()];
    action.state.enabled = enabled;
    action
}

fn menu_action_from_panel_metadata(metadata: &PanelOpenActionMetadata, checked: bool) -> MenuItem {
    let action_id = metadata
        .default_open_action
        .as_ref()
        .expect("showcase panel descriptors declare default open actions");
    let mut action = ActionDescriptor::new(action_id.as_str(), metadata.title.clone());
    action.state.checked = Some(checked);
    MenuItem::Action(action)
}

fn panel_type_for_open_action(action_id: &str) -> Option<PanelTypeId> {
    match action_id {
        ACTION_OPEN_VIEWPORT => Some(PANEL_TYPE_VIEWPORT),
        ACTION_OPEN_EXPLORER => Some(PANEL_TYPE_SCENE),
        ACTION_OPEN_PROPERTIES => Some(PANEL_TYPE_INSPECTOR),
        ACTION_OPEN_ASSET_BROWSER => Some(PANEL_TYPE_ASSETS),
        ACTION_OPEN_TIMELINE => Some(PANEL_TYPE_TIMELINE),
        ACTION_OPEN_CONSOLE => Some(PANEL_TYPE_CONSOLE),
        ACTION_OPEN_NODE_GRAPH => Some(PANEL_TYPE_NODE_GRAPH),
        _ => None,
    }
}

fn panel_category_label(category: &PanelTypeCategory) -> &str {
    match category {
        PanelTypeCategory::General => "General",
        PanelTypeCategory::Hierarchy => "Hierarchy",
        PanelTypeCategory::Inspector => "Inspector",
        PanelTypeCategory::Viewport => "Viewport",
        PanelTypeCategory::Assets => "Assets",
        PanelTypeCategory::Timeline => "Timeline",
        PanelTypeCategory::Diagnostics => "Diagnostics",
        PanelTypeCategory::Custom(label) => label.as_str(),
    }
}

fn ctrl_char(character: &str) -> Shortcut {
    shortcut_with_modifiers(
        Key::Character(character.to_owned()),
        Modifiers::new(false, true, false, false),
    )
}

fn shortcut(key: Key) -> Shortcut {
    shortcut_with_modifiers(key, Modifiers::default())
}

fn shortcut_with_modifiers(key: Key, modifiers: Modifiers) -> Shortcut {
    Shortcut::new(modifiers, key)
}

fn shortcut_label(shortcut: &Shortcut) -> String {
    let mut parts = Vec::new();
    if shortcut.modifiers.ctrl {
        parts.push("Ctrl".to_owned());
    }
    if shortcut.modifiers.shift {
        parts.push("Shift".to_owned());
    }
    if shortcut.modifiers.alt {
        parts.push("Alt".to_owned());
    }
    if shortcut.modifiers.super_key {
        parts.push("Super".to_owned());
    }
    parts.push(key_label(&shortcut.key));
    parts.join("+")
}

fn key_label(key: &Key) -> String {
    match key {
        Key::Character(character) => character.to_uppercase(),
        Key::Function(number) => format!("F{number}"),
        Key::Delete => "Del".to_owned(),
        Key::Escape => "Esc".to_owned(),
        Key::Enter => "Enter".to_owned(),
        Key::Tab => "Tab".to_owned(),
        Key::Space => "Space".to_owned(),
        Key::ArrowLeft => "Left".to_owned(),
        Key::ArrowRight => "Right".to_owned(),
        Key::ArrowUp => "Up".to_owned(),
        Key::ArrowDown => "Down".to_owned(),
        Key::Backspace => "Backspace".to_owned(),
        Key::Home => "Home".to_owned(),
        Key::End => "End".to_owned(),
        Key::PageUp => "PageUp".to_owned(),
        Key::PageDown => "PageDown".to_owned(),
        Key::Insert => "Insert".to_owned(),
        Key::Unidentified => "?".to_owned(),
    }
}

fn editor_panel_type_descriptors() -> Vec<PanelTypeDescriptor> {
    vec![
        PanelTypeDescriptor::new(PANEL_TYPE_VIEWPORT, "Viewport")
            .with_category(PanelTypeCategory::Viewport)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(760.0, 520.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_VIEWPORT)),
        PanelTypeDescriptor::new(PANEL_TYPE_SCENE, "Explorer")
            .with_category(PanelTypeCategory::Hierarchy)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(300.0, 420.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_EXPLORER)),
        PanelTypeDescriptor::new(PANEL_TYPE_INSPECTOR, "Properties")
            .with_category(PanelTypeCategory::Inspector)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(280.0, 520.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_PROPERTIES)),
        PanelTypeDescriptor::new(PANEL_TYPE_ASSETS, "Asset Browser")
            .with_category(PanelTypeCategory::Assets)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(300.0, 260.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_ASSET_BROWSER)),
        PanelTypeDescriptor::new(PANEL_TYPE_TIMELINE, "Timeline")
            .with_category(PanelTypeCategory::Timeline)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(640.0, 180.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_TIMELINE)),
        PanelTypeDescriptor::new(PANEL_TYPE_CONSOLE, "Console")
            .with_category(PanelTypeCategory::Diagnostics)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(640.0, 180.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_CONSOLE)),
        PanelTypeDescriptor::new(PANEL_TYPE_NODE_GRAPH, "Node Graph")
            .with_category(PanelTypeCategory::Timeline)
            .with_instance_policy(PanelInstancePolicy::Singleton)
            .with_default_size(Size::new(520.0, 220.0))
            .with_default_open_action(ActionId::new(ACTION_OPEN_NODE_GRAPH)),
    ]
}

fn editor_panel_registry() -> PanelRegistry {
    PanelRegistry::from_descriptors(editor_panel_type_descriptors())
        .expect("showcase panel descriptors must be unique")
}

fn editor_open_panel_metadata() -> Vec<PanelOpenActionMetadata> {
    editor_panel_registry().open_actions().collect()
}

fn editor_panel_instances() -> Vec<PanelInstanceSnapshot> {
    EDITOR_PANEL_INSTANCES
        .iter()
        .map(|spec| {
            PanelInstanceSnapshot::new(spec.id, spec.panel_type, spec.title)
                .with_state_key(spec.state_key)
        })
        .collect()
}

fn default_workspace_snapshot() -> WorkspaceSnapshot {
    default_dock_layout().workspace_snapshot(editor_panel_instances())
}

fn default_dock() -> Dock {
    let registry = editor_panel_registry();
    Dock::restore_workspace(default_workspace_snapshot(), registry.descriptors())
        .expect("default editor workspace snapshot should restore")
}

fn default_dock_layout() -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.19,
        min_first: 220.0,
        min_second: 520.0,
        first: Box::new(DockNode::Split {
            axis: Axis::Vertical,
            ratio: 0.56,
            min_first: 180.0,
            min_second: 160.0,
            first: Box::new(DockNode::Frame(Frame::new(
                FRAME_SCENE,
                vec![editor_panel(PANEL_SCENE_INSTANCE)],
            ))),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_ASSETS,
                vec![editor_panel(PANEL_ASSETS_INSTANCE)],
            ))),
        }),
        second: Box::new(DockNode::Split {
            axis: Axis::Horizontal,
            ratio: 0.76,
            min_first: 300.0,
            min_second: 180.0,
            first: Box::new(DockNode::Split {
                axis: Axis::Vertical,
                ratio: 0.74,
                min_first: 300.0,
                min_second: 140.0,
                first: Box::new(DockNode::Frame(Frame::new(
                    FRAME_VIEWPORT,
                    vec![editor_panel(PANEL_VIEWPORT_INSTANCE)],
                ))),
                second: Box::new(DockNode::Frame(Frame::new(
                    FRAME_BOTTOM,
                    vec![
                        editor_panel(PANEL_CONSOLE_INSTANCE),
                        editor_panel(PANEL_TIMELINE_INSTANCE),
                        editor_panel(PANEL_NODE_GRAPH_INSTANCE),
                    ],
                ))),
            }),
            second: Box::new(DockNode::Frame(Frame::new(
                FRAME_INSPECTOR,
                vec![editor_panel(PANEL_INSPECTOR_INSTANCE)],
            ))),
        }),
    })
}

fn editor_panel(instance: PanelInstanceId) -> Panel {
    let spec = EDITOR_PANEL_INSTANCES
        .iter()
        .find(|spec| spec.id == instance)
        .expect("editor panel instance is declared");
    Panel::from_instance_id(spec.id, spec.title)
}

fn scene_model() -> TreeModel {
    TreeModel::new(vec![
        tree_item(1, None, true),
        tree_item(2, Some(1), true),
        tree_item(3, Some(2), false),
        tree_item(4, Some(2), false),
        tree_item(5, Some(2), false),
        tree_item(6, Some(1), true),
        tree_item(7, Some(6), false),
        tree_item(8, Some(6), false),
        tree_item(9, Some(1), false),
        tree_item(10, Some(1), false),
        tree_item(11, Some(1), false),
    ])
}

const MASS_VALIDATION_ERROR: &str = "Mass must be positive";

fn mass_status(mass_text: &str) -> PropertyGridRowStatus {
    let is_positive_finite = classify_numeric_input_draft(mass_text)
        .value()
        .is_some_and(|mass| mass.is_finite() && mass > 0.0);

    if is_positive_finite {
        PropertyGridRowStatus::default()
    } else {
        PropertyGridRowStatus::error(MASS_VALIDATION_ERROR)
    }
}

fn inspector_rows(mass_text: &str) -> Vec<PropertyGridRow> {
    vec![
        PropertyGridRow::section(item_id(1), "Transform"),
        PropertyGridRow::property(item_id(2), "Position", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, true),
        PropertyGridRow::property(item_id(5), "Uniform Scale", 0)
            .with_resettable(true, true)
            .with_keyframeable(true, false),
        PropertyGridRow::section(item_id(6), "Rendering"),
        PropertyGridRow::property(item_id(7), "Exposure", 0)
            .with_status(PropertyGridRowStatus::warning("Preview range exceeded"))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(8), "Roughness", 0)
            .with_resettable(true, true)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(9), "Material", 0)
            .with_status(PropertyGridRowStatus::info("Inherited material override"))
            .with_resettable(true, false),
        PropertyGridRow::section(item_id(10), "Editor"),
        PropertyGridRow::property(item_id(11), "Snap", 0).with_resettable(true, false),
        PropertyGridRow::section(item_id(12), "Physics"),
        PropertyGridRow::property(item_id(13), "Mass", 0)
            .with_status(mass_status(mass_text))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(14), "Collider", 0)
            .with_resettable(true, false)
            .with_keyframeable(true, false),
        PropertyGridRow::property(item_id(15), "Script", 0)
            .with_read_only(true)
            .with_status(PropertyGridRowStatus::info("Script comes from prefab"))
            .with_resettable(true, false)
            .with_keyframeable(true, false),
    ]
}

fn inspector_label_width(grid_width: f32) -> f32 {
    if !grid_width.is_finite() {
        return 72.0;
    }

    (grid_width * 0.42).clamp(52.0, 96.0)
}

fn frame_tab_rects(frame: &Frame, frame_rect: Rect, tab_height: f32) -> Vec<(FrameTab, Rect)> {
    let mut tab_x = frame_rect.x + 1.0;
    frame_tab_strip(frame)
        .tabs()
        .iter()
        .cloned()
        .map(|tab| {
            let width = (tab.title.len() as f32 * 7.0 + 42.0).clamp(82.0, 146.0);
            let tab_rect = Rect::new(tab_x, frame_rect.y + 1.0, width, tab_height);
            tab_x += width + 1.0;
            (tab, tab_rect)
        })
        .collect()
}

fn frame_tab_strip(frame: &Frame) -> TabStrip {
    TabStrip::from_frame_tabs(frame_tabs(frame))
}

fn dock_drop_status(target: DockDropTarget) -> String {
    match target {
        DockDropTarget::Tab { frame } => {
            format!("Dock tab merged into frame {}", frame.raw())
        }
        DockDropTarget::Split {
            frame, placement, ..
        } => {
            let placement = match placement {
                DockPlacement::Left => "left of",
                DockPlacement::Right => "right of",
                DockPlacement::Top => "above",
                DockPlacement::Bottom => "below",
            };
            format!("Dock tab split {placement} frame {}", frame.raw())
        }
    }
}

fn draw_dock_drop_affordance(ui: &mut Ui<'_>, frame_rect: Rect, target: DockDropTarget) {
    let preview = match target {
        DockDropTarget::Tab { .. } => frame_rect.inset(24.0),
        DockDropTarget::Split {
            placement: DockPlacement::Left,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.y + 6.0,
            frame_rect.width * 0.35,
            frame_rect.height - 12.0,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Right,
            ..
        } => Rect::new(
            frame_rect.max_x() - frame_rect.width * 0.35 - 6.0,
            frame_rect.y + 6.0,
            frame_rect.width * 0.35,
            frame_rect.height - 12.0,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Top,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.y + 6.0,
            frame_rect.width - 12.0,
            frame_rect.height * 0.35,
        ),
        DockDropTarget::Split {
            placement: DockPlacement::Bottom,
            ..
        } => Rect::new(
            frame_rect.x + 6.0,
            frame_rect.max_y() - frame_rect.height * 0.35 - 6.0,
            frame_rect.width - 12.0,
            frame_rect.height * 0.35,
        ),
    };
    rect_fill(
        ui,
        preview,
        rgba(78, 142, 245, 0.18),
        Some(rgb(86, 151, 245)),
        CornerRadius::all(3.0),
    );
}

fn run_toolbar_buttons(
    viewport: Rect,
    chrome: EditorChromeMetrics,
) -> [(usize, ToolbarIcon, &'static str, &'static str, Rect); 5] {
    let right = viewport.max_x() - (chrome.toolbar_stride * 4.0 + chrome.toolbar_button);
    [
        (
            0,
            ToolbarIcon::Play,
            "Play",
            ACTION_PLAY,
            Rect::new(
                right,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            1,
            ToolbarIcon::Pause,
            "Pause",
            ACTION_PLAY,
            Rect::new(
                right + chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            2,
            ToolbarIcon::Stop,
            "Stop",
            ACTION_STOP,
            Rect::new(
                right + 2.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            3,
            ToolbarIcon::Rocket,
            "Build",
            ACTION_BUILD,
            Rect::new(
                right + 3.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
        (
            4,
            ToolbarIcon::Download,
            "Export",
            ACTION_BUILD,
            Rect::new(
                right + 4.0 * chrome.toolbar_stride,
                TOOLBAR_Y,
                chrome.toolbar_button,
                chrome.toolbar_button,
            ),
        ),
    ]
}
