//! Bounded icon fallback, geometry, and accessible-name conformance evidence.

use stern_core::{
    Alignment, CursorShape, FrameOutput, ImageId, PathElement, PlatformRequest, Point,
    PointerButtonState, PointerInput, Primitive, Rect, Response, SemanticActionKind, Size, UiInput,
    UiMemory, WidgetId, default_dark_theme, fit_box,
};
use stern_widgets::{
    IconGraphic, IconId, IconLibrary, IconPath, Ui, WidgetOutput, icon_button_with_library,
    image_icon_button,
};

const OUTER: Rect = Rect::new(12.25, 20.5, 40.0, 32.0);
const LABEL: &str = "Open inspector";

fn icon_id(raw: u64) -> IconId {
    IconId::from_raw(raw)
}

fn widget_id() -> WidgetId {
    WidgetId::from_key("bounded-icon")
}

fn graphic(inset: f32) -> IconGraphic {
    IconGraphic::new(
        Rect::new(0.0, 0.0, 16.0, 16.0),
        vec![IconPath::filled(vec![
            PathElement::MoveTo(Point::new(inset, inset)),
            PathElement::LineTo(Point::new(16.0 - inset, inset)),
            PathElement::LineTo(Point::new(16.0 - inset, 16.0 - inset)),
            PathElement::LineTo(Point::new(inset, 16.0 - inset)),
            PathElement::Close,
        ])],
    )
}

fn render_vector(
    icons: &IconLibrary,
    icon: IconId,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> WidgetOutput {
    icon_button_with_library(
        widget_id(),
        OUTER,
        icon,
        LABEL,
        icons,
        input,
        memory,
        &default_dark_theme(),
        disabled,
    )
}

fn fresh_vector(
    icons: &IconLibrary,
    icon: IconId,
    input: &UiInput,
    disabled: bool,
) -> WidgetOutput {
    render_vector(icons, icon, input, &mut UiMemory::new(), disabled)
}

fn optical_box() -> Rect {
    let theme = default_dark_theme();
    fit_box(
        OUTER,
        Size::new(theme.sizes.icon.md, theme.sizes.icon.md),
        Alignment::Center,
        Alignment::Center,
    )
}

fn pointer_input(down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(OUTER.center()),
            primary: PointerButtonState::new(down, pressed, released),
            click_count: u8::from(released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn render_frame(
    icons: &IconLibrary,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> (Response, FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme).with_icons(icons);
    let response = ui.icon_button_with_label("frame-icon", OUTER, icon_id(23), LABEL, disabled);
    (response, ui.finish_output())
}

fn fresh_frame(icons: &IconLibrary, input: &UiInput, disabled: bool) -> (Response, FrameOutput) {
    render_frame(icons, input, &mut UiMemory::new(), disabled)
}

fn pointer_cycle(icons: &IconLibrary) -> (Response, FrameOutput, FrameOutput) {
    let mut memory = UiMemory::new();
    let press = pointer_input(true, true, false);
    let (_, pressed) = render_frame(icons, &press, &mut memory, false);
    let release = pointer_input(false, false, true);
    let (released, finished) = render_frame(icons, &release, &mut memory, false);
    (released, pressed, finished)
}

fn assert_outer_contract(output: &WidgetOutput) {
    let response = output.response.expect("icon response");
    assert_eq!(response.id, widget_id());
    assert_eq!(response.rect, OUTER);
    let [semantic] = output.semantics.as_slice() else {
        panic!("one icon semantic node");
    };
    assert_eq!(semantic.id, widget_id());
    assert_eq!(semantic.bounds, OUTER);
    assert_eq!(semantic.label.as_deref(), Some(LABEL));
}

fn assert_point_in(rect: Rect, point: Point) {
    assert!(rect.contains_point(point));
}

fn assert_icon_tail_contained(output: &WidgetOutput, count: usize) {
    assert!(count > 0 && output.primitives.len() > count);
    let icon_primitives = &output.primitives[output.primitives.len() - count..];
    for primitive in icon_primitives {
        match primitive {
            Primitive::Path(path) => {
                assert!(!path.elements.is_empty());
                for element in &path.elements {
                    match element {
                        PathElement::MoveTo(point) | PathElement::LineTo(point) => {
                            assert_point_in(optical_box(), *point);
                        }
                        PathElement::Close => {}
                        other => panic!("unexpected icon path element {other:?}"),
                    }
                }
            }
            Primitive::Line(line) => {
                assert_point_in(optical_box(), line.from);
                assert_point_in(optical_box(), line.to);
            }
            Primitive::Image(image) => assert!(optical_box().contains_rect(image.rect)),
            other => panic!("unexpected icon-tail primitive {other:?}"),
        }
    }
}

#[test]
fn registered_missing_and_unpaintable_icons_keep_exact_outer_contract_bounds() {
    let icon = icon_id(7);
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut icons = IconLibrary::new();

    let missing_before = render_vector(&icons, icon, &input, &mut memory, false);
    icons.register(icon, graphic(2.0));
    let registered = render_vector(&icons, icon, &input, &mut memory, false);
    icons = IconLibrary::new();
    let missing_after = render_vector(&icons, icon, &input, &mut memory, false);

    let invalid_graphics = [
        IconGraphic::new(
            Rect::new(0.0, 0.0, 0.0, 16.0),
            vec![IconPath::filled(vec![PathElement::Close])],
        ),
        IconGraphic::new(Rect::new(0.0, 0.0, 16.0, 16.0), Vec::new()),
        IconGraphic::new(
            Rect::new(0.0, 0.0, 16.0, 16.0),
            vec![IconPath {
                elements: vec![PathElement::MoveTo(Point::new(4.0, 4.0))],
                fill: false,
                stroke_width: None,
            }],
        ),
    ];
    let invalid = invalid_graphics.map(|graphic| {
        let mut icons = IconLibrary::new();
        icons.register(icon, graphic);
        render_vector(&icons, icon, &input, &mut memory, false)
    });

    let expected_response = missing_before.response.expect("missing response");
    for output in [
        &missing_before,
        &registered,
        &missing_after,
        &invalid[0],
        &invalid[1],
        &invalid[2],
    ] {
        assert_outer_contract(output);
        assert_eq!(output.response, Some(expected_response));
    }

    assert_icon_tail_contained(&registered, 1);
    for output in [
        &missing_before,
        &missing_after,
        &invalid[0],
        &invalid[1],
        &invalid[2],
    ] {
        assert_icon_tail_contained(output, 2);
    }
    assert_eq!(missing_before.primitives, missing_after.primitives);
    assert_ne!(registered.primitives, missing_before.primitives);
}

#[test]
fn icon_bounds_survive_idle_hover_press_and_disabled_states() {
    let icon = icon_id(11);
    let mut icons = IconLibrary::new();
    icons.register(icon, graphic(3.0));

    let idle = fresh_vector(&icons, icon, &UiInput::default(), false);
    let hovered = fresh_vector(&icons, icon, &pointer_input(false, false, false), false);
    let pressed = fresh_vector(&icons, icon, &pointer_input(true, true, false), false);
    let disabled = fresh_vector(&icons, icon, &pointer_input(true, true, false), true);

    for output in [&idle, &hovered, &pressed, &disabled] {
        assert_outer_contract(output);
        assert_icon_tail_contained(output, 1);
    }
    assert!(!idle.response.expect("idle response").state.hovered);
    assert!(hovered.response.expect("hover response").state.hovered);
    assert!(pressed.response.expect("pressed response").state.pressed);

    let disabled_response = disabled.response.expect("disabled response");
    assert!(disabled_response.state.disabled);
    assert!(!disabled_response.clicked);
    assert!(!disabled_response.keyboard_activated);
    let disabled_semantic = &disabled.semantics[0];
    assert!(disabled_semantic.state.disabled);
    assert!(!disabled_semantic.focusable);
    assert_eq!(disabled_semantic.label.as_deref(), Some(LABEL));
}

#[test]
fn accessible_name_is_independent_of_icon_identity_and_graphic_content() {
    let first = icon_id(17);
    let second = icon_id(19);
    let mut first_graphic = IconLibrary::new();
    first_graphic.register(first, graphic(2.0));
    let mut changed_graphic = IconLibrary::new();
    changed_graphic.register(first, graphic(5.0));
    changed_graphic.register(second, graphic(3.0));
    let missing = IconLibrary::new();

    let outputs = [
        fresh_vector(&first_graphic, first, &UiInput::default(), false),
        fresh_vector(&changed_graphic, first, &UiInput::default(), false),
        fresh_vector(&changed_graphic, second, &UiInput::default(), false),
        fresh_vector(&missing, first, &UiInput::default(), false),
        image_icon_button(
            widget_id(),
            OUTER,
            ImageId::from_raw(31),
            LABEL,
            &UiInput::default(),
            &mut UiMemory::new(),
            &default_dark_theme(),
            false,
        ),
    ];

    for output in &outputs {
        assert_outer_contract(output);
        let semantic = &output.semantics[0];
        assert!(
            semantic
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
        );
    }
    assert_ne!(outputs[0].primitives, outputs[1].primitives);
    assert_ne!(outputs[0].primitives, outputs[2].primitives);
    assert!(
        outputs[4]
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Image(_)))
    );
}

#[test]
fn icon_presentation_never_queues_an_action_without_activation() {
    let mut registered = IconLibrary::new();
    registered.register(icon_id(23), graphic(2.0));
    let missing = IconLibrary::new();

    let (registered_idle, registered_idle_frame) =
        fresh_frame(&registered, &UiInput::default(), false);
    let (missing_idle, missing_idle_frame) = fresh_frame(&missing, &UiInput::default(), false);
    assert!(!registered_idle.clicked && !missing_idle.clicked);
    assert_eq!(
        registered_idle_frame.platform_requests,
        missing_idle_frame.platform_requests
    );
    assert!(registered_idle_frame.platform_requests.is_empty());
    assert!(registered_idle_frame.actions.is_empty());
    assert!(missing_idle_frame.actions.is_empty());

    let (_, registered_hover_frame) =
        fresh_frame(&registered, &pointer_input(false, false, false), false);
    let (_, missing_hover_frame) =
        fresh_frame(&missing, &pointer_input(false, false, false), false);
    assert_eq!(
        registered_hover_frame.platform_requests,
        missing_hover_frame.platform_requests
    );
    assert_eq!(
        registered_hover_frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
    assert!(registered_hover_frame.actions.is_empty());
    assert!(missing_hover_frame.actions.is_empty());

    let mut pointer_memory = UiMemory::new();
    let (_, down) = render_frame(
        &registered,
        &pointer_input(true, true, false),
        &mut pointer_memory,
        false,
    );
    let (up, finished) = render_frame(
        &registered,
        &pointer_input(false, false, true),
        &mut pointer_memory,
        false,
    );
    assert!(up.clicked);
    assert!(!up.keyboard_activated);
    assert!(down.actions.is_empty() && finished.actions.is_empty());
    let fallback = pointer_cycle(&missing);
    assert_eq!(fallback.0.clicked, up.clicked);
    assert_eq!(fallback.0.keyboard_activated, up.keyboard_activated);
    assert_eq!(fallback.1.platform_requests, down.platform_requests);
    assert_eq!(fallback.2.platform_requests, finished.platform_requests);
    assert_eq!(fallback.1.actions, down.actions);
    assert_eq!(fallback.2.actions, finished.actions);

    let mut disabled_memory = UiMemory::new();
    let (_, disabled_press_frame) = render_frame(
        &registered,
        &pointer_input(true, true, false),
        &mut disabled_memory,
        true,
    );
    let (disabled, disabled_release_frame) = render_frame(
        &registered,
        &pointer_input(false, false, true),
        &mut disabled_memory,
        true,
    );
    assert!(!disabled.clicked && !disabled.keyboard_activated);
    assert!(disabled_press_frame.platform_requests.is_empty());
    assert!(disabled_release_frame.platform_requests.is_empty());
    assert!(disabled_press_frame.actions.is_empty());
    assert!(disabled_release_frame.actions.is_empty());
}
