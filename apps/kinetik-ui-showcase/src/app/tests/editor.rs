use super::helpers::{
    Key, KeyEvent, KeyState, KeyboardInput, Modifiers, Point, Primitive, RenderImageSampling,
    RepaintRequest, ShowcaseApp, ShowcasePage, Size, UiInput, click, frame_context, has_text,
    phosphor_icons,
};

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
fn showcase_action_truth_editor_file_menu_disables_unfinished_actions() {
    let mut app = ShowcaseApp::new();

    click(&mut app, Point::new(145.0, 14.0));

    for label in [
        "New Scene (Experimental)",
        "Save Scene (Experimental)",
        "Export Build (Experimental)",
    ] {
        assert!(
            app.primitives().iter().any(|primitive| {
                matches!(primitive, Primitive::Text(text) if text.text == label)
            }),
            "{label}"
        );
    }

    click(&mut app, Point::new(170.0, 93.0));

    assert_eq!(app.action_count(), 0);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Save Scene (Experimental)")
    }));
}

#[test]
fn showcase_action_truth_unfinished_editor_shortcut_does_not_invoke() {
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

    assert_eq!(app.action_count(), 0);
    assert!(app.primitives().iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "Actions: 0")
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
