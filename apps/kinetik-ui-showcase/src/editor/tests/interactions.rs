#[test]
fn inspector_snap_toggle_updates_status_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(1290.0, 362.0, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(1290.0, 362.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(!editor.snap_enabled);
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Snap off")
    }));
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
fn frame_tab_click_updates_body_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bottom = bottom_frame_rect(&editor);
    let timeline = editor
        .dock
        .frame(FRAME_BOTTOM)
        .and_then(|frame| {
            frame_tab_rects(frame, bottom, 26.0)
                .into_iter()
                .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                .map(|(_tab, rect)| rect)
        })
        .expect("timeline tab");
    let point = Point::new(
        timeline.x + timeline.width * 0.5,
        timeline.y + timeline.height * 0.5,
    );

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
    let output = ui.finish_output();

    assert_eq!(editor.dock.active_frame(), Some(FRAME_BOTTOM));
    assert_eq!(focused_frame_semantic_count(&output), 1);
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Frame
            && node.label.as_deref() == Some("Frame 4")
            && node.state.focused
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

#[test]
fn tab_drag_drop_uses_dock_drag_and_target_without_panel_metadata_mutation() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let bottom = bottom_frame_rect(&editor);
    let inspector = editor_frame_rect(&editor, FRAME_INSPECTOR);
    let timeline = editor
        .dock
        .frame(FRAME_BOTTOM)
        .and_then(|frame| {
            frame_tab_rects(frame, bottom, 26.0)
                .into_iter()
                .find(|(tab, _rect)| tab.panel == PANEL_TIMELINE)
                .map(|(_tab, rect)| rect)
        })
        .expect("timeline tab");
    let start = timeline.center();
    let target = inspector.center();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(start.x, start.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let drag_delta = Vec2::new(target.x - start.x, target.y - start.y);
    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at_with_delta(
            target.x, target.y, true, false, false, drag_delta,
        )),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let dragging_output = ui.finish_output();

    assert_eq!(editor.status, "Dragging Timeline tab");
    assert!(dragging_output.primitives.iter().any(|primitive| {
            matches!(
                primitive,
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == rgba(78, 142, 245, 0.18))
                        && inspector.contains_point(rect.rect.center())
            )
        }));

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(target.x, target.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let inspector_frame = editor.dock.frame(FRAME_INSPECTOR).expect("inspector frame");
    let timeline_panel = inspector_frame
        .panels
        .iter()
        .find(|panel| panel.id == PANEL_TIMELINE)
        .expect("moved timeline panel");

    assert_eq!(timeline_panel.title, "Timeline");
    assert_eq!(
        inspector_frame.active_panel().map(|panel| panel.id),
        Some(PANEL_TIMELINE)
    );
    assert_eq!(editor.dock.active_frame(), Some(FRAME_INSPECTOR));
    assert!(
        !editor
            .dock
            .frame(FRAME_BOTTOM)
            .expect("bottom frame")
            .panels
            .iter()
            .any(|panel| panel.id == PANEL_TIMELINE)
    );
    assert!(editor.status.contains("Dock tab merged into frame"));
    assert_eq!(output.repaint, RepaintRequest::NextFrame);

    let registry = super::editor_panel_registry();
    let moved_workspace = editor
        .dock
        .workspace_snapshot(super::editor_panel_instances());
    moved_workspace
        .validate(registry.descriptors())
        .expect("moved workspace validates");
    let moved_timeline = moved_workspace
        .panel_instances
        .iter()
        .find(|instance| instance.id == PANEL_TIMELINE.instance_id())
        .expect("timeline instance metadata");
    assert_eq!(moved_timeline.panel_type, super::PANEL_TYPE_TIMELINE);
    assert_eq!(moved_timeline.title, "Timeline");
    assert_eq!(moved_timeline.state_key.as_deref(), Some("editor.timeline"));
    let restored = super::Dock::restore_workspace(moved_workspace.clone(), registry.descriptors())
        .expect("moved workspace restores");
    assert_eq!(restored.snapshot(), moved_workspace.dock);
}
