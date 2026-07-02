use kinetik_ui_core::{Rect, ScaleFactor};

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn f64_to_f32(value: f64) -> f32 {
    if !value.is_finite() {
        return 0.0;
    }
    value.clamp(f64::from(f32::MIN), f64::from(f32::MAX)) as f32
}

fn f32_or_zero(value: f32) -> f32 {
    if value.is_finite() { value } else { 0.0 }
}

pub(crate) fn sanitize_scale_factor(scale_factor: ScaleFactor) -> ScaleFactor {
    if scale_factor.is_valid() {
        scale_factor
    } else {
        ScaleFactor::ONE
    }
}

pub(crate) fn sanitize_rect_for_platform(rect: Rect) -> Rect {
    Rect::new(
        f32_or_zero(rect.x),
        f32_or_zero(rect.y),
        if rect.width.is_finite() {
            rect.width.max(0.0)
        } else {
            0.0
        },
        if rect.height.is_finite() {
            rect.height.max(0.0)
        } else {
            0.0
        },
    )
}
