//! Frame-cost probe for scoped input localization (KNOWN-GAPS #5, issue #945).
//!
//! Composes deterministic synthetic scenes (many primitives, several input
//! events per frame) through the real `Ui::begin_frame` .. `end_frame` path
//! and reports the average compose cost per frame. Run in release mode:
//!
//! ```text
//! cargo run --release -p stern-core --example input_localization_bench
//! ```
//!
//! Absolute numbers vary by machine; compare the before/after ratio on the
//! same machine.

// Scene coordinates come from small loop indices; f32 precision is ample.
#![allow(clippy::cast_precision_loss)]

use std::hint::black_box;
use std::time::{Duration, Instant};

use stern_core::{
    Brush, ClipId, Color, CornerRadius, FrameContext, InputWheelDelta, MouseButton, PhysicalSize,
    Point, Primitive, Rect, RectPrimitive, ScaleFactor, Size, TimeInfo, Transform, Ui, UiInput,
    UiInputEvent, UiMemory, Vec2, ViewportInfo, WidgetId,
};

const WARMUP_FRAMES: usize = 10;
const MEASURED_FRAMES: usize = 100;

type SceneFn = fn(&mut Ui<'_>);

fn main() {
    let scenes: [(&str, SceneFn); 3] = [
        ("flat 10k rects", compose_flat),
        ("100 clips x 100 rects", compose_clipped_groups),
        ("nested transform+clip", compose_nested_scopes),
    ];

    println!(
        "input localization frame cost ({MEASURED_FRAMES} measured frames, {} events/frame)",
        frame_input().events.len()
    );
    for (name, compose) in scenes {
        let (primitives, events, elapsed) = run_scene(compose);
        let per_frame = elapsed / u32::try_from(MEASURED_FRAMES).expect("frame count fits u32");
        println!(
            "  {name:<24} primitives {primitives:>6}  localized events {events}  \
             avg {per_frame:>10.3?}/frame"
        );
    }
}

fn run_scene(compose: SceneFn) -> (usize, usize, Duration) {
    let mut memory = UiMemory::new();
    let mut primitives = 0;
    for _ in 0..WARMUP_FRAMES {
        primitives = compose_frame(&mut memory, compose);
    }
    let events = frame_input().events.len();
    let start = Instant::now();
    for _ in 0..MEASURED_FRAMES {
        black_box(compose_frame(&mut memory, compose));
    }
    (primitives, events, start.elapsed())
}

fn compose_frame(memory: &mut UiMemory, compose: SceneFn) -> usize {
    let context = FrameContext::new(
        ViewportInfo::new(
            Size::new(1920.0, 1080.0),
            PhysicalSize::new(1920, 1080),
            ScaleFactor::ONE,
        ),
        frame_input(),
        TimeInfo::default(),
    );
    let mut ui = Ui::begin_frame(context, memory);
    ui.set_semantic_root(WidgetId::from_key("bench-root"));
    compose(&mut ui);
    let output = ui.end_frame();
    assert!(output.warnings.is_empty(), "bench scenes must stay valid");
    black_box(&output);
    output.primitives.len()
}

/// Eight canonical pointer events: moves, a primary click, a secondary click,
/// and a wheel tick, all at finite in-viewport positions.
fn frame_input() -> UiInput {
    let mut input = UiInput::default();
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(398.0, 298.0),
        delta: Vec2::new(3.0, 2.0),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: true,
        click_count: 1,
        position: Some(Point::new(398.0, 298.0)),
    });
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(400.0, 300.0),
        delta: Vec2::new(2.0, 2.0),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Primary,
        down: false,
        click_count: 1,
        position: Some(Point::new(400.0, 300.0)),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Secondary,
        down: true,
        click_count: 1,
        position: Some(Point::new(400.0, 300.0)),
    });
    input.push_event(UiInputEvent::PointerButton {
        button: MouseButton::Secondary,
        down: false,
        click_count: 1,
        position: Some(Point::new(400.0, 300.0)),
    });
    input.push_event(UiInputEvent::Wheel {
        delta: InputWheelDelta::Pixels(Vec2::new(0.0, -24.0)),
        position: Some(Point::new(400.0, 300.0)),
    });
    input.push_event(UiInputEvent::PointerMoved {
        position: Point::new(401.0, 302.0),
        delta: Vec2::new(1.0, 2.0),
    });
    input
}

fn rect_at(index: usize) -> Primitive {
    let column = index % 100;
    let row = index / 100;
    Primitive::Rect(RectPrimitive {
        rect: Rect::new((column * 19) as f32, (row * 10) as f32 % 1080.0, 18.0, 9.0),
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(2.0),
    })
}

/// 10,000 draw primitives at root scope: the pure early-out case.
fn compose_flat(ui: &mut Ui<'_>) {
    for index in 0..10_000 {
        ui.push_primitive(rect_at(index));
    }
}

/// 100 sibling clip scopes with 100 rects each: scope re-entry dominates.
fn compose_clipped_groups(ui: &mut Ui<'_>) {
    for group in 0..100usize {
        let clip = ClipId::from_raw(u64::try_from(group).expect("group fits u64") + 1);
        let origin_x = (group % 10 * 192) as f32;
        let origin_y = (group / 10 * 108) as f32;
        ui.push_primitive(Primitive::ClipBegin {
            id: clip,
            rect: Rect::new(origin_x, origin_y, 192.0, 108.0),
        });
        for index in 0..100 {
            ui.push_primitive(rect_at(group * 100 + index));
        }
        ui.push_primitive(Primitive::ClipEnd { id: clip });
    }
}

/// 50 transform scopes each holding 4 nested clip scopes of 48 rects.
fn compose_nested_scopes(ui: &mut Ui<'_>) {
    let mut clip_serial = 1_000u64;
    for group in 0..50usize {
        let offset = Vec2::new((group % 10 * 192) as f32, (group / 10 * 216) as f32);
        ui.push_primitive(Primitive::TransformBegin(Transform::translation(offset)));
        for inner in 0..4usize {
            clip_serial += 1;
            let clip = ClipId::from_raw(clip_serial);
            ui.push_primitive(Primitive::ClipBegin {
                id: clip,
                rect: Rect::new(0.0, (inner * 54) as f32, 192.0, 54.0),
            });
            for index in 0..48 {
                ui.push_primitive(rect_at(group * 192 + inner * 48 + index));
            }
            ui.push_primitive(Primitive::ClipEnd { id: clip });
        }
        ui.push_primitive(Primitive::TransformEnd);
    }
}
