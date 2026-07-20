//! Deterministic reserved-gutter conformance for public Dock scenes.

#![allow(clippy::float_cmp)]

use stern_core::{Axis, Rect, WidgetId};
use stern_widgets::dock::{
    Dock, DockChromeStyle, DockNode, DockScene, DockSceneConfig, Frame, FrameId, Panel, PanelId,
};

fn panel(id: u64) -> Panel {
    Panel::new(PanelId::from_raw(id), format!("Panel {id}"))
}

fn frame(id: u64) -> DockNode {
    DockNode::Frame(Frame::new(FrameId::from_raw(id), vec![panel(id * 10)]))
}

fn split(
    axis: Axis,
    ratio: f32,
    min_first: f32,
    min_second: f32,
    first: DockNode,
    second: DockNode,
) -> DockNode {
    DockNode::Split {
        axis,
        ratio,
        min_first,
        min_second,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn scene(root: DockNode, bounds: Rect, thickness: f32) -> DockScene {
    DockScene::new(
        DockSceneConfig::new(
            WidgetId::from_key((
                "reserved-dock-gutter",
                bounds.x.to_bits(),
                bounds.y.to_bits(),
                bounds.width.to_bits(),
                bounds.height.to_bits(),
            )),
            bounds,
        )
        .with_chrome_style(DockChromeStyle::default().with_splitter_hit_thickness(thickness)),
        &Dock::new(root),
    )
}

fn overlaps_positive(first: Rect, second: Rect) -> bool {
    first.min_x() < second.max_x()
        && second.min_x() < first.max_x()
        && first.min_y() < second.max_y()
        && second.min_y() < first.max_y()
}

fn assert_finite(rect: Rect) {
    assert!(rect.x.is_finite());
    assert!(rect.y.is_finite());
    assert!(rect.width.is_finite());
    assert!(rect.height.is_finite());
    assert!(rect.width >= 0.0);
    assert!(rect.height >= 0.0);
}

fn assert_reserved_gutters(scene: &DockScene) {
    let layout = scene.layout();
    for frame in &layout.frames {
        assert_finite(frame.rect);
        for splitter in &layout.splitters {
            assert!(
                !overlaps_positive(frame.rect, splitter.rect),
                "frame {:?} overlaps splitter {:?}",
                frame.rect,
                splitter.rect
            );
            assert!(!overlaps_positive(frame.tab_list_rect, splitter.rect));
            for tab in &frame.tabs {
                assert!(!overlaps_positive(tab.rect, splitter.rect));
                if let Some(close) = tab.close_rect {
                    assert!(!overlaps_positive(close, splitter.rect));
                }
            }
            if let Some(panel) = &frame.panel {
                assert!(!overlaps_positive(panel.rect, splitter.rect));
            }
        }
    }

    for (index, splitter) in layout.splitters.iter().enumerate() {
        assert_finite(splitter.rect);
        assert!(splitter.rect.width > 0.0);
        assert!(splitter.rect.height > 0.0);
        assert!(splitter.rect.min_x() >= layout.bounds.min_x());
        assert!(splitter.rect.min_y() >= layout.bounds.min_y());
        assert!(splitter.rect.max_x() <= layout.bounds.max_x());
        assert!(splitter.rect.max_y() <= layout.bounds.max_y());

        let divider = splitter.divider_rect();
        assert_finite(divider);
        assert!(divider.min_x() >= splitter.rect.min_x());
        assert!(divider.min_y() >= splitter.rect.min_y());
        assert!(divider.max_x() <= splitter.rect.max_x());
        assert!(divider.max_y() <= splitter.rect.max_y());
        match splitter.axis {
            Axis::Horizontal => assert_eq!(divider.width, 1.0),
            Axis::Vertical => assert_eq!(divider.height, 1.0),
        }

        for other in layout.splitters.iter().skip(index + 1) {
            assert!(
                !overlaps_positive(splitter.rect, other.rect),
                "splitter gutters overlap: {:?} and {:?}",
                splitter.rect,
                other.rect
            );
        }
    }
}

#[test]
fn root_and_nested_t_junction_splitters_use_reserved_non_overlapping_gutters() {
    let scene = scene(
        split(
            Axis::Horizontal,
            0.4,
            0.0,
            0.0,
            frame(1),
            split(Axis::Vertical, 0.5, 0.0, 0.0, frame(2), frame(3)),
        ),
        Rect::new(0.0, 0.0, 600.0, 400.0),
        8.0,
    );

    assert_eq!(
        scene
            .layout()
            .frames
            .iter()
            .map(|frame| frame.rect)
            .collect::<Vec<_>>(),
        vec![
            Rect::new(0.0, 0.0, 236.0, 400.0),
            Rect::new(244.0, 0.0, 356.0, 196.0),
            Rect::new(244.0, 204.0, 356.0, 196.0),
        ]
    );
    assert_eq!(
        scene
            .layout()
            .splitters
            .iter()
            .map(|splitter| splitter.rect)
            .collect::<Vec<_>>(),
        vec![
            Rect::new(236.0, 0.0, 8.0, 400.0),
            Rect::new(244.0, 196.0, 356.0, 8.0),
        ]
    );
    assert_reserved_gutters(&scene);
}

#[test]
fn fractional_clipped_and_clamped_layouts_preserve_contained_actionable_geometry() {
    let fractional = scene(
        split(
            Axis::Horizontal,
            0.333,
            55.5,
            66.25,
            frame(1),
            split(Axis::Vertical, 0.625, 24.5, 31.75, frame(2), frame(3)),
        ),
        Rect::new(10.25, -4.75, 333.5, 211.25),
        7.25,
    );
    assert_reserved_gutters(&fractional);

    let clipped = scene(
        split(Axis::Horizontal, 0.5, 20.0, 20.0, frame(4), frame(5)),
        Rect::new(0.0, 0.0, 37.0, 29.0),
        8.0,
    );
    assert_eq!(clipped.layout().frames[0].rect.width, 20.0);
    assert_eq!(clipped.layout().splitters[0].rect.width, 8.0);
    assert_eq!(clipped.layout().frames[1].rect.width, 9.0);
    assert_reserved_gutters(&clipped);

    for (ratio, expected_first, expected_second) in [(0.0, 25.0, 67.0), (1.0, 62.0, 30.0)] {
        let clamped = scene(
            split(Axis::Horizontal, ratio, 25.0, 30.0, frame(6), frame(7)),
            Rect::new(0.0, 0.0, 100.0, 50.0),
            8.0,
        );
        assert_eq!(clamped.layout().frames[0].rect.width, expected_first);
        assert_eq!(clamped.layout().frames[1].rect.width, expected_second);
        assert_reserved_gutters(&clamped);
    }
}
