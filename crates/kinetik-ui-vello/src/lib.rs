//! Vello renderer boundary for Kinetik UI render primitives.

mod command;
mod encoding;
mod geometry;
mod image;
mod renderer;
mod sanitize;
mod snapshot;
mod text;
mod translation;

pub use command::{RenderClip, RenderCommand, RenderCommandKind, Translation};
pub use kinetik_ui_render::{
    ImageAtlasRegion, ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput,
    RenderImage, RenderImageAlpha, RenderImageFormat, RenderImageSampling, RenderResources,
    RendererBackend, TextLayoutResource, TextureResource, Translation as RenderTranslation,
};
pub use renderer::{VelloRenderer, VelloRendererError};
pub use snapshot::render_translation_snapshot;
pub use translation::translate_primitives;

#[cfg(test)]
use encoding::{
    image_region_transform, snap_filled_path_elements_to_device,
    snap_stroked_path_elements_to_device, snapped_image_region_transform,
};
#[cfg(test)]
use geometry::{
    crisp_rect_border_segments, quantize_stroke_width_to_device, root_transform,
    snap_axis_aligned_translation, snap_image_rect_to_device, snap_point_to_device,
    snap_radius_to_device, snap_rect_to_device, snap_stroke_center_to_device,
    snap_stroked_line_to_device, snap_stroked_rect_to_device, viewport_device_scale,
    viewport_size_device_scale,
};
#[cfg(test)]
use image::{
    ImageDataCache, MAX_CACHED_IMAGE_ENTRIES, MAX_CACHED_TEXTURE_ENTRIES,
    MAX_CACHED_TINTED_IMAGE_BYTES, MAX_TINTED_IMAGE_CACHE_ENTRIES, PackedTint, image_quality,
};
#[cfg(test)]
use text::{
    MAX_CACHED_TEXT_LAYOUTS, ShapedTextCache, physical_text_layout, physical_text_layout_for_key,
    quantize_physical_text_extent, snap_text_glyph_baseline_to_device,
    snap_text_glyph_position_to_device, snap_text_origin_to_device,
    snap_text_transform_origin_to_device, transform_point,
};

#[cfg(test)]
mod tests;
