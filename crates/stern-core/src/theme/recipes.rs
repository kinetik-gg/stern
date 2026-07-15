use super::{FontToken, ShadowRecipe};
use crate::{
    Brush, Color, CornerRadius, PathElement, PathPrimitive, Point, Primitive, Rect, RectPrimitive,
    Stroke,
};

/// Component state used by style recipes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[allow(clippy::struct_excessive_bools)]
pub struct ComponentState {
    /// Hovered state.
    pub hovered: bool,
    /// Pressed state.
    pub pressed: bool,
    /// Focused state.
    pub focused: bool,
    /// Disabled state.
    pub disabled: bool,
    /// Selected state.
    pub selected: bool,
}

/// Independent two-tone focus-ring paint recipe.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FocusRingRecipe {
    /// Outer primary focus indicator stroke.
    pub primary: Stroke,
    /// Inner contrast separator stroke.
    pub separator: Stroke,
}

impl FocusRingRecipe {
    /// Builds exact nested outer-primary then inner-separator filled contours.
    #[must_use]
    pub fn primitives(self, rect: Rect, radius: CornerRadius) -> [Primitive; 2] {
        let total_width = self.separator.width + self.primary.width;
        [
            Primitive::Rect(RectPrimitive {
                rect: rect.outset(total_width),
                fill: Some(self.primary.brush),
                stroke: None,
                radius: expanded_radius(radius, total_width),
            }),
            Primitive::Rect(RectPrimitive {
                rect: rect.outset(self.separator.width),
                fill: Some(self.separator.brush),
                stroke: None,
                radius: expanded_radius(radius, self.separator.width),
            }),
        ]
    }

    /// Builds hollow focus annuli outside an unconstrained component boundary.
    #[must_use]
    pub fn outward_annulus_primitives(self, rect: Rect, radius: CornerRadius) -> [Primitive; 2] {
        let rect = sanitized_rect(rect);
        let radius = sanitized_radius(radius);
        let primary_width = sanitized_width(self.primary.width);
        let separator_width = sanitized_width(self.separator.width);
        let total_width = finite_sum(primary_width, separator_width);
        let inner = rounded_boundary(rect, radius);
        let primary_outer = outset_boundary(rect, radius, total_width);
        let separator_outer = outset_boundary(rect, radius, separator_width);

        [
            compound_annulus(primary_outer, inner, self.primary.brush),
            compound_annulus(separator_outer, inner, self.separator.brush),
        ]
    }

    /// Builds hollow focus annuli inside a clip-constrained component boundary.
    #[must_use]
    pub fn inward_annulus_primitives(
        self,
        rect: Rect,
        radius: CornerRadius,
        boundary_width: f32,
    ) -> [Primitive; 2] {
        let rect = sanitized_rect(rect);
        let radius = sanitized_radius(radius);
        let primary_width = sanitized_width(self.primary.width);
        let separator_width = sanitized_width(self.separator.width);
        let boundary_width = sanitized_width(boundary_width);
        let total_width = finite_sum(primary_width, separator_width);
        let primary_outer = inset_boundary(rect, radius, boundary_width);
        let separator_outer =
            inset_boundary(rect, radius, finite_sum(boundary_width, primary_width));
        let inner = inset_boundary(rect, radius, finite_sum(boundary_width, total_width));

        [
            compound_annulus(primary_outer, inner, self.primary.brush),
            compound_annulus(separator_outer, inner, self.separator.brush),
        ]
    }
}

const KAPPA: f32 = 0.552_284_8;

#[derive(Clone, Copy)]
struct RoundedBoundary {
    rect: Rect,
    radius: CornerRadius,
}

fn sanitized_width(value: f32) -> f32 {
    if value.is_finite() && value >= 0.0 {
        value
    } else {
        0.0
    }
}

fn finite_sum(left: f32, right: f32) -> f32 {
    let sum = left + right;
    if sum.is_finite() { sum } else { f32::MAX }
}

fn sanitized_rect(rect: Rect) -> Rect {
    let (x, width) = sanitized_axis(rect.x, rect.width);
    let (y, height) = sanitized_axis(rect.y, rect.height);
    Rect::new(x, y, width, height)
}

fn sanitized_axis(origin: f32, extent: f32) -> (f32, f32) {
    let origin = if origin.is_finite() { origin } else { 0.0 };
    let extent = if extent.is_finite() { extent } else { 0.0 };
    if extent >= 0.0 {
        (origin, extent)
    } else {
        let center = origin + extent * 0.5;
        let center = if center.is_finite() {
            center
        } else if center.is_sign_negative() {
            f32::MIN
        } else {
            f32::MAX
        };
        (center, 0.0)
    }
}

fn sanitized_radius(radius: CornerRadius) -> CornerRadius {
    CornerRadius {
        top_left: sanitized_width(radius.top_left),
        top_right: sanitized_width(radius.top_right),
        bottom_right: sanitized_width(radius.bottom_right),
        bottom_left: sanitized_width(radius.bottom_left),
    }
}

fn expanded_annulus_radius(radius: CornerRadius, amount: f32) -> CornerRadius {
    CornerRadius {
        top_left: finite_sum(radius.top_left, amount),
        top_right: finite_sum(radius.top_right, amount),
        bottom_right: finite_sum(radius.bottom_right, amount),
        bottom_left: finite_sum(radius.bottom_left, amount),
    }
}

fn contracted_annulus_radius(radius: CornerRadius, amount: f32) -> CornerRadius {
    CornerRadius {
        top_left: (radius.top_left - amount).max(0.0),
        top_right: (radius.top_right - amount).max(0.0),
        bottom_right: (radius.bottom_right - amount).max(0.0),
        bottom_left: (radius.bottom_left - amount).max(0.0),
    }
}

fn outset_boundary(rect: Rect, radius: CornerRadius, amount: f32) -> RoundedBoundary {
    rounded_boundary(
        Rect::new(
            rect.x - amount,
            rect.y - amount,
            finite_sum(rect.width, finite_sum(amount, amount)),
            finite_sum(rect.height, finite_sum(amount, amount)),
        ),
        expanded_annulus_radius(radius, amount),
    )
}

fn inset_boundary(rect: Rect, radius: CornerRadius, amount: f32) -> RoundedBoundary {
    let limit = rect.width.min(rect.height) * 0.5;
    let amount = amount.min(limit);
    rounded_boundary(
        Rect::new(
            rect.x + amount,
            rect.y + amount,
            (rect.width - amount * 2.0).max(0.0),
            (rect.height - amount * 2.0).max(0.0),
        ),
        contracted_annulus_radius(radius, amount),
    )
}

#[allow(clippy::cast_possible_truncation)]
fn rounded_boundary(rect: Rect, radius: CornerRadius) -> RoundedBoundary {
    let top = f64::from(radius.top_left) + f64::from(radius.top_right);
    let bottom = f64::from(radius.bottom_left) + f64::from(radius.bottom_right);
    let left = f64::from(radius.top_left) + f64::from(radius.bottom_left);
    let right = f64::from(radius.top_right) + f64::from(radius.bottom_right);
    let mut factor = 1.0_f64;
    for (extent, sum) in [
        (f64::from(rect.width), top),
        (f64::from(rect.width), bottom),
        (f64::from(rect.height), left),
        (f64::from(rect.height), right),
    ] {
        if sum > 0.0 {
            factor = factor.min(extent / sum);
        }
    }
    // Every sanitized radius is finite and nonnegative, and `factor` is at most
    // one, so these deliberate narrowing conversions remain finite and in range.
    RoundedBoundary {
        rect,
        radius: CornerRadius {
            top_left: (f64::from(radius.top_left) * factor) as f32,
            top_right: (f64::from(radius.top_right) * factor) as f32,
            bottom_right: (f64::from(radius.bottom_right) * factor) as f32,
            bottom_left: (f64::from(radius.bottom_left) * factor) as f32,
        },
    }
}

fn compound_annulus(outer: RoundedBoundary, inner: RoundedBoundary, brush: Brush) -> Primitive {
    let mut elements = Vec::with_capacity(20);
    append_clockwise_rounded_rect(&mut elements, outer);
    append_counter_clockwise_rounded_rect(&mut elements, inner);
    Primitive::Path(PathPrimitive::new(elements, Some(brush), None))
}

fn append_clockwise_rounded_rect(elements: &mut Vec<PathElement>, boundary: RoundedBoundary) {
    let rect = boundary.rect;
    let radius = boundary.radius;
    let min_x = rect.min_x();
    let min_y = rect.min_y();
    let max_x = rect.max_x();
    let max_y = rect.max_y();

    elements.push(PathElement::MoveTo(Point::new(
        min_x + radius.top_left,
        min_y,
    )));
    elements.push(PathElement::LineTo(Point::new(
        max_x - radius.top_right,
        min_y,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(max_x - radius.top_right * (1.0 - KAPPA), min_y),
        ctrl2: Point::new(max_x, min_y + radius.top_right * (1.0 - KAPPA)),
        to: Point::new(max_x, min_y + radius.top_right),
    });
    elements.push(PathElement::LineTo(Point::new(
        max_x,
        max_y - radius.bottom_right,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(max_x, max_y - radius.bottom_right * (1.0 - KAPPA)),
        ctrl2: Point::new(max_x - radius.bottom_right * (1.0 - KAPPA), max_y),
        to: Point::new(max_x - radius.bottom_right, max_y),
    });
    elements.push(PathElement::LineTo(Point::new(
        min_x + radius.bottom_left,
        max_y,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(min_x + radius.bottom_left * (1.0 - KAPPA), max_y),
        ctrl2: Point::new(min_x, max_y - radius.bottom_left * (1.0 - KAPPA)),
        to: Point::new(min_x, max_y - radius.bottom_left),
    });
    elements.push(PathElement::LineTo(Point::new(
        min_x,
        min_y + radius.top_left,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(min_x, min_y + radius.top_left * (1.0 - KAPPA)),
        ctrl2: Point::new(min_x + radius.top_left * (1.0 - KAPPA), min_y),
        to: Point::new(min_x + radius.top_left, min_y),
    });
    elements.push(PathElement::Close);
}

fn append_counter_clockwise_rounded_rect(
    elements: &mut Vec<PathElement>,
    boundary: RoundedBoundary,
) {
    let rect = boundary.rect;
    let radius = boundary.radius;
    let min_x = rect.min_x();
    let min_y = rect.min_y();
    let max_x = rect.max_x();
    let max_y = rect.max_y();

    elements.push(PathElement::MoveTo(Point::new(
        min_x + radius.top_left,
        min_y,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(min_x + radius.top_left * (1.0 - KAPPA), min_y),
        ctrl2: Point::new(min_x, min_y + radius.top_left * (1.0 - KAPPA)),
        to: Point::new(min_x, min_y + radius.top_left),
    });
    elements.push(PathElement::LineTo(Point::new(
        min_x,
        max_y - radius.bottom_left,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(min_x, max_y - radius.bottom_left * (1.0 - KAPPA)),
        ctrl2: Point::new(min_x + radius.bottom_left * (1.0 - KAPPA), max_y),
        to: Point::new(min_x + radius.bottom_left, max_y),
    });
    elements.push(PathElement::LineTo(Point::new(
        max_x - radius.bottom_right,
        max_y,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(max_x - radius.bottom_right * (1.0 - KAPPA), max_y),
        ctrl2: Point::new(max_x, max_y - radius.bottom_right * (1.0 - KAPPA)),
        to: Point::new(max_x, max_y - radius.bottom_right),
    });
    elements.push(PathElement::LineTo(Point::new(
        max_x,
        min_y + radius.top_right,
    )));
    elements.push(PathElement::CubicTo {
        ctrl1: Point::new(max_x, min_y + radius.top_right * (1.0 - KAPPA)),
        ctrl2: Point::new(max_x - radius.top_right * (1.0 - KAPPA), min_y),
        to: Point::new(max_x - radius.top_right, min_y),
    });
    elements.push(PathElement::LineTo(Point::new(
        min_x + radius.top_left,
        min_y,
    )));
    elements.push(PathElement::Close);
}

const fn expanded_radius(radius: CornerRadius, amount: f32) -> CornerRadius {
    CornerRadius {
        top_left: radius.top_left + amount,
        top_right: radius.top_right + amount,
        bottom_right: radius.bottom_right + amount,
        bottom_left: radius.bottom_left + amount,
    }
}

/// Button visual variant.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Neutral raised button.
    #[default]
    Standard,
    /// Primary call-to-action button.
    Primary,
    /// Low-emphasis button with transparent fill.
    Ghost,
    /// Destructive button.
    Danger,
}

/// Button visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ButtonRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
}

/// Text visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextRecipe {
    /// Foreground text color.
    pub foreground: Color,
    /// Text font token.
    pub font: FontToken,
}

/// Panel visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PanelRecipe {
    /// Background brush.
    pub background: Brush,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Optional panel shadow.
    pub shadow: Option<ShadowRecipe>,
}

/// Separator visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SeparatorRecipe {
    /// Separator stroke.
    pub stroke: Stroke,
}

/// Tab visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Optional active indicator brush.
    pub indicator: Option<Brush>,
    /// Active indicator thickness.
    pub indicator_thickness: f32,
}

/// List or table row recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text/icon color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
}

/// Checkbox and radio visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CheckRecipe {
    /// Box or circle fill.
    pub fill: Brush,
    /// Mark color.
    pub mark: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Box or circle side length.
    pub size: f32,
}

/// Toggle visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ToggleRecipe {
    /// Track fill.
    pub track: Brush,
    /// Thumb fill.
    pub thumb: Brush,
    /// Track border.
    pub border: Stroke,
    /// Inner track padding.
    pub padding: f32,
}

/// Slider visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SliderRecipe {
    /// Track fill.
    pub track: Brush,
    /// Filled range brush.
    pub fill: Brush,
    /// Track border.
    pub border: Stroke,
    /// Track radius.
    pub radius: CornerRadius,
}

/// Text field visual recipe output.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextFieldRecipe {
    /// Background brush.
    pub background: Brush,
    /// Text color.
    pub foreground: Color,
    /// Border stroke.
    pub border: Stroke,
    /// Corner radius.
    pub radius: CornerRadius,
    /// Selection fill brush.
    pub selection: Brush,
    /// Caret color.
    pub caret: Color,
    /// Horizontal padding.
    pub padding_x: f32,
    /// Vertical padding.
    pub padding_y: f32,
    /// Font token.
    pub font: FontToken,
}
