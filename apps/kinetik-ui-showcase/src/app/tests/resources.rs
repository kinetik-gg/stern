use super::helpers::{
    ImageId, Primitive, RenderImageSampling, ShowcaseApp, ShowcasePage, TextureId,
    static_render_resources,
};

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
