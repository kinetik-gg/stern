use super::helpers::{
    Point, ShowcaseApp, ShowcaseInput, ShowcasePage, click, has_text, viewport_texture_rect,
};

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
