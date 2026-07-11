use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    sync::Arc,
};

use kinetik_ui_core::{Color, ImageId, Rect, Size, TextureId};
use kinetik_ui_render::{
    ImageResource, RenderImage, RenderImageAlpha, RenderImageFormat, RenderImageSampling,
};
use vello::peniko::{Blob, ImageAlphaType, ImageData, ImageFormat, ImageQuality};

use crate::geometry::logical_size_matches;

pub(crate) const MAX_CACHED_IMAGE_ENTRIES: usize = 512;
pub(crate) const MAX_CACHED_TEXTURE_ENTRIES: usize = 256;
pub(crate) const MAX_TINTED_IMAGE_CACHE_ENTRIES: usize = 64;
pub(crate) const MAX_CACHED_TINTED_IMAGE_BYTES: usize = 512 * 1024;

#[derive(Debug, Clone)]
pub(crate) struct CachedImageData {
    pub(crate) signature: ImageSignature,
    pub(crate) data: ImageData,
}

#[derive(Debug, Clone)]
pub(crate) struct ImageSignature {
    pub(crate) width: u32,
    pub(crate) height: u32,
    pub(crate) format: RenderImageFormat,
    pub(crate) alpha: RenderImageAlpha,
    pub(crate) data: Arc<[u8]>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(crate) struct PackedTint(u32);

impl ImageSignature {
    pub(crate) fn matches(&self, image: &RenderImage) -> bool {
        self.width == image.width
            && self.height == image.height
            && self.format == image.format
            && self.alpha == image.alpha
            && Arc::ptr_eq(&self.data, &image.data)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ImageDataCache {
    pub(crate) images: HashMap<ImageId, CachedImageData>,
    pub(crate) image_order: VecDeque<ImageId>,
    pub(crate) tinted_images: HashMap<(ImageId, PackedTint), CachedImageData>,
    pub(crate) tinted_image_order: VecDeque<(ImageId, PackedTint)>,
    pub(crate) textures: HashMap<TextureId, CachedImageData>,
    pub(crate) texture_order: VecDeque<TextureId>,
}

impl ImageDataCache {
    pub(crate) fn image_data(&mut self, id: ImageId, image: &RenderImage) -> ImageData {
        cached_image_data(
            &mut self.images,
            &mut self.image_order,
            MAX_CACHED_IMAGE_ENTRIES,
            id,
            image,
        )
    }

    pub(crate) fn image_data_with_tint(
        &mut self,
        id: ImageId,
        image: &RenderImage,
        tint: Option<Color>,
    ) -> ImageData {
        let Some(tint) = tint else {
            return self.image_data(id, image);
        };
        cached_tinted_image_data(
            &mut self.tinted_images,
            &mut self.tinted_image_order,
            id,
            image,
            PackedTint::from_color(tint),
        )
    }

    pub(crate) fn texture_data(&mut self, id: TextureId, image: &RenderImage) -> ImageData {
        cached_image_data(
            &mut self.textures,
            &mut self.texture_order,
            MAX_CACHED_TEXTURE_ENTRIES,
            id,
            image,
        )
    }

    #[cfg(test)]
    pub(crate) fn image_len(&self) -> usize {
        self.images.len()
    }

    #[cfg(test)]
    pub(crate) fn texture_len(&self) -> usize {
        self.textures.len()
    }
}

pub(crate) fn cached_image_data<Id>(
    cache: &mut HashMap<Id, CachedImageData>,
    order: &mut VecDeque<Id>,
    capacity: usize,
    id: Id,
    image: &RenderImage,
) -> ImageData
where
    Id: Copy + Eq + Hash,
{
    let signature = image_signature(image);
    if let Some(cached) = cache.get(&id)
        && cached.signature.matches(image)
    {
        touch_cache_key(order, id);
        return cached.data.clone();
    }

    let data = image_data_from_render_image(image);
    remember_cache_key(cache, order, capacity, id);
    cache.insert(
        id,
        CachedImageData {
            signature,
            data: data.clone(),
        },
    );
    data
}

pub(crate) fn cached_tinted_image_data(
    cache: &mut HashMap<(ImageId, PackedTint), CachedImageData>,
    order: &mut VecDeque<(ImageId, PackedTint)>,
    id: ImageId,
    image: &RenderImage,
    tint: PackedTint,
) -> ImageData {
    let signature = image_signature(image);
    let key = (id, tint);
    if let Some(cached) = cache.get(&key)
        && cached.signature.matches(image)
    {
        touch_cache_key(order, key);
        return cached.data.clone();
    }

    let data = tinted_image_data_from_render_image(image, tint);
    if image.data.len() > MAX_CACHED_TINTED_IMAGE_BYTES {
        return data;
    }
    remember_cache_key(cache, order, MAX_TINTED_IMAGE_CACHE_ENTRIES, key);
    cache.insert(
        key,
        CachedImageData {
            signature,
            data: data.clone(),
        },
    );
    data
}

pub(crate) fn remember_cache_key<Id, Value>(
    cache: &mut HashMap<Id, Value>,
    order: &mut VecDeque<Id>,
    capacity: usize,
    id: Id,
) where
    Id: Copy + Eq + Hash,
{
    touch_cache_key(order, id);
    if cache.contains_key(&id) {
        return;
    }

    while cache.len() >= capacity {
        let Some(evicted) = order.pop_front() else {
            break;
        };
        cache.remove(&evicted);
    }
}

pub(crate) fn touch_cache_key<Id>(order: &mut VecDeque<Id>, id: Id)
where
    Id: Copy + Eq,
{
    if let Some(position) = order.iter().position(|existing| *existing == id) {
        order.remove(position);
    }
    order.push_back(id);
}

pub(crate) fn image_signature(image: &RenderImage) -> ImageSignature {
    ImageSignature {
        width: image.width,
        height: image.height,
        format: image.format,
        alpha: image.alpha,
        data: Arc::clone(&image.data),
    }
}

pub(crate) fn image_data_from_render_image(image: &RenderImage) -> ImageData {
    ImageData {
        data: Blob::from(image.data.to_vec()),
        format: image_format(image.format),
        alpha_type: image_alpha(image.alpha),
        width: image.width,
        height: image.height,
    }
}

pub(crate) fn tinted_image_data_from_render_image(
    image: &RenderImage,
    tint: PackedTint,
) -> ImageData {
    let [red, green, blue, alpha] = tint.channels();
    let premultiplied = image.alpha == RenderImageAlpha::Premultiplied;
    let mut data = image.data.to_vec();
    for pixel in data.chunks_exact_mut(4) {
        match image.format {
            RenderImageFormat::Rgba8 => {
                pixel[0] = multiply_color_channel(pixel[0], red, alpha, premultiplied);
                pixel[1] = multiply_color_channel(pixel[1], green, alpha, premultiplied);
                pixel[2] = multiply_color_channel(pixel[2], blue, alpha, premultiplied);
                pixel[3] = multiply_channel(pixel[3], alpha);
            }
            RenderImageFormat::Bgra8 => {
                pixel[0] = multiply_color_channel(pixel[0], blue, alpha, premultiplied);
                pixel[1] = multiply_color_channel(pixel[1], green, alpha, premultiplied);
                pixel[2] = multiply_color_channel(pixel[2], red, alpha, premultiplied);
                pixel[3] = multiply_channel(pixel[3], alpha);
            }
        }
    }
    ImageData {
        data: Blob::from(data),
        format: image_format(image.format),
        alpha_type: image_alpha(image.alpha),
        width: image.width,
        height: image.height,
    }
}

impl PackedTint {
    pub(crate) fn from_color(color: Color) -> Self {
        Self(
            (unit_channel(color.r) << 24)
                | (unit_channel(color.g) << 16)
                | (unit_channel(color.b) << 8)
                | unit_channel(color.a),
        )
    }

    pub(crate) fn channels(self) -> [u8; 4] {
        [
            ((self.0 >> 24) & 0xff) as u8,
            ((self.0 >> 16) & 0xff) as u8,
            ((self.0 >> 8) & 0xff) as u8,
            (self.0 & 0xff) as u8,
        ]
    }
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
pub(crate) fn unit_channel(value: f32) -> u32 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u32
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn multiply_channel(source: u8, tint: u8) -> u8 {
    ((u16::from(source) * u16::from(tint) + 127) / 255) as u8
}

pub(crate) fn multiply_color_channel(
    source: u8,
    tint: u8,
    tint_alpha: u8,
    premultiplied: bool,
) -> u8 {
    if premultiplied {
        multiply_premultiplied_channel(source, tint, tint_alpha)
    } else {
        multiply_channel(source, tint)
    }
}

#[allow(clippy::cast_possible_truncation)]
pub(crate) fn multiply_premultiplied_channel(source: u8, tint: u8, tint_alpha: u8) -> u8 {
    let product = u32::from(source) * u32::from(tint) * u32::from(tint_alpha);
    ((product + 32_512) / 65_025) as u8
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn full_image_source(image: &RenderImage) -> Rect {
    Rect::new(0.0, 0.0, image.width as f32, image.height as f32)
}

pub(crate) fn atlas_source_is_finite_positive(source: Rect) -> bool {
    source.x.is_finite()
        && source.y.is_finite()
        && source.width.is_finite()
        && source.height.is_finite()
        && source.width > 0.0
        && source.height > 0.0
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn atlas_source_fits_image(source: Rect, image: &RenderImage) -> bool {
    atlas_source_is_finite_positive(source)
        && source.x >= 0.0
        && source.y >= 0.0
        && source.max_x() <= image.width as f32
        && source.max_y() <= image.height as f32
}

pub(crate) fn source_size_matches_snapshot(source_size: Size, image: &RenderImage) -> bool {
    (f64::from(source_size.width) - f64::from(image.width)).abs() <= f64::EPSILON
        && (f64::from(source_size.height) - f64::from(image.height)).abs() <= f64::EPSILON
}

#[allow(clippy::cast_precision_loss)]
pub(crate) fn logical_size_matches_snapshot(size: Size, image: &RenderImage) -> bool {
    logical_size_matches(size, Size::new(image.width as f32, image.height as f32))
}

pub(crate) fn image_resource_size_matches_pixels(
    resource: &ImageResource,
    pixels: &RenderImage,
) -> bool {
    logical_size_matches_snapshot(resource.size, pixels)
}

pub(crate) fn image_resource_size_matches_atlas_source(
    resource: &ImageResource,
    source: Rect,
) -> bool {
    logical_size_matches(resource.size, Size::new(source.width, source.height))
}

pub(crate) fn image_quality(sampling: RenderImageSampling) -> ImageQuality {
    match sampling {
        RenderImageSampling::Pixelated | RenderImageSampling::UiIcon => ImageQuality::Low,
        RenderImageSampling::Smooth => ImageQuality::Medium,
        RenderImageSampling::HighQuality => ImageQuality::High,
    }
}

pub(crate) fn image_format(format: RenderImageFormat) -> ImageFormat {
    match format {
        RenderImageFormat::Rgba8 => ImageFormat::Rgba8,
        RenderImageFormat::Bgra8 => ImageFormat::Bgra8,
    }
}

pub(crate) fn image_alpha(alpha: RenderImageAlpha) -> ImageAlphaType {
    match alpha {
        RenderImageAlpha::Alpha => ImageAlphaType::Alpha,
        RenderImageAlpha::Premultiplied => ImageAlphaType::AlphaPremultiplied,
    }
}
