#[test]
fn inspector_label_width_preserves_value_space_at_narrow_widths() {
    assert_eq!(inspector_label_width(120.0), 52.0);
    assert!((inspector_label_width(180.0) - 75.6).abs() < f32::EPSILON);
    assert_eq!(inspector_label_width(400.0), 96.0);
    assert_eq!(inspector_label_width(f32::NAN), 72.0);
}

fn mass_fixture_row(mass_text: &str) -> kinetik_ui::widgets::PropertyGridRow {
    super::inspector_rows(mass_text)
        .into_iter()
        .find(|row| row.id == item_id(13))
        .expect("mass fixture row")
}

fn mass_fixture_rendered_label_brush(mass_text: &str) -> Brush {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();
    mass_text.clone_into(&mut editor.mass.text);

    editor.inspector(&mut ui, Rect::new(0.0, 0.0, 400.0, 500.0));

    ui.finish_output()
        .primitives
        .into_iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "Mass" => Some(text.brush),
            _ => None,
        })
        .expect("rendered mass label")
}

#[test]
fn mass_fixture_positive_finite_values_have_no_error() {
    let editor = EditorShowcase::new();
    assert_eq!(editor.mass.text, "84.0");

    for mass_text in ["84.0", "0.001", " 12.5 ", "3.4028235e38"] {
        let status = mass_fixture_row(mass_text).state.status;
        assert_eq!(
            status.severity,
            kinetik_ui::widgets::PropertyGridStatusSeverity::None,
            "unexpected status for {mass_text:?}"
        );
        assert_eq!(status.message, None, "unexpected message for {mass_text:?}");
    }
}

#[test]
fn mass_fixture_non_positive_or_non_finite_values_have_exact_error() {
    for mass_text in [
        "0",
        "-0.0",
        "-3.5",
        "NaN",
        "nan",
        "inf",
        "+inf",
        "-inf",
        "Infinity",
        "-Infinity",
        "",
        "   ",
        "heavy",
    ] {
        let status = mass_fixture_row(mass_text).state.status;
        assert_eq!(
            status.severity,
            kinetik_ui::widgets::PropertyGridStatusSeverity::Error,
            "unexpected status for {mass_text:?}"
        );
        assert_eq!(
            status.message.as_deref(),
            Some(super::MASS_VALIDATION_ERROR),
            "unexpected message for {mass_text:?}"
        );
    }
}

#[test]
fn mass_fixture_render_uses_state_derived_status() {
    assert_eq!(
        mass_fixture_rendered_label_brush("84.0"),
        Brush::Solid(default_dark_theme().colors.text_muted)
    );
    assert_eq!(
        mass_fixture_rendered_label_brush("0"),
        Brush::Solid(default_dark_theme().colors.danger)
    );
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
                && action.shortcut.is_none()
    )));
    assert!(overlay.visible_items().iter().any(|item| matches!(
        item,
        MenuItem::Action(action)
            if action.label == "Quit (Experimental)" && !action.can_invoke()
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

    let run_items = toolbar
        .group(EditorToolbarGroupKind::Run.id())
        .expect("run group")
        .visible_items();
    let invocation = toolbar
        .invocation_for_group_visible(EditorToolbarGroupKind::Run.id(), 0, ActionContext::Editor)
        .expect("run invocation");
    assert_eq!(invocation.action_id, ActionId::new(ACTION_PLAY));
    assert_eq!(invocation.source, ActionSource::Button);
    assert_eq!(invocation.context, ActionContext::Editor);
    assert!(run_items[0].can_invoke());

    assert!(editor.apply_action(ACTION_PLAY));
    let toolbar = editor.toolbar_model();
    let run_items = toolbar
        .group(EditorToolbarGroupKind::Run.id())
        .expect("run group")
        .visible_items();
    assert_eq!(run_items[0].label(), "Play");
    assert_eq!(run_items[0].checked(), Some(true));
    assert!(!run_items[0].can_invoke());
    assert_eq!(run_items[1].label(), "Pause (Experimental)");
    assert_eq!(run_items[1].checked(), None);
    assert!(!run_items[1].can_invoke());
}

#[test]
fn showcase_action_truth_editor_menu_descriptors_are_unique_and_invocable_only_when_implemented() {
    let editor = EditorShowcase::new();
    let mut ids = HashSet::new();

    for kind in [
        EditorMenuKind::File,
        EditorMenuKind::Edit,
        EditorMenuKind::View,
        EditorMenuKind::Project,
        EditorMenuKind::Build,
        EditorMenuKind::Window,
        EditorMenuKind::Help,
    ] {
        let menu = editor.menu_model(kind);
        for item in menu.visible_items() {
            let MenuItem::Action(action) = item else {
                continue;
            };
            assert!(
                ids.insert(action.id.as_str().to_owned()),
                "duplicate action ID {}",
                action.id.as_str()
            );
            if action.can_invoke() {
                assert!(!action.label.ends_with(" (Experimental)"));
                assert!(
                    EditorShowcase::new().apply_action(action.id.as_str()),
                    "enabled action {} has no outcome",
                    action.id.as_str()
                );
            } else {
                assert!(
                    action.label.ends_with(" (Experimental)"),
                    "disabled action {} lacks truth label",
                    action.id.as_str()
                );
                assert_eq!(action.shortcut, None);
            }
        }
    }
}

#[test]
fn showcase_action_truth_toolbar_ids_and_disabled_contract_are_distinct() {
    let toolbar = EditorShowcase::new().toolbar_model();
    let mut ids = HashSet::new();

    for item in toolbar
        .groups()
        .iter()
        .flat_map(kinetik_ui::widgets::ToolbarGroup::items)
    {
        assert!(
            ids.insert(item.action_id().as_str().to_owned()),
            "duplicate toolbar action ID {}",
            item.action_id().as_str()
        );
        if item.can_invoke() {
            assert!(!item.label().ends_with(" (Experimental)"));
        } else {
            assert!(item.label().ends_with(" (Experimental)"));
            assert_eq!(item.action.shortcut, None);
        }
    }

    let run = toolbar
        .group(EditorToolbarGroupKind::Run.id())
        .expect("run group")
        .items();
    assert_eq!(run[0].action_id().as_str(), ACTION_PLAY);
    assert_eq!(run[1].action_id().as_str(), ACTION_PAUSE);
    assert_eq!(run[3].action_id().as_str(), ACTION_BUILD);
    assert_eq!(run[4].action_id().as_str(), ACTION_EXPORT);

    let chrome = EditorChromeMetrics::from_theme(&default_dark_theme());
    for ((_, _, _, painted_action, _), item) in
        super::run_toolbar_buttons(Rect::new(0.0, 0.0, 1440.0, 900.0), chrome)
            .into_iter()
            .zip(run)
    {
        assert_eq!(painted_action, item.action_id().as_str());
    }
}

#[test]
fn showcase_action_truth_apply_action_rejects_every_unfinished_outcome() {
    for action_id in [
        super::ACTION_NEW_SCENE,
        super::ACTION_OPEN_PROJECT,
        super::ACTION_IMPORT_ASSET,
        ACTION_EXPORT,
        super::ACTION_QUIT,
        super::ACTION_UNDO,
        super::ACTION_REDO,
        super::ACTION_DUPLICATE,
        super::ACTION_DELETE,
        super::ACTION_PREFERENCES,
        super::ACTION_VIEW_PERSPECTIVE,
        super::ACTION_SHOW_OVERLAYS,
        ACTION_VIEWPORT_FOCUS_SELECTED,
        ACTION_VIEWPORT_FIT_CONTENT,
        ACTION_VIEWPORT_FIT_SELECTION,
        ACTION_VIEWPORT_ACTUAL_SIZE,
        ACTION_VIEWPORT_ZOOM_IN,
        ACTION_VIEWPORT_ZOOM_OUT,
        ACTION_VIEWPORT_PAN,
        ACTION_PAUSE,
        super::ACTION_PROJECT_SETTINGS,
        ACTION_BUILD,
        super::ACTION_PACKAGE_WINDOWS,
        super::ACTION_RUN_PROFILER,
        ACTION_PALETTE,
        super::ACTION_KEYBOARD_SHORTCUTS,
    ] {
        let mut editor = EditorShowcase::new();
        let before = editor.status.clone();
        assert!(!editor.apply_action(action_id), "{action_id}");
        assert_eq!(editor.status, before, "{action_id}");
    }
}

#[test]
fn showcase_action_truth_enabled_editor_actions_mutate_dedicated_state() {
    let mut editor = EditorShowcase::new();

    assert!(editor.apply_action(ACTION_PLAY));
    assert!(editor.running);
    assert_eq!(editor.status, "Play mode running");

    assert!(editor.apply_action(ACTION_STOP));
    assert!(!editor.running);
    assert_eq!(editor.timeline, 0.0);
    assert_eq!(editor.status, "Play mode stopped");

    let grid_before = editor.grid_visible;
    assert!(editor.apply_action(ACTION_GRID));
    assert_ne!(editor.grid_visible, grid_before);

    assert!(editor.apply_action(super::ACTION_TOOL_SELECT));
    assert_eq!(editor.selected_tool, EditorTool::Select);

    assert!(editor.apply_action(ACTION_SAVE));
    assert_eq!(editor.save_revision, 1);
    assert!(editor.saved_project.is_some());
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
fn showcase_about_modal_action_truth_and_open_is_idempotent() {
    let mut editor = EditorShowcase::new();
    let viewport = Rect::new(0.0, 0.0, 1440.0, 900.0);
    let help = editor.menu_model(EditorMenuKind::Help);
    let help_actions = help
        .visible_items()
        .into_iter()
        .filter_map(|item| match item {
            MenuItem::Action(action) => Some(action),
            _ => None,
        })
        .collect::<Vec<_>>();
    let about = help_actions
        .iter()
        .find(|action| action.id.as_str() == ACTION_ABOUT)
        .expect("Help menu declares About");
    let documentation = help_actions
        .iter()
        .find(|action| action.id.as_str() == ACTION_DOCS)
        .expect("Help menu declares Documentation");

    assert_ne!(about.id, documentation.id);
    assert_eq!(about.label, ABOUT_MODAL_DIALOG_TITLE);
    assert!(about.can_invoke());
    assert_eq!(about.shortcut, None);
    assert_eq!(documentation.label, "Online Docs");
    assert!(documentation.can_invoke());
    let documentation_shortcut = documentation.shortcut.as_ref().expect("F1 shortcut");
    assert_eq!(documentation_shortcut.modifiers, Modifiers::default());
    assert_eq!(documentation_shortcut.key, Key::Function(1));
    assert_eq!(documentation_shortcut.physical_key, None);
    assert!(editor.apply_action(ACTION_DOCS));
    assert_eq!(editor.status, "Online documentation requested");

    assert!(editor.apply_action(ACTION_ABOUT));
    assert!(editor.about_modal_open);
    let opened_status = editor.status.clone();
    assert!(editor.apply_action(ACTION_ABOUT));
    assert!(editor.about_modal_open);
    assert_eq!(editor.status, opened_status);

    let overlay = editor.about_modal_overlay_model(viewport);

    assert_eq!(overlay.entry.kind, OverlayKind::Modal);
    assert!(overlay.entry.modal);
    assert_eq!(
        overlay.entry.dismissal,
        OverlayDismissal::OutsideClickOrEscape
    );
    assert_eq!(overlay.context, ActionContext::Editor);
    assert_eq!(overlay.dialog.title, ABOUT_MODAL_DIALOG_TITLE);
    let expected_body = format!("{ABOUT_MODAL_VERSION}\n{ABOUT_MODAL_READINESS}");
    assert_eq!(
        overlay.dialog.body.as_ref().map(|body| body.text.as_str()),
        Some(expected_body.as_str())
    );
    assert_eq!(overlay.visible_actions().len(), 2);
    let modal_documentation = overlay
        .visible_action_by_role(ModalActionRole::Primary)
        .expect("Documentation action");
    let close = overlay
        .visible_action_by_role(ModalActionRole::Cancel)
        .expect("Close action");
    assert_eq!(modal_documentation.action.id.as_str(), ACTION_DOCS);
    assert_eq!(modal_documentation.action.label, "Documentation");
    assert!(modal_documentation.can_invoke());
    assert_eq!(modal_documentation.action.shortcut, None);
    assert_eq!(close.action.id.as_str(), ACTION_ABOUT_CLOSE);
    assert_eq!(close.action.label, "Close");
    assert!(close.can_invoke());
    assert_eq!(close.action.shortcut, None);
    assert_eq!(
        overlay
            .invocation_for_role(ModalActionRole::Primary)
            .expect("Documentation invokes")
            .action_id,
        ActionId::new(ACTION_DOCS)
    );
    assert_eq!(
        overlay
            .invocation_for_role(ModalActionRole::Cancel)
            .expect("Close invokes")
            .action_id,
        ActionId::new(ACTION_ABOUT_CLOSE)
    );
    assert_ne!(
        overlay.visible_actions()[0].action.id,
        overlay.visible_actions()[1].action.id
    );
    assert!(editor.apply_action(ACTION_ABOUT_CLOSE));
    assert!(!editor.about_modal_open);
    let closed_status = editor.status.clone();
    assert!(editor.apply_action(ACTION_ABOUT_CLOSE));
    assert_eq!(editor.status, closed_status);
}
