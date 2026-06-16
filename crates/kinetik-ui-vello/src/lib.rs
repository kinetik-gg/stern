//! Vello renderer boundary for Kinetik UI render primitives.

use kinetik_ui_core::{
    Brush, ClipId, Color, CornerRadius, ImageId, LayerId, LinearGradient, PathElement, Point,
    Primitive, Rect, ShadowPrimitive, Size, Stroke, TextLayoutId, TextureId, Transform, Vec2,
};
pub use kinetik_ui_render::{
    ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput, RenderImage,
    RenderImageAlpha, RenderImageFormat, RenderResources, RendererBackend, TextLayoutResource,
    TextureResource, Translation as RenderTranslation,
};
use kinetik_ui_text::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextStyle};
use vello::{
    Glyph, Scene,
    kurbo::{
        Affine, BezPath, Line as KurboLine, Rect as KurboRect, RoundedRect, RoundedRectRadii, Shape,
    },
    peniko::{
        Blob, Fill, Gradient as PenikoGradient, ImageAlphaType, ImageBrush, ImageData, ImageFormat,
    },
};

/// Deterministic command produced before backend drawing.
#[derive(Debug, Clone, PartialEq)]
pub struct RenderCommand {
    /// Layer used by the command.
    pub layer: LayerId,
    /// Clip stack active for the command, outermost to innermost.
    pub clips: Vec<RenderClip>,
    /// Transform used by the command.
    pub transform: Transform,
    /// Command kind.
    pub kind: RenderCommandKind,
}

/// Clip scope captured for a render command.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderClip {
    /// Clip rectangle.
    pub rect: Rect,
    /// Transform active when the clip scope began.
    pub transform: Transform,
}

/// Command kind produced by primitive translation.
#[derive(Debug, Clone, PartialEq)]
pub enum RenderCommandKind {
    /// Filled and/or stroked rectangle.
    Rect {
        /// Rectangle bounds.
        rect: Rect,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
        /// Corner radii.
        radius: CornerRadius,
    },
    /// Stroked line.
    Line {
        /// Start x.
        x0: f32,
        /// Start y.
        y0: f32,
        /// End x.
        x1: f32,
        /// End y.
        y1: f32,
        /// Stroke style.
        stroke: Stroke,
    },
    /// Box shadow.
    Shadow {
        /// Source rectangle.
        rect: Rect,
        /// Shadow offset.
        offset: Vec2,
        /// Gaussian blur radius.
        blur_radius: f32,
        /// Spread amount.
        spread: f32,
        /// Uniform corner radius.
        radius: f32,
        /// Shadow color.
        color: Color,
    },
    /// Filled and/or stroked vector path.
    Path {
        /// Path elements in drawing order.
        elements: Vec<PathElement>,
        /// Fill brush.
        fill: Option<Brush>,
        /// Stroke style.
        stroke: Option<Stroke>,
    },
    /// Text command backed by a shaped layout resource or renderer fallback shaping.
    Text {
        /// Optional shaped layout resource.
        layout: Option<TextLayoutId>,
        /// Baseline origin.
        origin: kinetik_ui_core::Point,
        /// Text content.
        text: String,
        /// Font size in logical units.
        size: f32,
        /// Text color.
        color: Color,
    },
    /// Image resource draw command.
    Image {
        /// Image resource.
        image: ImageId,
        /// Destination rectangle.
        rect: Rect,
    },
    /// Texture resource draw command.
    Texture {
        /// Texture resource.
        texture: TextureId,
        /// Destination rectangle.
        rect: Rect,
    },
}

/// Vello renderer boundary.
pub struct VelloRenderer {
    scene: Scene,
    text_engine: CosmicTextEngine,
}

impl VelloRenderer {
    /// Creates a renderer boundary with an empty Vello scene.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            text_engine: CosmicTextEngine::new(),
        }
    }

    /// Returns the current Vello scene.
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    /// Submits a frame for translation.
    pub fn submit_frame(&mut self, input: RenderFrameInput<'_>) -> RenderFrameOutput {
        let translated = translate_primitives(input.primitives, input.resources);
        self.scene.reset();
        encode_scene(
            &mut self.scene,
            &translated.commands,
            input.resources,
            &mut self.text_engine,
        );
        RenderFrameOutput {
            primitive_count: input.primitives.len(),
            diagnostics: translated.diagnostics,
        }
    }
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererBackend for VelloRenderer {
    type Error = core::convert::Infallible;

    fn render_frame(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error> {
        Ok(self.submit_frame(input))
    }
}

/// Translation result used by tests and renderer internals.
pub type Translation = RenderTranslation<RenderCommand>;

/// Translates primitives into deterministic renderer commands.
#[must_use]
#[allow(clippy::too_many_lines)]
pub fn translate_primitives(primitives: &[Primitive], resources: &RenderResources) -> Translation {
    let mut commands = Vec::new();
    let mut diagnostics = Vec::new();
    let mut layers = vec![LayerId::from_raw(0)];
    let mut clips = Vec::<(ClipId, RenderClip)>::new();
    let mut transforms = Vec::<Transform>::new();
    let mut transform = Transform::IDENTITY;

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
                        size,
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
                match resources.image(image.image) {
                    None => diagnostics.push(RenderDiagnostic::MissingImage(image.image)),
                    Some(resource) if resource.pixels.is_none() => {
                        diagnostics.push(RenderDiagnostic::MissingImagePixels(image.image));
                    }
                    Some(_) => {}
                }
                commands.push(render_command(
                    &layers,
                    &clips,
                    transform,
                    RenderCommandKind::Image {
                        image: image.image,
                        rect,
                    },
                ));
            }
            Primitive::Texture(texture) => {
                let Some(rect) = sanitize_rect(texture.rect, &mut diagnostics, "texture") else {
                    continue;
                };
                if sanitize_size(texture.source_size).is_none() {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("texture_source_size"));
                }
                match resources.texture(texture.texture) {
                    None => diagnostics.push(RenderDiagnostic::MissingTexture(texture.texture)),
                    Some(resource) if resource.snapshot.is_none() => {
                        diagnostics.push(RenderDiagnostic::MissingTextureSnapshot(texture.texture));
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
                }
            }
            Primitive::LayerBegin { id } => {
                layers.push(*id);
            }
            Primitive::LayerEnd { id } => {
                if layers.len() > 1 && layers.last() == Some(id) {
                    layers.pop();
                }
            }
            Primitive::TransformBegin(next_transform) => {
                transforms.push(transform);
                let Some(next_transform) =
                    sanitize_transform(*next_transform, &mut diagnostics, "transform")
                else {
                    continue;
                };
                let next = compose_transform(transform, next_transform);
                if transform_is_finite(next) {
                    transform = next;
                } else {
                    diagnostics.push(RenderDiagnostic::InvalidGeometry("transform"));
                }
            }
            Primitive::TransformEnd => {
                transform = transforms.pop().unwrap_or(Transform::IDENTITY);
            }
        }
    }

    Translation {
        commands,
        diagnostics,
    }
}

/// Formats a translated command stream as stable line-oriented snapshot text.
#[must_use]
pub fn render_translation_snapshot(translation: &Translation) -> String {
    let mut lines = Vec::new();
    lines.push("commands:".to_owned());
    for (index, command) in translation.commands.iter().enumerate() {
        lines.push(format!(
            "  {index}: layer={} transform={} clips={} {}",
            command.layer.raw(),
            format_transform(command.transform),
            format_clips(&command.clips),
            format_command_kind(&command.kind),
        ));
    }
    lines.push("diagnostics:".to_owned());
    for diagnostic in &translation.diagnostics {
        lines.push(format!("  {}", format_diagnostic(diagnostic)));
    }
    lines.join("\n")
}

fn format_command_kind(kind: &RenderCommandKind) -> String {
    match kind {
        RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } => format!(
            "rect rect={} fill={} stroke={} radius={}",
            format_rect(*rect),
            format_optional_brush(fill.as_ref()),
            format_optional_stroke(stroke.as_ref()),
            format_radius(*radius),
        ),
        RenderCommandKind::Line {
            x0,
            y0,
            x1,
            y1,
            stroke,
        } => format!(
            "line from=({}, {}) to=({}, {}) stroke={}",
            format_f32(*x0),
            format_f32(*y0),
            format_f32(*x1),
            format_f32(*y1),
            format_stroke(*stroke),
        ),
        RenderCommandKind::Shadow {
            rect,
            offset,
            blur_radius,
            spread,
            radius,
            color,
        } => format!(
            "shadow rect={} offset=({}, {}) blur={} spread={} radius={} color={}",
            format_rect(*rect),
            format_f32(offset.x),
            format_f32(offset.y),
            format_f32(*blur_radius),
            format_f32(*spread),
            format_f32(*radius),
            format_color(*color),
        ),
        RenderCommandKind::Path {
            elements,
            fill,
            stroke,
        } => format!(
            "path elements={} fill={} stroke={}",
            format_path_elements(elements),
            format_optional_brush(fill.as_ref()),
            format_optional_stroke(stroke.as_ref()),
        ),
        RenderCommandKind::Text {
            layout,
            origin,
            text,
            size,
            color,
        } => format!(
            "text layout={} origin=({}, {}) size={} color={} text={:?}",
            layout.map_or_else(|| "none".to_owned(), |layout| layout.raw().to_string()),
            format_f32(origin.x),
            format_f32(origin.y),
            format_f32(*size),
            format_color(*color),
            text,
        ),
        RenderCommandKind::Image { image, rect } => {
            format!("image#{} rect={}", image.raw(), format_rect(*rect))
        }
        RenderCommandKind::Texture { texture, rect } => {
            format!("texture#{} rect={}", texture.raw(), format_rect(*rect))
        }
    }
}

fn format_clips(clips: &[RenderClip]) -> String {
    if clips.is_empty() {
        return "[]".to_owned();
    }
    let clips = clips
        .iter()
        .map(|clip| {
            format!(
                "{{rect={} transform={}}}",
                format_rect(clip.rect),
                format_transform(clip.transform)
            )
        })
        .collect::<Vec<_>>();
    format!("[{}]", clips.join(", "))
}

fn format_path_elements(elements: &[PathElement]) -> String {
    let elements = elements
        .iter()
        .map(|element| match element {
            PathElement::MoveTo(point) => {
                format!("M({}, {})", format_f32(point.x), format_f32(point.y))
            }
            PathElement::LineTo(point) => {
                format!("L({}, {})", format_f32(point.x), format_f32(point.y))
            }
            PathElement::QuadTo { ctrl, to } => format!(
                "Q({}, {}; {}, {})",
                format_f32(ctrl.x),
                format_f32(ctrl.y),
                format_f32(to.x),
                format_f32(to.y),
            ),
            PathElement::CubicTo { ctrl1, ctrl2, to } => format!(
                "C({}, {}; {}, {}; {}, {})",
                format_f32(ctrl1.x),
                format_f32(ctrl1.y),
                format_f32(ctrl2.x),
                format_f32(ctrl2.y),
                format_f32(to.x),
                format_f32(to.y),
            ),
            PathElement::Close => "Z".to_owned(),
        })
        .collect::<Vec<_>>();
    format!("[{}]", elements.join(", "))
}

fn format_optional_brush(brush: Option<&Brush>) -> String {
    brush.map_or_else(|| "none".to_owned(), |brush| format_brush(*brush))
}

fn format_brush(brush: Brush) -> String {
    match brush {
        Brush::Solid(color) => format_color(color),
        Brush::LinearGradient(gradient) => {
            let stops = gradient
                .stops()
                .iter()
                .map(|stop| format!("{}@{}", format_color(stop.color), format_f32(stop.offset)))
                .collect::<Vec<_>>();
            format!(
                "linear({},{})-({},{})[{}]",
                format_f32(gradient.start().x),
                format_f32(gradient.start().y),
                format_f32(gradient.end().x),
                format_f32(gradient.end().y),
                stops.join(",")
            )
        }
    }
}

fn format_optional_stroke(stroke: Option<&Stroke>) -> String {
    stroke.map_or_else(|| "none".to_owned(), |stroke| format_stroke(*stroke))
}

fn format_stroke(stroke: Stroke) -> String {
    format!(
        "{} {}",
        format_f32(stroke.width),
        format_brush(stroke.brush)
    )
}

fn format_rect(rect: Rect) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(rect.x),
        format_f32(rect.y),
        format_f32(rect.width),
        format_f32(rect.height),
    )
}

fn format_radius(radius: CornerRadius) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(radius.top_left),
        format_f32(radius.top_right),
        format_f32(radius.bottom_right),
        format_f32(radius.bottom_left),
    )
}

fn format_transform(transform: Transform) -> String {
    format!(
        "[{}, {}, {}, {}, {}, {}]",
        format_f32(transform.m11),
        format_f32(transform.m12),
        format_f32(transform.m21),
        format_f32(transform.m22),
        format_f32(transform.dx),
        format_f32(transform.dy),
    )
}

fn format_color(color: Color) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        format_f32(color.r),
        format_f32(color.g),
        format_f32(color.b),
        format_f32(color.a),
    )
}

fn format_diagnostic(diagnostic: &RenderDiagnostic) -> String {
    match diagnostic {
        RenderDiagnostic::MissingTextLayout(id) => format!("missing_text_layout#{}", id.raw()),
        RenderDiagnostic::MissingImage(id) => format!("missing_image#{}", id.raw()),
        RenderDiagnostic::MissingImagePixels(id) => {
            format!("missing_image_pixels#{}", id.raw())
        }
        RenderDiagnostic::MissingTexture(id) => format!("missing_texture#{}", id.raw()),
        RenderDiagnostic::MissingTextureSnapshot(id) => {
            format!("missing_texture_snapshot#{}", id.raw())
        }
        RenderDiagnostic::UnsupportedPrimitive(kind) => format!("unsupported_primitive:{kind}"),
        RenderDiagnostic::InvalidGeometry(kind) => format!("invalid_geometry:{kind}"),
    }
}

fn format_f32(value: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.3}")
}

fn render_command(
    layers: &[LayerId],
    clips: &[(ClipId, RenderClip)],
    transform: Transform,
    kind: RenderCommandKind,
) -> RenderCommand {
    RenderCommand {
        layer: layers
            .last()
            .copied()
            .unwrap_or_else(|| LayerId::from_raw(0)),
        clips: clips.iter().map(|(_, clip)| *clip).collect(),
        transform,
        kind,
    }
}

fn brush_fallback_color(brush: &Brush) -> Color {
    match brush {
        Brush::Solid(color) => *color,
        Brush::LinearGradient(gradient) => gradient
            .stops()
            .first()
            .map_or(Color::TRANSPARENT, |stop| stop.color),
    }
}

fn finite_non_negative(value: f32) -> f32 {
    if value.is_finite() {
        value.max(0.0)
    } else {
        0.0
    }
}

fn finite_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

fn finite_positive(value: f32) -> Option<f32> {
    value
        .is_finite()
        .then_some(value)
        .filter(|value| *value > 0.0)
}

fn finite_unit(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn sanitize_point(
    point: Point,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Point> {
    if point.x.is_finite() && point.y.is_finite() {
        Some(point)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

fn sanitize_vec2(
    offset: Vec2,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Vec2 {
    if offset.x.is_finite() && offset.y.is_finite() {
        offset
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        Vec2::new(finite_or_zero(offset.x), finite_or_zero(offset.y))
    }
}

fn sanitize_size(size: Size) -> Option<Size> {
    Some(Size::new(
        finite_positive(size.width)?,
        finite_positive(size.height)?,
    ))
}

fn sanitize_rect(
    rect: Rect,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Rect> {
    let Some(width) = finite_positive(rect.width) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    let Some(height) = finite_positive(rect.height) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    let mut invalid = false;
    let x = if rect.x.is_finite() {
        rect.x
    } else {
        invalid = true;
        0.0
    };
    let y = if rect.y.is_finite() {
        rect.y
    } else {
        invalid = true;
        0.0
    };
    if invalid {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
    }
    Some(Rect::new(x, y, width, height))
}

fn sanitize_color(
    color: Color,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Color {
    if color.r.is_finite() && color.g.is_finite() && color.b.is_finite() && color.a.is_finite() {
        color
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        Color::rgba(
            finite_unit(color.r),
            finite_unit(color.g),
            finite_unit(color.b),
            finite_unit(color.a),
        )
    }
}

fn sanitize_brush(
    brush: Brush,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Brush {
    match brush {
        Brush::Solid(color) => Brush::Solid(sanitize_color(color, diagnostics, context)),
        Brush::LinearGradient(gradient) => sanitize_linear_gradient(gradient, diagnostics, context)
            .map(Brush::LinearGradient)
            .unwrap_or_else(|| {
                Brush::Solid(sanitize_color(
                    brush_fallback_color(&Brush::LinearGradient(gradient)),
                    diagnostics,
                    context,
                ))
            }),
    }
}

fn sanitize_linear_gradient(
    gradient: LinearGradient,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<LinearGradient> {
    let start = sanitize_point(gradient.start(), diagnostics, context)?;
    let end = sanitize_point(gradient.end(), diagnostics, context)?;
    if start == end {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }

    let mut stops = gradient.stops().to_vec();
    if stops.len() < 2 {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }
    for stop in &mut stops {
        if !stop.offset.is_finite() || !(0.0..=1.0).contains(&stop.offset) {
            diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        }
        stop.offset = finite_unit(stop.offset);
        stop.color = sanitize_color(stop.color, diagnostics, context);
    }
    stops.sort_by(|a, b| {
        a.offset
            .partial_cmp(&b.offset)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    LinearGradient::new(start, end, &stops).ok()
}

fn sanitize_stroke(
    stroke: Stroke,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Stroke> {
    let Some(width) = finite_positive(stroke.width) else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    };
    Some(Stroke::new(
        width,
        sanitize_brush(stroke.brush, diagnostics, context),
    ))
}

fn sanitize_shadow(
    shadow: ShadowPrimitive,
    diagnostics: &mut Vec<RenderDiagnostic>,
) -> Option<ShadowPrimitive> {
    let rect = sanitize_rect(shadow.rect, diagnostics, "shadow")?;
    let offset = sanitize_vec2(shadow.offset, diagnostics, "shadow_offset");
    let blur_radius = sanitize_non_negative(shadow.blur_radius, diagnostics, "shadow_blur");
    let spread = sanitize_finite(shadow.spread, diagnostics, "shadow_spread");
    let radius = sanitize_non_negative(shadow.radius, diagnostics, "shadow_radius");
    let color = sanitize_color(shadow.color, diagnostics, "shadow_color");
    let shadow_rect = rect.translate(offset).outset(spread).max_zero();
    if shadow_rect.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry("shadow_spread"));
        return None;
    }
    Some(ShadowPrimitive::new(
        rect,
        offset,
        blur_radius,
        spread,
        radius,
        color,
    ))
}

fn sanitize_non_negative(
    value: f32,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        finite_non_negative(value)
    }
}

fn sanitize_finite(
    value: f32,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> f32 {
    if value.is_finite() {
        value
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        0.0
    }
}

fn sanitize_radius(
    radius: CornerRadius,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> CornerRadius {
    let invalid = !radius.top_left.is_finite()
        || !radius.top_right.is_finite()
        || !radius.bottom_right.is_finite()
        || !radius.bottom_left.is_finite()
        || radius.top_left < 0.0
        || radius.top_right < 0.0
        || radius.bottom_right < 0.0
        || radius.bottom_left < 0.0;
    if invalid {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
    }
    CornerRadius {
        top_left: finite_non_negative(radius.top_left),
        top_right: finite_non_negative(radius.top_right),
        bottom_right: finite_non_negative(radius.bottom_right),
        bottom_left: finite_non_negative(radius.bottom_left),
    }
}

fn sanitize_path_elements(
    elements: &[PathElement],
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Vec<PathElement>> {
    if elements.is_empty() {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        return None;
    }

    let mut sanitized = Vec::with_capacity(elements.len());
    let mut saw_point = false;
    for element in elements {
        match *element {
            PathElement::MoveTo(point) => {
                let point = sanitize_point(point, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::MoveTo(point));
            }
            PathElement::LineTo(point) => {
                let point = sanitize_point(point, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::LineTo(point));
            }
            PathElement::QuadTo { ctrl, to } => {
                let ctrl = sanitize_point(ctrl, diagnostics, context)?;
                let to = sanitize_point(to, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::QuadTo { ctrl, to });
            }
            PathElement::CubicTo { ctrl1, ctrl2, to } => {
                let ctrl1 = sanitize_point(ctrl1, diagnostics, context)?;
                let ctrl2 = sanitize_point(ctrl2, diagnostics, context)?;
                let to = sanitize_point(to, diagnostics, context)?;
                saw_point = true;
                sanitized.push(PathElement::CubicTo { ctrl1, ctrl2, to });
            }
            PathElement::Close => sanitized.push(PathElement::Close),
        }
    }

    if saw_point {
        Some(sanitized)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

fn transform_is_finite(transform: Transform) -> bool {
    transform.m11.is_finite()
        && transform.m12.is_finite()
        && transform.m21.is_finite()
        && transform.m22.is_finite()
        && transform.dx.is_finite()
        && transform.dy.is_finite()
}

fn sanitize_transform(
    transform: Transform,
    diagnostics: &mut Vec<RenderDiagnostic>,
    context: &'static str,
) -> Option<Transform> {
    if transform_is_finite(transform) {
        Some(transform)
    } else {
        diagnostics.push(RenderDiagnostic::InvalidGeometry(context));
        None
    }
}

fn encode_scene(
    scene: &mut Scene,
    commands: &[RenderCommand],
    resources: &RenderResources,
    text_engine: &mut CosmicTextEngine,
) {
    for command in commands {
        for clip in &command.clips {
            scene.push_clip_layer(
                Fill::NonZero,
                transform_to_affine(clip.transform),
                &kurbo_rect(clip.rect),
            );
        }

        encode_command(scene, command, resources, text_engine);

        for _ in &command.clips {
            scene.pop_layer();
        }
    }
}

fn encode_command(
    scene: &mut Scene,
    command: &RenderCommand,
    resources: &RenderResources,
    text_engine: &mut CosmicTextEngine,
) {
    let transform = transform_to_affine(command.transform);
    match &command.kind {
        RenderCommandKind::Rect {
            rect,
            fill,
            stroke,
            radius,
        } => {
            let shape = rounded_rect(*rect, *radius);
            if let Some(fill) = fill {
                fill_shape(scene, transform, fill, &shape);
            }
            if let Some(stroke) = stroke {
                stroke_shape(scene, transform, stroke, &shape);
            }
        }
        RenderCommandKind::Line {
            x0,
            y0,
            x1,
            y1,
            stroke,
        } => {
            let line = KurboLine::new(
                (f64::from(*x0), f64::from(*y0)),
                (f64::from(*x1), f64::from(*y1)),
            );
            stroke_shape(scene, transform, stroke, &line);
        }
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
        } => encode_path(scene, transform, elements, *fill, *stroke),
        RenderCommandKind::Text {
            layout,
            origin,
            text,
            size,
            color,
        } => {
            if let Some(layout) = layout.and_then(|id| resources.text_layout(id)) {
                encode_shaped_text(scene, transform, *origin, layout, *color);
            } else {
                let layout = shape_fallback_text(text_engine, text, *size);
                encode_shaped_text(scene, transform, *origin, &layout, *color);
            }
        }
        RenderCommandKind::Image { image, rect } => {
            if let Some(pixels) = resources
                .image(*image)
                .and_then(|resource| resource.pixels.as_ref())
            {
                encode_image(scene, transform, *rect, pixels);
            } else {
                encode_resource_placeholder(
                    scene,
                    transform,
                    *rect,
                    Color::rgba(0.24, 0.32, 0.42, 0.35),
                    Color::rgba(0.62, 0.72, 0.86, 0.75),
                );
            }
        }
        RenderCommandKind::Texture { texture, rect } => {
            if let Some(snapshot) = resources
                .texture(*texture)
                .and_then(|resource| resource.snapshot.as_ref())
            {
                encode_image(scene, transform, *rect, snapshot);
            } else {
                encode_resource_placeholder(
                    scene,
                    transform,
                    *rect,
                    Color::rgba(0.20, 0.34, 0.24, 0.35),
                    Color::rgba(0.60, 0.84, 0.62, 0.75),
                );
            }
        }
    }
}

fn encode_shadow(scene: &mut Scene, transform: Affine, shadow: ShadowPrimitive) {
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

fn fill_shape(scene: &mut Scene, transform: Affine, brush: &Brush, shape: &impl Shape) {
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

fn stroke_shape(scene: &mut Scene, transform: Affine, stroke: &Stroke, shape: &impl Shape) {
    let style = vello::kurbo::Stroke::new(f64::from(stroke.width));
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

fn encode_path(
    scene: &mut Scene,
    transform: Affine,
    elements: &[PathElement],
    fill: Option<Brush>,
    stroke: Option<Stroke>,
) {
    let path = bez_path(elements);
    if let Some(fill) = fill {
        fill_shape(scene, transform, &fill, &path);
    }
    if let Some(stroke) = stroke {
        stroke_shape(scene, transform, &stroke, &path);
    }
}

fn bez_path(elements: &[PathElement]) -> BezPath {
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

fn shape_fallback_text(
    text_engine: &mut CosmicTextEngine,
    text: &str,
    size: f32,
) -> ShapedTextLayout {
    text_engine.shape_text(&TextLayoutKey::new(
        text,
        TextStyle::new("sans-serif", size, size + 5.0),
        0.0,
        false,
    ))
}

fn encode_shaped_text(
    scene: &mut Scene,
    transform: Affine,
    origin: kinetik_ui_core::Point,
    layout: &ShapedTextLayout,
    color: Color,
) {
    for run in &layout.runs {
        scene
            .draw_glyphs(&run.font)
            .transform(transform)
            .font_size(run.font_size)
            .brush(vello_color(color))
            .draw(
                Fill::NonZero,
                run.glyphs.iter().map(|glyph| Glyph {
                    id: glyph.id,
                    x: origin.x + glyph.x,
                    y: origin.y + glyph.y,
                }),
            );
    }
}

fn encode_image(scene: &mut Scene, transform: Affine, rect: Rect, image: &RenderImage) {
    if image.width == 0 || image.height == 0 || rect.width <= 0.0 || rect.height <= 0.0 {
        return;
    }

    let image_data = ImageData {
        data: Blob::from(image.data.clone()),
        format: image_format(image.format),
        alpha_type: image_alpha(image.alpha),
        width: image.width,
        height: image.height,
    };
    let brush = ImageBrush::new(image_data);
    let scale_x = f64::from(rect.width) / f64::from(image.width);
    let scale_y = f64::from(rect.height) / f64::from(image.height);
    let image_transform = transform
        * Affine::translate((f64::from(rect.x), f64::from(rect.y)))
        * Affine::scale_non_uniform(scale_x, scale_y);
    scene.draw_image(brush.as_ref(), image_transform);
}

fn image_format(format: RenderImageFormat) -> ImageFormat {
    match format {
        RenderImageFormat::Rgba8 => ImageFormat::Rgba8,
        RenderImageFormat::Bgra8 => ImageFormat::Bgra8,
    }
}

fn image_alpha(alpha: RenderImageAlpha) -> ImageAlphaType {
    match alpha {
        RenderImageAlpha::Alpha => ImageAlphaType::Alpha,
        RenderImageAlpha::Premultiplied => ImageAlphaType::AlphaPremultiplied,
    }
}

fn encode_resource_placeholder(
    scene: &mut Scene,
    transform: Affine,
    rect: Rect,
    fill: Color,
    stroke: Color,
) {
    let shape = rounded_rect(rect, CornerRadius::all(2.0));
    scene.fill(Fill::NonZero, transform, vello_color(fill), None, &shape);
    scene.stroke(
        &vello::kurbo::Stroke::new(1.0),
        transform,
        vello_color(stroke),
        None,
        &shape,
    );
    let first = KurboLine::new(
        (f64::from(rect.min_x()), f64::from(rect.min_y())),
        (f64::from(rect.max_x()), f64::from(rect.max_y())),
    );
    let second = KurboLine::new(
        (f64::from(rect.max_x()), f64::from(rect.min_y())),
        (f64::from(rect.min_x()), f64::from(rect.max_y())),
    );
    scene.stroke(
        &vello::kurbo::Stroke::new(1.0),
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &first,
    );
    scene.stroke(
        &vello::kurbo::Stroke::new(1.0),
        transform,
        vello_color(stroke.with_alpha(0.45)),
        None,
        &second,
    );
}

fn transform_to_affine(transform: Transform) -> Affine {
    Affine::new([
        f64::from(transform.m11),
        f64::from(transform.m12),
        f64::from(transform.m21),
        f64::from(transform.m22),
        f64::from(transform.dx),
        f64::from(transform.dy),
    ])
}

fn compose_transform(parent: Transform, child: Transform) -> Transform {
    Transform {
        m11: parent.m11.mul_add(child.m11, parent.m21 * child.m12),
        m12: parent.m12.mul_add(child.m11, parent.m22 * child.m12),
        m21: parent.m11.mul_add(child.m21, parent.m21 * child.m22),
        m22: parent.m12.mul_add(child.m21, parent.m22 * child.m22),
        dx: parent
            .m11
            .mul_add(child.dx, parent.m21.mul_add(child.dy, parent.dx)),
        dy: parent
            .m12
            .mul_add(child.dx, parent.m22.mul_add(child.dy, parent.dy)),
    }
}

fn rounded_rect(rect: Rect, radius: CornerRadius) -> RoundedRect {
    RoundedRect::from_rect(kurbo_rect(rect), kurbo_radius(radius))
}

fn kurbo_rect(rect: Rect) -> KurboRect {
    KurboRect::new(
        f64::from(rect.min_x()),
        f64::from(rect.min_y()),
        f64::from(rect.max_x()),
        f64::from(rect.max_y()),
    )
}

fn kurbo_radius(radius: CornerRadius) -> RoundedRectRadii {
    RoundedRectRadii::new(
        f64::from(radius.top_left),
        f64::from(radius.top_right),
        f64::from(radius.bottom_right),
        f64::from(radius.bottom_left),
    )
}

fn vello_color(color: Color) -> vello::peniko::Color {
    vello::peniko::Color::new([
        finite_unit(color.r),
        finite_unit(color.g),
        finite_unit(color.b),
        finite_unit(color.a),
    ])
}

fn vello_gradient(gradient: &LinearGradient) -> PenikoGradient {
    let stops: Vec<(f32, vello::peniko::Color)> = gradient
        .stops()
        .iter()
        .map(|stop| (finite_unit(stop.offset), vello_color(stop.color)))
        .collect();
    PenikoGradient::new_linear(
        (f64::from(gradient.start().x), f64::from(gradient.start().y)),
        (f64::from(gradient.end().x), f64::from(gradient.end().y)),
    )
    .with_stops(stops.as_slice())
}

#[cfg(test)]
mod tests {
    use super::{
        ImageResource, RenderCommand, RenderCommandKind, RenderDiagnostic, RenderFrameInput,
        RenderImage, RenderResources, RendererBackend, TextLayoutResource, TextureResource,
        VelloRenderer, render_translation_snapshot, translate_primitives,
    };
    use kinetik_ui_core::render::TexturePrimitive;
    use kinetik_ui_core::{
        Brush, ClipId, Color, CornerRadius, GradientStop, ImageId, ImagePrimitive, LayerId,
        LinePrimitive, LinearGradient, PathElement, PathPrimitive, Point, Primitive, Rect,
        RectPrimitive, ScaleFactor, ShadowPrimitive, Size, Stroke, TextLayoutId, TextPrimitive,
        TextureId, Transform, Vec2, ViewportInfo,
    };
    use kinetik_ui_text::{
        CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle,
    };

    fn resources() -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(64.0, 64.0),
            pixels: Some(tiny_image()),
        });
        resources.register_texture(TextureResource {
            id: TextureId::from_raw(2),
            size: Size::new(128.0, 128.0),
            snapshot: Some(tiny_image()),
        });
        resources
    }

    fn size_only_resources() -> RenderResources {
        let mut resources = RenderResources::new();
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(64.0, 64.0),
            pixels: None,
        });
        resources.register_texture(TextureResource {
            id: TextureId::from_raw(2),
            size: Size::new(128.0, 128.0),
            snapshot: None,
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

    fn text_layout_resource(id: TextLayoutId, text: &str) -> TextLayoutResource {
        let mut engine = CosmicTextEngine::new();
        let layout = engine.shape_text(&TextLayoutKey::new(
            text,
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ));
        TextLayoutResource { id, layout }
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
    fn invalid_texture_source_size_is_diagnosed_without_dropping_rect() {
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
        assert_eq!(translation.commands.len(), 1);
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
                size: 12.0,
                brush: Brush::Solid(Color::BLACK),
            }),
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(9),
                rect: Rect::new(0.0, 20.0, 16.0, 16.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(20.0, 20.0, 16.0, 16.0),
                source_size: Size::new(16.0, 16.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert_eq!(
            render_translation_snapshot(&translation),
            "commands:\n  0: layer=3 transform=[1.000, 0.000, 0.000, 1.000, 2.000, 3.000] clips=[{rect=(0.000, 0.000, 20.000, 12.000) transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000]}] rect rect=(1.000, 1.000, 8.000, 4.000) fill=rgba(1.000, 1.000, 1.000, 1.000) stroke=none radius=(2.000, 2.000, 2.000, 2.000)\n  1: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] text layout=7 origin=(4.000, 16.000) size=12.000 color=rgba(0.000, 0.000, 0.000, 1.000) text=\"Hi\"\n  2: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] image#9 rect=(0.000, 20.000, 16.000, 16.000)\n  3: layer=0 transform=[1.000, 0.000, 0.000, 1.000, 0.000, 0.000] clips=[] texture#2 rect=(20.000, 20.000, 16.000, 16.000)\ndiagnostics:\n  missing_text_layout#7\n  missing_image#9"
        );
    }

    #[test]
    fn reports_missing_image_and_texture_resources() {
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(9),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
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
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                source_size: Size::new(10.0, 10.0),
            }),
        ];

        let translation = translate_primitives(&primitives, &resources());

        assert!(translation.diagnostics.is_empty());
    }

    #[test]
    fn registered_size_only_resources_emit_payload_diagnostics() {
        let primitives = vec![
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(1),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(0.0, 0.0, 10.0, 10.0),
                source_size: Size::new(10.0, 10.0),
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
            size: 12.0,
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
            size: 12.0,
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
        .expect("Vello frame submission is infallible before GPU presentation");

        assert_eq!(output.primitive_count, 0);
        assert!(output.diagnostics.is_empty());
        assert!(renderer.scene().encoding().is_empty());
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
                size: 12.0,
                brush: Brush::Solid(Color::WHITE),
            }),
            Primitive::Image(ImagePrimitive {
                image: ImageId::from_raw(1),
                rect: Rect::new(0.0, 24.0, 32.0, 24.0),
            }),
            Primitive::Texture(TexturePrimitive {
                texture: TextureId::from_raw(2),
                rect: Rect::new(40.0, 24.0, 32.0, 24.0),
                source_size: Size::new(32.0, 24.0),
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
    fn frame_submission_encodes_registered_shaped_text_layout() {
        let layout = TextLayoutId::from_raw(44);
        let mut resources = RenderResources::new();
        resources.register_text_layout(text_layout_resource(layout, "Label"));
        let primitives = vec![Primitive::Text(TextPrimitive {
            layout: Some(layout),
            origin: Point::new(4.0, 16.0),
            text: "Label".to_owned(),
            size: 12.0,
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
}
