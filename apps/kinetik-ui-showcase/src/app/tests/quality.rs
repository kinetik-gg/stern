use super::helpers::{Primitive, ShowcaseApp, ShowcasePage};

#[test]
fn showcase_app_does_not_define_fake_control_helpers() {
    let sources = [
        ("app.rs", include_str!("../../app.rs")),
        ("app/runtime.rs", include_str!("../runtime.rs")),
        (
            "app/runtime/actions.rs",
            include_str!("../runtime/actions.rs"),
        ),
        (
            "app/runtime/chrome.rs",
            include_str!("../runtime/chrome.rs"),
        ),
        (
            "app/runtime/components.rs",
            include_str!("../runtime/components.rs"),
        ),
        (
            "app/runtime/layout.rs",
            include_str!("../runtime/layout.rs"),
        ),
        (
            "app/runtime/lifecycle.rs",
            include_str!("../runtime/lifecycle.rs"),
        ),
        (
            "app/runtime/systems.rs",
            include_str!("../runtime/systems.rs"),
        ),
        (
            "app/runtime/viewport.rs",
            include_str!("../runtime/viewport.rs"),
        ),
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
