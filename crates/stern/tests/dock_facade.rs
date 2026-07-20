//! Public-facade source compatibility for Dock scene splitters.

use stern::{
    core::{Axis, Rect, WidgetId},
    widgets::dock::{DockSceneSplitter, DockSplitPath},
};

#[test]
fn dock_scene_splitter_preserves_prior_exhaustive_field_surface() {
    let splitter = DockSceneSplitter {
        id: WidgetId::from_raw(41),
        path: DockSplitPath::root(),
        axis: Axis::Horizontal,
        rect: Rect::new(10.0, 20.0, 8.0, 120.0),
    };

    let DockSceneSplitter {
        id,
        path,
        axis,
        rect,
    } = splitter.clone();

    assert_eq!(id, WidgetId::from_raw(41));
    assert_eq!(path, DockSplitPath::root());
    assert_eq!(axis, Axis::Horizontal);
    assert_eq!(rect, Rect::new(10.0, 20.0, 8.0, 120.0));
    assert_eq!(splitter.divider_rect(), Rect::new(13.5, 20.0, 1.0, 120.0));
}
