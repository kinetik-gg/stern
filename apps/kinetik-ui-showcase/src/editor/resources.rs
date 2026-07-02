/// Registers editor media and icon resources.
pub fn register_resources(resources: &mut RenderResources) {
    if let Some(snapshot) = RenderImage::rgba8(
        1280,
        720,
        include_bytes!("../../assets/viewport-1280x720.rgba").to_vec(),
    ) {
        resources.register_texture(TextureResource {
            id: VIEWPORT_TEXTURE,
            size: VIEWPORT_SIZE,
            sampling: RenderImageSampling::Pixelated,
            snapshot: Some(snapshot),
        });
    }
    for atlas in ICON_ATLASES {
        if let Some(snapshot) = RenderImage::rgba8(atlas.width, atlas.height, atlas.bytes.to_vec())
        {
            resources.register_image(ImageResource {
                id: atlas.image,
                size: Size::new(atlas.width as f32, atlas.height as f32),
                sampling: RenderImageSampling::UiIcon,
                pixels: Some(snapshot),
                atlas_region: None,
            });
        }
    }
    for entry in ICON_ENTRIES {
        resources.register_image(ImageResource {
            id: entry.image,
            size: Size::new(entry.logical_size as f32, entry.logical_size as f32),
            sampling: RenderImageSampling::UiIcon,
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: entry.atlas,
                source: entry.source,
            }),
        });
    }
}

#[cfg(test)]
fn icon_atlas_image(physical_size: u32) -> Option<RenderImage> {
    ICON_ATLASES
        .iter()
        .find(|atlas| atlas.physical_size == physical_size)
        .and_then(|atlas| RenderImage::rgba8(atlas.width, atlas.height, atlas.bytes.to_vec()))
}
