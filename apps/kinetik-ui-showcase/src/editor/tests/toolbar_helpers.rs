#[test]
fn editor_toolbar_atlas_icons_preserve_icon_button_semantics() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let toolbar_labels = [
        ("Select", false),
        ("Move", false),
        ("Rotate", false),
        ("Scale", false),
        ("Toggle grid", false),
        ("Frame selected (Experimental)", true),
        ("Reset view (Experimental)", true),
        ("Play", false),
        ("Pause (Experimental)", true),
        ("Stop", false),
        ("Build (Experimental)", true),
        ("Export (Experimental)", true),
    ];

    for (label, disabled) in toolbar_labels {
        assert!(
            output.semantics.nodes().iter().any(|node| {
                node.role == SemanticRole::IconButton
                    && node.label.as_deref() == Some(label)
                    && node.state.disabled == disabled
                    && node.focusable != disabled
            }),
            "missing toolbar icon semantics for {label}"
        );
    }
}

#[test]
fn editor_toolbar_atlas_icons_request_hover_cursor() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(pointer_input_at(20.0, 44.0, false, false, false));
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(
        output
            .platform_requests
            .contains(&PlatformRequest::SetCursor(CursorShape::PointingHand))
    );
}

fn editor_test_context(input: UiInput) -> FrameContext {
    editor_test_context_scaled(input, ScaleFactor::ONE)
}

fn editor_test_context_scaled(input: UiInput, scale_factor: ScaleFactor) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(1440.0, 900.0),
            PhysicalSize::new(
                (1440.0 * scale_factor.value()).round() as u32,
                (900.0 * scale_factor.value()).round() as u32,
            ),
            scale_factor,
        ),
        input,
        TimeInfo::default(),
    )
}

fn bottom_frame_rect(editor: &EditorShowcase) -> Rect {
    editor_frame_rect(editor, FRAME_BOTTOM)
}

fn editor_frame_rect(editor: &EditorShowcase, frame: super::FrameId) -> Rect {
    solve_dock_layout(&editor.dock, editor_workspace_bounds())
        .into_iter()
        .find(|layout| layout.frame == frame)
        .map(|layout| layout.rect.inset(2.0))
        .expect("editor frame")
}

fn editor_workspace_bounds() -> Rect {
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let theme = default_dark_theme();
    let workspace_top = super::workspace_top(&theme);
    Rect::new(
        4.0,
        workspace_top,
        viewport.width - 8.0,
        viewport.height - workspace_top - 28.0,
    )
}

fn frame_body_rect(frame_rect: Rect) -> Rect {
    let tab_height = 26.0;
    Rect::new(
        frame_rect.x + 1.0,
        frame_rect.y + tab_height + 2.0,
        (frame_rect.width - 2.0).max(0.0),
        (frame_rect.height - tab_height - 3.0).max(0.0),
    )
}

fn point_is_in_toolbar(point: Point) -> bool {
    let chrome = EditorChromeMetrics::from_theme(&default_dark_theme());
    point.y >= TOOLBAR_Y && point.y <= TOOLBAR_Y + chrome.toolbar_button
}

fn count_primitives(primitives: &[Primitive], predicate: impl Fn(&Primitive) -> bool) -> usize {
    primitives
        .iter()
        .filter(|primitive| predicate(primitive))
        .count()
}

fn count_semantic_role(output: &kinetik_ui::core::FrameOutput, role: &SemanticRole) -> usize {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| &node.role == role)
        .count()
}

fn focused_frame_semantic_count(output: &kinetik_ui::core::FrameOutput) -> usize {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::Frame && node.state.focused)
        .count()
}

fn pointer_input_at(x: f32, y: f32, down: bool, pressed: bool, released: bool) -> UiInput {
    pointer_input_at_with_delta(x, y, down, pressed, released, Vec2::ZERO)
}

fn pointer_input_at_with_delta(
    x: f32,
    y: f32,
    down: bool,
    pressed: bool,
    released: bool,
    delta: Vec2,
) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(x, y)),
            delta,
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn atlas_pixel(data: &[u8], width: u32, x: u32, y: u32) -> &[u8] {
    let start = ((y * width + x) * 4) as usize;
    &data[start..start + 4]
}

fn is_editor_icon(image: kinetik_ui::core::ImageId) -> bool {
    phosphor_icons::ICON_ENTRIES
        .iter()
        .any(|entry| entry.image == image)
}

fn icon_entry(image: kinetik_ui::core::ImageId) -> &'static phosphor_icons::PhosphorIconEntry {
    phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| entry.image == image)
        .expect("icon entry")
}
