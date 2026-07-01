use super::{
    ImageAtlasRegion, ImageDataCache, ImageResource, PackedTint, RenderCommand, RenderCommandKind,
    RenderDiagnostic, RenderFrameInput, RenderImage, RenderImageSampling, RenderResources,
    RendererBackend, ShapedTextCache, TextLayoutResource, TextureResource, VelloRenderer,
    VelloRendererError, crisp_rect_border_segments, image_quality, image_region_transform,
    physical_text_layout, physical_text_layout_for_key, quantize_physical_text_extent,
    quantize_stroke_width_to_device, render_translation_snapshot, root_transform,
    snap_axis_aligned_translation, snap_filled_path_elements_to_device, snap_image_rect_to_device,
    snap_point_to_device, snap_radius_to_device, snap_rect_to_device, snap_stroke_center_to_device,
    snap_stroked_line_to_device, snap_stroked_path_elements_to_device, snap_stroked_rect_to_device,
    snap_text_glyph_baseline_to_device, snap_text_glyph_position_to_device,
    snap_text_origin_to_device, snap_text_transform_origin_to_device,
    snapped_image_region_transform, transform_point, translate_primitives, viewport_device_scale,
    viewport_size_device_scale,
};
use kinetik_ui_core::render::TexturePrimitive;
use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, GradientStop, ImageId, ImagePrimitive, LayerId,
    LinePrimitive, LinearGradient, PathElement, PathPrimitive, Point, Primitive, Rect,
    RectPrimitive, ScaleFactor, ShadowPrimitive, Size, Stroke, TextLayoutId, TextPrimitive,
    TextureId, Transform, Vec2, ViewportInfo,
};
use kinetik_ui_text::{
    CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle, fonts,
};
use vello::{
    kurbo::{Affine, Point as KurboPoint},
    peniko::ImageQuality,
};

fn resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        pixels: Some(tiny_image()),
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(2),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        snapshot: Some(tiny_image()),
    });
    resources
}

fn size_only_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(64.0, 64.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(2),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        snapshot: None,
    });
    resources
}

fn atlas_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        pixels: Some(tiny_image()),
        atlas_region: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(3),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(1.0, 0.0, 1.0, 1.0),
        }),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(4),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(0.0, 1.0, 1.0, 1.0),
        }),
    });
    resources
}

fn tiny_image() -> RenderImage {
    RenderImage::rgba8(
        2,
        2,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ],
    )
    .expect("valid tiny image")
}

fn one_pixel_image() -> RenderImage {
    RenderImage::rgba8(1, 1, vec![255, 255, 255, 255]).expect("valid one pixel image")
}

fn text_layout_resource(id: TextLayoutId, text: &str) -> TextLayoutResource {
    let mut engine = CosmicTextEngine::new();
    let key = TextLayoutKey::new(text, TextStyle::new("sans-serif", 12.0, 16.0), 200.0, false);
    let layout = engine.shape_text(&key);
    TextLayoutResource {
        id,
        key,
        layout: std::sync::Arc::new(layout),
    }
}

fn shaped_glyph_x_positions(
    layout: &ShapedTextLayout,
    snapped_origin_x: f32,
    scale: f32,
) -> Vec<f32> {
    layout
        .runs
        .iter()
        .flat_map(|run| run.glyphs.iter())
        .map(|glyph| snapped_origin_x + glyph.x * scale)
        .collect()
}

fn clip_rects(command: &RenderCommand) -> Vec<Rect> {
    command.clips.iter().map(|clip| clip.rect).collect()
}

fn clip_transforms(command: &RenderCommand) -> Vec<Transform> {
    command.clips.iter().map(|clip| clip.transform).collect()
}

fn assert_approx(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() < f32::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

fn assert_approx64(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < f64::EPSILON,
        "expected {actual} to equal {expected}"
    );
}

#[test]
fn translates_rectangles_and_lines_in_order() {
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(10.0, 10.0),
            stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(matches!(
        translation.commands[0].kind,
        RenderCommandKind::Rect { .. }
    ));
    assert!(matches!(
        translation.commands[1].kind,
        RenderCommandKind::Line { .. }
    ));
}

#[test]
fn translates_paths_in_order() {
    let primitives = vec![Primitive::Path(PathPrimitive::new(
        vec![
            PathElement::MoveTo(Point::new(0.0, 0.0)),
            PathElement::LineTo(Point::new(10.0, 0.0)),
            PathElement::QuadTo {
                ctrl: Point::new(12.0, 4.0),
                to: Point::new(10.0, 8.0),
            },
            PathElement::CubicTo {
                ctrl1: Point::new(8.0, 10.0),
                ctrl2: Point::new(2.0, 10.0),
                to: Point::new(0.0, 8.0),
            },
            PathElement::Close,
        ],
        Some(Brush::Solid(Color::WHITE)),
        Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Path {
        elements,
        fill,
        stroke,
    } = &translation.commands[0].kind
    else {
        panic!("expected path command");
    };
    assert_eq!(elements.len(), 5);
    assert_eq!(*fill, Some(Brush::Solid(Color::WHITE)));
    assert_eq!(*stroke, Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))));
}

#[test]
fn translates_linear_gradient_brushes() {
    let gradient = LinearGradient::from_colors(
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        &[Color::BLACK, Color::rgb(0.5, 0.5, 0.5), Color::WHITE],
    )
    .expect("valid gradient");
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: Some(Stroke::new(1.0, Brush::LinearGradient(gradient))),
        radius: CornerRadius::all(2.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Rect { fill, stroke, .. } = &translation.commands[0].kind else {
        panic!("expected rect command");
    };
    assert_eq!(*fill, Some(Brush::LinearGradient(gradient)));
    assert_eq!(
        *stroke,
        Some(Stroke::new(1.0, Brush::LinearGradient(gradient)))
    );
}

#[test]
fn translates_shadows_in_order() {
    let shadow = ShadowPrimitive::new(
        Rect::new(2.0, 4.0, 20.0, 12.0),
        Vec2::new(1.0, 3.0),
        8.0,
        2.0,
        5.0,
        Color::rgba(0.0, 0.0, 0.0, 0.35),
    );
    let primitives = vec![Primitive::Shadow(shadow)];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    let RenderCommandKind::Shadow {
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    } = &translation.commands[0].kind
    else {
        panic!("expected shadow command");
    };
    assert_eq!(*rect, shadow.rect);
    assert_eq!(*offset, shadow.offset);
    assert_approx(*blur_radius, 8.0);
    assert_approx(*spread, 2.0);
    assert_approx(*radius, 5.0);
    assert_eq!(*color, shadow.color);
}

#[test]
fn sanitizes_linear_gradient_stops_before_encoding() {
    let gradient = LinearGradient::new(
        Point::new(0.0, 0.0),
        Point::new(20.0, 0.0),
        &[
            GradientStop::new(1.0, Color::WHITE),
            GradientStop::new(f32::NAN, Color::rgba(f32::NAN, 0.25, 0.5, 1.0)),
            GradientStop::new(-0.25, Color::BLACK),
        ],
    )
    .expect("valid stop count");
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
        ]
    );
    let RenderCommandKind::Rect {
        fill: Some(Brush::LinearGradient(gradient)),
        ..
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized gradient fill");
    };
    assert_approx(gradient.stops()[0].offset, 0.0);
    assert_approx(gradient.stops()[1].offset, 0.0);
    assert_approx(gradient.stops()[2].offset, 1.0);
    assert_eq!(gradient.stops()[0].color, Color::rgba(0.0, 0.25, 0.5, 1.0));
}

#[test]
fn invalid_linear_gradient_endpoint_falls_back_to_solid_brush() {
    let gradient = LinearGradient::between(
        Point::new(f32::NAN, 0.0),
        Point::new(20.0, 0.0),
        Color::WHITE,
        Color::BLACK,
    );
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        fill: Some(Brush::LinearGradient(gradient)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("rect_fill")]
    );
    let RenderCommandKind::Rect {
        fill: Some(Brush::Solid(color)),
        ..
    } = &translation.commands[0].kind
    else {
        panic!("expected solid fallback");
    };
    assert_eq!(*color, Color::WHITE);
}

#[test]
fn invalid_shadow_geometry_is_diagnosed_and_sanitized() {
    let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
        Rect::new(f32::NAN, 2.0, 20.0, 12.0),
        Vec2::new(f32::NAN, 3.0),
        -4.0,
        f32::NAN,
        -2.0,
        Color::rgba(f32::NAN, 0.0, 0.0, 0.25),
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("shadow"),
            RenderDiagnostic::InvalidGeometry("shadow_offset"),
            RenderDiagnostic::InvalidGeometry("shadow_blur"),
            RenderDiagnostic::InvalidGeometry("shadow_spread"),
            RenderDiagnostic::InvalidGeometry("shadow_radius"),
            RenderDiagnostic::InvalidGeometry("shadow_color"),
        ]
    );
    let RenderCommandKind::Shadow {
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized shadow");
    };
    assert_approx(rect.x, 0.0);
    assert_eq!(*offset, Vec2::new(0.0, 3.0));
    assert_approx(*blur_radius, 0.0);
    assert_approx(*spread, 0.0);
    assert_approx(*radius, 0.0);
    assert_approx(color.r, 0.0);
}

#[test]
fn shadow_spread_that_erases_rect_is_diagnosed_and_skipped() {
    let primitives = vec![Primitive::Shadow(ShadowPrimitive::new(
        Rect::new(0.0, 0.0, 10.0, 10.0),
        Vec2::ZERO,
        0.0,
        -6.0,
        0.0,
        Color::BLACK,
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("shadow_spread")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn invalid_geometry_is_diagnosed_and_sanitized_before_encoding() {
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, -10.0, 10.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(f32::NAN, 0.0),
            to: Point::new(10.0, 10.0),
            stroke: Stroke::new(1.0, Brush::Solid(Color::WHITE)),
        }),
        Primitive::Path(PathPrimitive::new(
            vec![PathElement::MoveTo(Point::new(f32::NAN, 0.0))],
            Some(Brush::Solid(Color::WHITE)),
            None,
        )),
        Primitive::ClipBegin {
            id: ClipId::from_raw(9),
            rect: Rect::new(0.0, 0.0, f32::NAN, 10.0),
        },
        Primitive::TransformBegin(Transform {
            dx: f32::INFINITY,
            ..Transform::IDENTITY
        }),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(f32::NAN, 2.0, 10.0, 10.0),
            fill: Some(Brush::Solid(Color::rgba(f32::NAN, 0.5, 0.5, 1.0))),
            stroke: Some(Stroke::new(-1.0, Brush::Solid(Color::WHITE))),
            radius: CornerRadius::all(-3.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("rect"),
            RenderDiagnostic::InvalidGeometry("line"),
            RenderDiagnostic::InvalidGeometry("path"),
            RenderDiagnostic::InvalidGeometry("clip"),
            RenderDiagnostic::InvalidGeometry("transform"),
            RenderDiagnostic::InvalidGeometry("rect"),
            RenderDiagnostic::InvalidGeometry("rect_fill"),
            RenderDiagnostic::InvalidGeometry("rect_stroke"),
            RenderDiagnostic::InvalidGeometry("rect_radius"),
        ]
    );
    assert_eq!(translation.commands.len(), 1);
    assert_eq!(translation.commands[0].transform, Transform::IDENTITY);
    assert!(translation.commands[0].clips.is_empty());
    let RenderCommandKind::Rect {
        rect,
        fill,
        stroke,
        radius,
    } = &translation.commands[0].kind
    else {
        panic!("expected sanitized rect command");
    };
    assert_approx(rect.x, 0.0);
    assert_approx(rect.y, 2.0);
    assert!(stroke.is_none());
    assert_approx(radius.top_left, 0.0);
    let Some(Brush::Solid(color)) = fill else {
        panic!("expected solid fill");
    };
    assert_approx(color.r, 0.0);
    assert_approx(color.g, 0.5);
}

#[test]
fn invalid_empty_paths_are_diagnosed_and_skipped() {
    let primitives = vec![Primitive::Path(PathPrimitive::new(
        Vec::new(),
        Some(Brush::Solid(Color::WHITE)),
        None,
    ))];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("path")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn invalid_texture_source_size_is_diagnosed_and_dropped() {
    let primitives = vec![Primitive::Texture(TexturePrimitive {
        texture: TextureId::from_raw(2),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        source_size: Size::new(f32::NAN, 10.0),
    })];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("texture_source_size")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn applies_layer_clip_and_transform_to_following_commands() {
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(3),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(4),
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            fill: None,
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());
    let command = &translation.commands[0];

    assert_eq!(command.layer, LayerId::from_raw(3));
    assert_eq!(clip_rects(command), vec![Rect::new(0.0, 0.0, 20.0, 20.0)]);
    assert_eq!(clip_transforms(command), vec![Transform::IDENTITY]);
    assert_eq!(
        command.transform,
        Transform::translation(Vec2::new(2.0, 3.0))
    );
}

#[test]
fn restores_nested_layer_clip_and_transform_stacks() {
    let outer_clip = Rect::new(0.0, 0.0, 40.0, 40.0);
    let inner_clip = Rect::new(4.0, 4.0, 20.0, 20.0);
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(1),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(1),
            rect: outer_clip,
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
        Primitive::LayerBegin {
            id: LayerId::from_raw(2),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(2),
            rect: inner_clip,
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(5.0, 7.0))),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(1.0, 1.0, 4.0, 4.0),
            fill: None,
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(2),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(2),
        },
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(2.0, 2.0, 4.0, 4.0),
            fill: None,
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(1),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(1),
        },
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(3.0, 3.0, 4.0, 4.0),
            fill: None,
            stroke: None,
            radius: CornerRadius::all(0.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(translation.commands[0].layer, LayerId::from_raw(2));
    assert_eq!(
        clip_rects(&translation.commands[0]),
        vec![outer_clip, inner_clip]
    );
    assert_eq!(
        clip_transforms(&translation.commands[0]),
        vec![
            Transform::IDENTITY,
            Transform::translation(Vec2::new(2.0, 3.0))
        ]
    );
    assert_eq!(
        translation.commands[0].transform,
        Transform::translation(Vec2::new(7.0, 10.0))
    );
    assert_eq!(translation.commands[1].layer, LayerId::from_raw(1));
    assert_eq!(clip_rects(&translation.commands[1]), vec![outer_clip]);
    assert_eq!(
        clip_transforms(&translation.commands[1]),
        vec![Transform::IDENTITY]
    );
    assert_eq!(
        translation.commands[1].transform,
        Transform::translation(Vec2::new(2.0, 3.0))
    );
    assert_eq!(translation.commands[2].layer, LayerId::from_raw(0));
    assert!(translation.commands[2].clips.is_empty());
    assert_eq!(translation.commands[2].transform, Transform::IDENTITY);
}

#[test]
fn reports_mismatched_scope_stack_end_primitives() {
    let primitives = vec![
        Primitive::ClipEnd {
            id: ClipId::from_raw(4),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(3),
        },
        Primitive::TransformEnd,
    ];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("clip_stack"),
            RenderDiagnostic::InvalidGeometry("layer_stack"),
            RenderDiagnostic::InvalidGeometry("transform_stack"),
        ]
    );
}

#[test]
fn reports_unclosed_scope_stacks_at_end_of_translation() {
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(3),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(4),
            rect: Rect::new(0.0, 0.0, 20.0, 20.0),
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(1.0, 2.0))),
    ];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::InvalidGeometry("clip_stack"),
            RenderDiagnostic::InvalidGeometry("layer_stack"),
            RenderDiagnostic::InvalidGeometry("transform_stack"),
        ]
    );
}

#[test]
fn render_translation_snapshot_covers_commands_resources_and_diagnostics() {
    let missing_layout = TextLayoutId::from_raw(7);
    let primitives = vec![
        Primitive::LayerBegin {
            id: LayerId::from_raw(3),
        },
        Primitive::ClipBegin {
            id: ClipId::from_raw(4),
            rect: Rect::new(0.0, 0.0, 20.0, 12.0),
        },
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.0, 3.0))),
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(1.0, 1.0, 8.0, 4.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: None,
            radius: CornerRadius::all(2.0),
        }),
        Primitive::TransformEnd,
        Primitive::ClipEnd {
            id: ClipId::from_raw(4),
        },
        Primitive::LayerEnd {
            id: LayerId::from_raw(3),
        },
        Primitive::Text(TextPrimitive {
            layout: Some(missing_layout),
            origin: Point::new(4.0, 16.0),
            text: "Hi".to_owned(),
            family: "monospace".to_owned(),
            size: 12.0,
            line_height: 17.0,
            brush: Brush::Solid(Color::BLACK),
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(9),
            rect: Rect::new(0.0, 20.0, 16.0, 16.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(20.0, 20.0, 16.0, 16.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        render_translation_snapshot(&translation),
        "commands:\n  0: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000] clips=[{rect=(0.000, 0.000, 20.000, 12.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] rect rect=(1.000, 1.000, 8.000, 4.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(2.000, 2.000, 2.000, 2.000)\n  1: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] text layout=7 origin=(4.000, 16.000) family=\"monospace\" size=12.000 line_height=17.000 color=rgba(0.000, 0.000, 0.000, 1.000) text=\"Hi\"\n  2: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#9 rect=(0.000, 20.000, 16.000, 16.000) tint=none\n  3: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#2 rect=(20.000, 20.000, 16.000, 16.000) source_size=2.000x2.000\ndiagnostics:\n  missing_text_layout#7\n  missing_image#9"
    );
}

#[test]
fn reports_missing_image_and_texture_resources() {
    let primitives = vec![
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(9),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(8),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(10.0, 10.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::MissingImage(ImageId::from_raw(9)),
            RenderDiagnostic::MissingTexture(TextureId::from_raw(8)),
        ]
    );
}

#[test]
fn registered_resources_do_not_emit_missing_diagnostics() {
    let primitives = vec![
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &resources());

    assert!(translation.diagnostics.is_empty());
}

#[test]
fn atlas_backed_image_resources_do_not_emit_missing_diagnostics() {
    let primitives = vec![Primitive::Image(ImagePrimitive {
        image: ImageId::from_raw(3),
        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        tint: None,
    })];

    let translation = translate_primitives(&primitives, &atlas_resources());

    assert!(translation.diagnostics.is_empty());
}

#[test]
fn invalid_atlas_source_is_diagnosed() {
    let mut resources = atlas_resources();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(5),
        size: Size::new(4.0, 4.0),
        sampling: RenderImageSampling::default(),
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(1.0, 1.0, 4.0, 4.0),
        }),
    });
    let primitives = vec![Primitive::Image(ImagePrimitive {
        image: ImageId::from_raw(5),
        rect: Rect::new(0.0, 0.0, 16.0, 16.0),
        tint: None,
    })];

    let translation = translate_primitives(&primitives, &resources);

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("image_atlas_source")]
    );
}

#[test]
fn texture_source_size_mismatch_is_diagnosed_and_dropped() {
    let primitives = vec![Primitive::Texture(TexturePrimitive {
        texture: TextureId::from_raw(2),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        source_size: Size::new(3.0, 2.0),
    })];

    let translation = translate_primitives(&primitives, &resources());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("texture_source_size")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn texture_snapshot_size_mismatch_is_diagnosed_and_dropped() {
    let mut resources = RenderResources::new();
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(8),
        size: Size::new(2.0, 2.0),
        sampling: RenderImageSampling::default(),
        snapshot: Some(one_pixel_image()),
    });
    let primitives = vec![Primitive::Texture(TexturePrimitive {
        texture: TextureId::from_raw(8),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        source_size: Size::new(2.0, 2.0),
    })];

    let translation = translate_primitives(&primitives, &resources);

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::InvalidGeometry("texture_snapshot_size")]
    );
    assert!(translation.commands.is_empty());
}

#[test]
fn registered_size_only_resources_emit_payload_diagnostics() {
    let primitives = vec![
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            source_size: Size::new(2.0, 2.0),
        }),
    ];

    let translation = translate_primitives(&primitives, &size_only_resources());

    assert_eq!(
        translation.diagnostics,
        vec![
            RenderDiagnostic::MissingImagePixels(ImageId::from_raw(1)),
            RenderDiagnostic::MissingTextureSnapshot(TextureId::from_raw(2)),
        ]
    );
}

#[test]
fn render_image_validates_pixel_byte_length() {
    assert!(RenderImage::rgba8(2, 2, vec![0; 16]).is_some());
    assert!(RenderImage::rgba8(2, 2, vec![0; 15]).is_none());
}

#[test]
fn text_translation_accepts_unshaped_text_for_renderer_fallback() {
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(0.0, 0.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert!(translation.diagnostics.is_empty());
    assert!(matches!(
        translation.commands[0].kind,
        RenderCommandKind::Text { layout: None, .. }
    ));
}

#[test]
fn text_translation_reports_missing_shaped_layout_resource() {
    let layout = TextLayoutId::from_raw(7);
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(0.0, 0.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let translation = translate_primitives(&primitives, &RenderResources::new());

    assert_eq!(
        translation.diagnostics,
        vec![RenderDiagnostic::MissingTextLayout(layout)]
    );
}

#[test]
fn frame_submission_reports_primitive_count_and_diagnostics() {
    let mut renderer = VelloRenderer::new();
    let primitives = vec![Primitive::Image(ImagePrimitive {
        image: ImageId::from_raw(9),
        rect: Rect::new(0.0, 0.0, 10.0, 10.0),
        tint: None,
    })];
    let resources = RenderResources::new();
    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });

    assert_eq!(output.primitive_count, 1);
    assert_eq!(
        output.diagnostics,
        vec![RenderDiagnostic::MissingImage(ImageId::from_raw(9))]
    );
}

#[test]
fn renderer_backend_trait_submits_vello_frames() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();

    let output = RendererBackend::render_frame(
        &mut renderer,
        RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(100, 100),
                ScaleFactor::ONE,
            ),
            primitives: &[],
            resources: &resources,
        },
    )
    .expect("Vello CPU scene encoding should not return fatal submission errors");

    assert_eq!(output.primitive_count, 0);
    assert!(output.diagnostics.is_empty());
    assert!(renderer.scene().encoding().is_empty());
}

#[test]
fn renderer_backend_uses_concrete_vello_error_type() {
    fn assert_error_type<T: RendererBackend<Error = VelloRendererError>>(_: &T) {}

    let renderer = VelloRenderer::new();

    assert_error_type(&renderer);
}

#[test]
fn viewport_device_scale_uses_frame_scale_factor() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1200, 900),
        ScaleFactor::new(1.5),
    );

    assert!((viewport_device_scale(viewport) - 1.5).abs() < f64::EPSILON);
}

#[test]
fn viewport_device_scale_prefers_uniform_framebuffer_scale() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1000, 750),
        ScaleFactor::new(1.0),
    );

    assert_approx64(
        viewport_size_device_scale(viewport).expect("size scale"),
        1.25,
    );
    assert_approx64(viewport_device_scale(viewport), 1.25);
}

#[test]
fn viewport_device_scale_falls_back_when_framebuffer_axes_disagree() {
    let viewport = ViewportInfo::new(
        Size::new(800.0, 600.0),
        kinetik_ui_core::PhysicalSize::new(1000, 720),
        ScaleFactor::new(1.5),
    );

    assert_eq!(viewport_size_device_scale(viewport), None);
    assert_approx64(viewport_device_scale(viewport), 1.5);
}

#[test]
fn renderer_snaps_geometry_to_device_pixel_grid() {
    let point = snap_point_to_device(Point::new(10.2, 20.6), 2.0);
    let rect = snap_rect_to_device(Rect::new(1.2, 2.2, 9.1, 10.1), 2.0);
    let radius = snap_radius_to_device(
        CornerRadius {
            top_left: 2.0,
            top_right: 3.2,
            bottom_right: 0.0,
            bottom_left: -1.0,
        },
        1.25,
    );

    assert_eq!(point, Point::new(10.0, 20.5));
    assert_eq!(rect, Rect::new(1.0, 2.0, 9.5, 10.5));
    assert_eq!(
        radius,
        CornerRadius {
            top_left: 2.4,
            top_right: 3.2,
            bottom_right: 0.0,
            bottom_left: -1.0,
        }
    );
}

#[test]
fn image_rect_snapping_aligns_all_sampling_modes_to_device_bounds() {
    let rect = Rect::new(3.2, 4.2, 14.0, 14.0);
    let icon = snap_image_rect_to_device(rect, RenderImageSampling::UiIcon, 1.25);
    let smooth = snap_image_rect_to_device(rect, RenderImageSampling::Smooth, 1.25);
    let high_quality = snap_image_rect_to_device(rect, RenderImageSampling::HighQuality, 1.25);

    assert_approx(icon.x, 3.2);
    assert_approx(icon.y, 4.0);
    assert!((icon.width - 14.4).abs() < 0.000_01);
    assert!((icon.height - 14.4).abs() < 0.000_01);
    assert_eq!(smooth, icon);
    assert_eq!(high_quality, icon);
    assert!((icon.width * 1.25 - 18.0).abs() < 0.000_01);
    assert!((smooth.width * 1.25 - 18.0).abs() < 0.000_01);
    assert!((high_quality.width * 1.25 - 18.0).abs() < 0.000_01);
}

#[test]
fn renderer_snaps_stroke_centers_to_physical_pixel_coverage() {
    let one_px = snap_stroke_center_to_device(10.0, 1.0, 1.0);
    let one_px_fractional_scale = snap_stroke_center_to_device(10.0, 1.0, 1.25);
    let two_px = snap_stroke_center_to_device(10.0, 1.0, 2.0);
    let horizontal =
        snap_stroked_line_to_device(Point::new(0.2, 10.0), Point::new(20.2, 10.0), 1.0, 1.0);
    let rect = snap_stroked_rect_to_device(Rect::new(0.1, 0.1, 20.2, 12.2), 1.0, 1.0);
    let fractional_rect = snap_stroked_rect_to_device(Rect::new(0.0, 0.0, 20.0, 12.0), 1.0, 1.25);

    assert_approx(one_px, 10.5);
    assert_approx(one_px_fractional_scale, 10.0);
    assert_approx(two_px, 10.0);
    assert_eq!(horizontal.0, Point::new(0.0, 10.5));
    assert_eq!(horizontal.1, Point::new(20.0, 10.5));
    assert_eq!(rect, Rect::new(0.5, 0.5, 19.0, 11.0));
    assert_eq!(fractional_rect, Rect::new(0.4, 0.4, 19.2, 11.2));
}

#[test]
fn square_rect_borders_are_segmented_on_physical_pixels() {
    let segments = crisp_rect_border_segments(Rect::new(0.0, 0.0, 20.0, 12.0), 1.0, 1.25);

    assert_eq!(
        segments,
        vec![
            Rect::new(0.0, 0.0, 20.0, 0.8),
            Rect::new(0.0, 11.2, 20.0, 0.8),
            Rect::new(0.0, 0.8, 0.8, 10.4),
            Rect::new(19.2, 0.8, 0.8, 10.4),
        ]
    );
    for segment in segments {
        for value in [
            segment.x * 1.25,
            segment.y * 1.25,
            segment.width * 1.25,
            segment.height * 1.25,
        ] {
            assert!((value - value.round()).abs() <= 0.000_01, "{value}");
        }
    }
}

#[test]
fn square_rect_border_segments_collapse_tiny_rectangles() {
    assert_eq!(
        crisp_rect_border_segments(Rect::new(0.0, 0.0, 1.0, 1.0), 1.0, 1.25),
        vec![Rect::new(0.0, 0.0, 0.8, 0.8)]
    );
}

#[test]
fn renderer_snaps_line_based_stroked_paths_to_device_pixels() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 10.3)),
        PathElement::MoveTo(Point::new(4.2, 1.2)),
        PathElement::LineTo(Point::new(4.2, 11.2)),
        PathElement::Close,
    ];

    let snapped = snap_stroked_path_elements_to_device(&elements, 1.0, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(0.0, 10.0)),
            PathElement::LineTo(Point::new(20.0, 10.0)),
            PathElement::MoveTo(Point::new(4.4, 1.6)),
            PathElement::LineTo(Point::new(4.4, 11.2)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_snaps_filled_line_based_paths_to_device_pixels() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 10.3)),
        PathElement::LineTo(Point::new(20.2, 30.3)),
        PathElement::Close,
    ];

    let snapped = snap_filled_path_elements_to_device(&elements, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(0.0, 10.4)),
            PathElement::LineTo(Point::new(20.0, 10.4)),
            PathElement::LineTo(Point::new(20.0, 30.4)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_snaps_closed_stroked_polygon_vertices() {
    let elements = vec![
        PathElement::MoveTo(Point::new(10.2, 0.2)),
        PathElement::LineTo(Point::new(20.2, 10.2)),
        PathElement::LineTo(Point::new(10.2, 20.2)),
        PathElement::LineTo(Point::new(0.2, 10.2)),
        PathElement::Close,
    ];

    let snapped = snap_stroked_path_elements_to_device(&elements, 1.0, 1.25);

    assert_eq!(
        snapped,
        vec![
            PathElement::MoveTo(Point::new(10.4, 0.0)),
            PathElement::LineTo(Point::new(20.0, 10.4)),
            PathElement::LineTo(Point::new(10.4, 20.0)),
            PathElement::LineTo(Point::new(0.0, 10.4)),
            PathElement::Close,
        ]
    );
}

#[test]
fn renderer_leaves_curved_stroked_paths_unsnapped() {
    let elements = vec![
        PathElement::MoveTo(Point::new(0.2, 10.3)),
        PathElement::QuadTo {
            ctrl: Point::new(5.2, 4.2),
            to: Point::new(20.2, 10.3),
        },
    ];

    assert_eq!(
        snap_stroked_path_elements_to_device(&elements, 1.0, 1.25),
        elements
    );
}

#[test]
fn renderer_quantizes_stroke_widths_to_physical_pixels() {
    assert_approx(quantize_stroke_width_to_device(1.0, 1.0), 1.0);
    assert_approx(quantize_stroke_width_to_device(1.0, 1.25), 0.8);
    assert_approx(quantize_stroke_width_to_device(1.0, 1.5), 1.333_333_4);
    assert_approx(quantize_stroke_width_to_device(2.0, 1.25), 2.4);
}

#[test]
fn renderer_snaps_axis_aligned_transform_translation_to_device_pixels() {
    let transform =
        snap_axis_aligned_translation(root_transform(2.0) * Affine::translate((0.25, 0.25)));

    let coeffs = transform.as_coeffs();
    assert_approx64(coeffs[0], 2.0);
    assert_approx64(coeffs[1], 0.0);
    assert_approx64(coeffs[2], 0.0);
    assert_approx64(coeffs[3], 2.0);
    assert_approx64(coeffs[4], 1.0);
    assert_approx64(coeffs[5], 1.0);
}

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
    let byte_len = super::MAX_CACHED_TINTED_IMAGE_BYTES + 4;
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

    for raw in 1..=super::MAX_CACHED_IMAGE_ENTRIES {
        cache.image_data(
            ImageId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
        );
    }
    cache.image_data(first, &image);
    cache.image_data(
        ImageId::from_raw(
            u64::try_from(super::MAX_CACHED_IMAGE_ENTRIES + 1).expect("cache id fits u64"),
        ),
        &image,
    );

    assert_eq!(cache.images.len(), super::MAX_CACHED_IMAGE_ENTRIES);
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

    for raw in 1..=super::MAX_TINTED_IMAGE_CACHE_ENTRIES {
        cache.image_data_with_tint(
            ImageId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
            tint,
        );
    }
    cache.image_data_with_tint(first, &image, tint);
    cache.image_data_with_tint(
        ImageId::from_raw(
            u64::try_from(super::MAX_TINTED_IMAGE_CACHE_ENTRIES + 1).expect("cache id fits u64"),
        ),
        &image,
        tint,
    );

    assert_eq!(
        cache.tinted_images.len(),
        super::MAX_TINTED_IMAGE_CACHE_ENTRIES
    );
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

    for raw in 1..=super::MAX_CACHED_TEXTURE_ENTRIES {
        cache.texture_data(
            TextureId::from_raw(u64::try_from(raw).expect("cache id fits u64")),
            &image,
        );
    }
    cache.texture_data(first, &image);
    cache.texture_data(
        TextureId::from_raw(
            u64::try_from(super::MAX_CACHED_TEXTURE_ENTRIES + 1).expect("cache id fits u64"),
        ),
        &image,
    );

    assert_eq!(cache.textures.len(), super::MAX_CACHED_TEXTURE_ENTRIES);
    assert!(cache.textures.contains_key(&first));
    assert!(!cache.textures.contains_key(&second));
}

#[test]
fn frame_submission_encodes_vello_geometry() {
    let mut renderer = VelloRenderer::new();
    let primitives = vec![
        Primitive::Rect(RectPrimitive {
            rect: Rect::new(0.0, 0.0, 40.0, 24.0),
            fill: Some(Brush::Solid(Color::WHITE)),
            stroke: Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
            radius: CornerRadius::all(4.0),
        }),
        Primitive::Line(LinePrimitive {
            from: Point::new(0.0, 0.0),
            to: Point::new(40.0, 24.0),
            stroke: Stroke::new(2.0, Brush::Solid(Color::WHITE)),
        }),
        Primitive::Path(PathPrimitive::new(
            vec![
                PathElement::MoveTo(Point::new(6.0, 6.0)),
                PathElement::LineTo(Point::new(30.0, 6.0)),
                PathElement::LineTo(Point::new(18.0, 20.0)),
                PathElement::Close,
            ],
            Some(Brush::Solid(Color::rgba(0.2, 0.6, 0.9, 1.0))),
            Some(Stroke::new(1.0, Brush::Solid(Color::BLACK))),
        )),
    ];
    let resources = RenderResources::new();

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
    assert!(renderer.scene().encoding().n_paths >= 2);
}

#[test]
fn frame_submission_encodes_fallback_text_and_visible_resource_placeholders() {
    let mut renderer = VelloRenderer::new();
    let resources = resources();
    let primitives = vec![
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.0, 16.0),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::Image(ImagePrimitive {
            image: ImageId::from_raw(1),
            rect: Rect::new(0.0, 24.0, 32.0, 24.0),
            tint: None,
        }),
        Primitive::Texture(TexturePrimitive {
            texture: TextureId::from_raw(2),
            rect: Rect::new(40.0, 24.0, 32.0, 24.0),
            source_size: Size::new(2.0, 2.0),
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
    assert!(!renderer.scene().encoding().is_empty());
    assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
    assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
    assert!(renderer.scene().encoding().resources.patches.len() >= 2);
}

#[test]
fn frame_submission_encodes_axis_aligned_text_at_physical_font_size() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(200, 200),
            ScaleFactor::new(2.0),
        ),
        primitives: &primitives,
        resources: &resources,
    });

    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 24.0);
    assert!(glyph_run.hint);
}

#[test]
fn text_origin_snapping_rounds_x_and_baseline_y() {
    let origin = snap_text_origin_to_device(Point::new(5.375, 20.5));

    assert_approx(origin.x, 5.0);
    assert_approx(origin.y, 21.0);
}

#[test]
fn text_glyph_baseline_snapping_rounds_device_coordinates() {
    assert_approx(snap_text_glyph_baseline_to_device(11.49), 11.0);
    assert_approx(snap_text_glyph_baseline_to_device(11.5), 12.0);
}

#[test]
fn text_transform_origin_snapping_happens_in_device_space_for_non_uniform_scale() {
    let transform = root_transform(1.25) * Affine::scale_non_uniform(1.5, 1.0);
    let origin = Point::new(4.3, 16.4);

    let snapped = snap_text_transform_origin_to_device(transform, origin);
    let device_origin = transform_point(snapped, origin);

    assert_approx(device_origin.x, 8.0);
    assert_approx(device_origin.y, 21.0);
}

#[test]
fn text_transform_origin_snapping_happens_in_device_space_for_rotation() {
    let transform = root_transform(1.25) * Affine::rotate(0.5);
    let origin = Point::new(4.3, 16.4);

    let snapped = snap_text_transform_origin_to_device(transform, origin);
    let device_origin = transform_point(snapped, origin);

    assert!((device_origin.x - device_origin.x.round()).abs() <= 0.001);
    assert!((device_origin.y - device_origin.y.round()).abs() <= 0.001);
}

#[test]
fn physical_text_snaps_horizontal_origin_and_baseline() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph = renderer
        .scene()
        .encoding()
        .resources
        .glyphs
        .first()
        .expect("glyph");

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph.x, 5.0);
    assert_approx(glyph.y, 21.0);
}

#[test]
fn physical_text_snaps_shaped_horizontal_glyph_positions() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let mut text_engine = CosmicTextEngine::new();
    let mut text_cache = ShapedTextCache::default();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyphs = &renderer.scene().encoding().resources.glyphs;
    let layout = physical_text_layout(
        &mut text_engine,
        &mut text_cache,
        root_transform(1.25),
        "Kinetik",
        "sans-serif",
        13.0,
        18.0,
    )
    .expect("axis-aligned physical layout");
    let expected_x = shaped_glyph_x_positions(&layout, 5.0, 1.0);

    assert!(output.diagnostics.is_empty());
    assert_eq!(glyphs.len(), expected_x.len());
    for (glyph, expected) in glyphs.iter().zip(expected_x) {
        assert_approx(glyph.x, expected.round());
    }
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "glyph x positions should stay snapped to physical pixels"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "baselines should stay snapped to physical pixels"
    );
}

#[test]
fn physical_text_policy_holds_across_common_dpi_scales() {
    let resources = RenderResources::new();
    let origin = Point::new(4.3, 16.4);
    let font_size = 13.0;
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin,
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: font_size,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    for (scale, physical_size, expected_font_size, expected_x) in [
        (1.0, 100, 13.0, 4.0),
        (1.25, 125, 16.0, 5.0),
        (1.5, 150, 20.0, 6.0),
        (2.0, 200, 26.0, 9.0),
    ] {
        let mut renderer = VelloRenderer::new();
        let output = renderer.submit_frame(RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 100.0),
                kinetik_ui_core::PhysicalSize::new(physical_size, physical_size),
                ScaleFactor::new(scale),
            ),
            primitives: &primitives,
            resources: &resources,
        });
        let encoding = renderer.scene().encoding();
        let glyphs = &encoding.resources.glyphs;
        let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
        let first_glyph = glyphs.first().expect("glyph");

        assert!(output.diagnostics.is_empty());
        assert_approx(glyph_run.font_size, expected_font_size);
        assert!(glyph_run.hint);
        assert_approx(first_glyph.x, expected_x);
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
            "scale {scale} should snap glyph x positions"
        );
        assert!(
            glyphs
                .iter()
                .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
            "scale {scale} should snap glyph baselines"
        );
    }
}

#[test]
fn physical_text_uses_uniform_framebuffer_scale_when_declared_scale_is_stale() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.3, 16.4),
        text: "Kinetik".to_owned(),
        family: "sans-serif".to_owned(),
        size: 13.0,
        line_height: 18.0,
        brush: Brush::Solid(Color::WHITE),
    })];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.0),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 16.0);
    assert!(glyph_run.hint);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "framebuffer-derived scale should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "framebuffer-derived scale should snap glyph baselines"
    );
}

#[test]
fn translated_physical_text_stays_snapped_at_fractional_dpi() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.3, 16.4),
            text: "Kinetik".to_owned(),
            family: "sans-serif".to_owned(),
            size: 13.0,
            line_height: 18.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(150, 150),
            ScaleFactor::new(1.5),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 20.0);
    assert!(glyph_run.hint);
    assert!(glyphs.len() > 1);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "translated text should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "translated text should snap glyph baselines"
    );
}

#[test]
fn axis_aligned_non_uniform_text_preserves_x_scale_with_glyph_transform() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let text = "Kinetik";
    let origin = Point::new(4.3, 16.4);
    let font_size = 13.0;
    let line_height = 18.0;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.25,
            m22: 1.5,
            dx: 2.2,
            dy: 3.4,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin,
            text: text.to_owned(),
            family: "sans-serif".to_owned(),
            size: font_size,
            line_height,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
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
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 20.0);
    assert!(glyph_run.hint);
    assert_approx(glyph_run.transform.matrix[0], 1.0);
    assert_approx(glyph_run.transform.matrix[1], 0.0);
    assert_approx(glyph_run.transform.matrix[2], 0.0);
    assert_approx(glyph_run.transform.matrix[3], 1.0);
    assert_approx(glyph_run.transform.translation[0], 0.0);
    assert_approx(glyph_run.transform.translation[1], 0.0);
    let glyph_transform = glyph_run.glyph_transform.expect("x glyph transform");
    assert_approx(glyph_transform.matrix[0], 0.8125);
    assert_approx(glyph_transform.matrix[1], 0.0);
    assert_approx(glyph_transform.matrix[2], 0.0);
    assert_approx(glyph_transform.matrix[3], 1.0);
    assert!(glyphs.len() > 1);
    let mut engine = CosmicTextEngine::new();
    let layout = engine.shape_text(&TextLayoutKey::new(
        text,
        TextStyle::new("sans-serif", font_size, line_height),
        0.0,
        false,
    ));
    let logical_second_glyph = layout
        .runs
        .first()
        .and_then(|run| run.glyphs.iter().find(|glyph| glyph.x > 0.0))
        .expect("second logical glyph");
    let encoded_second_glyph = glyphs
        .iter()
        .find(|glyph| glyph.x > glyphs[0].x)
        .expect("second encoded glyph");
    let snapped_origin =
        snap_text_origin_to_device(Point::new(2.0 + origin.x * 1.25, 3.0 + origin.y * 1.5));
    assert_approx(
        encoded_second_glyph.x,
        snap_text_glyph_position_to_device(snapped_origin.x + logical_second_glyph.x * 1.25),
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.x - glyph.x.round()).abs() <= 0.001),
        "non-uniform text should snap glyph x positions"
    );
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "non-uniform text should snap glyph baselines"
    );
}

#[test]
fn rotated_text_fallback_snaps_transformed_origin_to_device_pixels() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let angle = 0.5_f32;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: angle.cos(),
            m12: angle.sin(),
            m21: -angle.sin(),
            m22: angle.cos(),
            dx: 2.2,
            dy: 3.4,
        }),
        Primitive::Text(TextPrimitive {
            layout: None,
            origin: Point::new(4.3, 16.4),
            text: "Kinetik".to_owned(),
            family: "sans-serif".to_owned(),
            size: 13.0,
            line_height: 18.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyph = encoding.resources.glyphs.first().expect("glyph");
    let mapped =
        glyph_run.transform.to_kurbo() * KurboPoint::new(f64::from(glyph.x), f64::from(glyph.y));

    assert!(output.diagnostics.is_empty());
    assert!(!glyph_run.hint);
    assert!((mapped.x - mapped.x.round()).abs() <= 0.001);
    assert!((mapped.y - mapped.y.round()).abs() <= 0.001);
}

#[test]
fn physical_text_layout_shapes_at_device_font_size() {
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();
    let layout = physical_text_layout(
        &mut engine,
        &mut cache,
        root_transform(1.5),
        "Label",
        "monospace",
        12.0,
        17.0,
    )
    .expect("axis-aligned physical layout");

    assert!(!layout.runs.is_empty());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.font.data.data() == fonts::GEIST_MONO_VARIABLE)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 26.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_layout_quantizes_fractional_device_metrics() {
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();
    let layout = physical_text_layout(
        &mut engine,
        &mut cache,
        root_transform(1.25),
        "Sharp",
        "sans-serif",
        14.0,
        19.0,
    )
    .expect("axis-aligned physical layout");

    assert!(!layout.runs.is_empty());
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .runs
            .iter()
            .all(|run| run.font.data.data() == fonts::INTER_VARIABLE)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 24.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_extent_quantizes_fractional_device_widths() {
    assert_approx(quantize_physical_text_extent(86.25), 86.0);
    assert_approx(quantize_physical_text_extent(86.5), 87.0);
    assert_approx(quantize_physical_text_extent(0.0), 0.0);
}

#[test]
fn physical_text_layout_for_key_quantizes_wrap_width_at_device_scale() {
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon",
        TextStyle::new("sans-serif", 12.0, 17.0),
        69.0,
        true,
    );
    let mut expected_engine = CosmicTextEngine::new();
    let expected = expected_engine.shape_text(&TextLayoutKey::new(
        key.text.clone(),
        TextStyle::new("sans-serif", 15.0, 21.0),
        86.0,
        true,
    ));
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();

    let layout = physical_text_layout_for_key(&mut engine, &mut cache, root_transform(1.25), &key)
        .expect("axis-aligned physical layout");

    assert_eq!(layout.line_count, expected.line_count);
    assert_eq!(layout.lines.len(), expected.lines.len());
    assert_approx(layout.size.width, expected.size.width);
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 15.0).abs() < f32::EPSILON)
    );
}

#[test]
fn physical_text_layout_for_key_preserves_wrap_width_at_device_scale() {
    let key = TextLayoutKey::new(
        "alpha beta gamma delta epsilon",
        TextStyle::new("sans-serif", 12.0, 17.0),
        68.0,
        true,
    );
    let mut expected_engine = CosmicTextEngine::new();
    let expected = expected_engine.shape_text(&TextLayoutKey::new(
        key.text.clone(),
        TextStyle::new("sans-serif", 18.0, 26.0),
        102.0,
        true,
    ));
    let mut engine = CosmicTextEngine::new();
    let mut cache = ShapedTextCache::default();

    let layout = physical_text_layout_for_key(&mut engine, &mut cache, root_transform(1.5), &key)
        .expect("axis-aligned physical layout");

    assert_eq!(layout.line_count, expected.line_count);
    assert_eq!(layout.lines.len(), expected.lines.len());
    assert!(layout.line_count > 1);
    assert_approx(layout.size.width, expected.size.width);
    assert!(
        layout
            .runs
            .iter()
            .all(|run| (run.font_size - 18.0).abs() < f32::EPSILON)
    );
    assert!(
        layout
            .lines
            .iter()
            .all(|line| (line.height - 26.0).abs() < f32::EPSILON)
    );
}

#[test]
fn frame_submission_encodes_registered_shaped_text_layout() {
    let layout = TextLayoutId::from_raw(44);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let mut renderer = VelloRenderer::new();

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
    assert!(!renderer.scene().encoding().resources.glyph_runs.is_empty());
    assert!(!renderer.scene().encoding().resources.glyphs.is_empty());
}

#[test]
fn registered_text_layout_renders_with_fractional_scale_physical_shape() {
    let layout = TextLayoutId::from_raw(45);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.3, 16.4),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");
    let glyph = renderer
        .scene()
        .encoding()
        .resources
        .glyphs
        .first()
        .expect("glyph");

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    assert_approx(glyph.x, 5.0);
    assert_approx(glyph.y, 21.0);
}

#[test]
fn translated_registered_text_layout_stays_snapped_at_fractional_dpi() {
    let layout = TextLayoutId::from_raw(47);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform::translation(Vec2::new(2.2, 3.4))),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(125, 125),
            ScaleFactor::new(1.25),
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let encoding = renderer.scene().encoding();
    let glyph_run = encoding.resources.glyph_runs.first().expect("glyph run");
    let glyphs = &encoding.resources.glyphs;

    assert!(output.diagnostics.is_empty());
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    let first_glyph = glyphs.first().expect("glyph");
    assert!((first_glyph.x - first_glyph.x.round()).abs() <= 0.001);
    assert!(
        glyphs
            .iter()
            .all(|glyph| (glyph.y - glyph.y.round()).abs() <= 0.001),
        "registered text should snap glyph baselines under translation"
    );
}

#[test]
fn near_uniform_registered_text_uses_physical_hinted_layout() {
    let layout = TextLayoutId::from_raw(57);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.250_01,
            m22: 1.249_99,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 1);
    assert_approx(glyph_run.font_size, 15.0);
    assert!(glyph_run.hint);
    assert!(glyph_run.glyph_transform.is_none());
}

#[test]
fn tiny_axis_aligned_skew_still_uses_device_text_path() {
    let layout = TextLayoutId::from_raw(58);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: 1.25,
            m12: 0.000_01,
            m21: -0.000_01,
            m22: 1.25,
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 1);
    assert!(glyph_run.hint);
}

#[test]
fn meaningful_rotation_keeps_general_text_path() {
    let layout = TextLayoutId::from_raw(59);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let angle = 0.01_f32;
    let primitives = vec![
        Primitive::TransformBegin(Transform {
            m11: angle.cos(),
            m12: angle.sin(),
            m21: -angle.sin(),
            m22: angle.cos(),
            ..Transform::IDENTITY
        }),
        Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.3, 16.4),
            text: "Label".to_owned(),
            family: "sans-serif".to_owned(),
            size: 12.0,
            line_height: 16.0,
            brush: Brush::Solid(Color::WHITE),
        }),
        Primitive::TransformEnd,
    ];
    let mut renderer = VelloRenderer::new();

    let output = renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    let glyph_run = renderer
        .scene()
        .encoding()
        .resources
        .glyph_runs
        .first()
        .expect("glyph run");

    assert!(output.diagnostics.is_empty());
    assert_eq!(renderer.cached_text_layout_count(), 0);
    assert!(!glyph_run.hint);
}

#[test]
fn repeated_registered_text_reuses_cached_physical_layout() {
    let layout = TextLayoutId::from_raw(48);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let viewport = ViewportInfo::new(
        Size::new(100.0, 100.0),
        kinetik_ui_core::PhysicalSize::new(125, 125),
        ScaleFactor::new(1.25),
    );
    let mut renderer = VelloRenderer::new();

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn shaped_text_cache_evicts_least_recent_entry_at_capacity() {
    let first = TextLayoutKey::new(
        "layout 1",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    );
    let second = TextLayoutKey::new(
        "layout 2",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    );
    let dummy_layout = std::sync::Arc::new(ShapedTextLayout {
        size: Size::new(0.0, 0.0),
        line_count: 0,
        lines: Vec::new(),
        runs: Vec::new(),
    });
    let mut cache = ShapedTextCache::default();

    for index in 1..=super::MAX_CACHED_TEXT_LAYOUTS {
        let key = TextLayoutKey::new(
            format!("layout {index}"),
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        );
        cache.layout_order.push_back(key.clone());
        cache
            .layouts
            .insert(key, std::sync::Arc::clone(&dummy_layout));
    }

    let mut engine = CosmicTextEngine::new();
    cache.layout(&mut engine, first.clone());
    cache.layout(
        &mut engine,
        TextLayoutKey::new(
            "layout overflow",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
    );

    assert_eq!(cache.layouts.len(), super::MAX_CACHED_TEXT_LAYOUTS);
    assert!(cache.layouts.contains_key(&first));
    assert!(!cache.layouts.contains_key(&second));
}

#[test]
fn repeated_fallback_text_reuses_cached_physical_layout() {
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.0, 16.0),
        text: "Fallback".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let viewport = ViewportInfo::new(
        Size::new(100.0, 100.0),
        kinetik_ui_core::PhysicalSize::new(125, 125),
        ScaleFactor::new(1.25),
    );
    let mut renderer = VelloRenderer::new();

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn render_resources_register_text_layout_store_entries() {
    let mut store = TextLayoutStore::new();
    let id = store.layout_id(TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    ));
    let mut resources = RenderResources::new();

    resources.register_text_layouts(store.layouts());

    assert!(resources.has_text_layout(id));
    assert_eq!(
        resources.text_layout(id).map(ShapedTextLayout::glyph_count),
        store.layout(id).map(ShapedTextLayout::glyph_count)
    );
}

#[test]
fn frame_submission_resets_retained_scene() {
    let mut renderer = VelloRenderer::new();
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Rect(RectPrimitive {
        rect: Rect::new(0.0, 0.0, 40.0, 24.0),
        fill: Some(Brush::Solid(Color::WHITE)),
        stroke: None,
        radius: CornerRadius::all(0.0),
    })];

    renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &primitives,
        resources: &resources,
    });
    assert!(!renderer.scene().encoding().is_empty());

    renderer.submit_frame(RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 100.0),
            kinetik_ui_core::PhysicalSize::new(100, 100),
            ScaleFactor::ONE,
        ),
        primitives: &[],
        resources: &resources,
    });

    assert!(renderer.scene().encoding().is_empty());
}
