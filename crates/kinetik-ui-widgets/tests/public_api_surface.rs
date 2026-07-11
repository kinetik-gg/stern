//! Compile-level coverage for public widget crate access paths.

use kinetik_ui_core::{Rect, UiInput, UiMemory, WidgetId, default_dark_theme};
use kinetik_ui_widgets::{self as widgets, prelude};

#[test]
fn root_exports_common_and_compatibility_widget_items() {
    let theme = default_dark_theme();
    let output = widgets::label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label", &theme);
    let panel = widgets::Panel::new(widgets::PanelId::from_raw(1), "Inspector");
    let node = widgets::NodeDescriptor::new(
        widgets::NodeId::from_raw(1),
        "Node",
        widgets::GraphRect::new(0.0, 0.0, 120.0, 80.0),
    );
    let frame_move_request = widgets::NodeGraphFrameMoveRequest {
        frame: widgets::NodeGraphFrameMove {
            frame: widgets::NodeFrameId::from_raw(2),
            delta: widgets::GraphVector::new(1.0, 0.0),
        },
        screen_delta: widgets::GraphVector::new(1.0, 0.0),
        graph_delta: widgets::GraphVector::new(1.0, 0.0),
        children: Vec::new(),
    };

    assert_eq!(output.primitives.len(), 1);
    assert_eq!(panel.title, "Inspector");
    assert_eq!(node.title, "Node");
    assert!(!frame_move_request.is_noop());
}

#[test]
fn prelude_exports_common_application_widget_items() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let button = prelude::button(
        WidgetId::from_key("button"),
        Rect::new(0.0, 0.0, 80.0, 24.0),
        "Run",
        &UiInput::default(),
        &mut memory,
        &theme,
        false,
    );
    let surface = prelude::ViewportSurface {
        texture: kinetik_ui_core::TextureId::from_raw(1),
        source_size: kinetik_ui_core::Size::new(100.0, 100.0),
        bounds: Rect::new(0.0, 0.0, 100.0, 100.0),
        pan_zoom: widgets::PanZoom::default(),
    };

    assert!(button.response.is_some());
    assert!((surface.content_scale() - 1.0).abs() <= f32::EPSILON);
}

#[test]
fn modules_remain_available_for_advanced_apis() {
    let descriptor = widgets::dock::PanelTypeDescriptor::new(
        widgets::dock::PanelTypeId::from_raw(1),
        "Inspector",
    );
    let target = widgets::node_graph::NodeGraphContextTarget::Canvas;
    let viewport_tool = widgets::viewport::ViewportToolDescriptor::new(
        widgets::viewport::ViewportToolId::from_raw(1),
        "Select",
    )
    .active(true);

    assert_eq!(descriptor.title, "Inspector");
    assert_eq!(target, widgets::node_graph::NodeGraphContextTarget::Canvas);
    assert!(viewport_tool.active);
}

#[test]
fn root_exports_access_aware_text_field_api_without_expanding_prelude_usage() {
    fn assert_traits<T: core::fmt::Debug + Clone + Copy + PartialEq + Eq + core::hash::Hash>() {}
    assert_traits::<widgets::TextFieldAccess>();

    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut single = kinetik_ui_text::TextEditState::new("single");
    let mut multi = kinetik_ui_text::TextEditState::new("multi\nline");
    let mut ui = widgets::Ui::new(&input, &mut memory, &theme);
    let single_output = ui.text_field_with_access(
        "single",
        Rect::new(0.0, 0.0, 120.0, 24.0),
        &mut single,
        widgets::TextFieldAccess::ReadOnly,
    );
    let multi_output = ui.multi_line_text_field_with_access(
        "multi",
        Rect::new(0.0, 28.0, 120.0, 60.0),
        &mut multi,
        widgets::TextFieldAccess::Disabled,
    );
    let _ = ui.finish_output();

    assert!(!single_output.changed);
    assert!(!multi_output.changed);
    assert_eq!(
        widgets::TextFieldAccess::Editable,
        widgets::TextFieldAccess::Editable
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn text_wrapper_signatures_and_output_shapes_remain_source_compatible() {
    fn numeric(_: &widgets::NumericInputOutput) {}
    fn scrub(_: &widgets::NumericScrubInputOutput) {}
    fn search(_: &widgets::SearchFieldOutput) {}
    fn path(_: &widgets::PathFieldOutput) {}
    fn vector<const N: usize>(_: &widgets::VectorScrubInputOutput<N>) {}

    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let rect = Rect::new(0.0, 0.0, 240.0, 24.0);
    let mut numeric_state = kinetik_ui_text::TextEditState::new("1");
    let numeric_output = widgets::numeric_input(
        WidgetId::from_key("numeric"),
        rect,
        &mut numeric_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    numeric(&numeric_output);

    let mut search_state = kinetik_ui_text::TextEditState::new("find");
    let search_output = prelude::search_field(
        WidgetId::from_key("search"),
        rect,
        &mut search_state,
        &input,
        &mut memory,
        &theme,
        false,
    );
    search(&search_output);

    let mut scrub_value = 1.0;
    let mut scrub_state = kinetik_ui_text::TextEditState::new("1");
    let scrub_output = prelude::numeric_scrub_input(
        WidgetId::from_key("scrub"),
        rect,
        &mut scrub_value,
        &mut scrub_state,
        widgets::NumericScrubInputConfig::default(),
        &input,
        &mut memory,
        &theme,
    );
    scrub(&scrub_output);

    let mut path_state = kinetik_ui_text::TextEditState::new("src/lib.rs");
    let path_output = widgets::path_field(
        WidgetId::from_key("path"),
        rect,
        "Source",
        &mut path_state,
        widgets::PathFieldConfig::default(),
        &input,
        &mut memory,
        &theme,
    );
    path(&path_output);

    let mut values = [1.0, 2.0];
    let mut states = [
        kinetik_ui_text::TextEditState::new("1"),
        kinetik_ui_text::TextEditState::new("2"),
    ];
    let vector_output = widgets::vector2_scrub_input(
        WidgetId::from_key("vector"),
        rect,
        "Offset",
        &mut values,
        &mut states,
        widgets::VectorScrubInputConfig::default(),
        &input,
        &mut memory,
        &theme,
    );
    vector(&vector_output);

    let mut memory = UiMemory::new();
    let mut ui_numeric = kinetik_ui_text::TextEditState::new("1");
    let mut ui_search = kinetik_ui_text::TextEditState::new("find");
    let mut ui_path = kinetik_ui_text::TextEditState::new("src/lib.rs");
    let mut ui_value = 1.0;
    let mut ui_scrub = kinetik_ui_text::TextEditState::new("1");
    let mut ui_values = [1.0, 2.0];
    let mut ui_states = [
        kinetik_ui_text::TextEditState::new("1"),
        kinetik_ui_text::TextEditState::new("2"),
    ];
    let mut ui = widgets::Ui::new(&input, &mut memory, &theme);
    numeric(&ui.numeric_input("numeric", rect, &mut ui_numeric, false));
    search(&ui.search_field("search", rect, &mut ui_search, false));
    path(&ui.path_field(
        "path",
        rect,
        "Source",
        &mut ui_path,
        widgets::PathFieldConfig::default(),
    ));
    scrub(&ui.numeric_scrub_input(
        "scrub",
        rect,
        &mut ui_value,
        &mut ui_scrub,
        widgets::NumericScrubInputConfig::default(),
    ));
    vector(&ui.vector2_scrub_input(
        "vector",
        rect,
        "Offset",
        &mut ui_values,
        &mut ui_states,
        widgets::VectorScrubInputConfig::default(),
    ));
    let _ = ui.finish_output();
}

#[test]
#[allow(clippy::too_many_lines)]
fn affected_wrapper_root_prelude_and_layout_paths_compile_call() {
    use kinetik_ui_text::{TextEditState, TextLayoutStore};

    let theme = default_dark_theme();
    let input = UiInput::default();
    let rect = Rect::new(0.0, 0.0, 320.0, 24.0);
    let mut memory = UiMemory::new();
    let mut layouts = TextLayoutStore::new();

    let mut numeric = TextEditState::new("1");
    let _: widgets::NumericInputOutput = widgets::numeric_input_with_text_layouts(
        WidgetId::from_key("root-numeric-layout"),
        rect,
        &mut numeric,
        &input,
        &mut memory,
        &theme,
        false,
        Some(&mut layouts),
    );
    let mut scrub_value = 1.0;
    let mut scrub = TextEditState::new("1");
    let _: widgets::NumericScrubInputOutput = widgets::numeric_scrub_input_with_text_layouts(
        WidgetId::from_key("root-scrub-layout"),
        rect,
        &mut scrub_value,
        &mut scrub,
        widgets::NumericScrubInputConfig::default(),
        &input,
        &mut memory,
        &theme,
        Some(&mut layouts),
    );
    let mut search = TextEditState::new("find");
    let _: widgets::SearchFieldOutput = widgets::search_field_with_text_layouts(
        WidgetId::from_key("root-search-layout"),
        rect,
        &mut search,
        &input,
        &mut memory,
        &theme,
        false,
        Some(&mut layouts),
    );
    let mut path = TextEditState::new("src/lib.rs");
    let _: widgets::PathFieldOutput = widgets::path_field_with_text_layouts(
        WidgetId::from_key("root-path-layout"),
        rect,
        "Source",
        &mut path,
        widgets::PathFieldConfig::default(),
        &input,
        &mut memory,
        &theme,
        Some(&mut layouts),
    );

    let mut values3 = [1.0, 2.0, 3.0];
    let mut states3 = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let _: widgets::VectorScrubInputOutput<3> = widgets::vector3_scrub_input(
        WidgetId::from_key("root-vector3"),
        rect,
        "Position",
        &mut values3,
        &mut states3,
        widgets::VectorScrubInputConfig::default(),
        &input,
        &mut memory,
        &theme,
    );
    let mut values4 = [1.0, 2.0, 3.0, 4.0];
    let mut states4 = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
        TextEditState::new("4"),
    ];
    let _: widgets::VectorScrubInputOutput<4> = prelude::vector4_scrub_input(
        WidgetId::from_key("prelude-vector4"),
        rect,
        "Color",
        &mut values4,
        &mut states4,
        widgets::VectorScrubInputConfig::default(),
        &input,
        &mut memory,
        &theme,
    );

    let mut prelude_numeric = TextEditState::new("2");
    let _: widgets::NumericInputOutput = prelude::numeric_input(
        WidgetId::from_key("prelude-numeric"),
        rect,
        &mut prelude_numeric,
        &input,
        &mut memory,
        &theme,
        false,
    );
    let mut prelude_path = TextEditState::new("Cargo.toml");
    let _: widgets::PathFieldOutput = prelude::path_field(
        WidgetId::from_key("prelude-path"),
        rect,
        "Manifest",
        &mut prelude_path,
        widgets::PathFieldConfig::default(),
        &input,
        &mut memory,
        &theme,
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn affected_ui_methods_compile_with_attached_text_layouts() {
    use kinetik_ui_text::{TextEditState, TextLayoutStore};

    let theme = default_dark_theme();
    let input = UiInput::default();
    let rect = Rect::new(0.0, 0.0, 320.0, 24.0);
    let mut memory = UiMemory::new();
    let mut layouts = TextLayoutStore::new();
    let mut numeric = TextEditState::new("1");
    let mut search = TextEditState::new("find");
    let mut path = TextEditState::new("src/lib.rs");
    let mut scrub_value = 1.0;
    let mut scrub = TextEditState::new("1");
    let mut values2 = [1.0, 2.0];
    let mut states2 = [TextEditState::new("1"), TextEditState::new("2")];
    let mut values3 = [1.0, 2.0, 3.0];
    let mut states3 = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
    ];
    let mut values4 = [1.0, 2.0, 3.0, 4.0];
    let mut states4 = [
        TextEditState::new("1"),
        TextEditState::new("2"),
        TextEditState::new("3"),
        TextEditState::new("4"),
    ];

    let mut ui = widgets::Ui::new(&input, &mut memory, &theme).with_text_layouts(&mut layouts);
    let _: widgets::NumericInputOutput = ui.numeric_input("numeric", rect, &mut numeric, false);
    let _: widgets::SearchFieldOutput = ui.search_field("search", rect, &mut search, false);
    let _: widgets::PathFieldOutput = ui.path_field(
        "path",
        rect,
        "Source",
        &mut path,
        widgets::PathFieldConfig::default(),
    );
    let _: widgets::NumericScrubInputOutput = ui.numeric_scrub_input(
        "scrub",
        rect,
        &mut scrub_value,
        &mut scrub,
        widgets::NumericScrubInputConfig::default(),
    );
    let _: widgets::VectorScrubInputOutput<2> = ui.vector2_scrub_input(
        "vector2",
        rect,
        "Offset",
        &mut values2,
        &mut states2,
        widgets::VectorScrubInputConfig::default(),
    );
    let _: widgets::VectorScrubInputOutput<3> = ui.vector3_scrub_input(
        "vector3",
        rect,
        "Position",
        &mut values3,
        &mut states3,
        widgets::VectorScrubInputConfig::default(),
    );
    let _: widgets::VectorScrubInputOutput<4> = ui.vector4_scrub_input(
        "vector4",
        rect,
        "Color",
        &mut values4,
        &mut states4,
        widgets::VectorScrubInputConfig::default(),
    );
    let _ = ui.finish_output();
}
