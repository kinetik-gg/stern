impl EditorShowcase {
    pub(super) fn console_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let diagnostics = Self::showcase_diagnostics();
        let jobs = Self::showcase_job_list();
        let feedback = Self::showcase_feedback_stack();
        let summary = diagnostics.summary();
        let active_feedback = feedback.active_items(showcase_feedback_now());
        let diagnostics_header = Rect::new(body.x + 8.0, body.y + 8.0, body.width - 16.0, 24.0);
        text(
            ui,
            diagnostics_header.x,
            diagnostics_header.y + 16.0,
            &format!(
                "Diagnostics: {} error, {} warning, {} info",
                summary.errors, summary.warnings, summary.info
            ),
            12.0,
            rgb(222, 225, 230),
        );

        let diagnostics_layout = ListLayout::new(22.0);
        let diagnostic_rows = diagnostics.ordered_items();
        let diagnostics_bounds = Rect::new(
            body.x + 8.0,
            body.y + 36.0,
            body.width - 16.0,
            (diagnostic_rows.len() as f32 * 22.0).min(72.0),
        );
        for item in diagnostics_layout.row_rects(
            diagnostics_bounds,
            diagnostic_rows.len(),
            0..diagnostic_rows.len(),
        ) {
            let diagnostic = diagnostic_rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 15.0,
                severity_label(diagnostic.severity),
                10.0,
                severity_color(diagnostic.severity),
            );
            text(
                ui,
                item.rect.x + 76.0,
                item.rect.y + 15.0,
                &diagnostic.code,
                10.0,
                rgb(178, 183, 190),
            );
            text(
                ui,
                item.rect.x + 216.0,
                item.rect.y + 15.0,
                &diagnostic.message,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let jobs_y = diagnostics_bounds.max_y() + 12.0;
        let job_summary = jobs.summary();
        text(
            ui,
            body.x + 8.0,
            jobs_y + 16.0,
            &format!(
                "Jobs: {} active, {} complete, {} failed",
                job_summary.active(),
                job_summary.succeeded,
                job_summary.failed
            ),
            12.0,
            rgb(222, 225, 230),
        );
        let job_layout = ListLayout::new(24.0);
        let job_bounds = Rect::new(
            body.x + 8.0,
            jobs_y + 28.0,
            body.width - 16.0,
            (jobs.rows().len() as f32 * 24.0).min(96.0),
        );
        for item in job_layout.row_rects(job_bounds, jobs.rows().len(), 0..jobs.rows().len()) {
            let job = &jobs.rows()[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 16.0,
                job_phase_label(job.phase),
                10.0,
                job_phase_color(job.phase),
            );
            text(
                ui,
                item.rect.x + 86.0,
                item.rect.y + 16.0,
                &job.label,
                11.0,
                rgb(218, 221, 226),
            );
            if let Some(progress) = job.progress.status_progress() {
                let bar = Rect::new(item.rect.max_x() - 136.0, item.rect.y + 9.0, 72.0, 6.0);
                rect(ui, bar, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
                rect(
                    ui,
                    Rect::new(bar.x, bar.y, bar.width * progress.value, bar.height),
                    rgb(69, 123, 220),
                    None,
                );
                text(
                    ui,
                    bar.max_x() + 8.0,
                    item.rect.y + 16.0,
                    &format!("{:.0}%", progress.value * 100.0),
                    10.0,
                    rgb(154, 160, 168),
                );
            }
        }

        let feedback_y = job_bounds.max_y() + 12.0;
        text(
            ui,
            body.x + 8.0,
            feedback_y + 16.0,
            &format!("Feedback: {} active toast(s)", active_feedback.len()),
            12.0,
            rgb(222, 225, 230),
        );
        for (index, item) in active_feedback.iter().enumerate() {
            let row = Rect::new(
                body.x + 8.0,
                feedback_y + 28.0 + index as f32 * 22.0,
                body.width - 16.0,
                20.0,
            );
            rect(ui, row, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                row.x + 8.0,
                row.y + 14.0,
                feedback_kind_label(item.kind),
                10.0,
                feedback_kind_color(item.kind),
            );
            text(
                ui,
                row.x + 78.0,
                row.y + 14.0,
                &item.text,
                10.0,
                rgb(218, 221, 226),
            );
        }

        let log_y = feedback_y + 84.0;
        let table = TableLayout {
            columns: vec![
                TableColumn {
                    id: item_id(1),
                    header: "Time".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(2),
                    header: "Level".to_owned(),
                    width: 74.0,
                },
                TableColumn {
                    id: item_id(3),
                    header: "Message".to_owned(),
                    width: (body.width - 160.0).max(120.0),
                },
            ],
            header_height: 24.0,
            row_height: 24.0,
            sort: None,
        };
        let bounds = Rect::new(
            body.x + 8.0,
            log_y,
            body.width - 16.0,
            (body.max_y() - log_y - 8.0).max(0.0),
        );
        for header in table.header_rects(bounds) {
            rect(ui, header.rect, rgb(31, 33, 36), Some(rgb(48, 50, 56)));
            text(
                ui,
                header.rect.x + 8.0,
                header.rect.y + 16.0,
                &table.columns[header.index].header,
                11.0,
                rgb(178, 183, 190),
            );
        }
        for cell in table.visible_body_cells(bounds, LOGS.len(), 0.0, 1) {
            let log = &LOGS[cell.row];
            let value = match cell.column {
                0 => log.time,
                1 => log.level,
                _ => log.message,
            };
            rect(ui, cell.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                cell.rect.x + 8.0,
                cell.rect.y + 16.0,
                value,
                11.0,
                log_color(log.level),
            );
        }
    }

    pub(super) fn timeline_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        let rows = [
            ("Intro camera pan", "Frame 001-072", 0.24),
            ("Character pickup", "Frame 073-144", 0.48),
            ("Vehicle reveal", "Frame 145-216", 0.72),
            ("Cut to gameplay", "Frame 217-300", 0.88),
        ];
        let layout = ListLayout::new(28.0);
        for item in layout.row_rects(body.inset(8.0), rows.len(), 0..rows.len()) {
            let (name, range, progress) = rows[item.index];
            rect(ui, item.rect, rgb(22, 23, 25), Some(rgb(38, 40, 45)));
            text(
                ui,
                item.rect.x + 8.0,
                item.rect.y + 18.0,
                name,
                11.0,
                rgb(222, 225, 230),
            );
            text(
                ui,
                item.rect.max_x() - 96.0,
                item.rect.y + 18.0,
                range,
                11.0,
                rgb(154, 160, 168),
            );
            let progress_rect = Rect::new(item.rect.max_x() - 220.0, item.rect.y + 10.0, 96.0, 6.0);
            rect(ui, progress_rect, rgb(39, 42, 47), Some(rgb(56, 59, 65)));
            rect(
                ui,
                Rect::new(
                    progress_rect.x,
                    progress_rect.y,
                    progress_rect.width * progress,
                    progress_rect.height,
                ),
                rgb(69, 123, 220),
                None,
            );
        }
    }

    pub(super) fn node_graph_panel(ui: &mut Ui<'_>, body: Rect) {
        rect(ui, body, rgb(20, 21, 23), None);
        text(
            ui,
            body.x + 10.0,
            body.y + 20.0,
            "Compositor Graph",
            12.0,
            rgb(218, 222, 228),
        );

        match Self::showcase_node_graph_output(
            ui.id("editor.node-graph.static-view"),
            Self::showcase_node_graph_viewport(body),
        ) {
            Ok(output) => {
                ui.extend(output.primitives);
                for semantic in output.semantics {
                    ui.push_semantic_node(semantic);
                }
            }
            Err(_) => {
                text(
                    ui,
                    body.x + 10.0,
                    body.y + 42.0,
                    "Node graph descriptor unavailable",
                    11.0,
                    rgb(236, 96, 96),
                );
            }
        }
    }

    pub(super) fn showcase_node_graph_output(
        id: WidgetId,
        viewport: NodeGraphViewport,
    ) -> Result<NodeGraphStaticOutput, NodeGraphEmissionError> {
        let graph = Self::showcase_node_graph_descriptor();
        let selection = NodeGraphSelection::from_targets([
            NodeGraphSelectionTarget::Node(NodeId::from_raw(2)),
            NodeGraphSelectionTarget::Edge(EdgeId::from_raw(51)),
            NodeGraphSelectionTarget::Reroute(RerouteId::from_raw(1)),
        ]);

        NodeGraphStaticView::new(id, viewport, &graph)
            .with_selection(selection)
            .with_incompatible_ports([PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(2))])
            .emit()
    }

    pub(super) fn showcase_node_graph_descriptor() -> NodeGraphDescriptor {
        const COLOR: PortTypeId = PortTypeId::from_raw(1);
        const MASK: PortTypeId = PortTypeId::from_raw(2);
        let frame = NodeFrameId::from_raw(1);
        let group = NodeGroupId::from_raw(1);

        NodeGraphDescriptor {
            nodes: vec![
                NodeDescriptor::new(
                    NodeId::from_raw(1),
                    "Texture",
                    GraphRect::new(8.0, 28.0, 92.0, 64.0),
                )
                .with_ports(vec![PortDescriptor::new(
                    PortId::from_raw(1),
                    PortDirection::Output,
                    "Color",
                    COLOR,
                )])
                .with_frame(frame),
                NodeDescriptor::new(
                    NodeId::from_raw(2),
                    "Color Grade",
                    GraphRect::new(142.0, 54.0, 118.0, 76.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(PortId::from_raw(1), PortDirection::Input, "In", COLOR),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Output, "Out", COLOR),
                    PortDescriptor::new(PortId::from_raw(3), PortDirection::Input, "Mask", MASK)
                        .with_enabled(false),
                ])
                .with_frame(frame)
                .with_group(group)
                .with_label("Selected preview"),
                NodeDescriptor::new(
                    NodeId::from_raw(3),
                    "Output",
                    GraphRect::new(314.0, 36.0, 96.0, 68.0),
                )
                .with_ports(vec![
                    PortDescriptor::new(
                        PortId::from_raw(1),
                        PortDirection::Input,
                        "Surface",
                        COLOR,
                    ),
                    PortDescriptor::new(PortId::from_raw(2), PortDirection::Input, "Mask", MASK),
                ])
                .with_bypassed(true),
            ],
            edges: vec![
                EdgeDescriptor::new(
                    EdgeId::from_raw(50),
                    PortEndpoint::new(NodeId::from_raw(1), PortId::from_raw(1)),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(1)),
                ),
                EdgeDescriptor::new(
                    EdgeId::from_raw(51),
                    PortEndpoint::new(NodeId::from_raw(2), PortId::from_raw(2)),
                    PortEndpoint::new(NodeId::from_raw(3), PortId::from_raw(1)),
                )
                .with_route_points(vec![NodeGraphEdgeRoutePoint::reroute(
                    RerouteId::from_raw(1),
                )]),
            ],
            reroutes: vec![RerouteDescriptor::new(
                RerouteId::from_raw(1),
                "Route A",
                GraphPoint::new(284.0, 88.0),
            )],
            frames: vec![NodeFrameDescriptor::new(
                frame,
                "Preview Frame",
                GraphRect::new(-4.0, 14.0, 282.0, 132.0),
            )],
            groups: vec![
                NodeGroupDescriptor::new(
                    group,
                    "Look Dev",
                    GraphRect::new(132.0, 44.0, 140.0, 96.0),
                )
                .with_nodes(vec![NodeId::from_raw(2)]),
            ],
        }
    }

    pub(super) fn showcase_node_graph_viewport(body: Rect) -> NodeGraphViewport {
        NodeGraphViewport::new(
            Rect::new(
                body.x + 8.0,
                body.y + 30.0,
                (body.width - 16.0).max(0.0),
                (body.height - 38.0).max(0.0),
            ),
            NodeGraphPanZoom::new(GraphVector::new(12.0, 8.0), 1.0),
        )
    }

    pub(super) fn status_bar(&self, ui: &mut Ui<'_>, viewport: Rect, action_count: u32) {
        let status_bar = self.status_bar_model(action_count);
        let visible_items = status_bar.visible_items();
        let bar = Rect::new(0.0, viewport.max_y() - 24.0, viewport.width, 24.0);
        rect(ui, bar, rgb(27, 29, 32), Some(rgb(52, 55, 62)));
        let message = visible_items
            .iter()
            .find(|item| item.id == EditorStatusItemKind::Message.id())
            .expect("editor status bar exposes message item");
        text(
            ui,
            10.0,
            bar.y + 16.0,
            &message.text,
            11.0,
            rgb(198, 203, 211),
        );
        let mut x = viewport.max_x() - 92.0;
        for item in visible_items
            .iter()
            .filter(|item| item.id != EditorStatusItemKind::Message.id())
            .rev()
        {
            let width = status_item_text_width(&item.text);
            let color = match item.kind {
                StatusItemKind::Error => rgb(236, 96, 96),
                StatusItemKind::Stale => rgb(232, 179, 90),
                StatusItemKind::Ready
                | StatusItemKind::Pending
                | StatusItemKind::Message
                | StatusItemKind::ActionCount
                | StatusItemKind::JobCount
                | StatusItemKind::Progress => rgb(154, 160, 168),
            };
            text(ui, x, bar.y + 16.0, &item.text, 11.0, color);
            x -= width;
        }
    }
}
