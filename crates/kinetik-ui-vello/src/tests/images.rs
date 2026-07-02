use super::common::{assert_approx64, atlas_resources};
use crate::{
    ImageDataCache, MAX_CACHED_IMAGE_ENTRIES, MAX_CACHED_TEXTURE_ENTRIES,
    MAX_CACHED_TINTED_IMAGE_BYTES, MAX_TINTED_IMAGE_CACHE_ENTRIES, PackedTint, RenderFrameInput,
    RenderImage, RenderImageSampling, RenderResources, TextureResource, VelloRenderer,
    image_quality, image_region_transform, root_transform, snapped_image_region_transform,
};
use kinetik_ui_core::render::TexturePrimitive;
use kinetik_ui_core::{
    Color, ImageId, ImagePrimitive, Primitive, Rect, ScaleFactor, Size, TextureId, ViewportInfo,
};
use vello::kurbo::{Affine, Point as KurboPoint};
use vello::peniko::ImageQuality;

#[test]
fn image_sampling_maps_to_vello_quality() {
    assert_eq!(
        image_quality(RenderImageSampling::Pixelated),
        ImageQuality::Low
    );
    assert_eq!(
        image_quality(RenderImageSampling::UiIcon),
        ImageQuality::Low
    );
    assert_eq!(
        image_quality(RenderImageSampling::Smooth),
        ImageQuality::Medium
    );
    assert_eq!(
        image_quality(RenderImageSampling::HighQuality),
        ImageQuality::High
    );
}

#[test]
fn native_size_image_regions_keep_atlas_pixels_at_native_scale() {
    let source = Rect::new(33.0, 34.0, 32.0, 32.0);
    let rect = Rect::new(101.0, 103.0, 32.0, 32.0);
    let transform = image_region_transform(Affine::IDENTITY, rect, source);

    let coeffs = transform.as_coeffs();
    assert_approx64(coeffs[0], 1.0);
    assert_approx64(coeffs[1], 0.0);
    assert_approx64(coeffs[2], 0.0);
    assert_approx64(coeffs[3], 1.0);
    assert_approx64(coeffs[4], 68.0);
    assert_approx64(coeffs[5], 69.0);
}

#[test]
fn native_size_image_regions_only_apply_root_scale_once() {
    let source = Rect::new(33.0, 34.0, 32.0, 32.0);
    let rect = Rect::new(101.0, 103.0, 32.0, 32.0);
    let transform = image_region_transform(root_transform(1.25), rect, source);

    let coeffs = transform.as_coeffs();
    assert_approx64(coeffs[0], 1.25);
    assert_approx64(coeffs[1], 0.0);
    assert_approx64(coeffs[2], 0.0);
    assert_approx64(coeffs[3], 1.25);
    assert_approx64(coeffs[4], 85.0);
    assert_approx64(coeffs[5], 86.25);
}

#[test]
fn snapped_image_regions_place_atlas_origin_on_physical_pixels() {
    let source = Rect::new(33.0, 34.0, 32.0, 32.0);
    let rect = Rect::new(101.0, 103.0, 32.0, 32.0);
    let transform = snapped_image_region_transform(
        root_transform(1.25),
        rect,
        source,
        RenderImageSampling::UiIcon,
        1.25,
    );

    let mapped = transform * KurboPoint::new(f64::from(source.x), f64::from(source.y));
    assert!((mapped.x - mapped.x.round()).abs() < 0.000_01);
    assert!((mapped.y - mapped.y.round()).abs() < 0.000_01);
    assert!((mapped.x - 126.0).abs() < 0.000_01);
    assert!((mapped.y - 129.0).abs() < 0.000_01);
}

#[test]
fn scaled_image_regions_encode_explicit_destination_scale() {
    let source = Rect::new(8.0, 12.0, 32.0, 16.0);
    let rect = Rect::new(20.0, 30.0, 64.0, 24.0);
    let transform = image_region_transform(Affine::IDENTITY, rect, source);

    let coeffs = transform.as_coeffs();
    assert_approx64(coeffs[0], 2.0);
    assert_approx64(coeffs[1], 0.0);
    assert_approx64(coeffs[2], 0.0);
    assert_approx64(coeffs[3], 1.5);
    assert_approx64(coeffs[4], 4.0);
    assert_approx64(coeffs[5], 12.0);
}

#[test]
fn frame_submission_encodes_atlas_backed_image_resource() {
    let mut renderer = VelloRenderer::new();
    let resources = atlas_resources();
    let primitives = vec![Primitive::Image(ImagePrimitive {
        image: ImageId::from_raw(3),
        rect: Rect::new(4.0, 4.0, 16.0, 16.0),
        tint: None,
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    assert!(!renderer.scene().encoding().is_empty());
    assert!(!renderer.scene().encoding().resources.patches.is_empty());
}

#[test]
fn frame_submission_reuses_cached_atlas_payload_for_regions() {
    let mut renderer = VelloRenderer::new();
    let resources = atlas_resources();
    let primitives = vec![
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(3),
            rect: Rect::new(4.0, 4.0, 16.0, 16.0),
            tint: None,
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(4),
            rect: Rect::new(24.0, 4.0, 16.0, 16.0),
            tint: None,
        }),
    ];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_image_count(), 1);
}

#[test]
fn image_cache_uses_shared_payload_identity_for_hits() {
    let id = ImageId::from_raw(11);
    let image = RenderImage::rgba8(2, 2, vec![1; 16]).expect("valid image");
    let clone = image.clone();
    let replacement = RenderImage::rgba8(2, 2, vec![2; 16]).expect("valid image");
    let mut cache = ImageDataCache::default();

    cache.image_data(id, &image);
    let cached_payload = cache
        .images
        .get(&id)
        .expect("cache entry")
        .signature
        .data
        .clone();
    cache.image_data(id, &clone);
    assert!(std::sync::Arc::ptr_eq(
        &cached_payload,
        &cache.images.get(&id).expect("cache entry").signature.data
    ));

    cache.image_data(id, &replacement);
    let replaced_payload = &cache.images.get(&id).expect("cache entry").signature.data;
    assert!(std::sync::Arc::ptr_eq(replaced_payload, &replacement.data));
    assert!(!std::sync::Arc::ptr_eq(&cached_payload, replaced_payload));
}

#[test]
fn tinted_image_cache_reuses_payload_for_same_color() {
    let id = ImageId::from_raw(12);
    let image = RenderImage::rgba8(2, 2, vec![255; 16]).expect("valid image");
    let mut cache = ImageDataCache::default();

    cache.image_data_with_tint(id, &image, Some(Color::rgb(1.0, 0.0, 0.0)));
    cache.image_data_with_tint(id, &image, Some(Color::rgb(1.0, 0.0, 0.0)));
    assert_eq!(cache.images.len(), 0);
    assert_eq!(cache.tinted_images.len(), 1);

    cache.image_data_with_tint(id, &image, Some(Color::rgb(0.0, 1.0, 0.0)));
    assert_eq!(cache.tinted_images.len(), 2);
}

#[test]
fn tinted_image_cache_does_not_retain_large_payloads() {
    let id = ImageId::from_raw(13);
    let byte_len = MAX_CACHED_TINTED_IMAGE_BYTES + 4;
    let pixel_count = byte_len / 4;
    let width = u32::try_from(pixel_count).expect("test image width fits u32");
    let image = RenderImage::rgba8(width, 1, vec![255; pixel_count * 4]).expect("valid image");
    let mut cache = ImageDataCache::default();

    cache.image_data_with_tint(id, &image, Some(Color::rgb(1.0, 0.0, 0.0)));

    assert_eq!(cache.tinted_images.len(), 0);
}

#[test]
fn image_cache_evicts_least_recent_entry_at_capacity() {
    let image = RenderImage::rgba8(2, 2, vec![1; 16]).expect("valid image");
    let first = ImageId::from_raw(1);
    let second = ImageId::from_raw(2);
    let mut cache = ImageDataCache::default();

    for raw in 1..=MAX_CACHED_IMAGE_ENTRIES {
        cache.image_data(
            ImageId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
        );
    }
    cache.image_data(first, &image);
    cache.image_data(
        ImageId::from_raw(u64::try_from(MAX_CACHED_IMAGE_ENTRIES + 1).expect("cache id fits u64")),
        &image,
    );

    assert_eq!(cache.images.len(), MAX_CACHED_IMAGE_ENTRIES);
    assert!(cache.images.contains_key(&first));
    assert!(!cache.images.contains_key(&second));
}

#[test]
fn tinted_image_cache_evicts_one_old_entry_at_capacity() {
    let image = RenderImage::rgba8(2, 2, vec![255; 16]).expect("valid image");
    let first = ImageId::from_raw(1);
    let second = ImageId::from_raw(2);
    let tint_color = Color::rgb(1.0, 0.0, 0.0);
    let tint = Some(tint_color);
    let tint_key = PackedTint::from_color(tint_color);
    let mut cache = ImageDataCache::default();

    for raw in 1..=MAX_TINTED_IMAGE_CACHE_ENTRIES {
        cache.image_data_with_tint(
            ImageId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
            tint,
        );
    }
    cache.image_data_with_tint(first, &image, tint);
    cache.image_data_with_tint(
        ImageId::from_raw(
            u64::try_from(MAX_TINTED_IMAGE_CACHE_ENTRIES + 1).expect("cache id fits u64"),
        ),
        &image,
        tint,
    );

    assert_eq!(cache.tinted_images.len(), MAX_TINTED_IMAGE_CACHE_ENTRIES);
    assert!(cache.tinted_images.contains_key(&(first, tint_key)));
    assert!(!cache.tinted_images.contains_key(&(second, tint_key)));
}

#[test]
fn frame_submission_reuses_cached_texture_snapshot_payload() {
    let texture = TextureId::from_raw(77);
    let snapshot = RenderImage::rgba8(4, 4, vec![64; 64]).expect("valid texture snapshot");
    let mut resources = RenderResources::new();
    resources.register_texture(TextureResource {
        id: texture,
        size: Size::new(4.0, 4.0),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(snapshot),
    });
    let primitives = vec![Primitive::Texture(TexturePrimitive {
        texture,
        rect: Rect::new(4.0, 4.0, 32.0, 32.0),
        source_size: Size::new(4.0, 4.0),
    })];
    let viewport = ViewportInfo::new(
        Size::new(100.0, 100.0),
        kinetik_ui_core::PhysicalSize::new(100, 100),
        ScaleFactor::ONE,
    );
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_texture_count(), 1);

    let output = renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_texture_count(), 1);
    assert_eq!(renderer.cached_image_count(), 0);
}

#[test]
fn texture_cache_evicts_least_recent_entry_at_capacity() {
    let image = RenderImage::rgba8(2, 2, vec![1; 16]).expect("valid texture");
    let first = TextureId::from_raw(1);
    let second = TextureId::from_raw(2);
    let mut cache = ImageDataCache::default();

    for raw in 1..=MAX_CACHED_TEXTURE_ENTRIES {
        cache.texture_data(
            TextureId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
        );
    }
    cache.texture_data(first, &image);
    cache.texture_data(
        TextureId::from_raw(
            u64::try_from(MAX_CACHED_TEXTURE_ENTRIES + 1).expect("cache id fits u64"),
        ),
        &image,
    );

    assert_eq!(cache.textures.len(), MAX_CACHED_TEXTURE_ENTRIES);
    assert!(cache.textures.contains_key(&first));
    assert!(!cache.textures.contains_key(&second));
}
