//! Public conformance tests for deterministic text field viewport geometry.

use kinetik_ui_core::{Rect, Size, Vec2};
use kinetik_ui_text::{TextViewport, TextViewportMode};

#[test]
fn single_line_clamps_horizontal_offset_and_zeroes_vertical_offset() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 500.0),
        Vec2::new(250.0, 99.0),
    );

    assert_eq!(viewport.mode(), TextViewportMode::SingleLine);
    assert_eq!(viewport.viewport_size(), Size::new(100.0, 20.0));
    assert_eq!(viewport.content_size(), Size::new(300.0, 500.0));
    assert_eq!(viewport.max_offset(), Vec2::new(200.0, 0.0));
    assert_eq!(viewport.offset(), Vec2::new(200.0, 0.0));
}

#[test]
fn wrapped_multi_line_clamps_vertical_offset_and_zeroes_horizontal_offset() {
    let viewport = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 500.0),
        Vec2::new(99.0, 900.0),
    );

    assert_eq!(viewport.mode(), TextViewportMode::WrappedMultiLine);
    assert_eq!(viewport.max_offset(), Vec2::new(0.0, 480.0));
    assert_eq!(viewport.offset(), Vec2::new(0.0, 480.0));
}

#[test]
fn constructor_sanitizes_negative_and_non_finite_geometry() {
    let single_line = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(-10.0, f32::NAN),
        Size::new(f32::INFINITY, 200.0),
        Vec2::new(f32::NAN, f32::NEG_INFINITY),
    );

    assert_eq!(single_line.viewport_size(), Size::ZERO);
    assert_eq!(single_line.content_size(), Size::new(0.0, 200.0));
    assert_eq!(single_line.max_offset(), Vec2::ZERO);
    assert_eq!(single_line.offset(), Vec2::ZERO);

    let wrapped = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(f32::NEG_INFINITY, 40.0),
        Size::new(f32::MAX, -5.0),
        Vec2::new(f32::INFINITY, -20.0),
    );

    assert_eq!(wrapped.viewport_size(), Size::new(0.0, 40.0));
    assert_eq!(wrapped.content_size(), Size::new(f32::MAX, 0.0));
    assert_eq!(wrapped.max_offset(), Vec2::ZERO);
    assert_eq!(wrapped.offset(), Vec2::ZERO);

    let infinite_offset = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(20.0, 20.0),
        Size::new(100.0, 20.0),
        Vec2::new(f32::INFINITY, 0.0),
    );

    assert_eq!(infinite_offset.offset(), Vec2::ZERO);
}

#[test]
fn scroll_by_sanitizes_components_and_clamps_both_horizontal_edges() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 20.0),
        Vec2::new(50.0, 8.0),
    );

    assert_eq!(
        viewport.scroll_by(Vec2::new(40.0, 300.0)),
        Vec2::new(90.0, 0.0)
    );
    assert_eq!(viewport.scroll_by(Vec2::new(-100.0, 0.0)), Vec2::ZERO);
    assert_eq!(
        viewport.scroll_by(Vec2::new(f32::MAX, 0.0)),
        Vec2::new(200.0, 0.0)
    );
    assert_eq!(
        viewport.scroll_by(Vec2::new(f32::NAN, f32::INFINITY)),
        Vec2::new(50.0, 0.0)
    );
}

#[test]
fn scroll_by_sanitizes_components_and_clamps_both_vertical_edges() {
    let viewport = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(100.0, 80.0),
        Size::new(100.0, 300.0),
        Vec2::new(8.0, 100.0),
    );

    assert_eq!(
        viewport.scroll_by(Vec2::new(300.0, 50.0)),
        Vec2::new(0.0, 150.0)
    );
    assert_eq!(viewport.scroll_by(Vec2::new(0.0, -150.0)), Vec2::ZERO);
    assert_eq!(
        viewport.scroll_by(Vec2::new(0.0, f32::INFINITY)),
        Vec2::new(0.0, 100.0)
    );
    assert_eq!(
        viewport.scroll_by(Vec2::new(f32::NEG_INFINITY, f32::MAX)),
        Vec2::new(0.0, 220.0)
    );
}

#[test]
fn reveal_preserves_visible_targets_and_inclusive_zero_size_edges() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 20.0),
        Vec2::new(50.0, 0.0),
    );

    assert_eq!(
        viewport.reveal(Rect::new(75.0, 0.0, 25.0, 10.0)),
        Vec2::new(50.0, 0.0)
    );
    assert_eq!(
        viewport.reveal(Rect::new(50.0, 0.0, 0.0, 0.0)),
        Vec2::new(50.0, 0.0)
    );
    assert_eq!(
        viewport.reveal(Rect::new(150.0, 20.0, 0.0, 0.0)),
        Vec2::new(50.0, 0.0)
    );
}

#[test]
fn reveal_moves_minimally_to_horizontal_leading_and_trailing_edges() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 20.0),
        Vec2::new(80.0, 0.0),
    );

    assert_eq!(
        viewport.reveal(Rect::new(60.0, 0.0, 10.0, 10.0)),
        Vec2::new(60.0, 0.0)
    );
    assert_eq!(
        viewport.reveal(Rect::new(170.0, 0.0, 30.0, 10.0)),
        Vec2::new(100.0, 0.0)
    );
}

#[test]
fn reveal_moves_minimally_to_vertical_leading_and_trailing_edges() {
    let viewport = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(100.0, 100.0),
        Size::new(100.0, 400.0),
        Vec2::new(0.0, 120.0),
    );

    assert_eq!(
        viewport.reveal(Rect::new(0.0, 90.0, 10.0, 10.0)),
        Vec2::new(0.0, 90.0)
    );
    assert_eq!(
        viewport.reveal(Rect::new(0.0, 210.0, 10.0, 40.0)),
        Vec2::new(0.0, 150.0)
    );
}

#[test]
fn oversized_targets_align_the_leading_edge_then_clamp() {
    let horizontal = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 20.0),
        Vec2::new(75.0, 0.0),
    );

    assert_eq!(
        horizontal.reveal(Rect::new(40.0, 0.0, 150.0, 10.0)),
        Vec2::new(40.0, 0.0)
    );
    assert_eq!(
        horizontal.reveal(Rect::new(-30.0, 0.0, 150.0, 10.0)),
        Vec2::ZERO
    );
    assert_eq!(
        horizontal.reveal(Rect::new(250.0, 0.0, 150.0, 10.0)),
        Vec2::new(200.0, 0.0)
    );

    let vertical = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(20.0, 100.0),
        Size::new(20.0, 300.0),
        Vec2::new(0.0, 75.0),
    );

    assert_eq!(
        vertical.reveal(Rect::new(0.0, 40.0, 10.0, 150.0)),
        Vec2::new(0.0, 40.0)
    );
    assert_eq!(
        vertical.reveal(Rect::new(0.0, 250.0, 10.0, 150.0)),
        Vec2::new(0.0, 200.0)
    );
}

#[test]
fn invalid_targets_preserve_the_current_offset() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 20.0),
        Vec2::new(80.0, 0.0),
    );
    let invalid_targets = [
        Rect::new(f32::NAN, 0.0, 10.0, 10.0),
        Rect::new(0.0, f32::INFINITY, 10.0, 10.0),
        Rect::new(0.0, 0.0, f32::NEG_INFINITY, 10.0),
        Rect::new(0.0, 0.0, 10.0, f32::NAN),
        Rect::new(0.0, 0.0, -1.0, 10.0),
        Rect::new(0.0, 0.0, 10.0, -1.0),
    ];

    for target in invalid_targets {
        assert_eq!(viewport.reveal(target), Vec2::new(80.0, 0.0));
    }
}

#[test]
fn zero_viewports_align_positive_targets_and_zero_content_cannot_scroll() {
    let horizontal = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::ZERO,
        Size::new(100.0, 0.0),
        Vec2::ZERO,
    );

    assert_eq!(
        horizontal.reveal(Rect::new(40.0, 0.0, 10.0, 0.0)),
        Vec2::new(40.0, 0.0)
    );
    assert_eq!(
        horizontal.reveal(Rect::new(100.0, 0.0, 0.0, 0.0)),
        Vec2::new(100.0, 0.0)
    );

    let no_content = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(100.0, 100.0),
        Size::ZERO,
        Vec2::new(50.0, 50.0),
    );

    assert_eq!(no_content.offset(), Vec2::ZERO);
    assert_eq!(no_content.scroll_by(Vec2::new(0.0, 50.0)), Vec2::ZERO);
    assert_eq!(
        no_content.reveal(Rect::new(0.0, 300.0, 0.0, 20.0)),
        Vec2::ZERO
    );
}

#[test]
fn exact_content_boundaries_clamp_without_cross_axis_motion() {
    let horizontal = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(100.0, 20.0),
        Size::new(300.0, 1_000.0),
        Vec2::ZERO,
    );

    assert_eq!(
        horizontal.reveal(Rect::new(290.0, 900.0, 10.0, 100.0)),
        Vec2::new(200.0, 0.0)
    );

    let vertical = TextViewport::new(
        TextViewportMode::WrappedMultiLine,
        Size::new(100.0, 100.0),
        Size::new(1_000.0, 300.0),
        Vec2::ZERO,
    );

    assert_eq!(
        vertical.reveal(Rect::new(900.0, 290.0, 100.0, 10.0)),
        Vec2::new(0.0, 200.0)
    );
}

#[test]
fn finite_target_arithmetic_overflow_clamps_to_the_content_edge() {
    let viewport_extent = f32::MAX / 2.0;
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::new(viewport_extent, 20.0),
        Size::new(f32::MAX, 20.0),
        Vec2::ZERO,
    );
    let target = Rect::new(f32::MAX, 0.0, viewport_extent, 0.0);

    assert!((target.x + target.width).is_infinite());
    assert_eq!(viewport.reveal(target), viewport.max_offset());
}

#[test]
fn finite_scroll_arithmetic_overflow_clamps_to_the_content_edge() {
    let viewport = TextViewport::new(
        TextViewportMode::SingleLine,
        Size::ZERO,
        Size::new(f32::MAX, 0.0),
        Vec2::new(f32::MAX, 0.0),
    );
    let delta = Vec2::new(f32::MAX, 0.0);

    assert!(viewport.offset().x.is_finite());
    assert!(delta.x.is_finite());
    assert!((viewport.offset().x + delta.x).is_infinite());
    assert_eq!(viewport.scroll_by(delta), viewport.max_offset());
}
