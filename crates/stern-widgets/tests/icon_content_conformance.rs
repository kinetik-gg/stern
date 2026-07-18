//! Direct borrowed static-icon geometry, identity, and accessibility evidence.

use stern_core::{
    Alignment, ComponentState, CursorShape, FrameOutput, ImageId, PlatformRequest,
    PointerButtonState, PointerInput, Primitive, Rect, Response, SemanticActionKind, Size,
    StaticIcon, UiInput, UiMemory, WidgetId, default_dark_theme, fit_box,
};
use stern_widgets::{Ui, WidgetOutput, icon_button, image_icon_button};

const OUTER: Rect = Rect::new(12.25, 20.5, 40.0, 32.0);
const LABEL: &str = "Open inspector";

fn widget_id() -> WidgetId {
    WidgetId::from_key("bounded-icon")
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

fn render_vector(
    icon: StaticIcon,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> WidgetOutput {
    icon_button(
        widget_id(),
        OUTER,
        icon,
        LABEL,
        input,
        memory,
        &default_dark_theme(),
        disabled,
    )
}

fn fresh_vector(icon: StaticIcon, input: &UiInput, disabled: bool) -> WidgetOutput {
    render_vector(icon, input, &mut UiMemory::new(), disabled)
}

fn render_frame(
    icon: impl Into<StaticIcon>,
    input: &UiInput,
    memory: &mut UiMemory,
    disabled: bool,
) -> (Response, FrameOutput) {
    let theme = default_dark_theme();
    let mut ui = Ui::new(input, memory, &theme);
    let response = ui.icon_button("frame-icon", OUTER, icon, LABEL, disabled);
    (response, ui.finish_output())
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

fn icon_primitive(output: &WidgetOutput) -> stern_core::IconPrimitive {
    output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Icon(icon) => Some(*icon),
            _ => None,
        })
        .expect("direct static icon primitive")
}

#[test]
fn phosphor_constant_converts_directly_and_preserves_borrowed_identity_and_geometry() {
    let source = stern_icons_phosphor::duotone::CUBE;
    let icon = source.icon();
    let output = fresh_vector(icon, &UiInput::default(), false);

    assert_outer_contract(&output);
    assert_eq!(output.primitives.len(), 2);
    let primitive = icon_primitive(&output);
    assert_eq!(primitive.icon.id(), icon.id());
    assert!(core::ptr::eq(primitive.icon.graphic(), icon.graphic()));
    assert_eq!(primitive.rect, optical_box());
    assert_eq!(
        primitive.tint,
        default_dark_theme()
            .button(ComponentState::default())
            .foreground
    );
    assert!(
        primitive
            .icon
            .graphic()
            .layers
            .iter()
            .any(|layer| layer.opacity < 1.0)
    );

    let generic_output = icon_button(
        WidgetId::from_key("phosphor-generic"),
        OUTER,
        source,
        "Generic conversion",
        &UiInput::default(),
        &mut UiMemory::new(),
        &default_dark_theme(),
        false,
    );
    assert_eq!(icon_primitive(&generic_output).icon, source.icon());
}

#[test]
fn icon_rect_and_accessibility_survive_idle_hover_press_and_disabled_states() {
    let icon = stern_icons_phosphor::regular::CHECK.icon();
    let idle = fresh_vector(icon, &UiInput::default(), false);
    let hovered = fresh_vector(icon, &pointer_input(false, false, false), false);
    let pressed = fresh_vector(icon, &pointer_input(true, true, false), false);
    let disabled = fresh_vector(icon, &pointer_input(true, true, false), true);

    for output in [&idle, &hovered, &pressed, &disabled] {
        assert_outer_contract(output);
        let primitive = icon_primitive(output);
        assert_eq!(primitive.icon, icon);
        assert_eq!(primitive.rect, optical_box());
        assert!(core::ptr::eq(primitive.icon.graphic(), icon.graphic()));
    }
    assert!(!idle.response.expect("idle response").state.hovered);
    assert!(hovered.response.expect("hover response").state.hovered);
    assert!(pressed.response.expect("pressed response").state.pressed);

    let disabled_response = disabled.response.expect("disabled response");
    assert!(disabled_response.state.disabled);
    assert!(!disabled_response.clicked);
    let disabled_semantic = &disabled.semantics[0];
    assert!(disabled_semantic.state.disabled);
    assert!(!disabled_semantic.focusable);
}

#[test]
fn accessible_name_is_independent_of_static_or_bitmap_icon_content() {
    let outputs = [
        fresh_vector(
            stern_icons_phosphor::regular::CHECK.icon(),
            &UiInput::default(),
            false,
        ),
        fresh_vector(
            stern_icons_phosphor::regular::FLOPPY_DISK.icon(),
            &UiInput::default(),
            false,
        ),
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
        assert!(
            output.semantics[0]
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
        );
    }
    assert_ne!(outputs[0].primitives, outputs[1].primitives);
    assert!(
        outputs[2]
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Image(_)))
    );
}

#[test]
fn icon_presentation_queues_no_action_and_activation_behavior_is_unchanged() {
    let icon = stern_icons_phosphor::regular::CHECK;
    let mut memory = UiMemory::new();
    let (idle, idle_frame) = render_frame(icon, &UiInput::default(), &mut memory, false);
    assert!(!idle.clicked);
    assert!(idle_frame.actions.is_empty());
    assert!(idle_frame.platform_requests.is_empty());

    let (_, hover_frame) = render_frame(
        icon,
        &pointer_input(false, false, false),
        &mut memory,
        false,
    );
    assert_eq!(
        hover_frame.platform_requests,
        vec![PlatformRequest::SetCursor(CursorShape::PointingHand)]
    );
    assert!(hover_frame.actions.is_empty());

    let (_, down) = render_frame(icon, &pointer_input(true, true, false), &mut memory, false);
    let (up, finished) = render_frame(icon, &pointer_input(false, false, true), &mut memory, false);
    assert!(up.clicked);
    assert!(down.actions.is_empty() && finished.actions.is_empty());
}
