use super::common::{
    atlas_resources, clip_rects, clip_transforms, one_pixel_image, resources, size_only_resources,
};
use crate::{
    ImageAtlasRegion, ImageResource, RenderCommandKind, RenderDiagnostic, RenderImage,
    RenderImageSampling, RenderResources, TextureResource, render_translation_snapshot,
    translate_primitives,
};
use kinetik_ui_core::render::TexturePrimitive;
use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, ImageId, ImagePrimitive, LayerId, Point, Primitive, Rect,
    RectPrimitive, Size, TextLayoutId, TextPrimitive, TextureId, Transform, Vec2,
};

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
