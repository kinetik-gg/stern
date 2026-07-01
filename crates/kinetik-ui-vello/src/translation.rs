use kinetik_ui_core::{ClipId, ImageId, LayerId, Primitive, Transform};
use kinetik_ui_render::{RenderDiagnostic, RenderResources};

use crate::{
    command::{RenderClip, RenderCommand, RenderCommandKind, Translation},
    geometry::{compose_transform, logical_size_matches, transform_is_finite},
    image::{
        atlas_source_fits_image, atlas_source_is_finite_positive,
        image_resource_size_matches_atlas_source, image_resource_size_matches_pixels,
        logical_size_matches_snapshot,
    },
    sanitize::{
        brush_fallback_color, finite_positive, sanitize_brush, sanitize_color,
        sanitize_path_elements, sanitize_point, sanitize_radius, sanitize_rect, sanitize_shadow,
        sanitize_size, sanitize_stroke, sanitize_transform,
    },
};

/// Translates primitives into deterministic renderer commands.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn translate_primitives(primitives: &[Primitive], resources: &RenderResources) -> Translation {
    let primitive_count = primitives.len();
    let mut commands = Vec::with_capacity(primitive_count);
    let mut diagnostics = Vec::with_capacity(primitive_count);
    let mut layers = Vec::with_capacity(primitive_count.saturating_add(1));
    let mut clips = Vec::<(ClipId, RenderClip)>::with_capacity(primitive_count);
    let mut transforms = Vec::<Transform>::with_capacity(primitive_count);
    let mut transform = Transform::IDENTITY;
    layers.push(LayerId::from_raw(0));

    for primitive in primitives {
        match primitive {
            Primitive::Rect(rect) => {
                let Some(rect_bounds) = sanitize_rect(rect.rect, &mut diagnostics, "rect") else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Rect {
                        rect: rect_bounds,
                        fill: rect
                            .fill
                            .map(|brush| sanitize_brush(brush, &mut diagnostics, "rect_fill")),
                        stroke: rect.stroke.and_then(|stroke| {
                            sanitize_stroke(stroke, &mut diagnostics, "rect_stroke")
                        }),
                        radius: sanitize_radius(rect.radius, &mut diagnostics, "rect_radius"),
                    },
                ));
            }
            Primitive::Line(line) => {
                let Some(from) = sanitize_point(line.from, &mut diagnostics, "line") else {
                    continue;
                };
                let Some(to) = sanitize_point(line.to, &mut diagnostics, "line") else {
                    continue;
                };
                let Some(stroke) = sanitize_stroke(line.stroke, &mut diagnostics, "line_stroke")
                else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Line {
                        x0: from.x,
                        y0: from.y,
                        x1: to.x,
                        y1: to.y,
                        stroke,
                    },
                ));
            }
            Primitive::Shadow(shadow) => {
                let Some(shadow) = sanitize_shadow(*shadow, &mut diagnostics) else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Shadow {
                        rect: shadow.rect,
                        offset: shadow.offset,
                        blur_radius: shadow.blur_radius,
                        spread: shadow.spread,
                        radius: shadow.radius,
                        color: shadow.color,
                    },
                ));
            }
            Primitive::Path(path) => {
                let Some(elements) =
                    sanitize_path_elements(&path.elements, &mut diagnostics, "path")
                else {
                    continue;
                };
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Path {
                        elements,
                        fill: path
                            .fill
                            .map(|brush| sanitize_brush(brush, &mut diagnostics, "path_fill")),
                        stroke: path.stroke.and_then(|stroke| {
                            sanitize_stroke(stroke, &mut diagnostics, "path_stroke")
                        }),
                    },
                ));
            }
            Primitive::Text(text) => {
                let Some(origin) = sanitize_point(text.origin, &mut diagnostics, "text") else {
                    continue;
                };
                let Some(size) = finite_positive(text.size) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("text_size"));
                    continue;
                };
                let Some(line_height) = finite_positive(text.line_height) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("text_line_height"));
                    continue;
                };
                match text.layout {
                    Some(layout) if !resources.has_text_layout(layout) => {
                        diagnostics.push(RenderDiagnostic::MissingTextLayout(layout));
                    }
                    Some(_) | None => {}
                }
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Text {
                        layout: text.layout,
                        origin,
                        text: text.text.clone(),
                        family: text.family.clone(),
                        size,
                        line_height,
                        color: brush_fallback_color(&sanitize_brush(
                            text.brush,
                            &mut diagnostics,
                            "text_brush",
                        )),
                    },
                ));
            }
            Primitive::Image(image) => {
                let Some(rect) = sanitize_rect(image.rect, &mut diagnostics, "image") else {
                    continue;
                };
                validate_image_resource(resources, image.image, &mut diagnostics);
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Image {
                        image: image.image,
                        rect,
                        tint: image
                            .tint
                            .map(|tint| sanitize_color(tint, &mut diagnostics, "image_tint")),
                    },
                ));
            }
            Primitive::Texture(texture) => {
                let Some(rect) = sanitize_rect(texture.rect, &mut diagnostics, "texture") else {
                    continue;
                };
                let Some(source_size) = sanitize_size(texture.source_size) else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("texture_source_size"));
                    continue;
                };
                match resources.texture(texture.texture) {
                    None => diagnostics.push(RenderDiagnostic::MissingTexture(texture.texture)),
                    Some(resource) if !logical_size_matches(source_size, resource.size) => {
                        diagnostics.push(RenderDiagnostic::InvalidGeometry("texture_source_size"));
                        continue;
                    }
                    Some(resource) if resource.snapshot.is_none() => {
                        diagnostics.push(RenderDiagnostic::MissingTextureSnapshot(texture.texture));
                    }
                    Some(resource)
                        if resource.snapshot.as_ref().is_some_and(|snapshot| {
                            !logical_size_matches_snapshot(resource.size, snapshot)
                        }) =>
                    {
                        diagnostics
                            .push(RenderDiagnostic::InvalidGeometry("texture_snapshot_size"));
                        continue;
                    }
                    Some(_) => {}
                }
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Texture {
                        texture: texture.texture,
                        rect,
                        source_size,
                    },
                ));
            }
            Primitive::ClipBegin { id, rect } => {
                let Some(rect) = sanitize_rect(*rect, &mut diagnostics, "clip") else {
                    continue;
                };
                clips.push((*id, RenderClip { rect, transform }));
            }
            Primitive::ClipEnd { id } => {
                if clips.last().is_some_and(|(open_id, _)| open_id == id) {
                    clips.pop();
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("clip_stack"));
                }
            }
            Primitive::LayerBegin { id } => {
                layers.push(*id);
            }
            Primitive::LayerEnd { id } => {
                if layers.len() > 1 && layers.last() == Some(id) {
                    layers.pop();
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("layer_stack"));
                }
            }
            Primitive::TransformBegin(next_transform) => {
                let Some(next_transform) =
                    sanitize_transform(*next_transform, &mut diagnostics, "transform")
                else {
                    continue;
                };
                transforms.push(transform);
                let next = compose_transform(transform, next_transform);
                if transform_is_finite(next) {
                    transform = next;
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("transform"));
                }
            }
            Primitive::TransformEnd => {
                if let Some(previous) = transforms.pop() {
                    transform = previous;
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("transform_stack"));
                    transform = Transform::IDENTITY;
                }
            }
        }
    }
    if !clips.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("clip_stack"));
    }
    if layers.len() > 1 {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("layer_stack"));
    }
    if !transforms.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("transform_stack"));
    }

    Translation {
        commands,
        diagnostics,
    }
}

pub(crate) fn validate_image_resource(
    resources: &RenderResources,
    image: ImageId,
    diagnostics: &mut Vec<RenderDiagnostic>,
) {
    let Some(resource) = resources.image(image) else {
        diagnostics.push(RenderDiagnostic::MissingImage(image));
        return;
    };
    if let Some(pixels) = resource.pixels.as_ref() {
        if !image_resource_size_matches_pixels(resource, pixels) {
            diagnostics.push(RenderDiagnostic::InvalidGeometry("image_source_size"));
        }
        return;
    }
    let Some(region) = resource.atlas_region else {
        diagnostics.push(RenderDiagnostic::MissingImagePixels(image));
        return;
    };
    if !atlas_source_is_finite_positive(region.source) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_atlas_source"));
        return;
    }
    let Some(atlas) = resources.image(region.atlas) else {
        diagnostics.push(RenderDiagnostic::MissingImage(region.atlas));
        return;
    };
    let Some(pixels) = atlas.pixels.as_ref() else {
        diagnostics.push(RenderDiagnostic::MissingImagePixels(region.atlas));
        return;
    };
    if !image_resource_size_matches_pixels(atlas, pixels) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_source_size"));
        return;
    }
    if !atlas_source_fits_image(region.source, pixels) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_atlas_source"));
        return;
    }
    if !image_resource_size_matches_atlas_source(resource, region.source) {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("image_source_size"));
    }
}

pub(crate) fn render_command(
    layers: &[LayerId],
    clips: &[(ClipId, RenderClip)],
    transform: Transform,
    kind: RenderCommandKind,
) -> RenderCommand {
    let mut command_clips = Vec::with_capacity(clips.len());
    command_clips.extend(clips.iter().map(|(_, clip)| *clip));

    RenderCommand {
        layer: layers
            .last()
            .copied()
            .unwrap_or_else(|| LayerId::from_raw(0)),
        clips: command_clips,
        transform,
        kind,
    }
}
