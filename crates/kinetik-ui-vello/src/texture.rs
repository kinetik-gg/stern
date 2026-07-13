use std::{
    collections::BTreeMap,
    num::NonZeroU64,
    sync::atomic::{AtomicU64, Ordering},
};

use kinetik_ui_core::TextureId;
use kinetik_ui_render::RenderImageSampling;
use vello::{peniko::ImageData, wgpu};

struct NativeTextureRecord {
    image: ImageData,
    extent: [u32; 2],
    sampling: RenderImageSampling,
}

/// Native-texture records owned by one Vello renderer.
///
/// This API requires the same renderer instance.
#[doc(hidden)]
pub struct VelloNativeTextureRegistry {
    renderer: NonZeroU64,
    active: BTreeMap<TextureId, NativeTextureRecord>,
    pending: BTreeMap<TextureId, NativeTextureRecord>,
}

/// Opaque identity for one Vello renderer's native-texture bridge.
///
/// This API requires the same renderer instance.
#[doc(hidden)]
pub struct VelloNativeTextureScope {
    renderer: NonZeroU64,
}

static NEXT_NATIVE_TEXTURE_SCOPE: AtomicU64 = AtomicU64::new(1);

impl VelloNativeTextureScope {
    /// Creates a unique lower-renderer scope.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    #[must_use]
    pub fn new() -> Option<Self> {
        let renderer = NEXT_NATIVE_TEXTURE_SCOPE
            .fetch_update(
                Ordering::Relaxed,
                Ordering::Relaxed,
                |value| /* Keep this closure expression explicit. */ value.checked_add(1),
            )
            .ok()?;
        Some(Self {
            renderer: NonZeroU64::new(renderer)?,
        })
    }
}

impl VelloNativeTextureRegistry {
    /// Creates an empty registry for a lower-renderer scope.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    #[must_use]
    pub fn new(scope: &VelloNativeTextureScope) -> Self {
        Self {
            renderer: scope.renderer,
            active: BTreeMap::new(),
            pending: BTreeMap::new(),
        }
    }

    /// Removes an active texture from lookup before mutation.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn begin_native_texture_update(
        &mut self,
        scope: &VelloNativeTextureScope,
        texture: TextureId,
    ) -> bool {
        if !self.scope_matches(scope) || self.pending.contains_key(&texture) {
            return false;
        }
        let Some(record) = self.active.remove(&texture) else {
            return false;
        };
        let _ = self.pending.insert(texture, record);
        true
    }

    /// Registers a native texture as pending.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn stage_native_texture(
        &mut self,
        scope: &VelloNativeTextureScope,
        renderer: &mut vello::Renderer,
        texture_id: TextureId,
        texture: wgpu::Texture,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> bool {
        if !self.scope_matches(scope)
            || self.active.contains_key(&texture_id)
            || self.pending.contains_key(&texture_id)
        {
            return false;
        }
        let image = renderer.register_texture(texture);
        let _ = self.pending.insert(
            texture_id,
            NativeTextureRecord {
                image,
                extent,
                sampling,
            },
        );
        true
    }

    /// Marks a pending native texture dirty.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn dirty_native_texture(
        &mut self,
        scope: &VelloNativeTextureScope,
        renderer: &mut vello::Renderer,
        texture: TextureId,
    ) -> bool {
        if !self.scope_matches(scope) {
            return false;
        }
        let Some(record) = self.pending.get(&texture) else {
            return false;
        };
        renderer.mark_override_image_dirty(&record.image);
        true
    }

    /// Replaces the image backing a pending native texture.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn replace_native_texture_image(
        &mut self,
        scope: &VelloNativeTextureScope,
        renderer: &mut vello::Renderer,
        texture_id: TextureId,
        texture: wgpu::Texture,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> bool {
        if !self.scope_matches(scope) {
            return false;
        }
        let Some(record) = self.pending.get_mut(&texture_id) else {
            return false;
        };
        let override_texture = wgpu::TexelCopyTextureInfoBase {
            texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        };
        let _ = renderer.override_image(&record.image, Some(override_texture));
        record.extent = extent;
        record.sampling = sampling;
        true
    }

    /// Unregisters and removes a pending native texture.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn retire_native_texture(
        &mut self,
        scope: &VelloNativeTextureScope,
        renderer: &mut vello::Renderer,
        texture: TextureId,
    ) -> bool {
        if !self.scope_matches(scope) {
            return false;
        }
        let Some(record) = self.pending.remove(&texture) else {
            return false;
        };
        renderer.unregister_texture(record.image);
        true
    }

    /// Publishes a pending native texture for lookup.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn commit_native_texture(
        &mut self,
        scope: &VelloNativeTextureScope,
        texture: TextureId,
    ) -> bool {
        if !self.scope_matches(scope) || self.active.contains_key(&texture) {
            return false;
        }
        let Some(record) = self.pending.remove(&texture) else {
            return false;
        };
        let _ = self.active.insert(texture, record);
        true
    }

    /// Invalidates all native texture lookups.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn clear_native_textures(&mut self, scope: &VelloNativeTextureScope) -> bool {
        if !self.scope_matches(scope) {
            return false;
        }
        self.active.clear();
        self.pending.clear();
        true
    }

    fn scope_matches(&self, scope: &VelloNativeTextureScope) -> bool {
        self.renderer == scope.renderer
    }

    pub(crate) fn resolve_native_texture(
        &self,
        scope: &VelloNativeTextureScope,
        texture: TextureId,
    ) -> Option<&ImageData> {
        if !self.scope_matches(scope) {
            return None;
        }
        self.active.get(&texture).map(|record| &record.image)
    }

    pub(crate) fn native_texture_metadata(
        &self,
        scope: &VelloNativeTextureScope,
        texture: TextureId,
    ) -> Option<([u32; 2], RenderImageSampling)> {
        if !self.scope_matches(scope) {
            return None;
        }
        self.active
            .get(&texture)
            .map(|record| (record.extent, record.sampling))
    }
}

#[cfg(test)]
impl VelloNativeTextureRegistry {
    pub(crate) fn install_test_native_texture(
        &mut self,
        scope: &VelloNativeTextureScope,
        texture: TextureId,
        image: ImageData,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> bool {
        if !self.scope_matches(scope)
            || self.active.contains_key(&texture)
            || self.pending.contains_key(&texture)
        {
            return false;
        }
        let _ = self.active.insert(
            texture,
            NativeTextureRecord {
                image,
                extent,
                sampling,
            },
        );
        true
    }
}
