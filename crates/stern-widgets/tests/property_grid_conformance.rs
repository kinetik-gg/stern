//! Public live property-grid conformance tests.

use std::collections::BTreeMap;

use stern_core::{
    Brush, Color, FrameOutput, MouseButton, Point, PointerInput, Primitive, Rect, SemanticRole,
    Theme, UiInput, UiInputEvent, UiMemory, Vec2, WidgetId, default_dark_theme,
};
use stern_widgets::{
    ItemId, Ui,
    inspector::{
        PropertyGridAccess, PropertyGridConfig, PropertyGridError, PropertyGridIntent,
        PropertyGridLayout, PropertyGridOutput, PropertyGridRow, PropertyGridRowStatus,
        property_grid_row_affordance_rects, property_grid_row_widget_id,
        property_grid_value_widget_id,
    },
};

const BOUNDS: Rect = Rect::new(10.0, 20.0, 360.0, 140.0);

type CellOutput = (PropertyGridAccess, WidgetId, Rect, bool, bool);

fn run_grid(
    rows: &[PropertyGridRow],
    bounds: Rect,
    config: PropertyGridConfig,
    input: &UiInput,
    memory: &mut UiMemory,
    theme: &Theme,
) -> (PropertyGridOutput<CellOutput>, FrameOutput) {
    let mut ui = Ui::new(input, memory, theme);
    let output = ui
        .property_grid("grid", bounds, rows, config, |ui, cell| {
            let response = ui.pressable("field", cell.value_rect, cell.access.disabled());
            ui.label_keyed(
                "live-value",
                cell.value_rect,
                format!("value-{}", cell.row.id.raw()),
            );
            (
                cell.access,
                response.id,
                cell.value_rect,
                response.state.pressed,
                response.clicked,
            )
        })
        .expect("valid property rows");
    (output, ui.finish_output())
}

fn pointer_button(point: Point, down: bool) -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down,
        click_count: 1,
        position: Some(point),
    });
    input
}

fn wheel_input(bounds: Rect, delta_y: f32) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(bounds.center()),
            wheel_delta: Vec2::new(0.0, delta_y),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

#[test]
fn live_state_drives_paint_semantics_access_and_theme() {
    let rows = vec![
        PropertyGridRow::section(ItemId::from_raw(1), "Transform"),
        PropertyGridRow::property(ItemId::from_raw(2), "Exposure", 0)
            .with_required(true)
            .with_help_text("Controls scene brightness")
            .with_status(PropertyGridRowStatus::warning("Preview range exceeded")),
        PropertyGridRow::property(ItemId::from_raw(3), "Script", 0).with_read_only(true),
        PropertyGridRow::property(ItemId::from_raw(4), "Missing", 0).with_disabled(true),
    ];
    let config = PropertyGridConfig::default();
    let mut memory = UiMemory::new();
    let mut theme = default_dark_theme();
    theme.colors.surface.sunken = Color::rgba(0.12, 0.23, 0.34, 1.0);
    let (output, frame) = run_grid(
        &rows,
        BOUNDS,
        config,
        &UiInput::default(),
        &mut memory,
        &theme,
    );

    assert_eq!(
        output
            .values
            .iter()
            .map(|value| value.value.0)
            .collect::<Vec<_>>(),
        vec![
            PropertyGridAccess::Editable,
            PropertyGridAccess::ReadOnly,
            PropertyGridAccess::Disabled,
        ]
    );
    let text = frame
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(text.contains(&"Transform"));
    assert!(text.contains(&"Exposure *"));
    assert!(text.contains(&"?"));
    assert!(text.contains(&"!"));
    assert!(text.contains(&"value-2"));
    assert!(frame.primitives.iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect)
            if rect.fill == Some(Brush::Solid(theme.colors.surface.sunken))
    )));

    let semantics = frame.semantics.nodes();
    assert!(
        semantics
            .iter()
            .any(|node| { node.role == SemanticRole::Grid && node.id == output.root })
    );
    assert!(semantics.iter().any(|node| {
        node.label.as_deref() == Some("Exposure")
            && node.description.as_deref().is_some_and(|description| {
                description.contains("Controls scene brightness")
                    && description.contains("Warning: Preview range exceeded")
            })
    }));
    assert!(semantics.iter().any(|node| {
        node.id == property_grid_row_widget_id(output.root, ItemId::from_raw(4))
            && node.state.disabled
    }));
    assert!(frame.warnings.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn stable_value_scopes_survive_reorder_and_offscreen_rows_are_inert() {
    let a = PropertyGridRow::property(ItemId::from_raw(10), "A", 0);
    let b = PropertyGridRow::property(ItemId::from_raw(20), "B", 0);
    let orders = [vec![a.clone(), b.clone()], vec![b, a]];
    let mut observed = Vec::new();

    for rows in &orders {
        let mut memory = UiMemory::new();
        let (output, frame) = run_grid(
            rows,
            BOUNDS,
            PropertyGridConfig::default(),
            &UiInput::default(),
            &mut memory,
            &default_dark_theme(),
        );
        let ids = output
            .values
            .iter()
            .map(|value| {
                assert_eq!(
                    value.value.1,
                    property_grid_value_widget_id(output.root, value.row).child("field")
                );
                (value.row, value.value.1)
            })
            .collect::<BTreeMap<_, _>>();
        assert_eq!(ids.len(), 2);
        assert_ne!(ids[&ItemId::from_raw(10)], ids[&ItemId::from_raw(20)]);
        assert!(frame.warnings.is_empty());
        observed.push(ids);
    }
    assert_eq!(observed[0], observed[1]);

    let rows = (0..5)
        .map(|index| {
            PropertyGridRow::property(ItemId::from_raw(100 + index), format!("Row {index}"), 0)
                .with_resettable(index == 0, false)
        })
        .collect::<Vec<_>>();
    let viewport = Rect::new(0.0, 0.0, 300.0, 24.0);
    let mut memory = UiMemory::new();
    let (initial, _) = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &mut memory,
        &default_dark_theme(),
    );
    assert_eq!(initial.values[0].row, ItemId::from_raw(100));

    let (scrolled, frame) = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &wheel_input(viewport, -1_000.0),
        &mut memory,
        &default_dark_theme(),
    );
    assert_eq!(
        scrolled.scroll.offset.y.to_bits(),
        scrolled.scroll.max_offset.y.to_bits()
    );
    assert_eq!(scrolled.values.len(), 1);
    assert_eq!(scrolled.values[0].row, ItemId::from_raw(104));
    assert!(matches!(
        frame.primitives.first(),
        Some(Primitive::ClipBegin { .. })
    ));
    assert!(matches!(
        frame.primitives.last(),
        Some(Primitive::ClipEnd { .. })
    ));

    let (retained, _) = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &UiInput::default(),
        &mut memory,
        &default_dark_theme(),
    );
    assert_eq!(retained.scroll.offset, scrolled.scroll.offset);
    assert_eq!(retained.values[0].row, ItemId::from_raw(104));

    let mut memory = UiMemory::new();
    let _ = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &wheel_input(viewport, -12.0),
        &mut memory,
        &default_dark_theme(),
    );
    let outside = Point::new(250.0, viewport.y - 4.0);
    let (pressed, _) = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &pointer_button(outside, true),
        &mut memory,
        &default_dark_theme(),
    );
    let (released, _) = run_grid(
        &rows,
        viewport,
        PropertyGridConfig::default(),
        &pointer_button(outside, false),
        &mut memory,
        &default_dark_theme(),
    );
    assert!(pressed.values.iter().all(|value| !value.value.3));
    assert!(released.values.iter().all(|value| !value.value.4));
    assert!(released.intents.is_empty());
}

#[test]
fn affordances_emit_typed_intents_without_mutating_application_state() {
    let row = PropertyGridRow::property(ItemId::from_raw(7), "Exposure", 0)
        .with_resettable(true, false)
        .with_keyframeable(true, false);
    let rows = vec![row.clone()];
    let config = PropertyGridConfig::default();
    let geometry = config.layout.visible_row_rects(BOUNDS, &rows, 0.0, 0)[0];
    let affordances = property_grid_row_affordance_rects(
        &row,
        geometry.value_rect.inset(2.0).max_zero(),
        config.affordances,
    );
    let reset_point = affordances.reset_rect.expect("reset affordance").center();
    let keyframe_point = affordances
        .keyframe_rect
        .expect("keyframe affordance")
        .center();
    let application_value = 41;

    let mut memory = UiMemory::new();
    let _ = run_grid(
        &rows,
        BOUNDS,
        config,
        &pointer_button(reset_point, true),
        &mut memory,
        &default_dark_theme(),
    );
    let (released, _) = run_grid(
        &rows,
        BOUNDS,
        config,
        &pointer_button(reset_point, false),
        &mut memory,
        &default_dark_theme(),
    );
    assert_eq!(
        released.intents,
        vec![PropertyGridIntent::Reset {
            row: ItemId::from_raw(7)
        }]
    );
    assert_eq!(application_value, 41);

    let mut memory = UiMemory::new();
    let _ = run_grid(
        &rows,
        BOUNDS,
        config,
        &pointer_button(keyframe_point, true),
        &mut memory,
        &default_dark_theme(),
    );
    let (released, _) = run_grid(
        &rows,
        BOUNDS,
        config,
        &pointer_button(keyframe_point, false),
        &mut memory,
        &default_dark_theme(),
    );
    assert_eq!(
        released.intents,
        vec![PropertyGridIntent::SetKeyed {
            row: ItemId::from_raw(7),
            keyed: true,
        }]
    );

    for blocked in [row.clone().with_read_only(true), row.with_disabled(true)] {
        let rows = vec![blocked];
        let mut memory = UiMemory::new();
        let _ = run_grid(
            &rows,
            BOUNDS,
            config,
            &pointer_button(reset_point, true),
            &mut memory,
            &default_dark_theme(),
        );
        let (released, _) = run_grid(
            &rows,
            BOUNDS,
            config,
            &pointer_button(reset_point, false),
            &mut memory,
            &default_dark_theme(),
        );
        assert!(released.intents.is_empty());
    }
}

#[test]
fn invalid_duplicate_and_tiny_geometry_fail_safely() {
    let row = PropertyGridRow::property(ItemId::from_raw(1), "Value", 0);
    for bounds in [
        Rect::ZERO,
        Rect::new(0.0, 0.0, -1.0, 20.0),
        Rect::new(f32::NAN, 0.0, 20.0, 20.0),
        Rect::new(0.0, f32::INFINITY, 20.0, 20.0),
    ] {
        let mut memory = UiMemory::new();
        let (output, frame) = run_grid(
            std::slice::from_ref(&row),
            bounds,
            PropertyGridConfig::default(),
            &UiInput::default(),
            &mut memory,
            &default_dark_theme(),
        );
        assert!(output.visible_rows.is_empty());
        assert!(output.values.is_empty());
        assert!(output.intents.is_empty());
        assert!(output.scroll.response.state.disabled);
        assert!(frame.primitives.is_empty());
        assert!(frame.semantics.nodes().is_empty());
        assert!(frame.warnings.is_empty());
    }

    let duplicates = vec![row.clone(), row.clone()];
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let result = ui.property_grid(
        "grid",
        BOUNDS,
        &duplicates,
        PropertyGridConfig::default(),
        |_, _| (),
    );
    assert_eq!(
        result,
        Err(PropertyGridError::DuplicateRowId {
            id: ItemId::from_raw(1)
        })
    );
    let frame = ui.finish_output();
    assert!(frame.primitives.is_empty());
    assert!(frame.semantics.nodes().is_empty());

    let tiny = PropertyGridConfig::new(PropertyGridLayout::new(2.0, 2.0, 20.0, 0.0, 0.0));
    let mut memory = UiMemory::new();
    let (output, frame) = run_grid(
        std::slice::from_ref(&row),
        Rect::new(0.0, 0.0, 80.0, 2.0),
        tiny,
        &UiInput::default(),
        &mut memory,
        &default_dark_theme(),
    );
    let cell = output.values[0].value.2;
    assert!(cell.x.is_finite() && cell.y.is_finite());
    assert!(cell.width >= 0.0 && cell.height >= 0.0);
    assert!(frame.primitives.iter().all(primitive_is_finite));
}

fn primitive_is_finite(primitive: &Primitive) -> bool {
    let rect = match primitive {
        Primitive::Rect(rect) => Some(rect.rect),
        Primitive::ClipBegin { rect, .. } => Some(*rect),
        _ => None,
    };
    rect.is_none_or(|rect| {
        rect.x.is_finite()
            && rect.y.is_finite()
            && rect.width.is_finite()
            && rect.height.is_finite()
            && rect.width >= 0.0
            && rect.height >= 0.0
    })
}
