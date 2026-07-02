#[test]
fn inspector_label_width_preserves_value_space_at_narrow_widths() {
    assert_eq!(inspector_label_width(120.0), 52.0);
    assert!((inspector_label_width(180.0) - 75.6).abs() < f32::EPSILON);
    assert_eq!(inspector_label_width(400.0), 96.0);
    assert_eq!(inspector_label_width(f32::NAN), 72.0);
}

#[test]
fn editor_chrome_metrics_follow_theme_controls() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);

    assert_eq!(
        chrome.toolbar_button,
        theme.controls.compact_control_height + theme.controls.padding_y
    );
    assert_eq!(
        chrome.toolbar_stride,
        chrome.toolbar_button + theme.controls.padding_x * 0.5
    );
    assert_eq!(chrome.toolbar_icon, theme.controls.icon_size);
    assert_eq!(chrome.asset_icon, theme.controls.icon_size);
    assert_eq!(chrome.toolbar_button, 26.0);
    assert_eq!(chrome.toolbar_stride, 30.0);
    assert_eq!(chrome.toolbar_icon, 16.0);
    assert_eq!(super::workspace_top(&theme), 68.0);
}

#[test]
fn editor_chrome_menu_bar_converts_active_menu_to_overlay_contract() {
    let mut editor = EditorShowcase::new();
    editor.open_menu = Some(EditorMenuKind::File);
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let menu_bar = editor.menu_bar_model();

    assert_eq!(menu_bar.menus().len(), 7);
    assert_eq!(
        menu_bar.active_id(),
        Some(EditorMenuKind::File.menu_bar_id())
    );
    assert_eq!(
        menu_bar.active_menu().expect("active file menu").title,
        "File"
    );

    let overlay = editor.menu_overlay_model(EditorMenuKind::File, viewport);

    assert_eq!(overlay.entry.kind, OverlayKind::Menu);
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.source, ActionSource::Menu);
    assert_eq!(overlay.context, ActionContext::Editor);
    assert!(overlay.entry.rect.y > super::menu_anchor(EditorMenuKind::File).max_y());
    assert!(overlay.visible_items().iter().any(|item| matches!(
        item,
        MenuItem::Action(action)
            if action.id.as_str() == ACTION_SAVE
                && action.label == "Save Scene"
                && action.can_invoke()
    )));
    assert!(overlay.visible_items().iter().any(|item| matches!(
        item,
        MenuItem::Action(action) if action.label == "Quit" && !action.can_invoke()
    )));
}

#[test]
fn editor_chrome_toolbar_contract_tracks_checked_action_state() {
    let mut editor = EditorShowcase::new();
    let toolbar = editor.toolbar_model();
    let tools = toolbar
        .group(EditorToolbarGroupKind::Tools.id())
        .expect("tools group")
        .visible_items();

    assert_eq!(
        tools.iter().map(|item| item.label()).collect::<Vec<_>>(),
        ["Select", "Move", "Rotate", "Scale"]
    );
    assert_eq!(tools[1].action_id().as_str(), super::ACTION_TOOL_MOVE);
    assert_eq!(tools[1].checked(), Some(true));
    assert_eq!(tools[0].checked(), Some(false));
    assert_eq!(
        tools[1].icon().map(kinetik_ui::core::ActionIcon::as_str),
        Some("move")
    );

    let viewport_tools = toolbar
        .group(EditorToolbarGroupKind::Viewport.id())
        .expect("viewport group")
        .visible_items();
    assert_eq!(viewport_tools[0].action_id().as_str(), ACTION_GRID);
    assert_eq!(viewport_tools[0].checked(), Some(true));

    assert!(editor.apply_action(ACTION_PLAY));
    let toolbar = editor.toolbar_model();
    let run_items = toolbar
        .group(EditorToolbarGroupKind::Run.id())
        .expect("run group")
        .visible_items();
    assert_eq!(run_items[0].label(), "Play");
    assert_eq!(run_items[0].checked(), Some(true));
    assert_eq!(run_items[1].label(), "Pause");
    assert_eq!(run_items[1].checked(), Some(false));

    let invocation = toolbar
        .invocation_for_group_visible(EditorToolbarGroupKind::Run.id(), 0, ActionContext::Editor)
        .expect("run invocation");
    assert_eq!(invocation.action_id, ActionId::new(ACTION_PLAY));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Editor);
}

#[test]
fn editor_chrome_status_bar_contract_preserves_order_counts_and_progress() {
    let mut editor = EditorShowcase::new();
    editor.status = "Busy".to_owned();
    editor.running = true;
    editor.timeline = 1.5;

    let status_bar = editor.status_bar_model(12);
    let visible = status_bar.visible_items();

    assert_eq!(
        visible
            .iter()
            .map(|item| item.text.as_str())
            .collect::<Vec<_>>(),
        [
            "Busy",
            "Actions: 12",
            "Snap 1m",
            "Vello / winit",
            "Jobs: 2 active / 4 total",
            "Diagnostics: 1E 1W 1I",
            "Feedback: 2"
        ]
    );
    let actions = status_bar
        .item(EditorStatusItemKind::Actions.id())
        .expect("action count status");
    assert_eq!(actions.kind, StatusItemKind::ActionCount);
    assert_eq!(actions.count, Some(12));

    let jobs = status_bar
        .item(EditorStatusItemKind::Jobs.id())
        .expect("job status");
    assert_eq!(jobs.kind, StatusItemKind::JobCount);
    assert_eq!(jobs.count, Some(2));
    assert!((jobs.progress.expect("job progress").value - 0.4).abs() < f32::EPSILON);
    assert!(jobs.visible);

    let diagnostics = status_bar
        .item(EditorStatusItemKind::Diagnostics.id())
        .expect("diagnostics status");
    assert_eq!(diagnostics.kind, StatusItemKind::Error);
    assert_eq!(diagnostics.count, Some(3));

    let feedback = status_bar
        .item(EditorStatusItemKind::Feedback.id())
        .expect("feedback status");
    assert_eq!(feedback.kind, StatusItemKind::Message);
    assert_eq!(feedback.count, Some(2));

    let progress = status_bar
        .item(EditorStatusItemKind::Timeline.id())
        .expect("timeline progress status");
    assert_eq!(progress.kind, StatusItemKind::Progress);
    assert_eq!(progress.progress.expect("progress metadata").value, 1.0);
    assert!(!progress.visible);
}

#[test]
fn editor_showcase_job_fixture_is_deterministic_and_app_owned() {
    let jobs = EditorShowcase::showcase_job_list();
    let summary = jobs.summary();
    let progress = jobs.active_progress().expect("active fixture jobs");

    assert_eq!(jobs.rows().len(), 4);
    assert_eq!(
        jobs.rows()
            .iter()
            .map(|row| row.label.as_str())
            .collect::<Vec<_>>(),
        [
            "Active showcase job",
            "Queued showcase job",
            "Completed showcase job",
            "Failed showcase job"
        ]
    );
    assert_eq!(summary.running, 1);
    assert_eq!(summary.queued, 1);
    assert_eq!(summary.succeeded, 1);
    assert_eq!(summary.failed, 1);
    assert_eq!(summary.active(), 2);
    assert_eq!(progress.active, 2);
    assert_eq!(progress.determinate, 2);
    assert_eq!(progress.indeterminate, 0);
    assert!(
        (progress.status_progress().expect("status progress").value - 0.4).abs() < f32::EPSILON
    );
    assert_eq!(jobs.rows()[0].phase, JobPhase::Running);
    assert!(jobs.rows()[0].can_cancel());
    assert_eq!(
        jobs.cancel_request(super::job_row_id(1))
            .expect("cancel request")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_CANCEL_ACTIVE_FIXTURE_JOB)
    );
}

#[test]
fn editor_showcase_diagnostics_fixture_summarizes_ordered_app_metadata() {
    let diagnostics = EditorShowcase::showcase_diagnostics();
    let summary = diagnostics.summary();
    let ordered = diagnostics.ordered_items();

    assert_eq!(summary.errors, 1);
    assert_eq!(summary.warnings, 1);
    assert_eq!(summary.info, 1);
    assert_eq!(summary.total(), 3);
    assert_eq!(
        ordered
            .iter()
            .map(|item| item.code.as_str())
            .collect::<Vec<_>>(),
        [
            "showcase.fixture.error",
            "showcase.fixture.warning",
            "showcase.fixture.info"
        ]
    );
    assert!(
        diagnostics.items().iter().all(|item| {
            item.source == Some(kinetik_ui::widgets::DiagnosticSource::Application)
        })
    );
}

#[test]
fn editor_showcase_feedback_fixture_preserves_lifetime_action_and_dismiss_metadata() {
    let feedback = EditorShowcase::showcase_feedback_stack();
    let active = feedback.active_items(super::showcase_feedback_now());

    assert_eq!(feedback.items().len(), 3);
    assert_eq!(active.len(), 2);
    assert_eq!(
        active.iter().map(|item| item.kind).collect::<Vec<_>>(),
        [FeedbackKind::Success, FeedbackKind::Warning]
    );
    assert_eq!(
        feedback
            .item(super::feedback_id(1))
            .expect("timed feedback")
            .remaining_lifetime(super::showcase_feedback_now()),
        Some(Duration::from_secs(4))
    );
    assert_eq!(
        feedback
            .item(super::feedback_id(3))
            .expect("expired feedback")
            .remaining_lifetime(super::showcase_feedback_now()),
        None
    );
    assert_eq!(
        feedback
            .action_request(super::feedback_id(2), super::showcase_feedback_now())
            .expect("feedback action")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_OPEN_FEEDBACK_REPORT)
    );
    assert_eq!(
        feedback
            .dismiss_request(super::feedback_id(2), super::showcase_feedback_now())
            .expect("feedback dismiss")
            .invocation
            .action_id,
        ActionId::new(super::ACTION_DISMISS_FEEDBACK_REPORT)
    );
}

#[test]
fn editor_showcase_frame_emits_no_core_warnings() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(
        output.diagnostics().is_empty(),
        "{:?}",
        output.diagnostics()
    );
}

#[test]
fn editor_chrome_tab_strip_contract_preserves_frame_tab_targets() {
    let editor = EditorShowcase::new();
    let bottom = editor.dock.frame(FRAME_BOTTOM).expect("bottom frame");
    let strip = frame_tab_strip(bottom);
    let rects = frame_tab_rects(bottom, bottom_frame_rect(&editor), 26.0);

    assert_eq!(strip.len(), 3);
    assert_eq!(rects.len(), strip.len());
    assert_eq!(strip.tabs()[0].title, "Console");
    assert_eq!(strip.tabs()[1].title, "Timeline");
    assert_eq!(strip.active_panel(), Some(strip.tabs()[0].panel));
    assert_eq!(
        strip
            .activation_target_by_index(1)
            .expect("timeline target")
            .panel,
        PANEL_TIMELINE
    );
    assert_eq!(
        strip
            .drag_target_by_panel(PANEL_TIMELINE)
            .expect("timeline drag target")
            .index,
        1
    );
}

#[test]
fn editor_timeline_fixture_exposes_data_only_semantics_snap_and_transport_requests() {
    let fixture = editor_timeline_fixture();

    fixture
        .descriptor
        .validate()
        .expect("editor-owned timeline descriptors validate");
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("timeline".to_owned()))
    );
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("timeline-item".to_owned()))
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Frame)
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Marker)
    );
    assert!(
        fixture
            .candidates
            .iter()
            .any(|candidate| candidate.source == TimelineSnapSource::Keyframe)
    );
    assert_eq!(
        fixture.transport_request.action_id,
        ActionId::new(ACTION_PLAY)
    );
    assert_eq!(fixture.transport_request.source, ActionSource::Button);
    assert_eq!(
        fixture
            .transport_request
            .timeline_context
            .expect("transport context")
            .timeline,
        TimelineId::from_raw(9_000)
    );
    assert!(
        fixture
            .state
            .selection
            .contains(TimelineSelectionTarget::Item(TimelineItemId::from_raw(11)))
    );
}

#[test]
fn editor_viewport_tool_fixture_exercises_app_owned_action_routing() {
    let fixture = editor_viewport_tool_fixture();

    assert_eq!(fixture.actions.len(), 9);
    assert_eq!(
        fixture
            .requests
            .iter()
            .map(|request| request.kind)
            .collect::<Vec<_>>(),
        vec![
            ViewportActionKind::ActivateTool,
            ViewportActionKind::FocusSelected,
            ViewportActionKind::FitContent,
            ViewportActionKind::FitSelection,
            ViewportActionKind::ActualSize,
            ViewportActionKind::ZoomIn,
            ViewportActionKind::ZoomOut,
            ViewportActionKind::PanMode,
            ViewportActionKind::ToggleOverlay,
        ]
    );
    assert!(fixture.requests.iter().all(|request| {
        request.source == ActionSource::Button
            && matches!(request.context, ActionContext::Widget(_))
    }));
    assert_eq!(
        fixture.requests[0].action_id,
        ActionId::new(super::ACTION_TOOL_SELECT)
    );
    assert_eq!(fixture.requests[0].checked, Some(true));
    assert_eq!(
        fixture.requests[1].target.selection,
        Some(ViewportSelectionTargetId::from_raw(70))
    );
    assert_eq!(
        fixture.requests[8].target.overlay,
        Some(ViewportOverlayId::from_raw(12))
    );
    assert_eq!(fixture.requests[8].checked, Some(true));
    assert!(
        fixture
            .semantic_roles
            .iter()
            .any(|role| *role == SemanticRole::Custom("viewport-actions".to_owned()))
    );
    assert!(fixture.semantic_roles.contains(&SemanticRole::Toggle));
    assert!(fixture.semantic_roles.contains(&SemanticRole::Button));
    assert_eq!(
        fixture.cursor_request.source,
        ViewportCursorRequestSource::HoveredOverlay
    );
    assert_eq!(
        fixture.cursor_request.cursor.shape,
        ViewportCursorShape::Crosshair
    );
    assert_eq!(
        fixture.cursor_request.overlay,
        Some(ViewportOverlayId::from_raw(12))
    );
}

#[test]
fn editor_chrome_modal_contract_exposes_data_only_action_metadata() {
    let editor = EditorShowcase::new();
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let before = editor.status.clone();
    let overlay = editor.about_modal_overlay_model(viewport);

    assert_eq!(overlay.entry.kind, OverlayKind::Modal);
    assert!(overlay.entry.modal);
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.context, ActionContext::Editor);
    assert_eq!(overlay.dialog.title, "About Kinetik Forge");
    assert_eq!(overlay.visible_actions().len(), 2);
    assert_eq!(
        overlay
            .visible_action_by_role(ModalActionRole::Cancel)
            .expect("cancel action")
            .action
            .label,
        "Close"
    );

    let invocation = overlay
        .invocation_for_role(ModalActionRole::Primary)
        .expect("primary modal action invocation");
    assert_eq!(invocation.action_id, ActionId::new(super::ACTION_PALETTE));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Editor);
    assert_eq!(editor.status, before);
}
