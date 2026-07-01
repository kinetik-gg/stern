use kinetik_ui_core::{Brush, Color, CornerRadius, PathElement, Rect, Size, Stroke, Transform};
use kinetik_ui_render::RenderDiagnostic;

use crate::command::{RenderClip, RenderCommandKind, Translation};

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

pub(crate) fn format_command_kind(kind: &RenderCommandKind) -> String {
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
            family,
            size,
            line_height,
            color,
        } => format!(
            "text layout={} origin=({}, {}) family={:?} size={} line_height={} color={} text={:?}",
            layout.map_or_else(|| "none".to_owned(), |layout| layout.raw().to_string()),
            format_f32(origin.x),
            format_f32(origin.y),
            family,
            format_f32(*size),
            format_f32(*line_height),
            format_color(*color),
            text,
        ),
        RenderCommandKind::Image { image, rect, tint } => {
            format!(
                "image#{} rect={} tint={}",
                image.raw(),
                format_rect(*rect),
                tint.map_or_else(|| "none".to_owned(), format_color)
            )
        }
        RenderCommandKind::Texture {
            texture,
            rect,
            source_size,
        } => {
            format!(
                "texture#{} rect={} source_size={}",
                texture.raw(),
                format_rect(*rect),
                format_size(*source_size)
            )
        }
    }
}

pub(crate) fn format_clips(clips: &[RenderClip]) -> String {
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

pub(crate) fn format_path_elements(elements: &[PathElement]) -> String {
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

pub(crate) fn format_optional_brush(brush: Option<&Brush>) -> String {
    brush.map_or_else(|| "none".to_owned(), |brush| format_brush(*brush))
}

pub(crate) fn format_brush(brush: Brush) -> String {
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

pub(crate) fn format_optional_stroke(stroke: Option<&Stroke>) -> String {
    stroke.map_or_else(|| "none".to_owned(), |stroke| format_stroke(*stroke))
}

pub(crate) fn format_stroke(stroke: Stroke) -> String {
    format!(
        "{} {}",
        format_f32(stroke.width),
        format_brush(stroke.brush)
    )
}

pub(crate) fn format_rect(rect: Rect) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(rect.x),
        format_f32(rect.y),
        format_f32(rect.width),
        format_f32(rect.height),
    )
}

pub(crate) fn format_size(size: Size) -> String {
    format!("{}x{}", format_f32(size.width), format_f32(size.height))
}

pub(crate) fn format_radius(radius: CornerRadius) -> String {
    format!(
        "({}, {}, {}, {})",
        format_f32(radius.top_left),
        format_f32(radius.top_right),
        format_f32(radius.bottom_right),
        format_f32(radius.bottom_left),
    )
}

pub(crate) fn format_transform(transform: Transform) -> String {
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

pub(crate) fn format_color(color: Color) -> String {
    format!(
        "rgba({}, {}, {}, {})",
        format_f32(color.r),
        format_f32(color.g),
        format_f32(color.b),
        format_f32(color.a),
    )
}

pub(crate) fn format_diagnostic(diagnostic: &RenderDiagnostic) -> String {
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

pub(crate) fn format_f32(value: f32) -> String {
    let value = if value.is_finite() { value } else { 0.0 };
    let value = if value == 0.0 { 0.0 } else { value };
    format!("{value:.3}")
}
