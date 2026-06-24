//! Fixed-extent collection virtualization conformance tests.

mod collections_virtualization_conformance {
    use std::ops::Range;

    use kinetik_ui_widgets::{
        VirtualRangeRequest, VirtualWindow, VirtualWindowRequest, virtual_range, virtual_window,
    };

    fn request(
        item_count: usize,
        scroll_offset: f32,
        viewport_extent: f32,
        item_extent: f32,
        overscan: usize,
    ) -> VirtualWindowRequest {
        VirtualWindowRequest {
            item_count,
            scroll_offset,
            viewport_extent,
            item_extent,
            overscan,
        }
    }

    fn assert_window_finite(window: &VirtualWindow) {
        assert!(window.content_extent.is_finite(), "{window:?}");
        assert!(window.max_scroll_offset.is_finite(), "{window:?}");
        assert!(window.clamped_scroll_offset.is_finite(), "{window:?}");
        assert!(window.content_extent >= 0.0, "{window:?}");
        assert!(window.max_scroll_offset >= 0.0, "{window:?}");
        assert!(window.clamped_scroll_offset >= 0.0, "{window:?}");
    }

    fn assert_approx(actual: f32, expected: f32) {
        assert!(
            (actual - expected).abs() < f32::EPSILON,
            "expected {actual} to equal {expected}"
        );
    }

    fn assert_empty_finite_window(window: &VirtualWindow) {
        assert_window_finite(window);
        assert_approx(window.content_extent, 0.0);
        assert_approx(window.max_scroll_offset, 0.0);
        assert_approx(window.clamped_scroll_offset, 0.0);
        assert_eq!(window.visible_range, 0..0);
        assert_eq!(window.materialized_range, 0..0);
    }

    fn assert_range(range: Range<usize>, expected: Range<usize>) {
        assert_eq!(range, expected);
    }

    #[test]
    fn virtual_window_distinguishes_visible_and_materialized_ranges() {
        let window = virtual_window(request(100, 50.0, 35.0, 10.0, 2));

        assert_window_finite(&window);
        assert_approx(window.content_extent, 1000.0);
        assert_approx(window.max_scroll_offset, 965.0);
        assert_approx(window.clamped_scroll_offset, 50.0);
        assert_range(window.visible_range, 5..9);
        assert_range(window.materialized_range, 3..12);
    }

    #[test]
    fn virtual_window_clamps_negative_and_overscrolled_offsets() {
        let negative = virtual_window(request(10, -25.0, 25.0, 10.0, 1));
        assert_approx(negative.clamped_scroll_offset, 0.0);
        assert_range(negative.visible_range, 0..3);
        assert_range(negative.materialized_range, 0..5);

        let overscrolled = virtual_window(request(10, 500.0, 30.0, 10.0, 1));
        assert_approx(overscrolled.max_scroll_offset, 70.0);
        assert_approx(overscrolled.clamped_scroll_offset, 70.0);
        assert_range(overscrolled.visible_range, 7..10);
        assert_range(overscrolled.materialized_range, 6..10);
    }

    #[test]
    fn virtual_window_handles_empty_invalid_and_nonfinite_inputs() {
        assert_empty_finite_window(&virtual_window(request(0, 0.0, 100.0, 20.0, 1)));

        for item_extent in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_empty_finite_window(&virtual_window(request(10, 0.0, 100.0, item_extent, 1)));
        }

        for viewport_extent in [0.0, -1.0, f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            assert_empty_finite_window(&virtual_window(request(10, 0.0, viewport_extent, 20.0, 1)));
        }

        for scroll_offset in [f32::NAN, f32::INFINITY, f32::NEG_INFINITY] {
            let window = virtual_window(request(10, scroll_offset, 25.0, 10.0, 0));
            assert_window_finite(&window);
            assert_approx(window.clamped_scroll_offset, 0.0);
            assert_range(window.visible_range, 0..3);
            assert_range(window.materialized_range, 0..4);
        }
    }

    #[test]
    fn virtual_window_handles_exact_row_boundaries_and_partial_rows() {
        let exact = virtual_window(request(20, 10.0, 30.0, 10.0, 0));
        assert_range(exact.visible_range, 1..4);
        assert_range(exact.materialized_range, 1..5);

        let partial_scroll = virtual_window(request(20, 15.0, 30.0, 10.0, 0));
        assert_range(partial_scroll.visible_range, 1..5);
        assert_range(partial_scroll.materialized_range, 1..5);

        let partial_viewport = virtual_window(request(20, 0.0, 35.0, 10.0, 0));
        assert_range(partial_viewport.visible_range, 0..4);
        assert_range(partial_viewport.materialized_range, 0..5);
    }

    #[test]
    fn virtual_window_bounds_extreme_overscan_to_item_count() {
        let window = virtual_window(request(6, 20.0, 20.0, 10.0, usize::MAX));

        assert_window_finite(&window);
        assert_range(window.visible_range, 2..4);
        assert_range(window.materialized_range, 0..6);
    }

    #[test]
    fn virtual_window_bounds_huge_finite_viewport_materialized_range() {
        let window = virtual_window(request(4, 0.0, f32::MAX, 1.0, 0));

        assert_window_finite(&window);
        assert_range(window.visible_range, 0..4);
        assert_range(window.materialized_range, 0..4);
    }

    #[test]
    fn virtual_window_keeps_content_and_scroll_extents_finite_for_large_counts() {
        let window = virtual_window(request(usize::MAX, f32::MAX, 1.0, 1.0e30, 1));

        assert_window_finite(&window);
        assert!(window.content_extent > 1.0e38);
        assert!(window.max_scroll_offset > 1.0e38);
        assert!(window.clamped_scroll_offset > 1.0e38);
        assert!(window.visible_range.start <= window.visible_range.end);
        assert!(window.materialized_range.start <= window.materialized_range.end);
    }

    #[test]
    fn virtual_range_still_returns_materialized_range_for_compatibility() {
        let cases = [
            request(100, 50.0, 40.0, 10.0, 2),
            request(100, 5000.0, 40.0, 10.0, 0),
            request(20, 10.0, 30.0, 10.0, 0),
            request(6, -200.0, 20.0, 10.0, usize::MAX),
        ];

        for request in cases {
            let range_request = VirtualRangeRequest {
                item_count: request.item_count,
                scroll_offset: request.scroll_offset,
                viewport_extent: request.viewport_extent,
                item_extent: request.item_extent,
                overscan: request.overscan,
            };
            assert_eq!(
                virtual_range(range_request),
                virtual_window(request).materialized_range
            );
        }
    }
}
