#[test]
fn showcase_about_modal_retains_across_frames_and_paints_above_guard() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);

    assert!(editor.apply_action(ACTION_ABOUT));
    let mut retained_output = None;
    for _ in 0..2 {
        let mut ui = Ui::begin_frame(
            editor_test_context(UiInput::default()),
            &mut memory,
            &theme,
        );
        let invocations = editor.render(&mut ui, 0);
        let output = ui.finish_output();
        assert!(invocations.is_empty());
        assert!(editor.about_modal_open);
        retained_output = Some(output);
    }

    let output = retained_output.expect("retained modal output");
    let overlay = editor.about_modal_overlay_model(viewport);
    let underlay_title = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == ABOUT_MODAL_PRODUCT_TITLE)
        })
        .expect("underlay title");
    let guard = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if rect.rect == viewport
                        && matches!(&rect.fill, Some(Brush::Solid(color)) if *color == rgba(0, 0, 0, 0.68))
            )
        })
        .expect("modal guard");
    let dialog = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if rect.rect == overlay.entry.rect
                        && matches!(&rect.fill, Some(Brush::Solid(color)) if *color == rgb(31, 34, 40))
            )
        })
        .expect("modal surface");
    let modal_title = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == ABOUT_MODAL_DIALOG_TITLE)
        })
        .expect("modal title");
    let modal_product = output
        .primitives
        .iter()
        .rposition(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == ABOUT_MODAL_PRODUCT_TITLE)
        })
        .expect("modal product title");
    let version = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == ABOUT_MODAL_VERSION)
        })
        .expect("modal version");
    let readiness = output
        .primitives
        .iter()
        .position(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == ABOUT_MODAL_READINESS)
        })
        .expect("modal readiness");

    assert!(underlay_title < guard);
    assert!(guard < dialog);
    assert!(dialog < modal_title);
    assert!(modal_title < modal_product);
    assert!(modal_product < version);
    assert!(version < readiness);
}

#[test]
fn showcase_about_modal_close_control_emits_unique_close_action() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    assert!(editor.apply_action(ACTION_ABOUT));
    let close = editor.about_modal_close_rect(viewport).center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(close.x, close.y, true, true, false)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    assert!(invocations.is_empty());
    assert!(editor.about_modal_open);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(close.x, close.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].action_id, ActionId::new(ACTION_ABOUT_CLOSE));
    assert_eq!(invocations[0].source, ActionSource::Button);
    assert!(!editor.about_modal_open);
    assert_eq!(editor.status, "About Kinetik Forge closed");
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn showcase_about_modal_escape_dismisses_once() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    assert!(editor.apply_action(ACTION_ABOUT));
    let mut input = UiInput::default();
    input.keyboard.events.push(KeyEvent::new(
        Key::Escape,
        KeyState::Pressed,
        Modifiers::default(),
        false,
    ));

    let mut ui = Ui::begin_frame(editor_test_context(input), &mut memory, &theme);
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(invocations.is_empty());
    assert!(!editor.about_modal_open);
    assert_eq!(editor.status, "About Kinetik Forge closed");
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let closed_status = editor.status.clone();
    let mut ui = Ui::begin_frame(
        editor_test_context(UiInput::default()),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    assert!(invocations.is_empty());
    assert_eq!(editor.status, closed_status);
}

#[test]
fn showcase_about_modal_outside_dismissal_cannot_click_through() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let select = Point::new(14.0, 40.0);
    assert_eq!(editor.selected_tool, EditorTool::Move);
    assert!(editor.apply_action(ACTION_ABOUT));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, true, true, false)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    assert!(invocations.is_empty());
    assert!(editor.about_modal_open);
    assert_eq!(editor.selected_tool, EditorTool::Move);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();
    assert!(invocations.is_empty());
    assert!(!editor.about_modal_open);
    assert_eq!(editor.selected_tool, EditorTool::Move);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].action_id, ActionId::new(super::ACTION_TOOL_SELECT));
    assert_eq!(editor.selected_tool, EditorTool::Select);
}

#[test]
fn showcase_menu_outside_dismissal_cannot_click_through_workspace() {
    let mut editor = EditorShowcase::new();
    editor.open_menu = Some(EditorMenuKind::File);
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let select = Point::new(14.0, 40.0);
    assert_eq!(editor.selected_tool, EditorTool::Move);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, true, true, false)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();
    assert!(invocations.is_empty());
    assert_eq!(editor.open_menu, Some(EditorMenuKind::File));
    assert_eq!(editor.selected_tool, EditorTool::Move);
    assert_eq!(memory.pointer_capture(), None);
    assert!(output.warnings.is_empty());

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(select.x, select.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();
    assert!(invocations.is_empty());
    assert_eq!(editor.open_menu, None);
    assert_eq!(editor.selected_tool, EditorTool::Move);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert!(output.warnings.is_empty());
}

#[test]
fn toolbar_tool_selection_updates_status_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(14.0, 40.0, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    let visible_tool_id =
        WidgetId::from_key("root").child(("editor.tool", super::ACTION_TOOL_SELECT));

    assert_eq!(memory.pressed(), Some(visible_tool_id));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(14.0, 40.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(editor.selected_tool, EditorTool::Select);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Select tool active")
    }));
}

#[test]
fn toolbar_run_click_invokes_through_visible_identity() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let (index, _icon, _label, action, rect) = super::run_toolbar_buttons(viewport, chrome)[0];
    let point = rect.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let _ = ui.finish_output();
    let visible_run_id = WidgetId::from_key("root").child(("editor.run", action, index));

    assert!(invocations.is_empty());
    assert_eq!(memory.pressed(), Some(visible_run_id));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
        &mut memory,
        &theme,
    );
    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(invocations.len(), 1);
    assert_eq!(invocations[0].action_id, ActionId::new(ACTION_PLAY));
    assert!(editor.running);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Play mode running")
    }));
}

#[test]
fn toolbar_tool_click_has_single_same_frame_selection_visual() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let rotate = Point::new(
        10.0 + 2.0 * chrome.toolbar_stride + chrome.toolbar_button * 0.5,
        TOOLBAR_Y + chrome.toolbar_button * 0.5,
    );

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(rotate.x, rotate.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(rotate.x, rotate.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let selected_fill = rgb(39, 69, 122);
    let selected_toolbar_buttons = output
        .primitives
        .iter()
        .filter(|primitive| match primitive {
            Primitive::Rect(rect) => {
                rect.rect.y == TOOLBAR_Y
                    && rect.rect.width == chrome.toolbar_button
                    && rect.rect.height == chrome.toolbar_button
                    && matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selected_fill)
            }
            _ => false,
        })
        .count();

    assert_eq!(editor.selected_tool, EditorTool::Rotate);
    assert_eq!(selected_toolbar_buttons, 1);
}

#[test]
fn frame_tab_click_updates_model_and_next_prepared_public_scene() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(
        editor_test_context(UiInput::default()),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let idle = ui.finish_output();
    let point = idle
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.role == SemanticRole::Tab && node.label.as_deref() == Some("Timeline")
        })
        .map(|node| node.bounds.center())
        .expect("public dock Timeline tab semantics");

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(point.x, point.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let released = ui.finish_output();

    assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
    assert_eq!(
        editor
            .dock
            .frame(FRAME_BOTTOM)
            .and_then(|frame| frame.active_panel())
            .map(|panel| panel.id),
        Some(PANEL_TIMELINE)
    );
    assert_eq!(released.repaint, RepaintRequest::NextFrame);

    let mut ui = Ui::begin_frame(
        editor_test_context(UiInput::default()),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Tab
            && node.label.as_deref() == Some("Timeline")
            && node.state.selected
    }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Intro camera pan")
    }));
    assert!(
        !output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Message")
        })
    );
}

#[test]
fn splitter_drag_routes_through_dock_resize_path() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bounds = editor_workspace_bounds();
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let before = splitter.ratio;
    let press = splitter.rect.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(press.x, press.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at_with_delta(
            press.x + 48.0,
            press.y,
            true,
            false,
            false,
            Vec2::new(48.0, 0.0),
        )),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let after =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter")
            .ratio;

    assert!(after > before, "{after} should be greater than {before}");
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn editor_splitter_join_action_uses_context_metadata_and_apply_request() {
    let mut editor = EditorShowcase::new();
    let bounds = editor_workspace_bounds();
    let layout = solve_dock_layout(&editor.dock, bounds);
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let action = resolve_dock_splitter_context_actions_with_policy(
        &editor.dock,
        &layout,
        &splitter,
        super::editor_dock_interaction_policy(),
    )
    .into_iter()
    .find(|action| action.kind == DockSplitterContextActionKind::Join && action.enabled)
    .expect("enabled join action");
    let request = action.join_request().expect("join request");
    let source = request.source_frame();
    let target = request.target_frame();

    assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Join));

    assert!(editor.dock.frame(source).is_none());
    assert!(editor.dock.frame(target).is_some());
    assert_eq!(editor.dock.active_frame(), Some(target));
    assert_eq!(
        editor.status,
        format!(
            "Dock splitter joined frame {} into frame {}",
            source.raw(),
            target.raw()
        )
    );
}

#[test]
fn editor_splitter_swap_action_uses_context_metadata_and_apply_request() {
    let mut editor = EditorShowcase::new();
    let bounds = editor_workspace_bounds();
    let layout = solve_dock_layout(&editor.dock, bounds);
    let splitter =
        solve_dock_splitters_with_style(&editor.dock, bounds, super::editor_dock_chrome_style())
            .into_iter()
            .next()
            .expect("root splitter");
    let action = resolve_dock_splitter_context_actions_with_policy(
        &editor.dock,
        &layout,
        &splitter,
        super::editor_dock_interaction_policy(),
    )
    .into_iter()
    .find(|action| action.kind == DockSplitterContextActionKind::Swap && action.enabled)
    .expect("enabled swap action");
    let request = action.swap_request().expect("swap request");
    let source = request.source_frame();
    let target = request.target_frame();
    let source_before = editor_frame_rect(&editor, source);
    let target_before = editor_frame_rect(&editor, target);

    assert!(editor.apply_splitter_context_action(bounds, DockSplitterContextActionKind::Swap));

    assert_eq!(editor_frame_rect(&editor, source), target_before);
    assert_eq!(editor_frame_rect(&editor, target), source_before);
    assert_eq!(
        editor.status,
        format!(
            "Dock splitter swapped frame {} with frame {}",
            source.raw(),
            target.raw()
        )
    );
}
