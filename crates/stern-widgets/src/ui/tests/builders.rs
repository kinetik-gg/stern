//! RFC 0001 Phase L1 seam tests: builder measurement, the legacy-path guard,
//! and geometry-before-behavior under the pointer plan.

#![allow(clippy::float_cmp)]

use stern_core::{
    PointerOrder, PointerTarget, Rect, Size, SizeRule, TextRole, UiInput, UiMemory,
    default_dark_theme,
};
use stern_text::{TextLayoutKey, TextLayoutStore, TextStyle};

use super::super::builders::{
    Button, Checkbox, IconButton, Label, MeasureContext, RadioButton, Slider, Toggle, Widget,
};
use super::{Ui, pressed_at};

static ICON_GRAPHIC: stern_core::IconGraphic =
    stern_core::IconGraphic::new(Rect::new(0.0, 0.0, 16.0, 16.0), &[]);

fn test_icon() -> stern_core::StaticIcon {
    stern_core::StaticIcon::new(stern_core::IconId::from_raw(1), &ICON_GRAPHIC)
}

fn shaped_width(store: &mut TextLayoutStore, text: &str, role: TextRole) -> f32 {
    let theme = default_dark_theme();
    let font = theme.font(role);
    store
        .shape_transient(&TextLayoutKey::new(
            text,
            TextStyle::new(font.family.to_owned(), font.size, font.line_height),
            f32::MAX,
            false,
        ))
        .size
        .width
}

#[test]
fn button_measures_padding_plus_shaped_label() {
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let expected_width = shaped_width(&mut store, "Analyze", TextRole::Label);
    assert!(expected_width > 0.0, "bundled fonts must shape the label");

    let mut ctx = MeasureContext::new(&theme, Some(&mut store));
    let measured = Button::new("k", "Analyze").measure(&mut ctx);

    assert_eq!(
        measured.desired,
        Size::new(
            theme.controls.padding_x * 2.0 + expected_width,
            theme.controls.control_height,
        )
    );
}

#[test]
fn control_builders_measure_theme_metrics() {
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let mut ctx = MeasureContext::new(&theme, Some(&mut store));

    let check_side = theme.checkbox(stern_core::ComponentState::default()).size;
    assert_eq!(
        Checkbox::new("c", "Check", false).measure(&mut ctx).desired,
        Size::new(check_side, check_side)
    );
    let radio_side = theme
        .radio_button(stern_core::ComponentState::default())
        .size;
    assert_eq!(
        RadioButton::new("r", "Radio", false)
            .measure(&mut ctx)
            .desired,
        Size::new(radio_side, radio_side)
    );
    assert_eq!(
        Toggle::new("t", "Toggle", false).measure(&mut ctx).desired,
        Size::new(26.0, 14.0)
    );
    let side = theme.controls.control_height;
    assert_eq!(
        IconButton::new("i", test_icon(), "Icon")
            .measure(&mut ctx)
            .desired,
        Size::new(side, side)
    );
    let mut value = 0.5;
    let slider = Slider::new("s", "Slider", &mut value, 0.0..=1.0);
    assert_eq!(
        slider.size_rules(),
        (SizeRule::Fill, SizeRule::Fit),
        "sliders have no intrinsic width and default to Fill"
    );
    assert_eq!(
        slider.measure(&mut ctx).desired.height,
        theme.sizes.control.md
    );

    let label_size = Label::new("Body text").measure(&mut ctx).desired;
    assert!(label_size.width > 0.0 && label_size.height > 0.0);
}

#[test]
fn text_measurement_without_store_is_zero() {
    let theme = default_dark_theme();
    let mut ctx = MeasureContext::new(&theme, None);
    let measured = Button::new("k", "Analyze").measure(&mut ctx);
    assert_eq!(
        measured.desired,
        Size::new(
            theme.controls.padding_x * 2.0,
            theme.controls.control_height
        )
    );
}

#[test]
fn builder_path_emits_identical_output_to_rect_path() {
    let theme = default_dark_theme();
    let rect = Rect::new(12.0, 8.0, 96.0, 24.0);

    let input = UiInput::default();
    let mut legacy_memory = UiMemory::new();
    let mut legacy_ui = Ui::new(&input, &mut legacy_memory, &theme);
    let _ = legacy_ui.button("guard", rect, "Analyze", false);
    let legacy = legacy_ui.finish_output();

    let mut builder_memory = UiMemory::new();
    let mut builder_ui = Ui::new(&input, &mut builder_memory, &theme);
    let mut slot = None;
    let layout = builder_ui.layout(rect, |l| {
        slot = Some(l.add_sized(
            Button::new("guard", "Analyze"),
            SizeRule::Fixed(rect.width),
            SizeRule::Fixed(rect.height),
        ));
    });
    let composed_rect = layout.rect(slot.expect("slot was added"));
    assert_eq!(
        composed_rect, rect,
        "layout must solve the exact legacy rect"
    );
    let _ = layout.compose(&mut builder_ui);
    let built = builder_ui.finish_output();

    assert_eq!(built.primitives, legacy.primitives);
    assert_eq!(built.semantics, legacy.semantics);
}

#[test]
fn layout_geometry_is_final_before_compose_and_routes_the_pointer_plan() {
    let theme = default_dark_theme();
    let mut store = TextLayoutStore::new();
    let first_width =
        theme.controls.padding_x * 2.0 + shaped_width(&mut store, "First", TextRole::Label);

    let bounds = Rect::new(0.0, 0.0, 400.0, 32.0);
    let gap = 8.0;
    let click = pressed_at(4.0, 4.0);
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&click, &mut memory, &theme).with_text_layouts(&mut store);

    let mut first = None;
    let mut second = None;
    let layout = ui.layout(bounds, |l| {
        l.row(SizeRule::Fill, SizeRule::Fit, gap, |l| {
            first = Some(l.add(Button::new("first", "First")));
            second = Some(l.add(Button::new("second", "Second")));
        });
    });
    let (first, second) = (first.unwrap(), second.unwrap());

    let first_rect = layout.rect(first);
    let second_rect = layout.rect(second);
    assert_eq!(
        first_rect,
        Rect::new(0.0, 0.0, first_width, theme.controls.control_height),
        "buttons must be content-sized, not caller-sized"
    );
    assert_eq!(second_rect.x, first_rect.max_x() + gap);

    let first_id = ui.make_id("first");
    let second_id = ui.make_id("second");
    ui.resolve_pointer_targets(|plan| {
        plan.target(PointerTarget::new(
            first_id,
            first_rect,
            PointerOrder::new(0),
        ));
        plan.target(PointerTarget::new(
            second_id,
            second_rect,
            PointerOrder::new(1),
        ));
    })
    .expect("plan installs before any behavior ran");

    let responses = layout.compose(&mut ui);
    assert!(
        responses.response(first).is_some_and(|r| r.state.pressed),
        "click inside the first solved rect presses the first button"
    );
    assert!(
        responses
            .response(second)
            .is_none_or(|r| !r.state.pressed && !r.clicked),
        "the second button stays unpressed"
    );
}
