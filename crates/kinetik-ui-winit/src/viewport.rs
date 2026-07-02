use kinetik_ui_core::{
    FrameContext, PhysicalSize, ScaleFactor, Size, TimeInfo, UiInput, ViewportInfo,
};
use winit::dpi::PhysicalSize as WinitPhysicalSize;

use crate::utils::{f64_to_f32, sanitize_scale_factor};
/// Converts a winit physical size and scale factor into viewport information.
#[must_use]
pub fn viewport_from_winit(size: WinitPhysicalSize<u32>, scale_factor: f64) -> ViewportInfo {
    let scale_factor = scale_factor_from_winit(scale_factor);
    let logical_width = f64::from(size.width) / scale_factor.value();
    let logical_height = f64::from(size.height) / scale_factor.value();

    ViewportInfo::new(
        Size::new(f64_to_f32(logical_width), f64_to_f32(logical_height)),
        PhysicalSize::new(size.width, size.height),
        scale_factor,
    )
}

/// Converts winit viewport data and a UI input snapshot into a full frame context.
#[must_use]
pub fn frame_context_from_winit(
    size: WinitPhysicalSize<u32>,
    scale_factor: f64,
    input: UiInput,
    time: TimeInfo,
) -> FrameContext {
    FrameContext::new(viewport_from_winit(size, scale_factor), input, time)
}

/// Converts a raw winit scale factor into a valid toolkit scale factor.
#[must_use]
pub fn scale_factor_from_winit(scale_factor: f64) -> ScaleFactor {
    sanitize_scale_factor(ScaleFactor::new(scale_factor))
}
