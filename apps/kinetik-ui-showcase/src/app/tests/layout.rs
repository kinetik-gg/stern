use super::helpers::{Point, Primitive, Rect, ShowcaseApp, ShowcasePage, click};
use kinetik_ui::{
    core::Axis,
    widgets::{Dock, DockNode, Frame, FrameId, Panel, PanelId, solve_dock_layout},
};

#[test]
fn layout_page_split_demo_changes_dock_preview() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Layout);

    click(&mut app, Point::new(700.0, 162.0));

    assert!(
        app.primitives().iter().any(|primitive| {
            matches!(primitive, Primitive::Text(text) if text.text == "Frame 9")
        })
    );
}

#[test]
fn dock_preview_tabs_stay_inside_solved_frames_without_cross_frame_overlap() {
    let app = ShowcaseApp::new();
    let dock = app.dock_model_preview();
    let frame_layouts = solve_dock_layout(&dock, Rect::new(20.0, 40.0, 500.0, 204.0));
    let tabs = ShowcaseApp::dock_preview_tab_layouts(&dock, &frame_layouts);

    assert_eq!(tabs.len(), 5);
    for tab in &tabs {
        let frame = frame_layouts
            .iter()
            .find(|frame| frame.frame == tab.frame)
            .expect("tab owns a solved frame");
        assert!(frame.rect.contains_rect(tab.rect));
    }
    for (index, tab) in tabs.iter().enumerate() {
        for other in tabs.iter().skip(index + 1) {
            if tab.frame != other.frame {
                assert_eq!(tab.rect.intersection(other.rect), None);
            }
        }
    }
}

#[test]
fn dock_preview_tabs_preserve_in_frame_panel_order() {
    let app = ShowcaseApp::new();
    let dock = app.dock_model_preview();
    let frame_layouts = solve_dock_layout(&dock, Rect::new(20.0, 40.0, 500.0, 204.0));
    let tabs = ShowcaseApp::dock_preview_tab_layouts(&dock, &frame_layouts);
    let first_frame = frame_layouts
        .iter()
        .find(|frame| frame.frame == FrameId::from_raw(1))
        .expect("first frame is solved");
    let first_frame_tabs: Vec<_> = tabs
        .iter()
        .filter(|tab| tab.frame == FrameId::from_raw(1))
        .collect();

    assert_eq!(first_frame_tabs.len(), 2);
    assert_eq!(first_frame_tabs[0].panel, PanelId::from_raw(1));
    assert_eq!(first_frame_tabs[1].panel, PanelId::from_raw(2));
    assert_close(first_frame_tabs[0].rect.x, first_frame.rect.x + 8.0);
    assert_close(first_frame_tabs[0].rect.y, first_frame.rect.max_y() - 30.0);
    assert_close(
        first_frame_tabs[1].rect.x,
        first_frame_tabs[0].rect.max_x() + 4.0,
    );
}

#[test]
fn dock_preview_tabs_ignore_frame_id_values() {
    let bounds = Rect::new(20.0, 40.0, 500.0, 204.0);
    let first = dock_with_frame_ids(FrameId::from_raw(1), FrameId::from_raw(2));
    let renamed = dock_with_frame_ids(FrameId::from_raw(91), FrameId::from_raw(7));

    let first_frames = solve_dock_layout(&first, bounds);
    let renamed_frames = solve_dock_layout(&renamed, bounds);
    let first_rects: Vec<_> = ShowcaseApp::dock_preview_tab_layouts(&first, &first_frames)
        .into_iter()
        .map(|tab| tab.rect)
        .collect();
    let renamed_rects: Vec<_> = ShowcaseApp::dock_preview_tab_layouts(&renamed, &renamed_frames)
        .into_iter()
        .map(|tab| tab.rect)
        .collect();

    assert_eq!(first_rects, renamed_rects);
}

fn dock_with_frame_ids(first: FrameId, second: FrameId) -> Dock {
    Dock::new(DockNode::Split {
        axis: Axis::Horizontal,
        ratio: 0.5,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(DockNode::Frame(Frame::new(
            first,
            vec![
                Panel::new(PanelId::from_raw(11), "First"),
                Panel::new(PanelId::from_raw(12), "Second"),
            ],
        ))),
        second: Box::new(DockNode::Frame(Frame::new(
            second,
            vec![Panel::new(PanelId::from_raw(21), "Third")],
        ))),
    })
}

fn assert_close(actual: f32, expected: f32) {
    assert!((actual - expected).abs() < 0.001);
}
