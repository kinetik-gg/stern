//! Contact sheet assembly.
//!
//! The sheet itself is composed through the widget `Ui` (panel, labels,
//! image cells) and rasterized by the same CPU path as the stories, so the
//! contact sheet exercises the pipeline it documents.

use stern::UiState;
use stern::core::{
    Color, FrameContext, ImageId, PhysicalSize, Rect, ScaleFactor, Size, TimeInfo, UiInput,
    ViewportInfo, default_dark_theme,
};
use stern::render::{ImageResource, RenderImageSampling, RenderResources};

use crate::diff::RgbaImage;
use crate::raster::{RasterFrame, rasterize};

/// One labeled cell on the contact sheet.
#[derive(Debug, Clone)]
pub struct SheetCell {
    /// Label under the thumbnail (story id and variant).
    pub label: String,
    /// Full-resolution rendered image for this cell.
    pub image: RgbaImage,
}

const COLUMNS: u32 = 3;
const THUMB_MAX_WIDTH: u32 = 320;
const THUMB_MAX_HEIGHT: u32 = 220;
const CELL_WIDTH: f32 = 344.0;
const CELL_HEIGHT: f32 = 264.0;
const MARGIN: f32 = 16.0;

/// Builds the contact sheet image from rendered cells.
///
/// Returns `None` when `cells` is empty or rasterization fails.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn build_contact_sheet(cells: &[SheetCell]) -> Option<RasterFrame> {
    if cells.is_empty() {
        return None;
    }
    let columns = COLUMNS.min(u32::try_from(cells.len()).unwrap_or(COLUMNS)).max(1);
    let rows = cells.len().div_ceil(columns as usize) as u32;
    let logical = Size::new(
        2.0f32.mul_add(MARGIN, columns as f32 * CELL_WIDTH),
        2.0f32.mul_add(MARGIN, rows as f32 * CELL_HEIGHT),
    );

    let mut resources_extra: Vec<(ImageId, RgbaImage)> = Vec::new();
    let mut state = UiState::new();
    let theme = default_dark_theme();
    let context = FrameContext::new(
        ViewportInfo::new(
            logical,
            PhysicalSize::new(logical.width as u32, logical.height as u32),
            ScaleFactor::ONE,
        ),
        UiInput::default(),
        TimeInfo::default(),
    );
    let output = {
        let mut ui = state.begin_frame(context, &theme);
        for (index, cell) in cells.iter().enumerate() {
            let column = (index as u32) % columns;
            let row = (index as u32) / columns;
            let cell_rect = Rect::new(
                (column as f32).mul_add(CELL_WIDTH, MARGIN),
                (row as f32).mul_add(CELL_HEIGHT, MARGIN),
                CELL_WIDTH - 8.0,
                CELL_HEIGHT - 8.0,
            );
            ui.panel_keyed(("sheet-cell", index), cell_rect);
            let thumb = downscale_to_fit(&cell.image, THUMB_MAX_WIDTH, THUMB_MAX_HEIGHT);
            let image_id = ImageId::from_raw(index as u64 + 1);
            let thumb_rect = Rect::new(
                cell_rect.x + 8.0,
                cell_rect.y + 8.0,
                thumb.width as f32,
                thumb.height as f32,
            );
            ui.image_keyed(("sheet-thumb", index), thumb_rect, image_id);
            ui.label_keyed(
                ("sheet-label", index),
                Rect::new(
                    cell_rect.x + 8.0,
                    cell_rect.y + cell_rect.height - 24.0,
                    cell_rect.width - 16.0,
                    16.0,
                ),
                cell.label.clone(),
            );
            resources_extra.push((image_id, thumb));
        }
        ui.finish_output()
    };

    let mut resources: RenderResources = state.text_render_resources();
    for (image_id, thumb) in &resources_extra {
        let pixels =
            stern::render::RenderImage::rgba8(thumb.width, thumb.height, thumb.pixels.clone())?;
        resources.register_image(ImageResource {
            id: *image_id,
            size: Size::new(thumb.width as f32, thumb.height as f32),
            sampling: RenderImageSampling::Pixelated,
            pixels: Some(pixels),
            atlas_region: None,
        });
    }

    rasterize(
        &output.primitives,
        &resources,
        logical,
        1.0,
        Color::rgb8(0x0B, 0x0B, 0x0B),
    )
}

/// Downscales by integer block averaging so the result is deterministic.
#[must_use]
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn downscale_to_fit(image: &RgbaImage, max_width: u32, max_height: u32) -> RgbaImage {
    let factor_w = image.width.div_ceil(max_width.max(1));
    let factor_h = image.height.div_ceil(max_height.max(1));
    let factor = factor_w.max(factor_h).max(1);
    if factor == 1 {
        return image.clone();
    }
    let out_width = image.width.div_ceil(factor).max(1);
    let out_height = image.height.div_ceil(factor).max(1);
    let mut pixels = Vec::with_capacity((out_width * out_height * 4) as usize);
    for out_y in 0..out_height {
        for out_x in 0..out_width {
            let mut sum = [0.0_f32; 4];
            let mut count = 0.0_f32;
            for source_y in (out_y * factor)..((out_y + 1) * factor).min(image.height) {
                for source_x in (out_x * factor)..((out_x + 1) * factor).min(image.width) {
                    let index = ((source_y * image.width + source_x) * 4) as usize;
                    for (channel, value) in sum.iter_mut().enumerate() {
                        *value += f32::from(image.pixels[index + channel]);
                    }
                    count += 1.0;
                }
            }
            for channel in sum {
                pixels.push((channel / count.max(1.0)).round().clamp(0.0, 255.0) as u8);
            }
        }
    }
    RgbaImage {
        width: out_width,
        height: out_height,
        pixels,
    }
}
