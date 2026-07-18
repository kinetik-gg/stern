#[allow(unused_imports)]
use super::{
    ActionContext, ActionDescriptor, ActionSource, Brush, Color, CursorShape, FrameContext,
    FrameOutput, FrameWarning, IconId, ImageId, Insets, Key, Modifiers, PhysicalSize,
    PlatformRequest, Point, PointerButtonState, Primitive, Rect, RepaintRequest, ScaleFactor,
    SemanticNode, SemanticRole, Size, TextEditState, TextInputEvent, TextLayoutKey,
    TextLayoutStore, TextPrimitive, TextRange, TextStyle, TimeInfo, Ui, UiInput, UiMemory, Vec2,
    ViewportInfo, WidgetId, committed_text, default_dark_theme, frame_context, frame_context_at,
    held_at, input_at, pressed_at, pressed_key, released_at, scrolled_at, text_field_has_caret,
};

#[test]
fn ui_exposes_neutral_behavior_primitives() {
    let theme = default_dark_theme();
    let input = pressed_at(4.0, 4.0);

    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let pressed = ui.pressable("pressable", Rect::new(0.0, 0.0, 20.0, 20.0), false);
    assert!(pressed.state.pressed);
    assert!(ui.finish_output().primitives.is_empty());

    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let selected = ui.selectable("selectable", Rect::new(0.0, 0.0, 20.0, 20.0), true, false);
    assert!(selected.state.selected);
    assert!(ui.finish_output().primitives.is_empty());

    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let dragged = ui.draggable("draggable", Rect::new(0.0, 0.0, 20.0, 20.0), false);
    assert!(dragged.state.active);
    assert!(ui.finish_output().primitives.is_empty());
}

#[test]
fn ui_pressable_with_id_supports_custom_semantics_without_duplicate_warning() {
    let theme = default_dark_theme();
    let input = pressed_at(4.0, 4.0);
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let id = ui.id("custom-pressable");

    let pressed = ui.pressable_with_id(id, Rect::new(0.0, 0.0, 20.0, 20.0), false);
    ui.push_semantic_node(SemanticNode::new(
        id,
        SemanticRole::IconButton,
        Rect::new(0.0, 0.0, 20.0, 20.0),
    ));
    let output = ui.finish_output();

    assert!(pressed.state.pressed);
    assert!(output.warnings.is_empty());
    assert_eq!(output.semantics.nodes()[0].id, id);
}

#[test]
fn ui_exposes_overlay_and_drop_behavior_primitives() {
    let theme = default_dark_theme();
    let source = WidgetId::from_key("source");
    let mut memory = UiMemory::new();
    memory.start_drag(source);
    memory.press_secondary(WidgetId::from_key("root").child("context"));
    let mut input = released_at(4.0, 4.0);
    input.pointer.secondary = PointerButtonState::new(false, false, true);
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let context = ui.context_menu_trigger("context", Rect::new(0.0, 0.0, 20.0, 20.0), false);
    let tooltip = ui.tooltip_trigger("tooltip", Rect::new(0.0, 0.0, 20.0, 20.0), false);
    let drop = ui.drop_target("drop", Rect::new(0.0, 0.0, 20.0, 20.0), false);

    assert!(context.context_requested);
    assert!(tooltip.tooltip_requested);
    assert_eq!(drop.source, Some(source));
    assert!(drop.dropped);
}

#[test]
fn ui_finish_output_preserves_core_runtime_warnings() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let id = ui.id("duplicate");
    ui.id("duplicate");
    let output = ui.finish_output();

    assert_eq!(
        output.warnings,
        vec![FrameWarning::DuplicateWidgetId { id }]
    );
}

#[test]
fn ui_finish_output_preserves_widget_semantics() {
    let theme = default_dark_theme();
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let mut state = TextEditState::new("abc");
    let mut ui = Ui::new(&input, &mut memory, &theme);

    ui.label(Rect::new(0.0, 0.0, 80.0, 18.0), "Label");
    ui.button("run", Rect::new(0.0, 24.0, 80.0, 28.0), "Run", false);
    ui.text_field(
        "field",
        Rect::new(0.0, 60.0, 120.0, 24.0),
        &mut state,
        false,
    );
    let output = ui.finish_output();

    let roles = output
        .semantics
        .nodes()
        .iter()
        .map(|node| node.role.clone())
        .collect::<Vec<_>>();
    assert!(roles.contains(&SemanticRole::Label));
    assert!(roles.contains(&SemanticRole::Button));
    assert!(roles.contains(&SemanticRole::TextField));
    assert_eq!(output.semantics.focus_order().len(), 2);
}
