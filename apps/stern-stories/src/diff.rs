//! Pixel diff logic for golden comparison.
//!
//! Comparison is exact: any channel delta is a difference. The diff image
//! dims matching pixels and paints differing pixels magenta so a human can
//! find the change instantly.

/// An RGBA8 image buffer (straight alpha).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RgbaImage {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// RGBA8 bytes, row-major, `width * height * 4` long.
    pub pixels: Vec<u8>,
}

impl RgbaImage {
    /// Creates an image buffer after validating dimensions.
    #[must_use]
    pub fn new(width: u32, height: u32, pixels: Vec<u8>) -> Option<Self> {
        if pixels.len()
            == (width as usize)
                .checked_mul(height as usize)?
                .checked_mul(4)?
        {
            Some(Self {
                width,
                height,
                pixels,
            })
        } else {
            None
        }
    }
}

/// Result of comparing a current render against a golden.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffOutcome {
    /// Same dimensions, identical pixels.
    Match,
    /// Same dimensions, differing pixels.
    PixelsDiffer {
        /// Number of pixels with any channel delta.
        differing_pixels: usize,
        /// Largest absolute channel delta observed.
        max_channel_delta: u8,
        /// Visual diff image.
        diff: RgbaImage,
    },
    /// Dimensions differ; pixel comparison is not meaningful.
    DimensionsDiffer {
        /// Current dimensions.
        current: (u32, u32),
        /// Golden dimensions.
        golden: (u32, u32),
    },
}

impl DiffOutcome {
    /// Reports whether the comparison found any difference.
    #[must_use]
    pub const fn is_match(&self) -> bool {
        matches!(self, Self::Match)
    }
}

/// Compares `current` against `golden` exactly.
#[must_use]
pub fn diff_images(current: &RgbaImage, golden: &RgbaImage) -> DiffOutcome {
    if (current.width, current.height) != (golden.width, golden.height) {
        return DiffOutcome::DimensionsDiffer {
            current: (current.width, current.height),
            golden: (golden.width, golden.height),
        };
    }
    let mut differing_pixels = 0_usize;
    let mut max_channel_delta = 0_u8;
    let mut diff_pixels = vec![0_u8; current.pixels.len()];
    for (index, (current_px, golden_px)) in current
        .pixels
        .chunks_exact(4)
        .zip(golden.pixels.chunks_exact(4))
        .enumerate()
    {
        let mut delta = 0_u8;
        for channel in 0..4 {
            delta = delta.max(current_px[channel].abs_diff(golden_px[channel]));
        }
        let out = &mut diff_pixels[index * 4..index * 4 + 4];
        if delta == 0 {
            // Dimmed grayscale of the golden pixel keeps context visible.
            let luma = luminance(golden_px) / 4;
            out.copy_from_slice(&[luma, luma, luma, 255]);
        } else {
            differing_pixels += 1;
            max_channel_delta = max_channel_delta.max(delta);
            out.copy_from_slice(&[255, 0, 200, 255]);
        }
    }
    if differing_pixels == 0 {
        return DiffOutcome::Match;
    }
    DiffOutcome::PixelsDiffer {
        differing_pixels,
        max_channel_delta,
        diff: RgbaImage {
            width: current.width,
            height: current.height,
            pixels: diff_pixels,
        },
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn luminance(rgba: &[u8]) -> u8 {
    let value = 0.114_f32.mul_add(
        f32::from(rgba[2]),
        0.299_f32.mul_add(f32::from(rgba[0]), 0.587 * f32::from(rgba[1])),
    );
    value.round().clamp(0.0, 255.0) as u8
}
