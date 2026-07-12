use super::helpers::{
    Point, Primitive, RenderFrameInput, ShowcaseApp, ShowcasePage, Size, VelloRenderer, click,
    test_viewport, test_viewport_scaled,
};

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
                resources,
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
                resources,
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
fn showcase_registered_text_resolves_at_exact_one_point_seven_five_scale() {
    let size = Size::new(800.0, 600.0);
    let viewport = test_viewport_scaled(size, 1.75);
    assert_eq!(viewport.physical_size.width, 1_400);
    assert_eq!(viewport.physical_size.height, 1_050);

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
        let text = app
            .output()
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Text(text) => Some(text),
                _ => None,
            })
            .collect::<Vec<_>>();

        assert!(!text.is_empty(), "{page:?} emitted no text primitives");
        assert!(text.iter().all(|text| {
            text.layout
                .is_some_and(|layout| resources.has_text_layout(layout))
        }));

        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport,
            primitives: &app.output().primitives,
            resources,
        });

        assert!(
            output.diagnostics.is_empty(),
            "{page:?} at 800x600 logical / 1400x1050 physical: {:?}",
            output.diagnostics
        );
        assert!(
            !renderer.scene().encoding().resources.glyph_runs.is_empty(),
            "{page:?} emitted no registered Vello glyph run"
        );
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
        resources,
    });

    assert!(output.diagnostics.is_empty(), "{:?}", output.diagnostics);
}
