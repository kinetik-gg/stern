#![allow(clippy::float_cmp)]

use std::sync::Arc;

use stern_core::{
    Brush, Color, CornerRadius, ImageId, PhysicalSize, Primitive, Rect, RectPrimitive, ScaleFactor,
    Size, ViewportInfo, default_dark_theme,
};
use stern_render::{
    RenderDiagnostic, RenderImage, RenderImageAlpha, RenderImageFormat, RenderResources,
};
use vello::peniko::{
    ImageAlphaType, ImageFormat, InterpolationAlphaSpace,
    color::{AlphaColor, ColorSpaceTag, HueDirection, Srgb},
};

use crate::{
    RenderCommandKind, RenderFrameInput, VelloRenderer,
    geometry::{vello_color, vello_gradient},
    image::{
        ImageDataCache, PackedTint, image_data_from_render_image, multiply_premultiplied_channel,
        tinted_image_data_from_render_image,
    },
    sanitize::sanitize_color,
    translate_primitives,
};

fn tint() -> Color {
    Color::rgba(64.0 / 255.0, 128.0 / 255.0, 192.0 / 255.0, 135.0 / 255.0)
}

fn render_image(format: RenderImageFormat, alpha: RenderImageAlpha, data: Vec<u8>) -> RenderImage {
    RenderImage::new(1, 1, data, format, alpha).expect("valid test image")
}

#[test]
fn sanitize_color_covers_range_nonfinite_and_negative_zero() {
    let mut diagnostics = Vec::new();
    let valid = sanitize_color(
        Color::rgba(-0.0, 0.0, 1.0, 0.5),
        &mut diagnostics,
        "valid_color",
    );
    assert!(diagnostics.is_empty());
    assert_eq!(valid, Color::rgba(0.0, 0.0, 1.0, 0.5));
    assert!(!valid.r.is_sign_negative());

    let invalid = sanitize_color(
        Color::rgba(-0.25, 1.25, f32::NAN, f32::NEG_INFINITY),
        &mut diagnostics,
        "invalid_color",
    );
    assert_eq!(invalid, Color::rgba(0.0, 1.0, 0.0, 0.0));
    assert_eq!(
        diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("invalid_color")]
    );

    let positive_infinity = sanitize_color(
        Color::rgba(f32::INFINITY, 0.25, 0.5, 0.75),
        &mut diagnostics,
        "positive_infinity",
    );
    assert_eq!(positive_infinity, Color::rgba(0.0, 0.25, 0.5, 0.75));
    assert_eq!(
        diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("invalid_color"),
            RenderDiagnostic::InvalidGeometry("positive_infinity"),
        ]
    );
}

#[test]
fn default_theme_accent_reaches_production_vello_encoding_exactly() {
    let accent = default_dark_theme().colors.accent.default;
    assert_eq!(accent, Color::rgb8(0x0C, 0x8C, 0xE9));
    let primitives = [Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 4.0, 4.0),
        fill: Some(Brush::Solid(accent)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];
    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Rect {
        fill: Some(Brush::Solid(command_color)),
        ..
    } = translation.commands[0].kind
    else {
        panic!("expected solid rectangle command");
    };
    assert_eq!(command_color, accent);
    assert_eq!(
        vello_color(command_color).components,
        [accent.r, accent.g, accent.b, accent.a]
    );

    let resources = RenderResources::new();
    let mut renderer = VelloRenderer::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(8.0, 8.0),
            PhysicalSize::new(8, 8),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert!(output.diagnostics.is_empty());
    let encoding = renderer.scene().encoding();
    assert_eq!(encoding.n_paths, 1);
    assert_eq!(encoding.draw_tags.len(), 1);
    assert_eq!(encoding.draw_tags[0].0, 0x44);
    assert_eq!(
        encoding.draw_data.as_slice(),
        &[u32::from_le_bytes([0x0C, 0x8C, 0xE9, 0xFF])]
    );
}

#[test]
fn peniko_mapping_is_explicit_srgb_with_premultiplied_interpolation() {
    let start = Color::rgba(1.0, 0.25, 0.0, 0.0);
    let end = Color::rgba(0.0, 0.5, 1.0, 1.0);
    let gradient = stern_core::LinearGradient::between(
        stern_core::Point::new(0.0, 0.0),
        stern_core::Point::new(10.0, 0.0),
        start,
        end,
    );

    let peniko = vello_gradient(&gradient);
    assert_eq!(peniko.interpolation_cs, ColorSpaceTag::Srgb);
    assert_eq!(
        peniko.interpolation_alpha_space,
        InterpolationAlphaSpace::Premultiplied
    );
    let stops: Vec<_> = peniko.stops.iter().collect();
    assert_eq!(stops.len(), 2);
    assert_eq!(stops[0].offset, 0.0);
    assert_eq!(stops[0].color.cs, ColorSpaceTag::Srgb);
    assert_eq!(stops[0].color.components, [1.0, 0.25, 0.0, 0.0]);
    assert_eq!(stops[1].offset, 1.0);
    assert_eq!(stops[1].color.components, [0.0, 0.5, 1.0, 1.0]);

    let midpoint = AlphaColor::<Srgb>::new([1.0, 0.0, 0.0, 0.0]).lerp(
        AlphaColor::<Srgb>::new([0.0, 0.0, 1.0, 1.0]),
        0.5,
        HueDirection::default(),
    );
    assert_eq!(midpoint.components, [0.0, 0.0, 1.0, 0.5]);
}

#[test]
fn straight_and_premultiplied_tints_have_exact_rgba_and_bgra_bytes() {
    let packed = PackedTint::from_color(tint());
    let cases = [
        (
            render_image(
                RenderImageFormat::Rgba8,
                RenderImageAlpha::Alpha,
                vec![64, 80, 96, 128],
            ),
            vec![16, 40, 72, 68],
            ImageFormat::Rgba8,
            ImageAlphaType::Alpha,
        ),
        (
            render_image(
                RenderImageFormat::Bgra8,
                RenderImageAlpha::Alpha,
                vec![96, 80, 64, 128],
            ),
            vec![72, 40, 16, 68],
            ImageFormat::Bgra8,
            ImageAlphaType::Alpha,
        ),
        (
            render_image(
                RenderImageFormat::Rgba8,
                RenderImageAlpha::Premultiplied,
                vec![64, 80, 96, 128],
            ),
            vec![9, 21, 38, 68],
            ImageFormat::Rgba8,
            ImageAlphaType::AlphaPremultiplied,
        ),
        (
            render_image(
                RenderImageFormat::Bgra8,
                RenderImageAlpha::Premultiplied,
                vec![96, 80, 64, 128],
            ),
            vec![38, 21, 9, 68],
            ImageFormat::Bgra8,
            ImageAlphaType::AlphaPremultiplied,
        ),
    ];

    for (source, expected, format, alpha_type) in cases {
        let source_bytes = Arc::clone(&source.data);
        let tinted = tinted_image_data_from_render_image(&source, packed);
        assert_eq!(tinted.data.data(), expected);
        assert_eq!(tinted.format, format);
        assert_eq!(tinted.alpha_type, alpha_type);
        assert_eq!(tinted.width, 1);
        assert_eq!(tinted.height, 1);
        assert_eq!(source.data.as_ref(), source_bytes.as_ref());
    }
}

#[test]
fn tint_identity_zero_alpha_and_one_round_witness_are_exact() {
    assert_eq!(multiply_premultiplied_channel(64, 64, 135), 9);

    for alpha in [RenderImageAlpha::Alpha, RenderImageAlpha::Premultiplied] {
        let source = RenderImage::new(
            2,
            1,
            vec![64, 80, 96, 0, 64, 80, 96, 255],
            RenderImageFormat::Rgba8,
            alpha,
        )
        .expect("valid edge-case image");
        let identity =
            tinted_image_data_from_render_image(&source, PackedTint::from_color(Color::WHITE));
        assert_eq!(identity.data.data(), source.data.as_ref());

        let transparent_tint =
            PackedTint::from_color(Color::rgba(64.0 / 255.0, 128.0 / 255.0, 192.0 / 255.0, 0.0));
        let transparent = tinted_image_data_from_render_image(&source, transparent_tint);
        let expected = if alpha == RenderImageAlpha::Alpha {
            vec![16, 40, 72, 0, 16, 40, 72, 0]
        } else {
            vec![0; 8]
        };
        assert_eq!(transparent.data.data(), expected);
    }
}

#[test]
fn tint_cache_invalidates_when_only_alpha_metadata_changes() {
    let payload: Arc<[u8]> = vec![64, 80, 96, 128].into();
    let straight = RenderImage {
        width: 1,
        height: 1,
        data: Arc::clone(&payload),
        format: RenderImageFormat::Rgba8,
        alpha: RenderImageAlpha::Alpha,
    };
    let premultiplied = RenderImage {
        alpha: RenderImageAlpha::Premultiplied,
        ..straight.clone()
    };
    assert!(Arc::ptr_eq(&straight.data, &premultiplied.data));

    let id = ImageId::from_raw(77);
    let mut cache = ImageDataCache::default();
    let first = cache.image_data_with_tint(id, &straight, Some(tint()));
    assert_eq!(first.data.data(), &[16, 40, 72, 68]);

    let second = cache.image_data_with_tint(id, &premultiplied, Some(tint()));
    assert_eq!(second.data.data(), &[9, 21, 38, 68]);
    assert_eq!(cache.tinted_images.len(), 1);

    let third = cache.image_data_with_tint(id, &premultiplied, Some(tint()));
    assert_eq!(third.data.data().as_ptr(), second.data.data().as_ptr());
}

#[test]
fn texture_upload_preserves_format_alpha_dimensions_and_bytes() {
    for (format, alpha, expected_format, expected_alpha) in [
        (
            RenderImageFormat::Rgba8,
            RenderImageAlpha::Alpha,
            ImageFormat::Rgba8,
            ImageAlphaType::Alpha,
        ),
        (
            RenderImageFormat::Rgba8,
            RenderImageAlpha::Premultiplied,
            ImageFormat::Rgba8,
            ImageAlphaType::AlphaPremultiplied,
        ),
        (
            RenderImageFormat::Bgra8,
            RenderImageAlpha::Alpha,
            ImageFormat::Bgra8,
            ImageAlphaType::Alpha,
        ),
        (
            RenderImageFormat::Bgra8,
            RenderImageAlpha::Premultiplied,
            ImageFormat::Bgra8,
            ImageAlphaType::AlphaPremultiplied,
        ),
    ] {
        let image = RenderImage::new(2, 1, vec![1, 2, 3, 4, 5, 6, 7, 8], format, alpha)
            .expect("valid texture snapshot");
        let upload = image_data_from_render_image(&image);
        assert_eq!(upload.data.data(), image.data.as_ref());
        assert_eq!(upload.format, expected_format);
        assert_eq!(upload.alpha_type, expected_alpha);
        assert_eq!((upload.width, upload.height), (2, 1));
    }
}
