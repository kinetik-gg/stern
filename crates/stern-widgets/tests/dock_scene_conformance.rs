//! Public, windowless Dock -> Frame -> Panel scene conformance.

#![allow(clippy::float_cmp)]

use stern_core::{
    Axis, Brush, ClipId, CornerRadius, FrameOutput, Point, PointerInput, PointerOrder,
    PointerRoute, PointerTarget, Primitive, Rect, RectPrimitive, SemanticRole, UiInput, UiMemory,
    WidgetId, default_dark_theme,
};
use stern_widgets::{
    Ui,
    dock::{
        Dock, DockChromeStyle, DockDropTarget, DockNode, DockPathElement, DockPlacement, DockScene,
        DockSceneConfig, DockScenePreviewKind, DockSplitPath, Frame, FrameId, Panel, PanelId,
    },
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 600.0, 400.0);

fn panel(id: u64, title: &str) -> Panel {
    Panel::new(PanelId::from_raw(id), title)
}

fn frame(id: u64, panels: Vec<Panel>) -> Frame {
    Frame::new(FrameId::from_raw(id), panels)
}

fn split(axis: Axis, ratio: f32, first: DockNode, second: DockNode) -> DockNode {
    DockNode::Split {
        axis,
        ratio,
        min_first: 0.0,
        min_second: 0.0,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn nested_dock() -> Dock {
    Dock::new(split(
        Axis::Horizontal,
        0.4,
        DockNode::Frame(frame(1, vec![panel(11, "Assets")])),
        split(
            Axis::Vertical,
            0.5,
            DockNode::Frame(frame(2, vec![panel(21, "Viewport")])),
            DockNode::Frame(frame(3, vec![panel(31, "Timeline")])),
        ),
    ))
}

fn two_frame_dock() -> Dock {
    let mut first = frame(1, vec![panel(11, "Inspector"), panel(12, "Details")]);
    assert!(first.select_panel(PanelId::from_raw(12)));
    let mut dock = Dock::new(split(
        Axis::Horizontal,
        0.5,
        DockNode::Frame(first),
        DockNode::Frame(frame(2, vec![panel(21, "Viewport")])),
    ));
    assert!(dock.set_active_frame(FrameId::from_raw(2)));
    dock
}

fn paint(scene: &DockScene) -> FrameOutput {
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let _ = ui.dock_scene(scene, |_, _| ());
    ui.finish_output()
}

fn rect_primitive_at(primitives: &[Primitive], rect: Rect) -> &RectPrimitive {
    primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Rect(primitive) if primitive.rect == rect => Some(primitive),
            _ => None,
        })
        .expect("painted rectangle")
}

fn clip_range(primitives: &[Primitive], rect: Rect) -> (usize, usize, ClipId) {
    let (begin, id) = primitives
        .iter()
        .enumerate()
        .find_map(|(index, primitive)| match primitive {
            Primitive::ClipBegin {
                id,
                rect: clip_rect,
            } if *clip_rect == rect => Some((index, *id)),
            _ => None,
        })
        .expect("clip begin");
    let end = primitives
        .iter()
        .enumerate()
        .skip(begin + 1)
        .find_map(|(index, primitive)| match primitive {
            Primitive::ClipEnd { id: end_id } if *end_id == id => Some(index),
            _ => None,
        })
        .expect("matching clip end");
    (begin, end, id)
}

#[test]
fn nested_splits_prepare_expected_geometry_and_splitter_primitives() {
    let dock = nested_dock();
    let snapshot = dock.snapshot();
    let root = WidgetId::from_key("dock-scene-nested");
    let config = DockSceneConfig::new(root, BOUNDS)
        .with_chrome_style(DockChromeStyle::default().with_splitter_hit_thickness(8.0));
    let scene = DockScene::new(config, &dock);

    let frames = &scene.layout().frames;
    assert_eq!(
        frames
            .iter()
            .map(|frame| (frame.frame, frame.rect))
            .collect::<Vec<_>>(),
        vec![
            (FrameId::from_raw(1), Rect::new(0.0, 0.0, 240.0, 400.0)),
            (FrameId::from_raw(2), Rect::new(240.0, 0.0, 360.0, 200.0),),
            (FrameId::from_raw(3), Rect::new(240.0, 200.0, 360.0, 200.0),),
        ]
    );

    let splitters = &scene.layout().splitters;
    assert_eq!(splitters.len(), 2);
    assert_eq!(splitters[0].path, DockSplitPath::root());
    assert_eq!(splitters[0].axis, Axis::Horizontal);
    assert_eq!(splitters[0].rect, Rect::new(236.0, 0.0, 8.0, 400.0));
    assert_eq!(
        splitters[1].path,
        DockSplitPath::root().child(DockPathElement::Second)
    );
    assert_eq!(splitters[1].axis, Axis::Vertical);
    assert_eq!(splitters[1].rect, Rect::new(240.0, 196.0, 360.0, 8.0));

    let output = paint(&scene);
    for splitter in splitters {
        let primitive = rect_primitive_at(&output.primitives, splitter.rect);
        assert!(primitive.fill.is_some());
    }
    assert_eq!(dock.snapshot(), snapshot);
}

#[test]
#[allow(clippy::too_many_lines)]
fn semantics_follow_dock_frame_tablist_tab_panel_order_and_callbacks_are_clipped() {
    let dock = two_frame_dock();
    let scene = DockScene::new(
        DockSceneConfig::new(WidgetId::from_key("dock-scene-semantics"), BOUNDS),
        &dock,
    );
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let mut callbacks = Vec::new();
    let callback_output = ui.dock_scene(&scene, |ui, panel| {
        callbacks.push(panel.panel);
        let marker = Rect::new(panel.rect.x + 1.0, panel.rect.y + 1.0, 1.0, 1.0);
        ui.primitive(Primitive::Rect(RectPrimitive {
            rect: marker,
            fill: None,
            stroke: None,
            radius: CornerRadius::default(),
        }));
        marker
    });
    let output = ui.finish_output();

    let frame_one = FrameId::from_raw(1);
    let frame_two = FrameId::from_raw(2);
    let panel_11 = PanelId::from_raw(11);
    let panel_12 = PanelId::from_raw(12);
    let panel_21 = PanelId::from_raw(21);
    assert_eq!(callbacks, vec![panel_12, panel_21]);
    assert_eq!(callback_output.len(), 2);

    let expected_order = vec![
        scene.root_widget_id(),
        scene.frame_widget_id(frame_one),
        scene.tab_list_widget_id(frame_one),
        scene.tab_widget_id(panel_11),
        scene.tab_widget_id(panel_12),
        scene.panel_widget_id(panel_12),
        scene.frame_widget_id(frame_two),
        scene.tab_list_widget_id(frame_two),
        scene.tab_widget_id(panel_21),
        scene.panel_widget_id(panel_21),
    ];
    assert_eq!(output.semantics.traversal_order(), expected_order);
    let root = output
        .semantics
        .get(scene.root_widget_id())
        .expect("dock semantics");
    assert_eq!(root.role, SemanticRole::Dock);
    assert_eq!(
        root.children,
        vec![
            scene.frame_widget_id(frame_one),
            scene.frame_widget_id(frame_two)
        ]
    );

    let inactive = output
        .semantics
        .get(scene.frame_widget_id(frame_one))
        .expect("inactive frame");
    let active = output
        .semantics
        .get(scene.frame_widget_id(frame_two))
        .expect("active frame");
    assert_eq!(inactive.role, SemanticRole::Frame);
    assert!(!inactive.state.selected);
    assert!(active.state.selected);
    assert_eq!(
        output
            .semantics
            .get(scene.tab_list_widget_id(frame_one))
            .expect("tab list")
            .children,
        vec![scene.tab_widget_id(panel_11), scene.tab_widget_id(panel_12)]
    );
    assert!(
        output
            .semantics
            .get(scene.tab_widget_id(panel_12))
            .expect("selected tab")
            .state
            .selected
    );
    assert_eq!(
        output.semantics.parent_of(scene.panel_widget_id(panel_12)),
        Some(scene.frame_widget_id(frame_one))
    );

    for (panel, marker) in scene
        .layout()
        .frames
        .iter()
        .filter_map(|frame| frame.panel.as_ref())
        .zip(callback_output)
    {
        let (begin, end, id) = clip_range(&output.primitives, panel.rect);
        assert_eq!(end, begin + 2, "callback should be the exact clip payload");
        assert_eq!(
            output.primitives[begin + 1],
            Primitive::Rect(RectPrimitive {
                rect: marker,
                fill: None,
                stroke: None,
                radius: CornerRadius::default(),
            })
        );
        assert!(
            matches!(output.primitives[end], Primitive::ClipEnd { id: end_id } if end_id == id)
        );
    }
}

#[test]
fn compact_tabs_do_not_overlap_and_each_title_is_painted_inside_its_clip() {
    let panels = vec![
        panel(1, "A very long first title"),
        panel(2, "A very long second title"),
        panel(3, "A very long third title"),
        panel(4, "A very long fourth title"),
    ];
    let dock = Dock::new(DockNode::Frame(frame(1, panels)));
    let scene = DockScene::new(
        DockSceneConfig::new(
            WidgetId::from_key("dock-scene-compact"),
            Rect::new(0.0, 0.0, 100.0, 120.0),
        )
        .with_tab_height(20.0),
        &dock,
    );
    let tabs = &scene.layout().frames[0].tabs;

    assert_eq!(tabs.len(), 4);
    assert!(tabs.iter().all(|tab| tab.rect.width == 25.0));
    assert!(tabs.iter().all(|tab| tab.close_rect.is_none()));
    for pair in tabs.windows(2) {
        assert!(pair[0].rect.max_x() <= pair[1].rect.x);
    }
    assert!(tabs[0].rect.x >= scene.layout().frames[0].tab_list_rect.x);
    assert!(
        tabs.last().expect("last tab").rect.max_x()
            <= scene.layout().frames[0].tab_list_rect.max_x()
    );

    let output = paint(&scene);
    for tab in tabs {
        let (begin, end, _) = clip_range(&output.primitives, tab.rect);
        assert!(
            output.primitives[begin + 1..end].iter().any(
                |primitive| matches!(primitive, Primitive::Text(text) if text.text == tab.title)
            )
        );
    }
}

#[test]
fn merge_and_edge_previews_have_distinct_geometry_and_theme_paint() {
    let dock = Dock::new(DockNode::Frame(frame(7, vec![panel(70, "Viewport")])));
    let root = WidgetId::from_key("dock-scene-preview");
    let bounds = Rect::new(0.0, 0.0, 400.0, 300.0);
    let merge = DockScene::new(
        DockSceneConfig::new(root, bounds)
            .with_drop_preview(Some(DockDropTarget::tab(FrameId::from_raw(7)))),
        &dock,
    );
    let edge = DockScene::new(
        DockSceneConfig::new(root, bounds).with_drop_preview(Some(DockDropTarget::split(
            FrameId::from_raw(7),
            DockPlacement::Right,
            FrameId::from_raw(8),
        ))),
        &dock,
    );
    let merge_preview = merge.layout().preview.expect("merge preview");
    let edge_preview = edge.layout().preview.expect("edge preview");

    assert_eq!(merge_preview.kind, DockScenePreviewKind::Merge);
    assert_eq!(
        edge_preview.kind,
        DockScenePreviewKind::Split(DockPlacement::Right)
    );
    assert_eq!(merge_preview.rect, Rect::new(48.0, 36.0, 304.0, 228.0));
    assert_eq!(edge_preview.rect, Rect::new(260.0, 0.0, 140.0, 300.0));
    assert_ne!(merge_preview.rect, edge_preview.rect);
    assert_eq!(merge_preview.id, edge_preview.id);

    let theme = default_dark_theme();
    let merge_output = paint(&merge);
    let edge_output = paint(&edge);
    let merge_paint = rect_primitive_at(&merge_output.primitives, merge_preview.rect);
    let edge_paint = rect_primitive_at(&edge_output.primitives, edge_preview.rect);
    assert_eq!(
        merge_paint.fill,
        Some(Brush::Solid(theme.colors.accent.default.with_alpha(0.20)))
    );
    assert_eq!(
        edge_paint.fill,
        Some(Brush::Solid(theme.colors.accent.default.with_alpha(0.32)))
    );
    assert_eq!(merge_paint.radius, theme.radii.sm);
    assert_eq!(edge_paint.radius, theme.radii.none);
}

#[test]
fn identity_derived_ids_survive_tab_reorder_and_topology_changes() {
    let root = WidgetId::from_key("dock-scene-stable-ids");
    let panel_a = PanelId::from_raw(11);
    let panel_b = PanelId::from_raw(12);
    let panel_c = PanelId::from_raw(21);
    let frame_one = FrameId::from_raw(1);
    let frame_two = FrameId::from_raw(2);
    let before_dock = Dock::new(split(
        Axis::Horizontal,
        0.5,
        DockNode::Frame(frame(
            1,
            vec![panel(panel_a.raw(), "A"), panel(panel_b.raw(), "B")],
        )),
        DockNode::Frame(frame(2, vec![panel(panel_c.raw(), "C")])),
    ));
    let before = DockScene::new(DockSceneConfig::new(root, BOUNDS), &before_dock);

    let mut reordered = frame(
        1,
        vec![panel(panel_b.raw(), "B"), panel(panel_a.raw(), "A")],
    );
    assert!(reordered.select_panel(panel_a));
    let after_dock = Dock::new(split(
        Axis::Vertical,
        0.6,
        DockNode::Frame(frame(2, vec![panel(panel_c.raw(), "C")])),
        DockNode::Frame(reordered),
    ));
    let after = DockScene::new(DockSceneConfig::new(root, BOUNDS), &after_dock);

    let before_one = before
        .layout()
        .frames
        .iter()
        .find(|frame| frame.frame == frame_one)
        .expect("frame one before");
    let after_one = after
        .layout()
        .frames
        .iter()
        .find(|frame| frame.frame == frame_one)
        .expect("frame one after");
    assert_eq!(
        before_one.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>(),
        vec![before.tab_widget_id(panel_a), before.tab_widget_id(panel_b)]
    );
    assert_eq!(
        after_one.tabs.iter().map(|tab| tab.id).collect::<Vec<_>>(),
        vec![after.tab_widget_id(panel_b), after.tab_widget_id(panel_a)]
    );
    assert_eq!(
        before.frame_widget_id(frame_one),
        after.frame_widget_id(frame_one)
    );
    assert_eq!(
        before.frame_widget_id(frame_two),
        after.frame_widget_id(frame_two)
    );
    assert_eq!(
        before.tab_list_widget_id(frame_one),
        after.tab_list_widget_id(frame_one)
    );
    assert_eq!(before.tab_widget_id(panel_a), after.tab_widget_id(panel_a));
    assert_eq!(before.tab_widget_id(panel_b), after.tab_widget_id(panel_b));
    assert_eq!(
        before.panel_widget_id(panel_a),
        after.panel_widget_id(panel_a)
    );
    assert_eq!(
        before.splitter_widget_id(&DockSplitPath::root()),
        after.splitter_widget_id(&DockSplitPath::root())
    );
    assert_eq!(
        after
            .layout()
            .frames
            .iter()
            .map(|frame| frame.frame)
            .collect::<Vec<_>>(),
        vec![frame_two, frame_one]
    );
}

#[test]
fn panel_content_scopes_include_the_scene_root() {
    let dock = Dock::new(DockNode::Frame(frame(1, vec![panel(11, "Inspector")])));
    let first = DockScene::new(
        DockSceneConfig::new(WidgetId::from_key("first-dock"), BOUNDS),
        &dock,
    );
    let second = DockScene::new(
        DockSceneConfig::new(WidgetId::from_key("second-dock"), BOUNDS),
        &dock,
    );
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);

    let first_ids = ui.dock_scene(&first, |ui, _| ui.make_id("field"));
    let second_ids = ui.dock_scene(&second, |ui, _| ui.make_id("field"));

    assert_eq!(first_ids.len(), 1);
    assert_eq!(second_ids.len(), 1);
    assert_ne!(first_ids[0], second_ids[0]);
}

#[test]
fn invalid_bounds_and_empty_frames_are_safe() {
    let root = WidgetId::from_key("dock-scene-invalid");
    let dock = Dock::new(DockNode::Frame(frame(1, vec![panel(1, "Panel")])));
    let invalid = DockScene::new(
        DockSceneConfig::new(root, Rect::new(f32::NAN, 0.0, 100.0, 100.0)),
        &dock,
    );
    assert_eq!(invalid.layout().bounds, Rect::ZERO);
    assert!(invalid.layout().frames.is_empty());
    assert!(invalid.layout().splitters.is_empty());
    assert!(invalid.layout().preview.is_none());
    let invalid_output = paint(&invalid);
    assert!(invalid_output.primitives.is_empty());
    assert!(invalid_output.semantics.is_empty());

    let empty_dock = Dock::new(DockNode::Frame(frame(2, Vec::new())));
    let empty = DockScene::new(
        DockSceneConfig::new(root, Rect::new(0.0, 0.0, 100.0, 100.0)),
        &empty_dock,
    );
    assert_eq!(empty.layout().frames.len(), 1);
    assert!(empty.layout().frames[0].tabs.is_empty());
    assert!(empty.layout().frames[0].panel.is_none());
    let input = UiInput::default();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let callbacks = ui.dock_scene(&empty, |_, _| panic!("empty frame has no panel callback"));
    let output = ui.finish_output();
    assert!(callbacks.is_empty());
    assert_eq!(output.semantics.len(), 3);
    assert_eq!(
        output
            .semantics
            .nodes()
            .iter()
            .map(|node| node.role.clone())
            .collect::<Vec<_>>(),
        vec![
            SemanticRole::Dock,
            SemanticRole::Frame,
            SemanticRole::TabList
        ]
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn pointer_plan_orders_dock_chrome_and_panel_content_without_collisions() {
    let root = WidgetId::from_key("dock-scene-pointer-plan");
    let panel_id = PanelId::from_raw(1);
    let dock = Dock::new(split(
        Axis::Horizontal,
        0.5,
        DockNode::Frame(frame(1, vec![panel(1, "Panel")])),
        DockNode::Frame(frame(2, vec![panel(2, "Other")])),
    ));
    let scene = DockScene::new(
        DockSceneConfig::new(root, Rect::new(0.0, 0.0, 200.0, 140.0)),
        &dock,
    );
    let lower_id = WidgetId::from_key("lower-content");
    let panel_content_id = WidgetId::from_key("panel-content");
    let panel_rect = scene.layout().frames[0]
        .panel
        .as_ref()
        .expect("active panel")
        .rect;
    let input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(20.0, 80.0)),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let mut ui = Ui::new(&input, &mut memory, &theme);
    let routes = ui
        .resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(
                lower_id,
                scene.layout().bounds,
                PointerOrder::new(1),
            ));
            let next = scene.declare_pointer_targets_with_content(
                plan,
                PointerOrder::new(100),
                |plan, content_order| {
                    plan.target(PointerTarget::new(
                        panel_content_id,
                        panel_rect,
                        content_order,
                    ));
                    PointerOrder::new(content_order.raw() + 1)
                },
            );
            assert!(next > PointerOrder::new(100));
        })
        .expect("dock and panel targets form one valid plan");
    assert_eq!(routes.ordinary, PointerRoute::Target(panel_content_id));

    let tab_input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(10.0, 10.0)),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut tab_memory = UiMemory::new();
    let mut tab_ui = Ui::new(&tab_input, &mut tab_memory, &theme);
    let tab_routes = tab_ui
        .resolve_pointer_targets(|plan| {
            plan.target(PointerTarget::new(
                lower_id,
                scene.layout().bounds,
                PointerOrder::new(1),
            ));
            scene.declare_pointer_targets(plan, PointerOrder::new(100));
        })
        .expect("dock targets do not collide");
    assert_eq!(
        tab_routes.ordinary,
        PointerRoute::Target(scene.tab_widget_id(panel_id))
    );

    let splitter_input = UiInput {
        pointer: PointerInput {
            position: Some(Point::new(99.0, 80.0)),
            ..PointerInput::default()
        },
        ..UiInput::default()
    };
    let mut splitter_memory = UiMemory::new();
    let mut splitter_ui = Ui::new(&splitter_input, &mut splitter_memory, &theme);
    let splitter_routes = splitter_ui
        .resolve_pointer_targets(|plan| {
            scene.declare_pointer_targets_with_content(
                plan,
                PointerOrder::new(100),
                |plan, content_order| {
                    plan.target(PointerTarget::new(
                        panel_content_id,
                        panel_rect,
                        content_order,
                    ));
                    PointerOrder::new(content_order.raw() + 1)
                },
            );
        })
        .expect("splitter target remains above overlapping panel content");
    assert_eq!(
        splitter_routes.ordinary,
        PointerRoute::Target(scene.splitter_widget_id(&DockSplitPath::root()))
    );
}
