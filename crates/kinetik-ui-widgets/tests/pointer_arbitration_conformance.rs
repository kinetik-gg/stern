//! Widget-facade pointer routing and frozen-scroll snapshot conformance.

use kinetik_ui_core::{
    Point, PointerButtonState, PointerInput, PointerOrder, PointerTarget, Rect, RepaintRequest,
    SemanticNode, SemanticRole, Size, UiInput, UiMemory, Vec2, WidgetId, inspect_primitives,
};
use kinetik_ui_widgets::{ScrollAreaOutput, Ui};

const VIEWPORT: Rect = Rect::new(0.0, 0.0, 100.0, 20.0);
const ROW_A: Rect = Rect::new(0.0, 0.0, 90.0, 20.0);
const ROW_B: Rect = Rect::new(0.0, 20.0, 80.0, 20.0);

fn ids() -> (WidgetId, WidgetId, WidgetId) {
    let root = WidgetId::from_key("root");
    let area = root.child("area");
    let content = root.child(("scroll_area_content", area.raw()));
    (area, content.child("row-a"), content.child("row-b"))
}

fn input(wheel: Vec2, pressed: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(Point::new(10.0, 10.0)),
            wheel_delta: wheel,
            primary: if pressed {
                PointerButtonState::new(false, true, true)
            } else {
                PointerButtonState::default()
            },
            ..PointerInput::default()
        },
        window_focused: true,
        ..UiInput::default()
    }
}

fn run_frame(
    memory: &mut UiMemory,
    input: &UiInput,
    reverse: bool,
) -> (
    ScrollAreaOutput<(kinetik_ui_core::Response, kinetik_ui_core::Response)>,
    kinetik_ui_core::FrameOutput,
) {
    let theme = kinetik_ui_core::default_dark_theme();
    let (area, row_a, row_b) = ids();
    let mut ui = Ui::new(input, memory, &theme);
    let frame_offset = ui.memory().scroll_offset(area);
    ui.resolve_pointer_targets(|plan| {
        plan.target(PointerTarget::wheel_only(
            area,
            VIEWPORT,
            PointerOrder::new(10),
        ));
        plan.with_clip(VIEWPORT, |plan| {
            plan.with_transform(
                kinetik_ui_core::Transform::translation(Vec2::new(0.0, -frame_offset.y)),
                |plan| {
                    if reverse {
                        plan.target(PointerTarget::new(row_b, ROW_B, PointerOrder::new(30)));
                        plan.target(PointerTarget::new(row_a, ROW_A, PointerOrder::new(20)));
                    } else {
                        plan.target(PointerTarget::new(row_a, ROW_A, PointerOrder::new(20)));
                        plan.target(PointerTarget::new(row_b, ROW_B, PointerOrder::new(30)));
                    }
                },
            );
        });
    })
    .expect("unique frozen target plan");

    let output = ui.scroll_area(
        "area",
        VIEWPORT,
        Size::new(100.0, 40.0),
        false,
        |ui, offset| {
            assert_eq!(offset, frame_offset);
            let evaluate_a = |ui: &mut Ui<'_>| {
                let response = ui.pressable("row-a", ROW_A, false);
                ui.panel_keyed("paint-a", ROW_A);
                ui.push_semantic_node(SemanticNode::new(response.id, SemanticRole::Button, ROW_A));
                response
            };
            let evaluate_b = |ui: &mut Ui<'_>| {
                let response = ui.pressable("row-b", ROW_B, false);
                ui.panel_keyed("paint-b", ROW_B);
                ui.push_semantic_node(SemanticNode::new(response.id, SemanticRole::Button, ROW_B));
                response
            };
            if reverse {
                let b = evaluate_b(ui);
                let a = evaluate_a(ui);
                (a, b)
            } else {
                let a = evaluate_a(ui);
                let b = evaluate_b(ui);
                (a, b)
            }
        },
    );
    let frame = ui.finish_output();
    (output, frame)
}

#[test]
fn wheel_stages_next_offset_while_current_frame_routing_and_geometry_stay_frozen() {
    for reverse in [false, true] {
        let (area, row_a, row_b) = ids();
        let mut memory = UiMemory::new();
        let (frame_n, output_n) =
            run_frame(&mut memory, &input(Vec2::new(0.0, -20.0), true), reverse);

        assert_eq!(frame_n.scroll.response.id, area);
        assert!(!frame_n.scroll.response.state.hovered);
        assert_eq!(frame_n.scroll.offset, Vec2::new(0.0, 20.0));
        assert_eq!(frame_n.scroll.delta, Vec2::new(0.0, 20.0));
        assert!(frame_n.inner.0.state.hovered);
        assert!(frame_n.inner.0.clicked);
        assert!(!frame_n.inner.1.state.hovered);
        assert_eq!(output_n.repaint, RepaintRequest::NextFrame);
        assert_eq!(memory.scroll_offset(area), Vec2::new(0.0, 20.0));
        let debug_n = inspect_primitives(&output_n.primitives);
        assert!(debug_n.iter().any(|item| item.bounds == Some(ROW_A)));
        assert!(
            !debug_n.iter().any(|item| {
                item.bounds == Some(Rect::new(0.0, 0.0, ROW_B.width, ROW_B.height))
            })
        );
        let semantic_a = output_n
            .semantics
            .nodes()
            .iter()
            .find(|node| node.id == row_a)
            .expect("row A semantics");
        let semantic_b = output_n
            .semantics
            .nodes()
            .iter()
            .find(|node| node.id == row_b)
            .expect("row B semantics");
        assert_eq!(semantic_a.bounds, ROW_A);
        assert_eq!(semantic_b.bounds, Rect::ZERO);

        let (frame_next, output_next) = run_frame(&mut memory, &input(Vec2::ZERO, false), !reverse);
        assert!(!frame_next.inner.0.state.hovered);
        assert!(frame_next.inner.1.state.hovered);
        assert_eq!(frame_next.scroll.delta, Vec2::ZERO);
        let row_b_screen = Rect::new(0.0, 0.0, ROW_B.width, ROW_B.height);
        let debug_next = inspect_primitives(&output_next.primitives);
        assert!(
            debug_next
                .iter()
                .any(|item| item.bounds == Some(row_b_screen))
        );
        let semantic_b = output_next
            .semantics
            .nodes()
            .iter()
            .find(|node| node.id == row_b)
            .expect("row B semantics next frame");
        assert_eq!(semantic_b.bounds, row_b_screen);
    }
}
