impl EditorShowcase {
    pub(super) fn inspector(&mut self, ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(24, 25, 27), None);
        let header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 34.0);
        rect(ui, header, rgb(37, 39, 43), Some(rgb(55, 58, 64)));
        draw_icon(
            ui,
            Rect::new(header.x + 7.0, header.y + 7.0, 20.0, 20.0),
            scene_icon(self.selected_node),
            DENSE_ICON_SIZE,
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 12.0,
            "Inspector",
            9.0,
            rgb(151, 158, 166),
        );
        text(
            ui,
            header.x + 34.0,
            header.y + 27.0,
            scene_label(self.selected_node),
            12.0,
            rgb(231, 233, 237),
        );
        draw_icon(
            ui,
            Rect::new(header.max_x() - 27.0, header.y + 7.0, 20.0, 20.0),
            ToolbarIcon::Gear,
            DENSE_ICON_SIZE,
        );

        let rows = inspector_rows(&self.mass.text);
        let grid = Rect::new(
            body.x + 8.0,
            body.y + 52.0,
            body.width - 16.0,
            body.height - 60.0,
        );
        let layout =
            PropertyGridLayout::new(24.0, 26.0, inspector_label_width(grid.width), 6.0, 12.0);
        ui.scroll_area(
            "editor.inspector.scroll",
            grid,
            Size::new(grid.width, layout.content_height(&rows).max(grid.height)),
            false,
            |ui, offset| {
                for row in layout.visible_row_rects_content(grid, &rows, offset.y, 2) {
                    match row.kind {
                        kinetik_ui::widgets::PropertyGridRowKind::Section => {
                            rect(ui, row.rect, rgb(31, 33, 36), Some(rgb(46, 49, 55)));
                            text(
                                ui,
                                row.label_rect.x + 8.0,
                                row.label_rect.y + 17.0,
                                &rows[row.index].label,
                                12.0,
                                rgb(205, 209, 216),
                            );
                        }
                        kinetik_ui::widgets::PropertyGridRowKind::Property { .. } => {
                            let model_row = &rows[row.index];
                            let status = model_row.state.status.presentation();
                            let label_color = match status.severity {
                                kinetik_ui::widgets::PropertyGridStatusSeverity::None => {
                                    rgb(154, 160, 168)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                    rgb(126, 179, 236)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                    rgb(232, 179, 90)
                                }
                                kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                    rgb(236, 96, 96)
                                }
                            };
                            rect(ui, row.rect, rgb(24, 25, 27), Some(rgb(38, 40, 45)));
                            if status.accented {
                                rect(
                                    ui,
                                    Rect::new(row.rect.x, row.rect.y, 3.0, row.rect.height),
                                    label_color,
                                    None,
                                );
                                text(
                                    ui,
                                    row.label_rect.max_x() - 10.0,
                                    row.label_rect.y + 16.0,
                                    match status.severity {
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Info => {
                                            "i"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Warning => {
                                            "!"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::Error => {
                                            "x"
                                        }
                                        kinetik_ui::widgets::PropertyGridStatusSeverity::None => "",
                                    },
                                    9.0,
                                    label_color,
                                );
                            }
                            text(
                                ui,
                                row.label_rect.x + 6.0,
                                row.label_rect.y + 16.0,
                                &model_row.label,
                                11.0,
                                label_color,
                            );
                            let affordance_rects = property_grid_row_affordance_rects(
                                model_row,
                                row.value_rect.inset(2.0),
                                PropertyGridAffordanceLayout::default(),
                            );
                            self.inspector_value(ui, model_row, affordance_rects.value_rect);
                            let affordance = ui.property_grid_row_affordance_controls(
                                ("editor.inspector.affordance", row.id.raw()),
                                model_row,
                                affordance_rects,
                            );
                            if affordance.reset_requested {
                                self.status = format!("Reset requested for {}", model_row.label);
                            } else if affordance.keyframe_toggle_requested {
                                let state = if affordance.requested_keyed {
                                    "add"
                                } else {
                                    "remove"
                                };
                                self.status =
                                    format!("Keyframe {state} requested for {}", model_row.label);
                            }
                        }
                    }
                }
            },
        );
    }

    pub(super) fn inspector_value(
        &mut self,
        ui: &mut Ui<'_>,
        row: &PropertyGridRow,
        rect_value: Rect,
    ) {
        let id = row.id;
        let disabled = row.state.disabled;
        let read_only = row.state.read_only;
        match id.raw() {
            2 => {
                ui.vector3_scrub_input(
                    "editor.inspector.position",
                    rect_value,
                    "Position",
                    &mut self.position,
                    &mut self.position_states,
                    VectorScrubInputConfig::new(
                        NumericScrubInputConfig::new(0.1).with_fine_step(0.01),
                    )
                    .disabled(disabled)
                    .read_only(read_only),
                );
            }
            5 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.scale",
                    rect_value,
                    &mut self.scale,
                    NumericScrubInputConfig::new(0.01)
                        .with_fine_step(0.001)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            7 => {
                ui.slider(
                    "editor.inspector.exposure",
                    rect_value,
                    &mut self.exposure,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            8 => {
                ui.slider(
                    "editor.inspector.roughness",
                    rect_value,
                    &mut self.roughness,
                    0.0..=1.0,
                    disabled || read_only,
                );
            }
            9 => {
                let asset = self.material_asset();
                let slot = ui.asset_slot_field(
                    "editor.inspector.material",
                    rect_value,
                    "Material",
                    Some(&asset),
                    AssetSlotConfig::new("Drop material")
                        .accepts_drop(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if slot.drop_received {
                    "Material drop requested".clone_into(&mut self.status);
                } else if slot.open_requested {
                    self.status = format!("Open material asset: {}", asset.label);
                } else if slot.pick_requested {
                    "Material asset picker requested".clone_into(&mut self.status);
                }
            }
            11 => {
                ui.toggle_value(
                    "editor.inspector.snap",
                    Rect::new(rect_value.x, rect_value.y + 2.0, 42.0, 18.0),
                    &mut self.snap_enabled,
                    disabled || read_only,
                );
            }
            13 => {
                inspector_numeric_scrub(
                    ui,
                    "editor.inspector.mass",
                    rect_value,
                    &mut self.mass,
                    NumericScrubInputConfig::new(0.5)
                        .with_fine_step(0.1)
                        .with_min(0.0)
                        .disabled(disabled)
                        .read_only(read_only),
                );
            }
            14 => {
                let model = self.collider_model();
                let select = ui.select_field(
                    "editor.inspector.collider",
                    rect_value,
                    "Collider",
                    &model,
                    SelectFieldConfig::new("Choose collider")
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if select.open_requested {
                    "Collider choices requested".clone_into(&mut self.status);
                }
            }
            15 => {
                let path = ui.path_field(
                    "editor.inspector.script",
                    rect_value,
                    "Script path",
                    &mut self.script_path,
                    PathFieldConfig::default()
                        .open(true)
                        .disabled(disabled)
                        .read_only(read_only),
                );
                if path.browse_requested {
                    "Script path browse requested".clone_into(&mut self.status);
                } else if path.open_requested {
                    self.status = format!("Open script path: {}", self.script_path.text);
                }
            }
            _ => {
                text(
                    ui,
                    rect_value.x + 4.0,
                    rect_value.y + 15.0,
                    inspector_value_label(id),
                    11.0,
                    rgb(218, 221, 226),
                );
            }
        }
    }

    pub(super) fn material_asset(&self) -> AssetSlotAsset {
        let asset = &ASSETS[self.selected_asset.min(ASSETS.len().saturating_sub(1))];
        AssetSlotAsset::new(format!("asset://{}", asset.name), asset.name).with_kind(asset.kind)
    }

    pub(super) fn collider_model(&self) -> DropdownModel {
        let mut model = DropdownModel::from_items([
            DropdownItem::new(DropdownItemId::from_raw(1), "Box"),
            DropdownItem::new(DropdownItemId::from_raw(2), "Capsule"),
            DropdownItem::new(DropdownItemId::from_raw(3), "Sphere"),
            DropdownItem::new(DropdownItemId::from_raw(4), "Mesh").with_enabled(false),
        ]);
        let _ = model.set_selected_id(self.collider_kind);
        model
    }

    pub(super) fn showcase_job_list() -> JobList {
        JobList::from_rows([
            JobRow::new(job_row_id(1), "Active showcase job", JobPhase::Running)
                .with_progress(JobProgress::determinate(0.60))
                .with_detail("Deterministic fixture progress 3/5")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_ACTIVE_FIXTURE_JOB, "Cancel active job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(2), "Queued showcase job", JobPhase::Queued)
                .with_progress(JobProgress::determinate(0.20))
                .with_detail("Waiting in fixture queue")
                .with_cancel(JobCancel::new(
                    ActionDescriptor::new(ACTION_CANCEL_QUEUED_FIXTURE_JOB, "Cancel queued job"),
                    ActionContext::Editor,
                )),
            JobRow::new(job_row_id(3), "Completed showcase job", JobPhase::Succeeded)
                .with_progress(JobProgress::determinate(1.0))
                .with_detail("Finished fixture row"),
            JobRow::new(job_row_id(4), "Failed showcase job", JobPhase::Failed)
                .with_progress(JobProgress::determinate(0.80))
                .with_detail("Fixture failure for diagnostics presentation"),
        ])
    }

    pub(super) fn showcase_diagnostics() -> DiagnosticStrip {
        DiagnosticStrip::from_items([
            DiagnosticStripItem::new(
                diagnostic_item_id(1),
                DiagnosticStripSeverity::Warning,
                "showcase.fixture.warning",
                "Fixture warning keeps diagnostics visible",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("panel", "Console"),
            DiagnosticStripItem::new(
                diagnostic_item_id(2),
                DiagnosticStripSeverity::Info,
                "showcase.fixture.info",
                "Fixture metadata is application-owned",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("state", "deterministic"),
            DiagnosticStripItem::new(
                diagnostic_item_id(3),
                DiagnosticStripSeverity::Error,
                "showcase.fixture.error",
                "Fixture error demonstrates summary counts",
            )
            .with_source(DiagnosticSource::Application)
            .with_field("recoverable", "true"),
        ])
    }

    pub(super) fn showcase_feedback_stack() -> FeedbackStack {
        FeedbackStack::from_items([
            FeedbackItem::timed(
                feedback_id(1),
                FeedbackKind::Success,
                "Saved",
                "Fixture save completed",
                Duration::from_secs(2),
                Duration::from_secs(8),
            )
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss feedback"),
                ActionContext::Editor,
            )),
            FeedbackItem::pinned(
                feedback_id(2),
                FeedbackKind::Warning,
                "Report",
                "Fixture report needs review",
            )
            .with_action(FeedbackAction::new(
                ActionDescriptor::new(ACTION_OPEN_FEEDBACK_REPORT, "Open report"),
                ActionContext::Editor,
            ))
            .with_dismiss(FeedbackDismiss::new(
                ActionDescriptor::new(ACTION_DISMISS_FEEDBACK_REPORT, "Dismiss report"),
                ActionContext::Editor,
            )),
            FeedbackItem::timed(
                feedback_id(3),
                FeedbackKind::Info,
                "Expired",
                "Expired fixture toast",
                Duration::from_secs(0),
                Duration::from_secs(2),
            ),
        ])
    }
}
