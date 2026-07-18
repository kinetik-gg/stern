//! Strict SVG/XML normalization with deterministic arc lowering.

use std::collections::BTreeMap;

use kurbo::{Arc, Point as KurboPoint, SvgArc, Vec2};
use quick_xml::{
    Reader,
    events::{BytesStart, Event},
};
use svgtypes::{PathParser, PathSegment};

use crate::{
    Error, ErrorKind, FillRule, NormalizedIcon, NormalizedPath, PathCommand, Point, Result,
    StrokeCap, StrokeJoin, StrokeStyle,
};

/// Normalizes one canonical SVG document into an owned arc-free vector model.
///
/// # Errors
///
/// Returns [`ErrorKind::Svg`] for malformed XML/path data, non-canonical view
/// boxes, unsupported elements or attributes, and invalid visual values.
pub fn normalize_svg(context: &str, source: &str) -> Result<NormalizedIcon> {
    let mut reader = Reader::from_str(source);
    reader.config_mut().trim_text(true);
    let mut saw_root = false;
    let mut inside_root = false;
    let mut closed_root = false;
    let mut open_path = false;
    let mut root_fill = true;
    let mut paths = Vec::new();
    loop {
        match reader.read_event() {
            Ok(Event::Start(element))
                if element.name().as_ref() == b"svg" && !saw_root && !inside_root =>
            {
                let attributes = attributes(context, &element)?;
                validate_root(context, &attributes)?;
                root_fill = parse_paint(
                    context,
                    attributes
                        .get("fill")
                        .map_or("currentColor", String::as_str),
                    "svg.fill",
                )?;
                saw_root = true;
                inside_root = true;
            }
            Ok(Event::Empty(element))
                if element.name().as_ref() == b"path" && inside_root && !open_path =>
            {
                paths.push(parse_path_element(context, &element, root_fill)?);
            }
            Ok(Event::Start(element))
                if element.name().as_ref() == b"path" && inside_root && !open_path =>
            {
                paths.push(parse_path_element(context, &element, root_fill)?);
                open_path = true;
            }
            Ok(Event::End(element)) if element.name().as_ref() == b"path" && open_path => {
                open_path = false;
            }
            Ok(Event::End(element))
                if element.name().as_ref() == b"svg" && inside_root && !open_path =>
            {
                inside_root = false;
                closed_root = true;
            }
            Ok(Event::Decl(_)) if !saw_root => {}
            Ok(Event::Comment(_)) if !open_path => {}
            Ok(Event::Text(text)) if text.as_ref().iter().all(u8::is_ascii_whitespace) => {}
            Ok(Event::Eof) => break,
            Ok(event) => {
                return Err(Error::new(
                    ErrorKind::Svg,
                    context,
                    format!("unsupported or misplaced XML event `{event:?}`"),
                ));
            }
            Err(error) => return Err(Error::new(ErrorKind::Svg, context, error.to_string())),
        }
    }
    if !saw_root || !closed_root || inside_root || open_path {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "document must contain one closed svg root",
        ));
    }
    if paths.is_empty() {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "icon contains no paths",
        ));
    }
    Ok(NormalizedIcon {
        width: 256.0,
        height: 256.0,
        paths,
    })
}

fn attributes(context: &str, element: &BytesStart<'_>) -> Result<BTreeMap<String, String>> {
    let mut result = BTreeMap::new();
    for attribute in element.attributes() {
        let attribute =
            attribute.map_err(|error| Error::new(ErrorKind::Svg, context, error.to_string()))?;
        let key = std::str::from_utf8(attribute.key.as_ref())
            .map_err(|error| Error::new(ErrorKind::Svg, context, error.to_string()))?
            .to_owned();
        let value = std::str::from_utf8(attribute.value.as_ref())
            .map_err(|error| Error::new(ErrorKind::Svg, context, error.to_string()))?
            .to_owned();
        if result.insert(key.clone(), value).is_some() {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                format!("duplicate attribute `{key}`"),
            ));
        }
    }
    Ok(result)
}

fn validate_root(context: &str, attributes: &BTreeMap<String, String>) -> Result<()> {
    for key in attributes.keys() {
        if !matches!(key.as_str(), "xmlns" | "viewBox" | "fill") {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                format!("unsupported svg attribute `{key}`"),
            ));
        }
    }
    if attributes.get("xmlns").map(String::as_str) != Some("http://www.w3.org/2000/svg") {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "svg xmlns must be `http://www.w3.org/2000/svg`",
        ));
    }
    if attributes.get("viewBox").map(String::as_str) != Some("0 0 256 256") {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "svg viewBox must be exactly `0 0 256 256`",
        ));
    }
    Ok(())
}

#[allow(clippy::too_many_lines)]
fn parse_path_element(
    context: &str,
    element: &BytesStart<'_>,
    inherited_fill: bool,
) -> Result<NormalizedPath> {
    let attributes = attributes(context, element)?;
    for key in attributes.keys() {
        if !matches!(
            key.as_str(),
            "d" | "fill"
                | "fill-rule"
                | "opacity"
                | "stroke"
                | "stroke-width"
                | "stroke-linecap"
                | "stroke-linejoin"
        ) {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                format!("unsupported path attribute `{key}`"),
            ));
        }
    }
    let data = attributes
        .get("d")
        .ok_or_else(|| Error::new(ErrorKind::Svg, context, "path d attribute is missing"))?;
    let commands = normalize_path_data(context, data)?;
    if commands.is_empty()
        || !commands
            .iter()
            .any(|command| !matches!(command, PathCommand::MoveTo(_) | PathCommand::Close))
    {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "path contains no drawable segments",
        ));
    }
    let filled = attributes.get("fill").map_or(Ok(inherited_fill), |value| {
        parse_paint(context, value, "path.fill")
    })?;
    let fill_rule = match attributes
        .get("fill-rule")
        .map_or("nonzero", String::as_str)
    {
        "nonzero" => FillRule::NonZero,
        "evenodd" => FillRule::EvenOdd,
        other => {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                format!("unsupported fill-rule `{other}`"),
            ));
        }
    };
    let opacity = parse_finite(
        context,
        attributes.get("opacity").map_or("1", String::as_str),
        "opacity",
    )?;
    if !(0.0..=1.0).contains(&opacity) {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "opacity must be between zero and one",
        ));
    }
    let stroked = attributes.get("stroke").map_or(Ok(false), |value| {
        parse_paint(context, value, "path.stroke")
    })?;
    let stroke = if stroked {
        let width = parse_finite(
            context,
            attributes.get("stroke-width").map_or("1", String::as_str),
            "stroke-width",
        )?;
        if width <= 0.0 {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                "stroke-width must be positive",
            ));
        }
        let cap = match attributes
            .get("stroke-linecap")
            .map_or("butt", String::as_str)
        {
            "butt" => StrokeCap::Butt,
            "round" => StrokeCap::Round,
            "square" => StrokeCap::Square,
            other => {
                return Err(Error::new(
                    ErrorKind::Svg,
                    context,
                    format!("unsupported stroke-linecap `{other}`"),
                ));
            }
        };
        let join = match attributes
            .get("stroke-linejoin")
            .map_or("miter", String::as_str)
        {
            "miter" => StrokeJoin::Miter,
            "round" => StrokeJoin::Round,
            "bevel" => StrokeJoin::Bevel,
            other => {
                return Err(Error::new(
                    ErrorKind::Svg,
                    context,
                    format!("unsupported stroke-linejoin `{other}`"),
                ));
            }
        };
        Some(StrokeStyle { width, cap, join })
    } else {
        if attributes.keys().any(|key| key.starts_with("stroke-")) {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                "stroke style is present without a visible stroke",
            ));
        }
        None
    };
    if !filled && stroke.is_none() {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            "path has neither fill nor stroke",
        ));
    }
    Ok(NormalizedPath {
        commands,
        filled,
        fill_rule,
        opacity,
        stroke,
    })
}

fn parse_paint(context: &str, value: &str, field: &str) -> Result<bool> {
    match value {
        "currentColor" => Ok(true),
        "none" => Ok(false),
        other => Err(Error::new(
            ErrorKind::Svg,
            context,
            format!("{field} must be `currentColor` or `none`, found `{other}`"),
        )),
    }
}

fn parse_finite(context: &str, value: &str, field: &str) -> Result<f64> {
    let parsed: f64 = value
        .parse()
        .map_err(|_| Error::new(ErrorKind::Svg, context, format!("{field} is not numeric")))?;
    if !parsed.is_finite() {
        return Err(Error::new(
            ErrorKind::Svg,
            context,
            format!("{field} must be finite"),
        ));
    }
    Ok(parsed)
}

#[allow(clippy::too_many_lines)]
fn normalize_path_data(context: &str, data: &str) -> Result<Vec<PathCommand>> {
    let mut output = Vec::new();
    let mut current = Point { x: 0.0, y: 0.0 };
    let mut subpath = current;
    let mut cubic_control: Option<Point> = None;
    let mut quad_control: Option<Point> = None;
    for segment in PathParser::from(data) {
        let segment =
            segment.map_err(|error| Error::new(ErrorKind::Svg, context, error.to_string()))?;
        let first_new_command = output.len();
        match segment {
            PathSegment::MoveTo { abs, x, y } => {
                current = resolve(abs, current, x, y);
                subpath = current;
                output.push(PathCommand::MoveTo(current));
                reset_controls(&mut cubic_control, &mut quad_control);
            }
            PathSegment::LineTo { abs, x, y } => {
                current = resolve(abs, current, x, y);
                output.push(PathCommand::LineTo(current));
                reset_controls(&mut cubic_control, &mut quad_control);
            }
            PathSegment::HorizontalLineTo { abs, x } => {
                current.x = if abs { x } else { current.x + x };
                output.push(PathCommand::LineTo(current));
                reset_controls(&mut cubic_control, &mut quad_control);
            }
            PathSegment::VerticalLineTo { abs, y } => {
                current.y = if abs { y } else { current.y + y };
                output.push(PathCommand::LineTo(current));
                reset_controls(&mut cubic_control, &mut quad_control);
            }
            PathSegment::CurveTo {
                abs,
                x1,
                y1,
                x2,
                y2,
                x,
                y,
            } => {
                let c1 = resolve(abs, current, x1, y1);
                let c2 = resolve(abs, current, x2, y2);
                let to = resolve(abs, current, x, y);
                output.push(PathCommand::CubicTo {
                    control1: c1,
                    control2: c2,
                    to,
                });
                current = to;
                cubic_control = Some(c2);
                quad_control = None;
            }
            PathSegment::SmoothCurveTo { abs, x2, y2, x, y } => {
                let c1 = cubic_control.map_or(current, |control| reflect(control, current));
                let c2 = resolve(abs, current, x2, y2);
                let to = resolve(abs, current, x, y);
                output.push(PathCommand::CubicTo {
                    control1: c1,
                    control2: c2,
                    to,
                });
                current = to;
                cubic_control = Some(c2);
                quad_control = None;
            }
            PathSegment::Quadratic { abs, x1, y1, x, y } => {
                let control = resolve(abs, current, x1, y1);
                let to = resolve(abs, current, x, y);
                output.push(PathCommand::QuadTo { control, to });
                current = to;
                quad_control = Some(control);
                cubic_control = None;
            }
            PathSegment::SmoothQuadratic { abs, x, y } => {
                let control = quad_control.map_or(current, |previous| reflect(previous, current));
                let to = resolve(abs, current, x, y);
                output.push(PathCommand::QuadTo { control, to });
                current = to;
                quad_control = Some(control);
                cubic_control = None;
            }
            PathSegment::EllipticalArc {
                abs,
                rx,
                ry,
                x_axis_rotation,
                large_arc,
                sweep,
                x,
                y,
            } => {
                if rx < 0.0 || ry < 0.0 {
                    return Err(Error::new(
                        ErrorKind::Svg,
                        context,
                        "arc radii cannot be negative",
                    ));
                }
                if ![rx, ry, x_axis_rotation, x, y]
                    .into_iter()
                    .all(f64::is_finite)
                {
                    return Err(Error::new(
                        ErrorKind::Svg,
                        context,
                        "arc geometry must be finite",
                    ));
                }
                let to = resolve(abs, current, x, y);
                lower_arc(
                    current,
                    to,
                    rx,
                    ry,
                    x_axis_rotation,
                    large_arc,
                    sweep,
                    &mut output,
                );
                current = to;
                reset_controls(&mut cubic_control, &mut quad_control);
            }
            PathSegment::ClosePath { .. } => {
                output.push(PathCommand::Close);
                current = subpath;
                reset_controls(&mut cubic_control, &mut quad_control);
            }
        }
        if !point_is_finite(current) || !output[first_new_command..].iter().all(command_is_finite) {
            return Err(Error::new(
                ErrorKind::Svg,
                context,
                "path arithmetic produced a non-finite coordinate",
            ));
        }
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
fn lower_arc(
    from: Point,
    to: Point,
    rx: f64,
    ry: f64,
    rotation_degrees: f64,
    large_arc: bool,
    sweep: bool,
    output: &mut Vec<PathCommand>,
) {
    if from == to {
        return;
    }
    let svg = SvgArc {
        from: KurboPoint::new(from.x, from.y),
        to: KurboPoint::new(to.x, to.y),
        radii: Vec2::new(rx, ry),
        x_rotation: rotation_degrees.to_radians(),
        large_arc,
        sweep,
    };
    if let Some(arc) = Arc::from_svg_arc(&svg) {
        arc.to_cubic_beziers(0.001, |c1, c2, end| {
            output.push(PathCommand::CubicTo {
                control1: point(c1),
                control2: point(c2),
                to: point(end),
            });
        });
    } else {
        output.push(PathCommand::LineTo(to));
    }
}

fn resolve(abs: bool, current: Point, x: f64, y: f64) -> Point {
    if abs {
        Point { x, y }
    } else {
        Point {
            x: current.x + x,
            y: current.y + y,
        }
    }
}
fn reflect(control: Point, around: Point) -> Point {
    Point {
        x: 2.0 * around.x - control.x,
        y: 2.0 * around.y - control.y,
    }
}
fn point(value: KurboPoint) -> Point {
    Point {
        x: value.x,
        y: value.y,
    }
}
fn point_is_finite(value: Point) -> bool {
    value.x.is_finite() && value.y.is_finite()
}
fn command_is_finite(command: &PathCommand) -> bool {
    match command {
        PathCommand::MoveTo(point) | PathCommand::LineTo(point) => point_is_finite(*point),
        PathCommand::QuadTo { control, to } => point_is_finite(*control) && point_is_finite(*to),
        PathCommand::CubicTo {
            control1,
            control2,
            to,
        } => point_is_finite(*control1) && point_is_finite(*control2) && point_is_finite(*to),
        PathCommand::Close => true,
    }
}
fn reset_controls(cubic: &mut Option<Point>, quad: &mut Option<Point>) {
    *cubic = None;
    *quad = None;
}
