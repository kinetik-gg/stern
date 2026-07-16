use super::common::assert_approx;
use crate::{
    RenderFrameInput, RenderResources, TextLayoutResource, VelloRenderer,
    project_text_point_to_device, root_transform, snap_axis_aligned_translation,
};
use std::time::Duration;
use stern_core::{
    ActionContext, ActionDescriptor, Brush, Color, FrameContext, FrameOutput, PhysicalSize, Point,
    PointerOrder, Primitive, Rect, ScaleFactor, Size, TextLayoutId, TextPrimitive, TimeInfo,
    Transform, UiInput, UiMemory, Vec2, ViewportInfo, default_dark_theme,
};
use stern_render::TextLayoutResourceSync;
use stern_text::{
    CosmicTextEngine, TextEditState, TextFeatureSet, TextLayoutKey, TextLayoutStore, TextOverflow,
    TextStyle,
};
use stern_widgets::{
    CollectionProjection, DropdownItem, DropdownItemId, DropdownModel, ItemId, SelectFieldConfig,
    TableColumn, TableLayout, Ui, VirtualTableConfig, VirtualTableRow, VirtualTableSelection,
    VirtualTableSelectionMode,
    inspector::{PropertyGridConfig, PropertyGridRow, PropertyGridRowStatus},
};

fn resource(id: TextLayoutId, key: TextLayoutKey) -> TextLayoutResource {
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&key);
    TextLayoutResource {
        id,
        key,
        layout: std::sync::Arc::new(layout),
    }
}

fn viewport(scale: f64) -> ViewportInfo {
    let physical = match scale {
        1.0 => 100,
        1.25 => 125,
        1.5 => 150,
        1.75 => 175,
        2.0 => 200,
        _ => panic!("unsupported fixture scale {scale}"),
    };
    ViewportInfo::new(
        Size::new(100.0, 100.0),
        PhysicalSize::new(physical, physical),
        ScaleFactor::new(scale),
    )
}

fn primitive(layout: Option<TextLayoutId>, text: &str) -> Primitive {
    Primitive::Text(TextPrimitive {
        layout,
        origin: Point::new(4.3, 16.4),
        text: text.to_owned(),
        family: "serif".to_owned(),
        size: 7.0,
        line_height: 9.0,
        brush: Brush::Solid(Color::WHITE),
    })
}

fn retained_virtual_table_frame(
    store: &mut TextLayoutStore,
    memory: &mut UiMemory,
    header: &str,
    body: &str,
) -> FrameOutput {
    let row_id = ItemId::from_raw(1);
    let column_id = ItemId::from_raw(10);
    let projection = CollectionProjection::from_source_ids(&[row_id]);
    let config = VirtualTableConfig::new(
        Rect::new(3.25, 5.5, 96.0, 64.0),
        TableLayout {
            columns: vec![TableColumn::new(column_id, header, 96.0)],
            header_height: 20.0,
            row_height: 20.0,
            sort: None,
        },
    )
    .label("Vello retained cell fixture")
    .overscan(0)
    .selection_mode(VirtualTableSelectionMode::Cell)
    .resizable(false);
    let context = FrameContext::new(
        viewport(1.0),
        UiInput::default(),
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    );
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context, memory, &theme).with_text_layouts(store);
    let table = ui
        .prepare_virtual_table("vello-retained-cell-table", config, &projection)
        .expect("valid Vello retained cell fixture");
    ui.resolve_pointer_targets(|plan| {
        table.declare_pointer_targets(plan, PointerOrder::new(100));
    })
    .expect("valid Vello retained cell pointer plan");
    let mut selection = VirtualTableSelection::new();
    let _ = ui.virtual_table(&table, &mut selection, |_| VirtualTableRow::new([body]));
    ui.finish_output()
}

#[test]
fn retained_numeric_widget_encodes_registered_tabular_glyphs_without_fallback() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut state = TextEditState::new("20486357");
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.numeric_input("number", Rect::new(0.0, 0.0, 96.0, 24.0), &mut state, false);
    let frame = ui.finish_output();
    let id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) => text.layout,
            _ => None,
        })
        .expect("retained numeric layout ID");
    let entry = store
        .layouts()
        .find(|entry| entry.id == id)
        .expect("feature-bearing store entry");
    assert_eq!(entry.key.style.features, TextFeatureSet::TABULAR_NUMBERS);
    let expected_ids = entry
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let logical_font_size = entry.key.style.size();

    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, 1);
    assert_eq!(report.retained, 1);
    assert_eq!(
        resources
            .text_layout_resource(id)
            .expect("reconciled numeric resource")
            .key
            .style
            .features,
        TextFeatureSet::TABULAR_NUMBERS
    );

    let mut renderer = VelloRenderer::new();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(f64::from(scale)),
            primitives: &frame.primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();

        assert!(output.diagnostics.is_empty());
        assert_eq!(
            encoding
                .resources
                .glyphs
                .iter()
                .map(|glyph| glyph.id)
                .collect::<Vec<_>>(),
            expected_ids
        );
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| { (run.font_size - logical_font_size * scale).abs() <= 0.000_1 })
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_standard_and_action_buttons_encode_exact_ellipsis_resources_at_all_scales() {
    let standard_source =
        "Canonical retained standard button source stays complete while Vello encodes ellipsis";
    let action_source =
        "Canonical delegated action button source stays complete while Vello encodes ellipsis";
    let action = ActionDescriptor::new("button.render", action_source);
    let standard_rect = Rect::new(0.0, 0.0, 96.0, 24.0);
    let action_rect = Rect::new(0.0, 32.0, 96.0, 24.0);
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.button("standard", standard_rect, standard_source, false);
    let _ = ui.action_button("action", action_rect, &action, ActionContext::Global);
    let frame = ui.finish_output();

    let standard_text = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == standard_source => Some(text),
            _ => None,
        })
        .expect("registered retained standard button label");
    let action_text = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == action_source => Some(text),
            _ => None,
        })
        .expect("registered retained action button label");
    let standard_id = standard_text
        .layout
        .expect("standard button retained identity");
    let action_id = action_text.layout.expect("action button retained identity");
    assert_ne!(standard_id, action_id);

    let standard = store
        .stored_layout(standard_id)
        .expect("retained standard button entry");
    let action = store
        .stored_layout(action_id)
        .expect("retained action button entry");
    let expected_width_bits = (standard_rect.width - theme.controls.padding_x * 2.0_f32).to_bits();
    assert_eq!(standard.key.text, standard_source);
    assert_eq!(action.key.text, action_source);
    assert_eq!(standard.key.width_bits, expected_width_bits);
    assert_eq!(action.key.width_bits, expected_width_bits);
    assert_eq!(standard.key.overflow, TextOverflow::EndEllipsis);
    assert_eq!(action.key.overflow, TextOverflow::EndEllipsis);
    assert!(standard.layout.is_elided());
    assert!(action.layout.is_elided());

    let standard_ids = standard
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let action_ids = action
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let standard_logical_points = standard
        .layout
        .runs
        .iter()
        .flat_map(|run| {
            run.glyphs.iter().map(|glyph| {
                Point::new(
                    standard_text.origin.x + glyph.x,
                    standard_text.origin.y + glyph.y,
                )
            })
        })
        .collect::<Vec<_>>();
    let action_logical_points = action
        .layout
        .runs
        .iter()
        .flat_map(|run| {
            run.glyphs.iter().map(|glyph| {
                Point::new(
                    action_text.origin.x + glyph.x,
                    action_text.origin.y + glyph.y,
                )
            })
        })
        .collect::<Vec<_>>();
    let standard_markers = standard
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .enumerate()
        .filter_map(|(index, glyph)| glyph.elided.then_some(index))
        .collect::<Vec<_>>();
    let action_markers = action
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .enumerate()
        .filter_map(|(index, glyph)| glyph.elided.then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(standard_markers.len(), 1);
    assert_eq!(action_markers.len(), 1);
    let logical_font_size = standard.key.style.size();
    assert_eq!(action.key.style.size_bits, standard.key.style.size_bits);

    let mut expected_ids = standard_ids.clone();
    expected_ids.extend_from_slice(&action_ids);
    let mut logical_points = standard_logical_points;
    logical_points.extend_from_slice(&action_logical_points);
    assert_eq!(logical_points.len(), expected_ids.len());
    let action_marker_index = standard_ids.len() + action_markers[0];
    let store_accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, 2);
    assert_eq!(report.retained, 2);
    assert_eq!(
        resources
            .text_layout_resource(standard_id)
            .expect("reconciled standard button resource")
            .key
            .text,
        standard_source
    );
    assert_eq!(
        resources
            .text_layout_resource(action_id)
            .expect("reconciled action button resource")
            .key
            .text,
        action_source
    );

    let mut renderer = VelloRenderer::new();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let device_scale = f64::from(scale);
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(device_scale),
            primitives: &frame.primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_ids = encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(encoded_ids, expected_ids);
        let effective = snap_axis_aligned_translation(root_transform(device_scale));
        assert_eq!(
            effective.as_coeffs().map(f64::to_bits),
            [device_scale, 0.0, 0.0, device_scale, 0.0, 0.0].map(f64::to_bits)
        );
        for (encoded, logical) in encoding.resources.glyphs.iter().zip(&logical_points) {
            let expected = project_text_point_to_device(effective, *logical);
            assert_eq!(Point::new(encoded.x, encoded.y), expected);
        }
        assert_eq!(
            encoded_ids[standard_markers[0]],
            standard_ids[standard_markers[0]]
        );
        assert_eq!(
            encoded_ids[action_marker_index],
            action_ids[action_markers[0]]
        );
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| { run.font_size.to_bits() == (logical_font_size * scale).to_bits() })
        );
        assert!(encoding.resources.glyphs.iter().all(|glyph| {
            (glyph.x - glyph.x.round()).abs() <= 0.001 && (glyph.y - glyph.y.round()).abs() <= 0.001
        }));
        assert_eq!(
            resources
                .text_layout_resource(standard_id)
                .expect("stable standard button resource")
                .key
                .width_bits,
            expected_width_bits
        );
        assert_eq!(
            resources
                .text_layout_resource(action_id)
                .expect("stable action button resource")
                .key
                .width_bits,
            expected_width_bits
        );
        assert_eq!(standard_text.layout, Some(standard_id));
        assert_eq!(action_text.layout, Some(action_id));
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            store_accounting
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_virtual_table_cell_encodes_exact_ellipsis_and_separate_header_at_all_scales() {
    let header_source = "Complete retained virtual-table header remains generic Visible";
    let body_source = "Complete retained virtual-table body-cell source stays intact while Vello encodes ellipsis";
    let mut store = TextLayoutStore::new();
    let mut memory = UiMemory::new();
    let frame = retained_virtual_table_frame(&mut store, &mut memory, header_source, body_source);
    let header_text = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == header_source => Some(text),
            _ => None,
        })
        .expect("registered retained virtual-table header");
    let body_text = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == body_source => Some(text),
            _ => None,
        })
        .expect("registered retained virtual-table body cell");
    let header_id = header_text.layout.expect("retained header identity");
    let body_id = body_text.layout.expect("retained body-cell identity");
    assert_ne!(header_id, body_id);
    let store_accounting = (
        store.len(),
        store.retained_payload_bytes(),
        store.change_cursor(),
    );
    let hot = retained_virtual_table_frame(&mut store, &mut memory, header_source, body_source);
    assert_eq!(
        hot.primitives.iter().find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == header_source => text.layout,
            _ => None,
        }),
        Some(header_id)
    );
    assert_eq!(
        hot.primitives.iter().find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == body_source => text.layout,
            _ => None,
        }),
        Some(body_id)
    );
    assert_eq!(
        (
            store.len(),
            store.retained_payload_bytes(),
            store.change_cursor()
        ),
        store_accounting
    );

    let header = store
        .stored_layout(header_id)
        .expect("retained virtual-table header entry");
    let body = store
        .stored_layout(body_id)
        .expect("retained virtual-table body-cell entry");
    assert_eq!(header.key.text, header_source);
    assert_eq!(header.key.overflow, TextOverflow::Visible);
    assert_eq!(header.key.width_bits, 0.0_f32.to_bits());
    assert_eq!(body.key.text, body_source);
    assert_eq!(body.key.overflow, TextOverflow::EndEllipsis);
    assert_eq!(body.key.width_bits, 80.0_f32.to_bits());
    assert!(body.layout.is_elided());
    let body_markers = body
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .enumerate()
        .filter_map(|(index, glyph)| glyph.elided.then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(body_markers.len(), 1);
    assert!(
        header
            .layout
            .runs
            .iter()
            .flat_map(|run| &run.glyphs)
            .all(|glyph| !glyph.elided)
    );

    let header_ids = header
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let body_ids = body
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let header_logical_points = header
        .layout
        .runs
        .iter()
        .flat_map(|run| {
            run.glyphs.iter().map(|glyph| {
                Point::new(
                    header_text.origin.x + glyph.x,
                    header_text.origin.y + glyph.y,
                )
            })
        })
        .collect::<Vec<_>>();
    let body_logical_points = body
        .layout
        .runs
        .iter()
        .flat_map(|run| {
            run.glyphs
                .iter()
                .map(|glyph| Point::new(body_text.origin.x + glyph.x, body_text.origin.y + glyph.y))
        })
        .collect::<Vec<_>>();
    let mut expected_ids = header_ids.clone();
    expected_ids.extend_from_slice(&body_ids);
    let mut logical_points = header_logical_points;
    logical_points.extend_from_slice(&body_logical_points);
    assert_eq!(expected_ids.len(), logical_points.len());
    let body_marker_index = header_ids.len() + body_markers[0];
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, 2);
    assert_eq!(report.retained, 2);
    assert_eq!(
        resources
            .text_layout_resource(header_id)
            .expect("reconciled virtual-table header resource")
            .key
            .text,
        header_source
    );
    assert_eq!(
        resources
            .text_layout_resource(body_id)
            .expect("reconciled virtual-table body-cell resource")
            .key
            .text,
        body_source
    );

    let logical_font_size = body.key.style.size();
    assert_eq!(header.key.style.size_bits, body.key.style.size_bits);
    let mut renderer = VelloRenderer::new();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let device_scale = f64::from(scale);
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(device_scale),
            primitives: &frame.primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_ids = encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(encoded_ids, expected_ids);
        assert_eq!(encoded_ids[body_marker_index], body_ids[body_markers[0]]);
        let effective = snap_axis_aligned_translation(root_transform(device_scale));
        assert_eq!(
            effective.as_coeffs().map(f64::to_bits),
            [device_scale, 0.0, 0.0, device_scale, 0.0, 0.0].map(f64::to_bits)
        );
        for (encoded, logical) in encoding.resources.glyphs.iter().zip(&logical_points) {
            assert_eq!(
                Point::new(encoded.x, encoded.y),
                project_text_point_to_device(effective, *logical)
            );
        }
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| run.font_size.to_bits() == (logical_font_size * scale).to_bits())
        );
        assert_eq!(
            resources
                .text_layout_resource(body_id)
                .expect("stable virtual-table body-cell resource")
                .key
                .width_bits,
            80.0_f32.to_bits()
        );
        assert_eq!(body_text.layout, Some(body_id));
        assert_eq!(header_text.layout, Some(header_id));
        assert_eq!(
            (
                store.len(),
                store.retained_payload_bytes(),
                store.change_cursor()
            ),
            store_accounting
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_select_widget_encodes_registered_ellipsis_without_arrow_fallback() {
    let source =
        "Canonical retained select source stays complete while Vello encodes its end ellipsis";
    let item_id = DropdownItemId::from_raw(17);
    let mut model = DropdownModel::from_items([DropdownItem::new(item_id, source)]);
    assert!(model.set_selected_id(item_id));
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let _ = ui.select_field(
        "material",
        Rect::new(0.0, 0.0, 96.0, 24.0),
        "Material",
        &model,
        SelectFieldConfig::new("Choose material"),
    );
    let frame = ui.finish_output();
    let value_id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => text.layout,
            _ => None,
        })
        .expect("registered retained select value");
    let arrow_id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "v" => text.layout,
            _ => None,
        })
        .expect("separate registered disclosure");
    assert_ne!(value_id, arrow_id);
    let value = store
        .stored_layout(value_id)
        .expect("retained select value entry");
    let arrow = store
        .stored_layout(arrow_id)
        .expect("retained select disclosure entry");
    assert_eq!(value.key.text, source);
    assert_eq!(value.key.overflow, TextOverflow::EndEllipsis);
    assert!(value.layout.is_elided());
    assert_eq!(arrow.key.text, "v");
    assert_eq!(arrow.key.overflow, TextOverflow::Visible);
    let value_ids = value
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let arrow_ids = arrow
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let marker_index = value
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .enumerate()
        .filter_map(|(index, glyph)| glyph.elided.then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(marker_index.len(), 1);
    let logical_font_size = value.key.style.size();

    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, 2);
    assert_eq!(report.retained, 2);
    assert_eq!(
        resources
            .text_layout_resource(value_id)
            .expect("reconciled select resource")
            .key
            .text,
        source
    );

    let mut renderer = VelloRenderer::new();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(f64::from(scale)),
            primitives: &frame.primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_ids = encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>();
        let split = value_ids.len();

        assert!(output.diagnostics.is_empty());
        assert_eq!(&encoded_ids[..split], value_ids.as_slice());
        assert_eq!(&encoded_ids[split..], arrow_ids.as_slice());
        assert_eq!(encoded_ids[marker_index[0]], value_ids[marker_index[0]]);
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| (run.font_size - logical_font_size * scale).abs() <= 0.000_1)
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_property_widget_encodes_label_ellipsis_without_state_glyph_fallback() {
    let source =
        "Canonical retained property label source stays complete while Vello encodes ellipsis";
    let row = PropertyGridRow::property(ItemId::from_raw(29), source, 0)
        .with_help_text("Complete help remains application-owned")
        .with_status(PropertyGridRowStatus::error("Error remains separate"));
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut store = TextLayoutStore::new();
    let mut ui = Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut store);
    let output = ui
        .property_grid(
            "properties",
            Rect::new(0.0, 0.0, 96.0, 24.0),
            &[row],
            PropertyGridConfig::default().with_overscan(0),
            |_, _| (),
        )
        .expect("valid retained property grid");
    let frame = ui.finish_output();
    let label_id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == source => text.layout,
            _ => None,
        })
        .expect("registered retained property label");
    let help_id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "?" => text.layout,
            _ => None,
        })
        .expect("separate registered property help glyph");
    let status_id = frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "x" => text.layout,
            _ => None,
        })
        .expect("separate registered property status glyph");
    assert_ne!(label_id, help_id);
    assert_ne!(label_id, status_id);
    assert_ne!(help_id, status_id);

    let label = store
        .stored_layout(label_id)
        .expect("retained property label entry");
    let help = store
        .stored_layout(help_id)
        .expect("retained property help entry");
    let status = store
        .stored_layout(status_id)
        .expect("retained property status entry");
    let geometry = output.visible_rows[0];
    assert_eq!(label.key.text, source);
    assert_eq!(label.key.overflow, TextOverflow::EndEllipsis);
    assert_eq!(
        label.key.width_bits,
        ((geometry.label_rect.width - 6.0_f32) - 22.0_f32)
            .max(0.0_f32)
            .to_bits()
    );
    assert!(label.layout.is_elided());
    assert_eq!(help.key.text, "?");
    assert_eq!(help.key.overflow, TextOverflow::Visible);
    assert_eq!(status.key.text, "x");
    assert_eq!(status.key.overflow, TextOverflow::Visible);
    let label_ids = label
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let help_ids = help
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let status_ids = status
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let marker_indexes = label
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .enumerate()
        .filter_map(|(index, glyph)| glyph.elided.then_some(index))
        .collect::<Vec<_>>();
    assert_eq!(marker_indexes.len(), 1);
    let logical_font_size = label.key.style.size();

    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, 3);
    assert_eq!(report.retained, 3);
    assert_eq!(
        resources
            .text_layout_resource(label_id)
            .expect("reconciled property label resource")
            .key
            .text,
        source
    );

    let mut expected_ids = label_ids.clone();
    expected_ids.extend_from_slice(&help_ids);
    expected_ids.extend_from_slice(&status_ids);
    let mut renderer = VelloRenderer::new();
    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(f64::from(scale)),
            primitives: &frame.primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_ids = encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(encoded_ids, expected_ids);
        assert_eq!(encoded_ids[marker_indexes[0]], label_ids[marker_indexes[0]]);
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| (run.font_size - logical_font_size * scale).abs() <= 0.000_1)
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
fn registered_end_ellipsis_encodes_engine_topology_without_fallback() {
    let id = TextLayoutId::from_raw(46);
    let source = "Registered Vello authority keeps this complete source while presenting ellipsis";
    let key = TextLayoutKey::new(source, TextStyle::new("Inter", 18.0, 24.0), 84.0, false)
        .with_overflow(TextOverflow::EndEllipsis);
    let expected = resource(id, key);
    assert_eq!(expected.key.text, source);
    assert_eq!(expected.key.overflow, TextOverflow::EndEllipsis);
    assert!(expected.layout.is_elided());
    let expected_ids = expected
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let expected_marker_ids = expected
        .layout
        .runs
        .iter()
        .flat_map(|run| &run.glyphs)
        .filter(|glyph| glyph.elided)
        .map(|glyph| glyph.id)
        .collect::<Vec<_>>();
    assert_eq!(expected_marker_ids.len(), 1);

    let mut resources = RenderResources::new();
    resources.register_text_layout(expected);
    let primitives = [primitive(Some(id), "conflicting fallback text")];
    let mut renderer = VelloRenderer::new();

    for scale in [1.0_f32, 1.25, 1.5, 2.0] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(f64::from(scale)),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let encoded_ids = encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>();

        assert!(output.diagnostics.is_empty());
        assert_eq!(encoded_ids, expected_ids);
        assert!(encoded_ids.contains(&expected_marker_ids[0]));
        assert!(
            encoding
                .resources
                .glyph_runs
                .iter()
                .all(|run| (run.font_size - 18.0 * scale).abs() <= 0.000_1)
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
        assert_eq!(renderer.cached_text_layout_payload_bytes(), 0);
    }
}

#[test]
fn layoutless_text_shapes_logically_and_scales_without_metric_quantization() {
    let primitives = [Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Fallback".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 17.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &RenderResources::new(),
    });
    let run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_approx(run.font_size, 16.25);
    assert!(run.hint);
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn registered_text_ignores_conflicting_primitive_metadata() {
    let id = TextLayoutId::from_raw(44);
    let key = TextLayoutKey::new(
        "Registered authority",
        TextStyle::new("sans-serif", 13.0, 17.0),
        200.0,
        false,
    );
    let expected = resource(id, key);
    let expected_ids = expected
        .layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter().map(|glyph| glyph.id))
        .collect::<Vec<_>>();
    let mut resources = RenderResources::new();
    resources.register_text_layout(expected);
    let primitives = [primitive(Some(id), "wrong fallback")];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.25),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();

    assert!(output.diagnostics.is_empty());
    assert_eq!(
        encoding
            .resources
            .glyphs
            .iter()
            .map(|glyph| glyph.id)
            .collect::<Vec<_>>(),
        expected_ids
    );
    assert!(
        encoding
            .resources
            .glyph_runs
            .iter()
            .all(|run| (run.font_size - 16.25).abs() <= 0.000_1)
    );
    assert_eq!(renderer.cached_text_layout_count(), 0);
}

#[test]
fn registered_wrapped_layout_keeps_its_original_line_and_glyph_topology() {
    let id = TextLayoutId::from_raw(45);
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon zeta",
        TextStyle::new("sans-serif", 13.0, 17.0),
        72.0,
        true,
    );
    let expected = resource(id, key);
    assert!(expected.layout.line_count > 1);
    let expected_glyphs = expected.layout.glyph_count();
    let mut resources = RenderResources::new();
    resources.register_text_layout(expected);
    let primitives = [primitive(Some(id), "unwrapped conflict")];
    let mut renderer = VelloRenderer::new();

    for scale in [1.25, 1.5, 1.75] {
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: viewport(scale),
            primitives: &primitives,
            resources: &resources,
        });
        assert!(output.diagnostics.is_empty());
        assert_eq!(
            renderer.scene().encoding().resources.glyphs.len(),
            expected_glyphs
        );
        assert_eq!(renderer.cached_text_layout_count(), 0);
    }
}

#[test]
fn translated_registered_text_uses_exact_scaled_font_size_and_absolute_snapping() {
    let id = TextLayoutId::from_raw(47);
    let key = TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 13.0, 17.0),
        200.0,
        false,
    );
    let mut resources = RenderResources::new();
    resources.register_text_layout(resource(id, key));
    let primitives = [
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        primitive(Some(id), "wrong"),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: viewport(1.5),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let run = encoding.resources.glyph_runs.first().expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_approx(run.font_size, 19.5);
    assert!(run.hint);
    assert!(
        encoding
            .resources
            .glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001
                && (glyph.y - glyph.y.round()).abs() <= 0.001)
    );
    assert_eq!(renderer.cached_text_layout_count(), 0);
}
