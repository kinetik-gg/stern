//! Public-facade source compatibility for Dock scene splitters and tab
//! insertion targets.

use stern::{
    core::{Axis, Rect, WidgetId},
    widgets::dock::{
        DockDropTarget, DockPlacement, DockScenePreviewKind, DockSceneSplitter, DockSplitPath,
        FrameId, PanelId,
    },
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

#[test]
fn dock_tab_insertion_targets_are_reachable_and_exhaustively_matchable() {
    let insert = DockDropTarget::Insert {
        frame: FrameId::from_raw(7),
        anchor: Some(PanelId::from_raw(70)),
    };

    // Exhaustive matches over the public enum must name every variant; the
    // Insert arm is part of the prerelease migration surface (#875).
    match insert {
        DockDropTarget::Tab { frame } => assert_eq!(frame, FrameId::from_raw(7)),
        DockDropTarget::Insert { frame, anchor } => {
            assert_eq!(frame, FrameId::from_raw(7));
            assert_eq!(anchor, Some(PanelId::from_raw(70)));
        }
        DockDropTarget::Split { frame, placement, .. } => {
            assert_eq!(frame, FrameId::from_raw(7));
            assert_eq!(placement, DockPlacement::Left);
        }
    }

    match DockScenePreviewKind::Insert {
        DockScenePreviewKind::Merge => {}
        DockScenePreviewKind::Split(_) => {}
        DockScenePreviewKind::Insert => {}
    }
}
