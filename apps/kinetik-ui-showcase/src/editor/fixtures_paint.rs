fn tree_item(raw: u64, parent: Option<u64>, has_children: bool) -> TreeItem {
    TreeItem {
        id: item_id(raw),
        parent: parent.map(item_id),
        has_children,
    }
}

const fn item_id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

const fn job_row_id(raw: u64) -> JobRowId {
    JobRowId::from_raw(raw)
}

const fn diagnostic_item_id(raw: u64) -> DiagnosticStripItemId {
    DiagnosticStripItemId::from_raw(raw)
}

const fn feedback_id(raw: u64) -> FeedbackId {
    FeedbackId::from_raw(raw)
}

fn showcase_feedback_now() -> Duration {
    Duration::from_secs(6)
}

fn status_item_text_width(text: &str) -> f32 {
    text.len() as f32 * 6.4 + 28.0
}

fn severity_label(severity: DiagnosticStripSeverity) -> &'static str {
    match severity {
        DiagnosticStripSeverity::Error => "Error",
        DiagnosticStripSeverity::Warning => "Warning",
        DiagnosticStripSeverity::Info => "Info",
    }
}

fn severity_color(severity: DiagnosticStripSeverity) -> Color {
    match severity {
        DiagnosticStripSeverity::Error => rgb(236, 96, 96),
        DiagnosticStripSeverity::Warning => rgb(232, 179, 90),
        DiagnosticStripSeverity::Info => rgb(135, 176, 236),
    }
}

fn job_phase_label(phase: JobPhase) -> &'static str {
    match phase {
        JobPhase::Queued => "Queued",
        JobPhase::Running => "Running",
        JobPhase::Cancelling => "Cancelling",
        JobPhase::Succeeded => "Done",
        JobPhase::Failed => "Failed",
    }
}

fn job_phase_color(phase: JobPhase) -> Color {
    match phase {
        JobPhase::Queued => rgb(154, 160, 168),
        JobPhase::Running => rgb(135, 176, 236),
        JobPhase::Cancelling => rgb(232, 179, 90),
        JobPhase::Succeeded => rgb(114, 190, 145),
        JobPhase::Failed => rgb(236, 96, 96),
    }
}

fn feedback_kind_label(kind: FeedbackKind) -> &'static str {
    match kind {
        FeedbackKind::Info => "Info",
        FeedbackKind::Success => "Success",
        FeedbackKind::Warning => "Warning",
        FeedbackKind::Error => "Error",
    }
}

fn feedback_kind_color(kind: FeedbackKind) -> Color {
    match kind {
        FeedbackKind::Info => rgb(135, 176, 236),
        FeedbackKind::Success => rgb(114, 190, 145),
        FeedbackKind::Warning => rgb(232, 179, 90),
        FeedbackKind::Error => rgb(236, 96, 96),
    }
}

struct Asset {
    name: &'static str,
    kind: &'static str,
    icon: ToolbarIcon,
}

const ASSETS: &[Asset] = &[
    Asset {
        name: "camp_scene",
        kind: "scene",
        icon: ToolbarIcon::Cube,
    },
    Asset {
        name: "terrain_forest",
        kind: "mesh",
        icon: ToolbarIcon::Box,
    },
    Asset {
        name: "van_body",
        kind: "mesh",
        icon: ToolbarIcon::Component,
    },
    Asset {
        name: "campfire",
        kind: "prefab",
        icon: ToolbarIcon::Tokens,
    },
    Asset {
        name: "night_sky",
        kind: "texture",
        icon: ToolbarIcon::Image,
    },
    Asset {
        name: "hero_ctrl",
        kind: "script",
        icon: ToolbarIcon::Code,
    },
    Asset {
        name: "audio_loop",
        kind: "asset",
        icon: ToolbarIcon::Archive,
    },
    Asset {
        name: "lighting_lut",
        kind: "texture",
        icon: ToolbarIcon::Image,
    },
];

struct LogRow {
    time: &'static str,
    level: &'static str,
    message: &'static str,
}

const LOGS: &[LogRow] = &[
    LogRow {
        time: "00:00.1",
        level: "Info",
        message: "Loaded project campfire_adventure.kforge",
    },
    LogRow {
        time: "00:00.3",
        level: "Info",
        message: "Registered 28 Phosphor toolbar icons",
    },
    LogRow {
        time: "00:01.2",
        level: "Warn",
        message: "Light probe bake uses preview samples",
    },
    LogRow {
        time: "00:02.6",
        level: "Info",
        message: "Viewport texture uploaded through TextureResource",
    },
    LogRow {
        time: "00:03.1",
        level: "Info",
        message: "Scene graph visible range solved deterministically",
    },
];

fn scene_label(id: ItemId) -> &'static str {
    match id.raw() {
        1 => "CampfireAdventure",
        2 => "World",
        3 => "DirectionalLight",
        4 => "MainCamera",
        5 => "ReflectionProbe",
        6 => "Actors",
        7 => "Player",
        8 => "Van",
        9 => "Terrain",
        10 => "CampfireFX",
        11 => "AudioBus",
        _ => "Node",
    }
}

fn scene_icon(id: ItemId) -> ToolbarIcon {
    match id.raw() {
        1 => ToolbarIcon::Layers,
        2 | 6 => ToolbarIcon::Caret,
        3 => ToolbarIcon::Eye,
        4 => ToolbarIcon::Crosshair,
        5 => ToolbarIcon::Grid,
        7 => ToolbarIcon::Component,
        8 | 9 => ToolbarIcon::Cube,
        10 => ToolbarIcon::Rocket,
        11 => ToolbarIcon::Archive,
        _ => ToolbarIcon::Box,
    }
}

fn inspector_value_label(id: ItemId) -> &'static str {
    match id.raw() {
        9 => "M_AdventureNight",
        14 => "Capsule",
        15 => "player_controller.lua",
        _ => "-",
    }
}

fn inspector_numeric_scrub(
    ui: &mut Ui<'_>,
    key: &'static str,
    rect: Rect,
    state: &mut TextEditState,
    config: NumericScrubInputConfig,
) {
    let mut value = classify_numeric_input_draft(&state.text)
        .value()
        .unwrap_or(0.0);
    ui.numeric_scrub_input(key, rect, &mut value, state, config);
}

fn log_color(level: &str) -> Color {
    match level {
        "Warn" => rgb(232, 179, 90),
        "Error" => rgb(236, 96, 96),
        _ => rgb(190, 197, 205),
    }
}

trait RectExt {
    fn with_width(self, width: f32) -> Self;
    fn right_strip(self, width: f32) -> Self;
}

impl RectExt for Rect {
    fn with_width(self, width: f32) -> Self {
        Rect::new(self.x, self.y, width.max(0.0), self.height)
    }

    fn right_strip(self, width: f32) -> Self {
        let width = width.max(0.0).min(self.width.max(0.0));
        Rect::new(self.max_x() - width, self.y, width, self.height)
    }
}

trait PanZoomExt {
    fn content_zoom(self) -> f32;
}

impl PanZoomExt for PanZoom {
    fn content_zoom(self) -> f32 {
        match self.fit {
            ViewportFit::Zoom => self.zoom,
            _ => 1.0,
        }
    }
}

fn rect(ui: &mut Ui<'_>, rect: Rect, fill: Color, stroke: Option<Color>) {
    rect_fill(ui, rect, fill, stroke, CornerRadius::all(0.0));
}

fn rect_fill(
    ui: &mut Ui<'_>,
    rect: Rect,
    fill: Color,
    stroke: Option<Color>,
    radius: CornerRadius,
) {
    ui.primitive(Primitive::Rect(RectPrimitive {
        rect,
        fill: Some(Brush::Solid(fill)),
        stroke: stroke.map(|stroke| Stroke::new(1.0, Brush::Solid(stroke))),
        radius,
    }));
}

fn line(ui: &mut Ui<'_>, from: Point, to: Point, color: Color, width: f32) {
    ui.primitive(Primitive::Line(LinePrimitive {
        from,
        to,
        stroke: Stroke::new(width, Brush::Solid(color)),
    }));
}

fn toolbar_icon_button(
    ui: &mut Ui<'_>,
    key: impl std::hash::Hash,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
) -> kinetik_ui::core::Response {
    toolbar_icon_button_sized(
        ui,
        key,
        rect,
        icon,
        label,
        selected,
        disabled,
        EditorChromeMetrics::from_theme(ui.theme()).toolbar_icon,
    )
}

#[allow(clippy::too_many_arguments)]
fn toolbar_icon_button_sized(
    ui: &mut Ui<'_>,
    key: impl std::hash::Hash,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
    icon_size: f32,
) -> kinetik_ui::core::Response {
    let id = ui.id(key);
    let response = ui.pressable_with_id(id, rect, disabled);
    paint_toolbar_icon_button_sized(
        ui, id, response, rect, icon, label, selected, disabled, icon_size,
    );

    response
}

#[allow(clippy::too_many_arguments)]
fn paint_toolbar_icon_button_sized(
    ui: &mut Ui<'_>,
    id: WidgetId,
    response: Response,
    rect: Rect,
    icon: ToolbarIcon,
    label: &str,
    selected: bool,
    disabled: bool,
    icon_size: f32,
) {
    let visual_selected = selected || response.clicked;
    let fill = if disabled {
        rgb(24, 25, 28)
    } else if visual_selected {
        rgb(39, 69, 122)
    } else if response.state.pressed {
        rgb(35, 37, 42)
    } else if response.state.hovered {
        rgb(31, 33, 38)
    } else {
        rgb(24, 25, 28)
    };
    let stroke = if visual_selected {
        rgb(83, 137, 230)
    } else {
        rgb(58, 61, 68)
    };
    let color = if disabled {
        rgb(112, 118, 128)
    } else if visual_selected {
        rgb(246, 248, 252)
    } else {
        rgb(218, 223, 232)
    };

    rect_fill(ui, rect, fill, Some(stroke), CornerRadius::all(4.0));
    let icon_size = clamped_icon_size(icon_size, rect);
    draw_tinted_icon(ui, rect, icon, icon_size, color);

    let mut semantics = icon_button_semantics(id, rect, label, disabled);
    semantics.state.focused = response.state.focused;
    semantics.state.pressed = response.state.pressed;
    semantics.state.selected = visual_selected;
    ui.push_semantic_node(semantics);
    if response.state.hovered && !disabled {
        ui.push_platform_request(PlatformRequest::SetCursor(CursorShape::PointingHand));
    }
}

fn draw_icon(ui: &mut Ui<'_>, bounds: Rect, icon: ToolbarIcon, size: f32) {
    draw_tinted_icon(ui, bounds, icon, size, rgb(205, 212, 222));
}

fn draw_tinted_icon(ui: &mut Ui<'_>, bounds: Rect, icon: ToolbarIcon, size: f32, color: Color) {
    let size = clamped_icon_size(size, bounds);
    let rect = Rect::new(
        bounds.x + (bounds.width - size) * 0.5,
        bounds.y + (bounds.height - size) * 0.5,
        size,
        size,
    );
    ui.primitive(Primitive::Image(ImagePrimitive {
        image: phosphor_icons::icon_image(
            icon.phosphor(),
            size,
            ui.viewport().scale_factor.value(),
        ),
        rect,
        tint: Some(color),
    }));
}

fn clamped_icon_size(size: f32, bounds: Rect) -> f32 {
    let requested = if size.is_finite() && size > 0.0 {
        size
    } else {
        DENSE_ICON_SIZE
    };
    let available = bounds.width.min(bounds.height).max(1.0);
    requested.min(available)
}

fn text(ui: &mut Ui<'_>, x: f32, baseline: f32, value: &str, size: f32, fill: Color) {
    ui.primitive(Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(x, baseline),
        text: value.to_owned(),
        family: "sans-serif".to_owned(),
        size,
        line_height: size + 5.0,
        brush: Brush::Solid(fill),
    }));
}

fn rgb(red: u8, green: u8, blue: u8) -> Color {
    Color::rgb(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
    )
}

fn rgba(red: u8, green: u8, blue: u8, alpha: f32) -> Color {
    Color::rgba(
        f32::from(red) / 255.0,
        f32::from(green) / 255.0,
        f32::from(blue) / 255.0,
        alpha,
    )
}
