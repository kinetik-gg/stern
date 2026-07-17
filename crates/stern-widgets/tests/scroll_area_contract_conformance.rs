//! Public scroll-area extent, clamping, staging, and ownership conformance.

use stern_core::{
    LayoutItem, Measurement, Rect, Size, SizeRule, UiInput, UiMemory, Vec2, WidgetId,
    clamp_scroll_offset, default_dark_theme, max_scroll_offset,
};
use stern_widgets::Ui;

fn fixed_item(width: f32, height: f32) -> LayoutItem {
    LayoutItem::new(
        SizeRule::Fixed(width),
        SizeRule::Fixed(height),
        Measurement::default(),
    )
}

#[test]
fn scroll_area_exposes_exact_viewport_extent_offset_and_maximum() {
    let viewport = Rect::new(10.0, 20.0, 80.0, 50.0);
    let content = Size::new(200.0, 140.0);
    let retained = Vec2::new(35.0, 25.0);
    let area_id = WidgetId::from_key("root").child("extent-area");
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(area_id, retained);
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let area = ui.scroll_area("extent-area", viewport, content, false, |_ui, offset| {
        (offset, String::from("logical extent retained"), 7_usize)
    });
    let frame = ui.finish_output();

    assert_eq!(area.scroll.response.id, area_id);
    assert_eq!(area.scroll.response.rect, viewport);
    assert_eq!(area.scroll.offset, retained);
    assert_eq!(area.scroll.delta, Vec2::ZERO);
    assert_eq!(area.scroll.max_offset, Vec2::new(120.0, 90.0));
    assert_eq!(area.inner.0, retained);
    assert_eq!(area.inner.1, "logical extent retained");
    assert_eq!(area.inner.2, 7);
    assert_eq!(memory.scroll_offset(area_id), retained);
    assert_eq!(
        frame
            .semantics
            .get(area_id)
            .expect("scroll semantics")
            .bounds,
        viewport
    );
    assert!(frame.warnings.is_empty());
}

#[test]
fn scroll_row_and_column_expose_deterministic_axis_policy() {
    let root = WidgetId::from_key("root");
    let row_id = root.child("row-policy");
    let column_id = root.child("column-policy");
    let generic_id = root.child("two-axis-policy");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(row_id, Vec2::new(999.0, 999.0));
    memory.set_scroll_offset(column_id, Vec2::new(999.0, 999.0));
    memory.set_scroll_offset(generic_id, Vec2::new(13.0, 17.0));
    let input = UiInput::default();
    let theme = default_dark_theme();
    let items = [fixed_item(30.0, 30.0), fixed_item(30.0, 30.0)];
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let row = ui.scroll_row(
        "row-policy",
        Rect::new(0.0, 0.0, 40.0, 40.0),
        &items,
        4.0,
        false,
        |_ui, _, rect| rect,
    );
    let column = ui.scroll_column(
        "column-policy",
        Rect::new(60.0, 0.0, 40.0, 40.0),
        &items,
        4.0,
        false,
        |_ui, _, rect| rect,
    );
    let generic = ui.scroll_area(
        "two-axis-policy",
        Rect::new(120.0, 0.0, 40.0, 40.0),
        Size::new(100.0, 90.0),
        false,
        |_ui, offset| offset,
    );
    let frame = ui.finish_output();

    assert_eq!(row.scroll.offset, Vec2::new(24.0, 0.0));
    assert_eq!(row.scroll.max_offset, Vec2::new(24.0, 0.0));
    assert_eq!(row.inner[1], Rect::new(34.0, 0.0, 30.0, 30.0));
    assert_eq!(column.scroll.offset, Vec2::new(0.0, 24.0));
    assert_eq!(column.scroll.max_offset, Vec2::new(0.0, 24.0));
    assert_eq!(column.inner[1], Rect::new(60.0, 34.0, 30.0, 30.0));
    assert_eq!(generic.scroll.offset, Vec2::new(13.0, 17.0));
    assert_eq!(generic.scroll.max_offset, Vec2::new(60.0, 50.0));
    assert_eq!(generic.inner, Vec2::new(13.0, 17.0));
    assert_eq!(memory.scroll_offset(row_id), Vec2::new(24.0, 0.0));
    assert_eq!(memory.scroll_offset(column_id), Vec2::new(0.0, 24.0));
    assert_eq!(memory.scroll_offset(generic_id), Vec2::new(13.0, 17.0));
    assert!(frame.warnings.is_empty());
}

#[test]
fn scroll_clamping_sanitizes_invalid_and_oversized_state() {
    let viewport = Size::new(40.0, 30.0);
    let content = Size::new(100.0, 80.0);

    assert_eq!(
        clamp_scroll_offset(Vec2::new(-10.0, -5.0), viewport, content),
        Vec2::ZERO
    );
    assert_eq!(
        clamp_scroll_offset(Vec2::new(f32::NAN, f32::INFINITY), viewport, content,),
        Vec2::ZERO
    );
    assert_eq!(
        clamp_scroll_offset(Vec2::new(999.0, 999.0), viewport, content),
        Vec2::new(60.0, 50.0)
    );
    assert_eq!(
        max_scroll_offset(Size::ZERO, Size::new(25.0, 35.0)),
        Vec2::new(25.0, 35.0)
    );
    assert_eq!(
        max_scroll_offset(Size::new(100.0, 90.0), Size::new(20.0, 30.0)),
        Vec2::ZERO
    );
    assert_eq!(
        max_scroll_offset(
            Size::new(f32::NAN, f32::NEG_INFINITY),
            Size::new(f32::INFINITY, 50.0),
        ),
        Vec2::new(0.0, 50.0)
    );

    let root = WidgetId::from_key("root");
    let zero_id = root.child("zero-viewport");
    let smaller_id = root.child("smaller-content");
    let mut memory = UiMemory::new();
    memory.set_scroll_offset(zero_id, Vec2::new(f32::NAN, f32::INFINITY));
    memory.set_scroll_offset(smaller_id, Vec2::new(999.0, 999.0));
    let input = UiInput::default();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let zero = ui.scroll_area(
        "zero-viewport",
        Rect::new(0.0, 0.0, 0.0, 0.0),
        Size::new(25.0, 35.0),
        false,
        |_ui, offset| offset,
    );
    let smaller = ui.scroll_area(
        "smaller-content",
        Rect::new(10.0, 10.0, 100.0, 90.0),
        Size::new(20.0, 30.0),
        false,
        |_ui, offset| offset,
    );
    let frame = ui.finish_output();

    assert_eq!(zero.scroll.offset, Vec2::ZERO);
    assert_eq!(zero.scroll.max_offset, Vec2::new(25.0, 35.0));
    assert_eq!(zero.inner, Vec2::ZERO);
    assert_eq!(smaller.scroll.offset, Vec2::ZERO);
    assert_eq!(smaller.scroll.max_offset, Vec2::ZERO);
    assert_eq!(smaller.inner, Vec2::ZERO);
    assert_eq!(memory.scroll_offset(zero_id), Vec2::ZERO);
    assert_eq!(memory.scroll_offset(smaller_id), Vec2::ZERO);
    for component in [
        zero.scroll.offset.x,
        zero.scroll.offset.y,
        zero.scroll.max_offset.x,
        zero.scroll.max_offset.y,
        smaller.scroll.offset.x,
        smaller.scroll.offset.y,
    ] {
        assert!(component.is_finite());
    }
    assert!(frame.warnings.is_empty());
}
