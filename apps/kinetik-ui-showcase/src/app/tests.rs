use super::{
    ACTION_EDITOR_DOCK_JOIN, ACTION_SYSTEMS_DISPATCH, ShowcaseApp, ShowcaseInput, ShowcasePage,
    frame_context, static_render_resources,
};
use crate::editor::phosphor_icons;
use kinetik_ui::{
    core::{
        ActionContext, ActionId, ActionInvocation, ActionSource, ImageId, Key, KeyEvent, KeyState,
        KeyboardInput, Modifiers, PhysicalSize, PlatformRequest, Point, Primitive, Rect,
        RepaintRequest, ScaleFactor, SemanticActionKind, SemanticRole, SemanticValue, Size,
        TextureId, UiInput, ViewportInfo, WidgetId,
    },
    render::{RenderFrameInput, RenderImageSampling},
    render_vello::VelloRenderer,
};

fn click(app: &mut ShowcaseApp, point: Point) {
    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: true,
        ..ShowcaseInput::default()
    });
    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: false,
        ..ShowcaseInput::default()
    });
}

fn has_text(app: &ShowcaseApp, value: &str) -> bool {
    app.primitives()
        .iter()
        .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == value))
}

fn count_primitives(app: &ShowcaseApp, predicate: impl Fn(&Primitive) -> bool) -> usize {
    app.output()
        .primitives
        .iter()
        .filter(|primitive| predicate(primitive))
        .count()
}

fn count_semantic_role(app: &ShowcaseApp, role: &SemanticRole) -> usize {
    app.output()
        .semantics
        .nodes()
        .iter()
        .filter(|node| &node.role == role)
        .count()
}

fn semantic_node(app: &ShowcaseApp, role: &SemanticRole, label: &str) -> bool {
    app.output()
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role && node.label.as_deref() == Some(label))
}

fn semantic_role_has_action(
    app: &ShowcaseApp,
    role: &SemanticRole,
    action: &SemanticActionKind,
) -> bool {
    app.output()
        .semantics
        .nodes()
        .iter()
        .any(|node| &node.role == role && node.actions.iter().any(|item| &item.kind == action))
}

fn text_labels(app: &ShowcaseApp) -> Vec<&str> {
    app.output()
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect()
}

fn contains_text_in_order(app: &ShowcaseApp, expected: &[&str]) -> bool {
    let mut cursor = 0;
    for label in text_labels(app) {
        if expected
            .get(cursor)
            .is_some_and(|expected| *expected == label)
        {
            cursor += 1;
            if cursor == expected.len() {
                return true;
            }
        }
    }
    false
}

fn viewport_texture_rect(app: &ShowcaseApp) -> Rect {
    app.primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Texture(texture) if texture.texture == TextureId::from_raw(99) => {
                Some(texture.rect)
            }
            _ => None,
        })
        .expect("viewport texture")
}

#[test]
fn clicking_button_changes_action_state() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(70.0, 154.0));

    assert_eq!(app.action_count(), 1);
}

#[test]
fn unknown_action_invocation_is_not_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new("showcase.unknown"),
        ActionSource::Button,
        ActionContext::Global,
    );

    app.handle_action_invocation(&invocation);

    assert_eq!(app.action_count(), 0);
    assert_eq!(
        app.status,
        "Ignored unhandled action showcase.unknown via Button"
    );
}

#[test]
fn unhandled_editor_action_invocation_is_not_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new("editor.unknown"),
        ActionSource::Button,
        ActionContext::Editor,
    );

    assert!(!app.handle_action_invocation(&invocation));

    assert_eq!(app.action_count(), 0);
    assert_eq!(
        app.status,
        "Ignored unhandled action editor.unknown via Button"
    );
}

#[test]
fn editor_rendered_dock_action_invocation_is_counted() {
    let mut app = ShowcaseApp::new();
    let invocation = ActionInvocation::new(
        ActionId::new(ACTION_EDITOR_DOCK_JOIN),
        ActionSource::Button,
        ActionContext::Editor,
    );

    assert!(app.handle_action_invocation(&invocation));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.status, "editor.dock.join via Button (1)");
    assert!(!app.status.contains("Ignored unhandled action"));
}

#[test]
fn explicit_showcase_demo_action_is_counted() {
    let mut app = ShowcaseApp::new();

    assert!(app.invoke_action(ACTION_SYSTEMS_DISPATCH, ActionSource::Button));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.status, "systems.dispatch via Button (1)");
}

#[test]
fn components_page_structural_smoke_emits_controls_semantics_and_platform_requests() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 120);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 40);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 25);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Image(_))) >= 2);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 1);
    assert!(contains_text_in_order(
        &app,
        &[
            "Component Gallery",
            "Controls",
            "Text Input",
            "Lists, Grids, Tables",
            "Reusable Panel States",
            "Primitive Stream",
        ]
    ));

    assert_eq!(count_semantic_role(&app, &SemanticRole::Button), 2);
    assert_eq!(count_semantic_role(&app, &SemanticRole::IconButton), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::CheckBox), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::Toggle), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::RadioButton), 2);
    assert_eq!(count_semantic_role(&app, &SemanticRole::Slider), 1);
    assert_eq!(count_semantic_role(&app, &SemanticRole::SearchField), 1);
    assert!(count_semantic_role(&app, &SemanticRole::TextField) >= 3);
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 5);
    assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 4);
    assert!(count_semantic_role(&app, &SemanticRole::Tab) >= 3);
    assert!(semantic_node(&app, &SemanticRole::Button, "Run Action"));
    assert!(semantic_node(&app, &SemanticRole::Button, "Disabled"));
    assert!(semantic_node(
        &app,
        &SemanticRole::IconButton,
        "Icon button"
    ));

    let slider = app
        .output()
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Slider)
        .expect("slider semantics");
    assert!(
        slider
            .actions
            .iter()
            .any(|action| action.kind == SemanticActionKind::SetValue)
    );
    assert!(matches!(
        slider.state.value,
        Some(SemanticValue::Number { current, min: 0.0, max: 1.0 })
            if (current - app.strength()).abs() < f32::EPSILON
    ));

    click(&mut app, Point::new(940.0, 160.0));

    assert!(!app.output().platform_requests.iter().any(|request| {
        matches!(
            request,
            PlatformRequest::StartTextInput { .. } | PlatformRequest::StopTextInput
        )
    }));
}

#[test]
fn layout_page_structural_smoke_emits_layout_dock_table_and_actions() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Layout);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 100);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 60);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 30);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Layout, Docking, and Data Surfaces",
            "Measurement-Aware Layout",
            "Interactive Dock Model",
            "Virtualized Table Model",
        ]
    ));
    assert!(has_text(&app, "Rows: 7 | Columns: 4 | Overscan: 0"));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text.starts_with("Frames: "))
    }));

    assert!(semantic_node(
        &app,
        &SemanticRole::Dock,
        "Interactive Dock Model"
    ));
    assert!(semantic_node(
        &app,
        &SemanticRole::Table,
        "Virtualized Table Model"
    ));
    assert!(semantic_node(&app, &SemanticRole::Button, "Split Tab"));
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 4);
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Slider,
        &SemanticActionKind::SetValue
    ));
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Button,
        &SemanticActionKind::Invoke
    ));

    click(&mut app, Point::new(700.0, 162.0));

    assert!(has_text(&app, "Frame 9"));
}

#[test]
fn component_status_reflects_toggle_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(231.0, 204.0));

    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Toggle: true")
    }));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Text(text)
                if text.text == "checkbox=true toggle=true radio=1 selected_row=2"
        )
    }));
}

#[test]
fn component_status_reflects_checkbox_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(71.0, 204.0));

    assert!(!app.checkbox);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Checkbox: false")
    }));
    assert!(app.primitives().iter().any(|primitive| {
        matches!(
            primitive,
            Primitive::Text(text)
                if text.text == "checkbox=false toggle=false radio=1 selected_row=2"
        )
    }));
}

#[test]
fn component_status_reflects_radio_click_same_frame() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(170.0, 252.0));

    assert_eq!(app.radio, 1);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Radio: Radio B")
    }));
}

#[test]
fn default_page_is_engine_editor_surface() {
    let app = ShowcaseApp::new();

    assert_eq!(app.page(), ShowcasePage::Editor);
    for label in ["Kinetik Forge", "Scene", "Viewport", "Inspector", "Console"] {
        assert!(
            app.primitives().iter().any(|primitive| {
                matches!(primitive, Primitive::Text(text) if text.text == label)
            }),
            "{label}"
        );
    }
}

#[test]
fn editor_file_menu_opens_dropdown_and_invokes_action() {
    let mut app = ShowcaseApp::new();

    click(&mut app, Point::new(145.0, 14.0));

    for label in ["New Scene", "Save Scene", "Export Build"] {
        assert!(
            app.primitives().iter().any(|primitive| {
                matches!(primitive, Primitive::Text(text) if text.text == label)
            }),
            "{label}"
        );
    }

    click(&mut app, Point::new(170.0, 93.0));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
    for label in ["Saved project snapshot", "Actions: 1"] {
        assert!(
            app.primitives().iter().any(|primitive| {
                matches!(primitive, Primitive::Text(text) if text.text == label)
            }),
            "{label}"
        );
    }
    assert!(!app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Save Scene")
    }));
}

#[test]
fn editor_shortcut_updates_visible_action_count_same_frame() {
    let mut app = ShowcaseApp::new();
    app.update_with_context(frame_context(
        Size::new(1440.0, 900.0),
        UiInput {
            keyboard: KeyboardInput {
                modifiers: Modifiers::new(false, true, false, false),
                events: vec![KeyEvent::new(
                    Key::Character("s".to_owned()),
                    KeyState::Pressed,
                    Modifiers::new(false, true, false, false),
                    false,
                )],
            },
            ..UiInput::default()
        },
    ));

    assert_eq!(app.action_count(), 1);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Actions: 1")
    }));
}

#[test]
fn editor_grid_toolbar_updates_status_same_frame_and_requests_repaint() {
    let mut app = ShowcaseApp::new();

    click(&mut app, Point::new(161.0, 45.0));

    assert_eq!(app.action_count(), 1);
    assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
    assert!(has_text(&app, "Viewport grid hidden"));
}

#[test]
fn editor_play_toolbar_updates_hint_same_frame() {
    let mut app = ShowcaseApp::new();

    click(&mut app, Point::new(1307.0, 45.0));

    assert_eq!(app.action_count(), 1);
    assert!(has_text(&app, "Play Mode: Running"));
    assert!(has_text(&app, "Play mode running"));
}

#[test]
fn editor_scene_add_requests_follow_up_repaint() {
    let mut app = ShowcaseApp::new();
    let add_node = app
        .output()
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Add node"))
        .expect("add node semantics")
        .bounds;

    click(
        &mut app,
        Point::new(
            add_node.x + add_node.width * 0.5,
            add_node.y + add_node.height * 0.5,
        ),
    );

    assert_eq!(app.output().repaint, RepaintRequest::NextFrame);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Create node requested")
    }));
}

#[test]
fn editor_resources_match_emitted_media_and_phosphor_atlas_icons() {
    let app = ShowcaseApp::new();
    let resources = app.render_resources();

    let primitives = app.primitives();
    let texture = app
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Texture(texture) => Some(texture.texture),
            _ => None,
        })
        .expect("editor emits viewport texture");

    assert!(resources.texture(texture).is_some());
    assert_eq!(
        resources.texture(texture).map(|resource| resource.sampling),
        Some(RenderImageSampling::Pixelated)
    );
    assert!(
        resources
            .texture(texture)
            .and_then(|resource| resource.snapshot.as_ref())
            .is_some_and(|snapshot| snapshot.width == 1280 && snapshot.height == 720)
    );
    let icon_images = primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Image(image)
                if phosphor_icons::ICON_ENTRIES
                    .iter()
                    .any(|entry| entry.image == image.image) =>
            {
                Some(image)
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(icon_images.len() >= 24);
    assert!(icon_images.iter().all(|image| image.tint.is_some()));
    assert!(
        icon_images
            .iter()
            .all(|image| resources.image(image.image).is_some())
    );
    assert!(
        primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Line(_) | Primitive::Path(_)))
    );
    assert!(!resources.snapshot().images.is_empty());
}

#[test]
fn editor_icons_are_registered_as_atlas_regions() {
    let app = ShowcaseApp::new();
    let resources = app.render_resources();

    for atlas in phosphor_icons::ICON_ATLASES {
        assert!(
            resources
                .image(atlas.image)
                .and_then(|resource| resource.pixels.as_ref())
                .is_some_and(|pixels| pixels.width == atlas.width && pixels.height == atlas.height),
            "missing atlas {}",
            atlas.physical_size
        );
    }
    let icon_regions = phosphor_icons::ICON_ENTRIES
        .iter()
        .filter_map(|entry| {
            resources
                .image(entry.image)
                .map(|resource| (entry, resource))
        })
        .filter(|(entry, resource)| {
            resource.pixels.is_none()
                && resource
                    .atlas_region
                    .is_some_and(|region| region.atlas == entry.atlas)
        })
        .count();

    assert_eq!(icon_regions, phosphor_icons::ICON_ENTRIES.len());
}

#[test]
fn generated_showcase_media_uses_intentional_sampling() {
    let resources = static_render_resources();

    for image in [ImageId::from_raw(7), ImageId::from_raw(11)] {
        assert_eq!(
            resources.image(image).map(|resource| resource.sampling),
            Some(RenderImageSampling::Pixelated),
            "{image:?}"
        );
    }

    for texture in [TextureId::from_raw(9_001), TextureId::from_raw(99)] {
        assert_eq!(
            resources.texture(texture).map(|resource| resource.sampling),
            Some(RenderImageSampling::Pixelated),
            "{texture:?}"
        );
    }

    assert_eq!(
        resources
            .texture(TextureId::from_raw(101))
            .map(|resource| resource.sampling),
        Some(RenderImageSampling::Smooth)
    );
}

#[test]
fn component_thumbnail_uses_native_pixel_rect() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    let thumbnail = app
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image) if image.image == ImageId::from_raw(7) => Some(image.rect),
            _ => None,
        })
        .expect("thumbnail image");
    let label = app
        .primitives()
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "Thumbnail" => Some(text.origin),
            _ => None,
        })
        .expect("thumbnail label");

    assert!((thumbnail.width - 64.0).abs() < f32::EPSILON);
    assert!((thumbnail.height - 48.0).abs() < f32::EPSILON);
    assert!(label.y > thumbnail.max_y());
}

#[test]
fn render_resources_reuse_static_media_and_append_text_layouts() {
    let app = ShowcaseApp::new();
    let static_snapshot = app.static_resources.snapshot();
    let fresh_static_snapshot = static_render_resources().snapshot();

    assert_eq!(static_snapshot, fresh_static_snapshot);
    assert!(!static_snapshot.images.is_empty());
    assert!(!static_snapshot.textures.is_empty());
    assert!(static_snapshot.text_layouts.is_empty());

    let frame_snapshot = app.render_resources().snapshot();
    assert_eq!(frame_snapshot.images, static_snapshot.images);
    assert_eq!(frame_snapshot.textures, static_snapshot.textures);
    assert!(!frame_snapshot.text_layouts.is_empty());
}

#[test]
fn render_resources_share_cached_static_texture_payloads() {
    let app = ShowcaseApp::new();
    let resources = app.render_resources();
    let static_texture = app
        .static_resources
        .texture(TextureId::from_raw(9_001))
        .and_then(|resource| resource.snapshot.as_ref())
        .expect("static editor texture");
    let frame_texture = resources
        .texture(TextureId::from_raw(9_001))
        .and_then(|resource| resource.snapshot.as_ref())
        .expect("frame editor texture");

    assert!(std::sync::Arc::ptr_eq(
        &static_texture.data,
        &frame_texture.data
    ));
}

#[test]
fn clicking_navigation_changes_page() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    let point = Point::new(620.0, 20.0);
    let visible_nav_id = WidgetId::from_key("root").child(("nav", ShowcasePage::Viewport as u8));

    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: true,
        ..ShowcaseInput::default()
    });

    assert_eq!(app.memory.pressed(), Some(visible_nav_id));

    app.update(&ShowcaseInput {
        mouse: Some(point),
        mouse_down: false,
        ..ShowcaseInput::default()
    });

    assert_eq!(app.page(), ShowcasePage::Viewport);
    assert!(has_text(&app, "Viewport, Texture, and Overlay Surface"));
    assert!(has_text(&app, "Page: Viewport"));
    assert!(!has_text(&app, "Component Gallery"));
}

#[test]
fn viewport_size_sets_logical_frame_context() {
    let mut app = ShowcaseApp::new();

    app.set_viewport_size(Size::new(720.0, 450.0));

    assert_eq!(app.viewport_size(), Size::new(720.0, 450.0));
    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.primitives().iter().any(|primitive| matches!(
        primitive,
        Primitive::Rect(rect) if rect.rect == Rect::new(0.0, 0.0, 720.0, 450.0)
    )));
}

#[test]
fn resized_hit_testing_uses_logical_coordinates() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    app.set_viewport_size(Size::new(720.0, 450.0));

    click(&mut app, Point::new(35.0, 77.0));
    assert_eq!(app.action_count(), 0);

    click(&mut app, Point::new(70.0, 154.0));

    assert_eq!(app.action_count(), 1);
}

#[test]
fn page_names_are_parseable_for_render_tools() {
    assert_eq!(
        ShowcaseApp::page_from_name("layout"),
        Some(ShowcasePage::Layout)
    );
    assert_eq!(ShowcaseApp::page_from_name("unknown"), None);
}

#[test]
fn showcase_docs_reach_s10_s13_review_matrix() {
    let matrix = include_str!("../../../../docs/catalogue-conformance-matrix.md");

    for slug in [
        "s10-outliner-tree-selection-semantics",
        "s10-asset-browser-grid-list-metadata",
        "s10-inline-edit-rename-lifecycle",
        "s10-collection-drag-drop-context",
        "s10-collection-filter-sort-selection-preservation",
        "s11-timeline-layout-coordinate-selection",
        "s11-ruler-ticks-timecode",
        "s11-transport-action-controls",
        "s11-timeline-snapping",
        "s11-timeline-preservation",
        "s12-viewport-surface-overlays",
        "s12-viewport-tools-transform-handles",
        "s12-viewport-action-routing",
        "s12-viewport-guides-rulers-safe-areas-hud",
        "s13-progress-indicator-metadata",
        "s13-job-list-progress-cancel",
        "s13-diagnostic-strip-codes-fields-ordering",
        "s13-feedback-stack-lifetime-repaint",
    ] {
        assert!(matrix.contains(slug), "{slug}");
    }

    for required in [
        "`Partial`",
        "Editor page",
        "Viewport page",
        "cargo run -p kinetik-ui-showcase -- --dump-review-artifacts s14-s10-s13-matrix",
        "Do not commit them as raster baselines",
    ] {
        assert!(matrix.contains(required), "{required}");
    }
}

#[test]
fn slider_drag_updates_value() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    app.update(&ShowcaseInput {
        mouse: Some(Point::new(360.0, 160.0)),
        mouse_down: true,
        ..ShowcaseInput::default()
    });
    app.update(&ShowcaseInput {
        mouse: Some(Point::new(600.0, 160.0)),
        mouse_down: true,
        ..ShowcaseInput::default()
    });

    assert!(app.strength() > 0.95);
}

#[test]
fn focused_search_accepts_keyboard_input() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(940.0, 160.0));
    app.update(&ShowcaseInput {
        typed: vec!['x'],
        ..ShowcaseInput::default()
    });

    assert!(app.search().ends_with('x'));
}

#[test]
fn focused_multi_line_field_accepts_text_and_enter() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);

    click(&mut app, Point::new(1070.0, 306.0));
    app.update(&ShowcaseInput {
        typed: vec!['x'],
        ..ShowcaseInput::default()
    });
    let actions_before_enter = app.action_count();
    app.update(&ShowcaseInput {
        enter: true,
        ..ShowcaseInput::default()
    });

    assert!(app.notes().contains('x'));
    assert!(app.notes().ends_with('\n'));
    assert_eq!(app.action_count(), actions_before_enter);
}

#[test]
fn viewport_buttons_change_zoom_state() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Viewport);

    click(&mut app, Point::new(1090.0, 240.0));
    assert!(app.zoom().abs() < f32::EPSILON);
    assert!(has_text(&app, "Zoom: 25%"));
    assert!((viewport_texture_rect(&app).width - 96.0).abs() < f32::EPSILON);

    click(&mut app, Point::new(1200.0, 240.0));
    assert!((app.zoom() - 0.2).abs() < f32::EPSILON);
    assert!(has_text(&app, "Zoom: 100%"));
    assert!((viewport_texture_rect(&app).width - 384.0).abs() < f32::EPSILON);
}

#[test]
fn viewport_page_structural_smoke_emits_texture_viewport_semantics_and_platform_requests() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Viewport);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 60);
    assert_eq!(
        count_primitives(&app, |primitive| matches!(primitive, Primitive::Texture(_))),
        2
    );
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Line(_))) >= 5);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Viewport, Texture, and Overlay Surface",
            "Viewport Controls",
            "Pan/Zoom Texture Surface",
            "3D/Video Boundary",
        ]
    ));
    assert!(has_text(
        &app,
        "Surface: 384x216 | Guides: 3 | Crosshair: 192,108"
    ));

    assert!(semantic_node(
        &app,
        &SemanticRole::Viewport,
        "Pan/Zoom Texture Surface"
    ));
    assert!(semantic_node(&app, &SemanticRole::Button, "Fit"));
    assert!(semantic_node(&app, &SemanticRole::Button, "Actual Size"));
    assert!(count_semantic_role(&app, &SemanticRole::Panel) >= 3);
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Viewport,
        &SemanticActionKind::Focus
    ));
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Slider,
        &SemanticActionKind::SetValue
    ));
    assert!(semantic_role_has_action(
        &app,
        &SemanticRole::Button,
        &SemanticActionKind::Invoke
    ));

    let resources = app.render_resources();
    assert!(resources.texture(TextureId::from_raw(99)).is_some());
    assert!(resources.texture(TextureId::from_raw(101)).is_some());

    app.update(&ShowcaseInput {
        mouse: Some(Point::new(1090.0, 240.0)),
        ..ShowcaseInput::default()
    });

    assert!(
        app.output()
            .platform_requests
            .contains(&PlatformRequest::SetCursor(
                kinetik_ui::core::CursorShape::PointingHand
            ))
    );
}

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
fn systems_palette_invokes_actions() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    click(&mut app, Point::new(930.0, 160.0));

    assert_eq!(app.action_count(), 1);
}

#[test]
fn systems_page_exposes_runtime_diagnostics() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    let has_snapshot = app.primitives().iter().any(
        |primitive| matches!(primitive, Primitive::Text(text) if text.text == "Runtime Snapshot"),
    );

    assert!(has_snapshot);
}

#[test]
fn systems_page_structural_smoke_emits_actions_overlays_palette_and_stress() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    assert_eq!(app.output().warnings, Vec::new());
    assert!(app.output().primitives.len() > 180);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Rect(_))) > 130);
    assert!(count_primitives(&app, |primitive| matches!(primitive, Primitive::Text(_))) > 20);
    assert!(
        count_primitives(&app, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 1
    );
    assert!(contains_text_in_order(
        &app,
        &[
            "Actions, Overlays, Diagnostics, Stress",
            "Action Router",
            "Overlay Stack",
            "Command Palette",
            "Primitive Stress",
            "Runtime Snapshot",
        ]
    ));

    assert!(semantic_node(&app, &SemanticRole::Button, "Dispatch"));
    assert!(semantic_node(&app, &SemanticRole::Button, "Menu Save"));
    assert!(semantic_node(&app, &SemanticRole::Menu, "Menu"));
    assert!(semantic_node(
        &app,
        &SemanticRole::CommandPalette,
        "Command Palette"
    ));
    assert!(semantic_node(
        &app,
        &SemanticRole::Custom("popover".to_owned()),
        "Popover"
    ));
    assert!(count_semantic_role(&app, &SemanticRole::ListItem) >= 3);
    assert!(app.output().semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Menu
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Dismiss)
    }));

    click(&mut app, Point::new(100.0, 210.0));

    assert_eq!(app.action_count(), 1);
    assert!(has_text(&app, "workspace.save via Menu (1)"));

    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Systems);

    click(&mut app, Point::new(930.0, 160.0));

    assert_eq!(app.action_count(), 1);
    assert!(has_text(&app, "workspace.save via CommandPalette (1)"));
}

#[test]
fn state_changes_produce_different_frames() {
    let mut app = ShowcaseApp::new();
    let before = crate::raster::rasterize(&app.primitives(), 1440, 900);

    click(&mut app, Point::new(70.0, 154.0));
    let after = crate::raster::rasterize(&app.primitives(), 1440, 900);

    assert_ne!(before.pixels, after.pixels);
}

#[test]
fn showcase_uses_widget_generated_primitives() {
    let app = ShowcaseApp::new();
    let primitives = app.primitives();

    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Texture(_)))
    );
    assert!(
        primitives
            .iter()
            .any(|item| matches!(item, Primitive::Line(_) | Primitive::Path(_)))
    );
    assert!(
        primitives
            .iter()
            .filter(|item| matches!(item, Primitive::Rect(_)))
            .count()
            > 20
    );
}

#[test]
fn showcase_app_does_not_define_fake_control_helpers() {
    let sources = [
        ("app.rs", include_str!("../app.rs")),
        ("app/runtime.rs", include_str!("runtime.rs")),
    ];

    for marker in [
        ["fn ", "button", "("].concat(),
        ["fn ", "slider", "("].concat(),
        ["fn ", "input_box", "("].concat(),
    ] {
        for (path, source) in sources {
            assert!(!source.contains(&marker), "{path}: {marker}");
        }
    }
}

#[test]
fn showcase_text_primitives_have_registered_layouts() {
    for page in [
        ShowcasePage::Editor,
        ShowcasePage::Components,
        ShowcasePage::Layout,
        ShowcasePage::Viewport,
        ShowcasePage::Systems,
    ] {
        let mut app = ShowcaseApp::new();
        app.set_page(page);
        let resources = app.render_resources();
        let mut text_count = 0;

        for primitive in app.primitives() {
            let Primitive::Text(text) = primitive else {
                continue;
            };
            text_count += 1;
            let layout = text
                .layout
                .unwrap_or_else(|| panic!("{page:?} text {:?} missing layout", text.text));
            assert!(
                resources.has_text_layout(layout),
                "{page:?} text {:?} references missing layout {layout:?}",
                text.text
            );
        }

        assert!(text_count > 0, "{page:?} emitted no text primitives");
    }
}

#[test]
fn showcase_pages_translate_to_vello_without_renderer_diagnostics() {
    for size in [Size::new(1440.0, 900.0), Size::new(820.0, 640.0)] {
        for page in [
            ShowcasePage::Editor,
            ShowcasePage::Components,
            ShowcasePage::Layout,
            ShowcasePage::Viewport,
            ShowcasePage::Systems,
        ] {
            let mut app = ShowcaseApp::new();
            app.set_viewport_size(size);
            app.set_page(page);
            let resources = app.render_resources();
            let mut renderer = VelloRenderer::new();
            let output = renderer.submit_frame(RenderFrameInput {
                viewport: test_viewport(size),
                primitives: &app.output().primitives,
                resources: &resources,
            });

            assert!(
                output.diagnostics.is_empty(),
                "{page:?} at {size:?}: {:?}",
                output.diagnostics
            );
        }
    }
}

#[test]
fn showcase_pages_snap_text_origins_and_baselines_at_fractional_dpi() {
    for (size, scale_factor) in [
        (Size::new(1151.2, 719.2), 1.25),
        (Size::new(960.7, 602.0), 1.5),
    ] {
        for page in [
            ShowcasePage::Editor,
            ShowcasePage::Components,
            ShowcasePage::Layout,
            ShowcasePage::Viewport,
            ShowcasePage::Systems,
        ] {
            let mut app = ShowcaseApp::new();
            app.set_viewport_size(size);
            app.set_page(page);
            let resources = app.render_resources();
            let mut renderer = VelloRenderer::new();
            let output = renderer.submit_frame(RenderFrameInput {
                viewport: test_viewport_scaled(size, scale_factor),
                primitives: &app.output().primitives,
                resources: &resources,
            });
            let encoding = renderer.scene().encoding();
            let glyphs = &encoding.resources.glyphs;

            assert!(
                output.diagnostics.is_empty(),
                "{page:?} at {size:?}: {:?}",
                output.diagnostics
            );
            assert!(
                !glyphs.is_empty(),
                "{page:?} at {size:?} emitted no glyphs at fractional DPI"
            );
            assert!(
                glyphs
                    .iter()
                    .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
                "{page:?} at {size:?} emitted fractional glyph x positions"
            );
            assert!(
                glyphs
                    .iter()
                    .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
                "{page:?} at {size:?} emitted fractional glyph baselines"
            );
        }
    }
}

#[test]
fn editor_open_menu_translates_to_vello_without_renderer_diagnostics() {
    let mut app = ShowcaseApp::new();
    click(&mut app, Point::new(145.0, 14.0));
    let resources = app.render_resources();
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: test_viewport(Size::new(1440.0, 900.0)),
        primitives: &app.output().primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}

fn test_viewport(size: Size) -> ViewportInfo {
    test_viewport_scaled(size, 1.0)
}

fn test_viewport_scaled(size: Size, scale_factor: f64) -> ViewportInfo {
    ViewportInfo::new(
        size,
        PhysicalSize::new(
            (f64::from(size.width) * scale_factor).round().max(1.0) as u32,
            (f64::from(size.height) * scale_factor).round().max(1.0) as u32,
        ),
        ScaleFactor::new(scale_factor),
    )
}
