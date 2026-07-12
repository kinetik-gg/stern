use kinetik_ui_core::{
    Brush, Color, CornerRadius, ImageId, PathElement, Point, Rect, ShadowPrimitive, Size, Stroke,
    TextLayoutId, TextureId,
};
use kinetik_ui_render::{RenderImage, RenderImageSampling, RenderResources};
use kinetik_ui_text::TextLayoutStore;
use vello::{
    Scene,
    kurbo::{Affine, BezPath, Line as KurboLine, Shape},
    peniko::{Fill, ImageBrush, ImageData},
};

use crate::{
    command::{RenderCommand, RenderCommandKind},
    geometry::{
        crisp_rect_border_segments, kurbo_rect, quantize_stroke_width_to_device, radius_is_zero,
        root_transform, rounded_rect, snap_axis_aligned_translation, snap_image_rect_to_device,
        snap_point_to_device, snap_radius_to_device, snap_rect_to_device,
        snap_stroked_line_to_device, snap_stroked_rect_to_device, transform_to_affine, vello_color,
        vello_gradient,
    },
    image::{
        ImageDataCache, atlas_source_fits_image, atlas_source_is_finite_positive,
        full_image_source, image_quality, image_resource_size_matches_atlas_source,
        image_resource_size_matches_pixels, source_size_matches_snapshot,
    },
    text::{encode_text_layout, shape_fallback_text},
};

pub(crate) fn encode_scene(
    scene: &mut Scene,
    commands: &[RenderCommand],
    resources: &RenderResources,
    fallback_text_layouts: &mut TextLayoutStore,
    image_cache: &mut ImageDataCache,
    device_scale: f64,
) {
    let root_transform = root_transform(device_scale);
    for command in commands {
        for clip in &command.clips {
            scene.push_clip_layer(
                Fill::NonZero,
                snap_axis_aligned_translation(root_transform * transform_to_affine(clip.transform)),
                &kurbo_rect(snap_rect_to_device(clip.rect, device_scale)),
            );
        }

        encode_command(
            scene,
            command,
            resources,
            fallback_text_layouts,
            image_cache,
            device_scale,
        );

        for _ in &command.clips {
            scene.pop_layer();
        }
    }
}

pub(crate) fn encode_command(
    scene: &mut Scene,
    command: &RenderCommand,
    resources: &RenderResources,
    fallback_text_layouts: &mut TextLayoutStore,
    image_cache: &mut ImageDataCache,
    device_scale: f64,
) {
    let raw_transform = root_transform(device_scale) * transform_to_affine(command.transform);
    let transform = snap_axis_aligned_translation(raw_transform);
    match &command.kind {
        RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } => encode_rect_command(
            scene,
            transform,
            *rect,
            *fill,
            *stroke,
            *radius,
            device_scale,
        ),
        RenderCommandKind::Line {
            x0,
            y0,
            x1,
            y1,
            stroke,
        } => encode_line_command(
            scene,
            transform,
            Point::new(*x0, *y0),
            Point::new(*x1, *y1),
            *stroke,
            device_scale,
        ),
        RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } => encode_shadow(
            scene,
            transform,
            ShadowPrimitive::new(*rect, *offset, *blur_radius, *spread, *radius, *color),
        ),
        RenderCommandKind::Path {
            elements,
            fill,
            stroke,
        } => encode_path(scene, transform, elements, *fill, *stroke, device_scale),
        RenderCommandKind::Text {
            layout,
            origin,
            text,
            family,
            size,
            line_height,
            color,
        } => encode_text_command(
            scene,
            raw_transform,
            transform,
            resources,
            fallback_text_layouts,
            *layout,
            *origin,
            text,
            family,
            *size,
            *line_height,
            *color,
        ),
        RenderCommandKind::Image { image, rect, tint } => encode_image_command(
            scene,
            transform,
            resources,
            image_cache,
            ImageCommandData::new(*image, *rect, *tint),
            device_scale,
        ),
        RenderCommandKind::Texture {
            texture,
            rect,
            source_size,
        } => {
            encode_texture_command(
                scene,
                transform,
                resources,
                image_cache,
                *texture,
                *rect,
                *source_size,
                device_scale,
            );
        }
    }
}

pub(crate) fn encode_line_command(
    scene: &mut Scene,
    transform: Affine,
    from: Point,
    to: Point,
    stroke: Stroke,
    device_scale: f64,
) {
    let (from, to) = snap_stroked_line_to_device(from, to, stroke.width, device_scale);
    let line = KurboLine::new(
        (f64::from(from.x), f64::from(from.y)),
        (f64::from(to.x), f64::from(to.y)),
    );
    stroke_shape(scene, transform, &stroke, &line, device_scale);
}

pub(crate) fn encode_rect_command(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    radius: CornerRadius,
    device_scale: f64,
) {
    if let Some(fill) = fill {
        let shape = rounded_rect(
            snap_rect_to_device(rect, device_scale),
            snap_radius_to_device(radius, device_scale),
        );
        fill_shape(scene, transform, &fill, &shape);
    }
    if let Some(stroke) = stroke
        && !encode_crisp_rect_border(scene, transform, rect, stroke, radius, device_scale)
    {
        let shape = rounded_rect(
            snap_stroked_rect_to_device(rect, stroke.width, device_scale),
            snap_radius_to_device(radius, device_scale),
        );
        stroke_shape(scene, transform, &stroke, &shape, device_scale);
    }
}

pub(crate) fn encode_crisp_rect_border(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    stroke: Stroke,
    radius: CornerRadius,
    device_scale: f64,
) -> bool {
    if !radius_is_zero(radius) {
        return false;
    }

    for segment in crisp_rect_border_segments(rect, stroke.width, device_scale) {
        let shape = kurbo_rect(segment);
        fill_shape(scene, transform, &stroke.brush, &shape);
    }
    true
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_text_command(
    scene: &mut Scene,
    raw_transform: Affine,
    effective_transform: Affine,
    resources: &RenderResources,
    fallback_text_layouts: &mut TextLayoutStore,
    layout: Option<TextLayoutId>,
    origin: Point,
    text: &str,
    family: &str,
    size: f32,
    line_height: f32,
    color: Color,
) {
    if let Some(resource) = layout.and_then(|id| resources.text_layout_resource(id)) {
        encode_text_layout(
            scene,
            raw_transform,
            effective_transform,
            origin,
            &resource.layout,
            color,
        );
    } else {
        let layout = shape_fallback_text(fallback_text_layouts, text, family, size, line_height);
        encode_text_layout(
            scene,
            raw_transform,
            effective_transform,
            origin,
            layout.as_ref(),
            color,
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ImageCommandData {
    image: ImageId,
    rect: Rect,
    tint: Option<Color>,
}

impl ImageCommandData {
    pub(crate) const fn new(image: ImageId, rect: Rect, tint: Option<Color>) -> Self {
        Self { image, rect, tint }
    }
}

pub(crate) fn encode_image_command(
    scene: &mut Scene,
    transform: Affine,
    resources: &RenderResources,
    image_cache: &mut ImageDataCache,
    command: ImageCommandData,
    device_scale: f64,
) {
    if let Some(draw) = resolve_image_draw(resources, command.image) {
        encode_image_region(
            scene,
            transform,
            command.rect,
            image_cache,
            draw,
            command.tint,
            device_scale,
        );
    } else {
        let rect = snap_rect_to_device(command.rect, device_scale);
        encode_resource_placeholder(
            scene,
            transform,
            rect,
            device_scale,
            Color::rgba(0.24, 0.32, 0.42, 0.35),
            Color::rgba(0.62, 0.72, 0.86, 0.75),
        );
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn encode_texture_command(
    scene: &mut Scene,
    transform: Affine,
    resources: &RenderResources,
    image_cache: &mut ImageDataCache,
    texture: TextureId,
    rect: Rect,
    source_size: Size,
    device_scale: f64,
) {
    if let Some(resource) = resources.texture(texture)
        && let Some(snapshot) = resource.snapshot.as_ref()
        && source_size_matches_snapshot(source_size, snapshot)
    {
        let source = full_image_source(snapshot);
        if snapshot.width == 0 || snapshot.height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
            return;
        }
        if !atlas_source_is_finite_positive(source) || !atlas_source_fits_image(source, snapshot) {
            return;
        }

        fill_image_region(
            scene,
            transform,
            rect,
            image_cache.texture_data(texture, snapshot),
            source,
            resource.sampling,
            device_scale,
        );
    } else {
        let rect = snap_rect_to_device(rect, device_scale);
        encode_resource_placeholder(
            scene,
            transform,
            rect,
            device_scale,
            Color::rgba(0.20, 0.34, 0.24, 0.35),
            Color::rgba(0.60, 0.84, 0.62, 0.75),
        );
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct ResolvedImageDraw<'a> {
    payload: ImageId,
    pixels: &'a RenderImage,
    source: Rect,
    sampling: RenderImageSampling,
}

pub(crate) fn resolve_image_draw(
    resources: &RenderResources,
    image: ImageId,
) -> Option<ResolvedImageDraw<'_>> {
    let resource = resources.image(image)?;
    if let Some(pixels) = resource.pixels.as_ref() {
        return image_resource_size_matches_pixels(resource, pixels).then_some(ResolvedImageDraw {
            payload: image,
            pixels,
            source: full_image_source(pixels),
            sampling: resource.sampling,
        });
    }
    let region = resource.atlas_region?;
    let atlas = resources.image(region.atlas)?;
    let pixels = atlas.pixels.as_ref()?;
    (image_resource_size_matches_pixels(atlas, pixels)
        && atlas_source_fits_image(region.source, pixels)
        && image_resource_size_matches_atlas_source(resource, region.source))
    .then_some(ResolvedImageDraw {
        payload: region.atlas,
        pixels,
        source: region.source,
        sampling: resource.sampling,
    })
}

pub(crate) fn encode_shadow(scene: &mut Scene, transform: Affine, shadow: ShadowPrimitive) {
    let shadow_rect = shadow
        .rect
        .translate(shadow.offset)
        .outset(shadow.spread)
        .max_zero();
    if shadow_rect.is_empty() {
        return;
    }
    let radius = (shadow.radius + shadow.spread).max(0.0);
    if shadow.blur_radius <= 0.0 {
        scene.fill(
            Fill::NonZero,
            transform,
            vello_color(shadow.color),
            None,
            &rounded_rect(shadow_rect, CornerRadius::all(radius)),
        );
        return;
    }
    scene.draw_blurred_rounded_rect(
        transform,
        kurbo_rect(shadow_rect),
        vello_color(shadow.color),
        f64::from(radius),
        f64::from(shadow.blur_radius),
    );
}

pub(crate) fn fill_shape(scene: &mut Scene, transform: Affine, brush: &Brush, shape: &impl Shape) {
    match brush {
        Brush::Solid(color) => {
            scene.fill(Fill::NonZero, transform, vello_color(*color), None, shape);
        }
        Brush::LinearGradient(gradient) => {
            let gradient = vello_gradient(gradient);
            scene.fill(Fill::NonZero, transform, &gradient, None, shape);
        }
    }
}

pub(crate) fn stroke_shape(
    scene: &mut Scene,
    transform: Affine,
    stroke: &Stroke,
    shape: &impl Shape,
    device_scale: f64,
) {
    let style = vello::kurbo::Stroke::new(f64::from(quantize_stroke_width_to_device(
        stroke.width,
        device_scale,
    )));
    match stroke.brush {
        Brush::Solid(color) => {
            scene.stroke(&style, transform, vello_color(color), None, shape);
        }
        Brush::LinearGradient(gradient) => {
            let gradient = vello_gradient(&gradient);
            scene.stroke(&style, transform, &gradient, None, shape);
        }
    }
}

pub(crate) fn encode_path(
    scene: &mut Scene,
    transform: Affine,
    elements: &[PathElement],
    fill: Option<Brush>,
    stroke: Option<Stroke>,
    device_scale: f64,
) {
    if let Some(fill) = fill {
        let snapped_elements = snap_filled_path_elements_to_device(elements, device_scale);
        let path = bez_path(&snapped_elements);
        fill_shape(scene, transform, &fill, &path);
    }
    if let Some(stroke) = stroke {
        let snapped_elements =
            snap_stroked_path_elements_to_device(elements, stroke.width, device_scale);
        let path = bez_path(&snapped_elements);
        stroke_shape(scene, transform, &stroke, &path, device_scale);
    }
}

pub(crate) fn bez_path(elements: &[PathElement]) -> BezPath {
    let mut path = BezPath::new();
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                path.move_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::LineTo(point) => {
                path.line_to((f64::from(point.x), f64::from(point.y)));
            }
            PathElement::QuadTo { ctrl, to } => {
                path.quad_to(
                    (f64::from(ctrl.x), f64::from(ctrl.y)),
                    (f64::from(to.x), f64::from(to.y)),
                );
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                path.curve_to(
                    (f64::from(ctrl1.x), f64::from(ctrl1.y)),
                    (f64::from(ctrl2.x), f64::from(ctrl2.y)),
                    (f64::from(to.x), f64::from(to.y)),
                );
            }
            PathElement::Close => path.close_path(),
        }
    }
    path
}

pub(crate) fn snap_filled_path_elements_to_device(
    elements: &[PathElement],
    device_scale: f64,
) -> Vec<PathElement> {
    if elements.iter().any(|element| {
        matches!(
            element,
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. }
        )
    }) {
        return elements.to_vec();
    }

    elements
        .iter()
        .map(|element| match *element {
            PathElement::MoveTo(point) => {
                PathElement::MoveTo(snap_point_to_device(point, device_scale))
            }
            PathElement::LineTo(point) => {
                PathElement::LineTo(snap_point_to_device(point, device_scale))
            }
            PathElement::Close => PathElement::Close,
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        })
        .collect()
}

pub(crate) fn snap_stroked_path_elements_to_device(
    elements: &[PathElement],
    stroke_width: f32,
    device_scale: f64,
) -> Vec<PathElement> {
    if elements.iter().any(|element| {
        matches!(
            element,
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. }
        )
    }) {
        return elements.to_vec();
    }

    let mut snapped = Vec::with_capacity(elements.len());
    let mut pending_move = None;
    let mut current = None;

    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                if let Some(point) = pending_move.replace(point) {
                    snapped.push(PathElement::MoveTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                current = Some(point);
            }
            PathElement::LineTo(point) => {
                if let Some(from) = current {
                    let (from, to) =
                        snap_stroked_line_to_device(from, point, stroke_width, device_scale);
                    if pending_move.take().is_some() {
                        snapped.push(PathElement::MoveTo(from));
                    }
                    snapped.push(PathElement::LineTo(to));
                } else {
                    snapped.push(PathElement::LineTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                current = Some(point);
            }
            PathElement::Close => {
                if let Some(point) = pending_move.take() {
                    snapped.push(PathElement::MoveTo(snap_point_to_device(
                        point,
                        device_scale,
                    )));
                }
                snapped.push(PathElement::Close);
                current = None;
            }
            PathElement::QuadTo { .. } | PathElement::CubicTo { .. } => unreachable!(),
        }
    }

    if let Some(point) = pending_move {
        snapped.push(PathElement::MoveTo(snap_point_to_device(
            point,
            device_scale,
        )));
    }

    snapped
}

pub(crate) fn encode_image_region(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    image_cache: &mut ImageDataCache,
    draw: ResolvedImageDraw<'_>,
    tint: Option<Color>,
    device_scale: f64,
) {
    let image = draw.pixels;
    let source = draw.source;
    if image.width == 0 || image.height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }
    if !atlas_source_is_finite_positive(source) || !atlas_source_fits_image(source, image) {
        return;
    }

    fill_image_region(
        scene,
        transform,
        rect,
        image_cache.image_data_with_tint(draw.payload, image, tint),
        source,
        draw.sampling,
        device_scale,
    );
}

pub(crate) fn fill_image_region(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    image_data: ImageData,
    source: Rect,
    sampling: RenderImageSampling,
    device_scale: f64,
) {
    let transform = snapped_image_region_transform(transform, rect, source, sampling, device_scale);
    let brush = ImageBrush::new(image_data).with_quality(image_quality(sampling));
    scene.fill(
        Fill::NonZero,
        transform,
        brush.as_ref(),
        None,
        &kurbo_rect(source),
    );
}

pub(crate) fn snapped_image_region_transform(
    transform: Affine,
    rect: Rect,
    source: Rect,
    sampling: RenderImageSampling,
    device_scale: f64,
) -> Affine {
    image_region_transform(
        transform,
        snap_image_rect_to_device(rect, sampling, device_scale),
        source,
    )
}

pub(crate) fn image_region_transform(transform: Affine, rect: Rect, source: Rect) -> Affine {
    let scale_x = f64::from(rect.width) / f64::from(source.width);
    let scale_y = f64::from(rect.height) / f64::from(source.height);
    transform
        * Affine::translate((f64::from(rect.x), f64::from(rect.y)))
        * Affine::scale_non_uniform(scale_x, scale_y)
        * Affine::translate((-f64::from(source.x), -f64::from(source.y)))
}

pub(crate) fn encode_resource_placeholder(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    device_scale: f64,
    fill: Color,
    stroke: Color,
) {
    let shape = rounded_rect(
        rect,
        snap_radius_to_device(CornerRadius::all(2.0), device_scale),
    );
    scene.fill(Fill::NonZero, transform, vello_color(fill), None, &shape);
    let stroke_style = vello::kurbo::Stroke::new(f64::from(quantize_stroke_width_to_device(
        1.0,
        device_scale,
    )));
    scene.stroke(&stroke_style, transform, vello_color(stroke), None, &shape);
    let first = KurboLine::new(
        (f64::from(rect.min_x()), f64::from(rect.min_y())),
        (f64::from(rect.max_x()), f64::from(rect.max_y())),
    );
    let second = KurboLine::new(
        (f64::from(rect.max_x()), f64::from(rect.min_y())),
        (f64::from(rect.min_x()), f64::from(rect.max_y())),
    );
    scene.stroke(
        &stroke_style,
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &first,
    );
    scene.stroke(
        &stroke_style,
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &second,
    );
}
