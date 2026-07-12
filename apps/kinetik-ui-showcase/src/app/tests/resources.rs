use super::helpers::{
    ImageId, Primitive, RenderImageSampling, ShowcaseApp, ShowcasePage, TextureId,
    static_render_resources,
};
use std::sync::Arc;

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
    let fresh_static_snapshot = static_render_resources().snapshot();
    let frame_snapshot = app.render_resources().snapshot();

    assert!(!fresh_static_snapshot.images.is_empty());
    assert!(!fresh_static_snapshot.textures.is_empty());
    assert!(fresh_static_snapshot.text_layouts.is_empty());
    assert_eq!(frame_snapshot.images, fresh_static_snapshot.images);
    assert_eq!(frame_snapshot.textures, fresh_static_snapshot.textures);
    assert!(!frame_snapshot.text_layouts.is_empty());
}

#[test]
fn render_resources_share_cached_static_texture_payloads() {
    let app = ShowcaseApp::new();
    let resources = app.render_resources();
    let static_texture = app
        .render_resources
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
fn repeated_resource_access_and_noop_frame_preserve_registry_keys_and_arcs() {
    let mut app = ShowcaseApp::new();
    assert!(std::ptr::eq(app.render_resources(), app.render_resources()));
    assert!(app.text_resource_sync.is_initialized());
    let id = app.text_layouts.layouts().next().expect("layout").id;
    let resource = app
        .render_resources()
        .text_layout_resource(id)
        .expect("resource");
    let text_pointer = resource.key.text.as_ptr();
    let family_pointer = resource.key.style.family.as_ptr();
    let layout = Arc::clone(&resource.layout);
    let texture = Arc::clone(
        &app.render_resources()
            .texture(TextureId::from_raw(9_001))
            .and_then(|resource| resource.snapshot.as_ref())
            .expect("static texture")
            .data,
    );

    app.redraw_idle();

    let resource = app
        .render_resources()
        .text_layout_resource(id)
        .expect("resource");
    assert_eq!(resource.key.text.as_ptr(), text_pointer);
    assert_eq!(resource.key.style.family.as_ptr(), family_pointer);
    assert!(Arc::ptr_eq(&resource.layout, &layout));
    assert!(Arc::ptr_eq(
        &texture,
        &app.render_resources()
            .texture(TextureId::from_raw(9_001))
            .and_then(|resource| resource.snapshot.as_ref())
            .expect("static texture")
            .data
    ));
    assert_eq!(
        app.render_resources().retained_text_layout_payload_bytes(),
        Some(app.text_layouts.retained_payload_bytes())
    );
}

#[test]
fn page_exclusive_text_layout_expires_from_store_and_persistent_resources() {
    let mut app = ShowcaseApp::new();
    app.set_page(ShowcasePage::Components);
    let thumbnail = app
        .output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(text) if text.text == "Thumbnail" => text.layout,
            _ => None,
        })
        .expect("component-exclusive Thumbnail layout");
    assert!(app.text_layouts.layout(thumbnail).is_some());
    assert!(app.render_resources().has_text_layout(thumbnail));

    app.set_page(ShowcasePage::Layout);
    for _ in 0..121 {
        app.redraw_idle();
    }

    assert!(app.text_layouts.layout(thumbnail).is_none());
    assert!(!app.render_resources().has_text_layout(thumbnail));
    let resource_ids = app
        .render_resources()
        .snapshot()
        .text_layouts
        .into_iter()
        .map(|resource| resource.id)
        .collect::<Vec<_>>();
    let store_ids = app
        .text_layouts
        .layouts()
        .map(|entry| entry.id.raw())
        .collect::<Vec<_>>();
    assert_eq!(resource_ids, store_ids);
    assert_eq!(
        app.render_resources().retained_text_layout_payload_bytes(),
        Some(app.text_layouts.retained_payload_bytes())
    );
    assert!(app.text_layouts.retained_payload_bytes() <= 32 * 1024 * 1024);
    for primitive in &app.output.primitives {
        if let Primitive::Text(text) = primitive {
            let id = text.layout.expect("showcase text layout");
            assert!(app.render_resources().has_text_layout(id));
        }
    }
}

#[test]
fn one_thousand_page_frames_keep_text_resources_exact_and_bounded() {
    let mut app = ShowcaseApp::new();
    for index in 0..1_000 {
        app.set_page(ShowcasePage::ALL[index % ShowcasePage::ALL.len()]);
        let resource_ids = app
            .render_resources()
            .snapshot()
            .text_layouts
            .into_iter()
            .map(|resource| resource.id)
            .collect::<Vec<_>>();
        let store_ids = app
            .text_layouts
            .layouts()
            .map(|entry| entry.id.raw())
            .collect::<Vec<_>>();
        assert_eq!(resource_ids, store_ids);
        assert_eq!(
            app.render_resources().retained_text_layout_payload_bytes(),
            Some(app.text_layouts.retained_payload_bytes())
        );
        assert!(app.text_layouts.retained_payload_bytes() <= 32 * 1024 * 1024);
        for primitive in &app.output.primitives {
            if let Primitive::Text(text) = primitive {
                let id = text.layout.expect("showcase text layout");
                assert!(app.render_resources().has_text_layout(id));
            }
        }
    }
}
