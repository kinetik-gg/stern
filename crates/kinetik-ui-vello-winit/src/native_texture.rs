use std::collections::BTreeMap;

use kinetik_ui_core::TextureId;
use kinetik_ui_render::{RenderImageSampling, TextureResource};
use kinetik_ui_vello::{VelloNativeTextureRegistry, VelloNativeTextureScope};
use vello::wgpu;

use crate::{
    PresenterDeviceScope, VelloNativeTextureValidationError, VelloPresenterError,
    VelloWindowPresenter,
};

/// Opaque registration for one presenter device generation and texture ID.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VelloNativeTextureRegistration {
    scope: PresenterDeviceScope,
    texture: TextureId,
    registration_generation: u64,
}

/// Result of updating a registered native texture revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloNativeTextureUpdateOutcome {
    /// The requested content revision was already current.
    Unchanged,
    /// The registered Vello image was marked dirty and republished.
    MarkedDirty,
}

impl VelloNativeTextureRegistration {
    /// Returns the neutral texture ID bound by this registration.
    #[must_use]
    pub const fn texture_id(&self) -> TextureId {
        self.texture
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeTextureDescriptor {
    extent: wgpu::Extent3d,
    format: wgpu::TextureFormat,
    usage: wgpu::TextureUsages,
    dimension: wgpu::TextureDimension,
    mip_level_count: u32,
    sample_count: u32,
}

impl NativeTextureDescriptor {
    fn from_texture(texture: &wgpu::Texture) -> Self {
        Self {
            extent: texture.size(),
            format: texture.format(),
            usage: texture.usage(),
            dimension: texture.dimension(),
            mip_level_count: texture.mip_level_count(),
            sample_count: texture.sample_count(),
        }
    }

    fn validate(
        &self,
        resource: &TextureResource,
    ) -> Result<[u32; 2], VelloNativeTextureValidationError> {
        if self.extent.width == 0 || self.extent.height == 0 {
            return Err(VelloNativeTextureValidationError::ZeroExtent);
        }
        let resource_width = resource.size.width;
        let resource_height = resource.size.height;
        if !resource_width.is_finite()
            || !resource_height.is_finite()
            || resource_width <= 0.0
            || resource_height <= 0.0
            || resource_width.floor() < resource_width
            || resource_width.ceil() > resource_width
            || resource_height.floor() < resource_height
            || resource_height.ceil() > resource_height
        {
            return Err(VelloNativeTextureValidationError::NonIntegralResourceExtent);
        }
        let native_width = f64::from(self.extent.width);
        let native_height = f64::from(self.extent.height);
        let resource_width = f64::from(resource_width);
        let resource_height = f64::from(resource_height);
        if resource_width.to_bits() != native_width.to_bits()
            || resource_height.to_bits() != native_height.to_bits()
        {
            return Err(VelloNativeTextureValidationError::ResourceExtentMismatch);
        }
        let extent = [self.extent.width, self.extent.height];
        if self.format != wgpu::TextureFormat::Rgba8Unorm {
            return Err(VelloNativeTextureValidationError::UnsupportedFormat);
        }
        if !self.usage.contains(wgpu::TextureUsages::COPY_SRC) {
            return Err(VelloNativeTextureValidationError::MissingCopySourceUsage);
        }
        if self.dimension != wgpu::TextureDimension::D2 {
            return Err(VelloNativeTextureValidationError::UnsupportedDimension);
        }
        if self.extent.depth_or_array_layers != 1 {
            return Err(VelloNativeTextureValidationError::UnsupportedArrayLayers);
        }
        if self.mip_level_count != 1 {
            return Err(VelloNativeTextureValidationError::UnsupportedMipLevels);
        }
        if self.sample_count != 1 {
            return Err(VelloNativeTextureValidationError::UnsupportedSampleCount);
        }
        Ok(extent)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NativeTextureEntry {
    registration_generation: u64,
    content_revision: u64,
    extent: [u32; 2],
    sampling: RenderImageSampling,
}

pub(super) struct NativeTextureMutationDriver {
    entries: BTreeMap<TextureId, NativeTextureEntry>,
    next_registration_generation: u64,
}

pub(super) trait NativeTextureOperations {
    fn invalidate(&mut self, texture: TextureId) -> Result<(), VelloPresenterError>;
    fn register_texture(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError>;
    fn mark_override_image_dirty(&mut self, texture: TextureId) -> Result<(), VelloPresenterError>;
    fn override_image(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError>;
    fn unregister_texture(&mut self, texture: TextureId) -> Result<(), VelloPresenterError>;
    fn publish(&mut self, texture: TextureId) -> Result<(), VelloPresenterError>;
    fn clear(&mut self);
}

pub(super) struct RealNativeTextureOperations<'a> {
    pub(super) registry: &'a mut VelloNativeTextureRegistry,
    pub(super) scope: &'a VelloNativeTextureScope,
    pub(super) renderer: &'a mut vello::Renderer,
    pub(super) source_texture: Option<&'a wgpu::Texture>,
}

impl NativeTextureOperations for RealNativeTextureOperations<'_> {
    fn invalidate(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        if self
            .registry
            .begin_native_texture_update(self.scope, texture)
        {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration { texture })
        }
    }

    fn register_texture(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        let texture = (*self
            .source_texture
            .ok_or(VelloPresenterError::DeviceUnavailable)?)
        .clone();
        if self.registry.stage_native_texture(
            self.scope,
            self.renderer,
            texture_id,
            texture,
            extent,
            sampling,
        ) {
            Ok(())
        } else {
            Err(VelloPresenterError::NativeTextureAlreadyRegistered {
                texture: texture_id,
            })
        }
    }

    fn mark_override_image_dirty(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        if self
            .registry
            .dirty_native_texture(self.scope, self.renderer, texture)
        {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration { texture })
        }
    }

    fn override_image(
        &mut self,
        texture_id: TextureId,
        extent: [u32; 2],
        sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        let texture = (*self
            .source_texture
            .ok_or(VelloPresenterError::DeviceUnavailable)?)
        .clone();
        if self.registry.replace_native_texture_image(
            self.scope,
            self.renderer,
            texture_id,
            texture,
            extent,
            sampling,
        ) {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: texture_id,
            })
        }
    }

    fn unregister_texture(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        if self
            .registry
            .retire_native_texture(self.scope, self.renderer, texture)
        {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration { texture })
        }
    }

    fn publish(&mut self, texture: TextureId) -> Result<(), VelloPresenterError> {
        if self.registry.commit_native_texture(self.scope, texture) {
            Ok(())
        } else {
            Err(VelloPresenterError::StaleNativeTextureRegistration { texture })
        }
    }

    fn clear(&mut self) {
        self.registry.clear_native_textures(self.scope);
    }
}

impl NativeTextureMutationDriver {
    pub(super) fn new() -> Self {
        Self {
            entries: BTreeMap::new(),
            next_registration_generation: 0,
        }
    }

    fn register(
        &mut self,
        scope: &PresenterDeviceScope,
        resource: &TextureResource,
        descriptor: NativeTextureDescriptor,
        content_revision: u64,
        operations: &mut dyn NativeTextureOperations,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> {
        if self.entries.contains_key(&resource.id) {
            return Err(VelloPresenterError::NativeTextureAlreadyRegistered {
                texture: resource.id,
            });
        }
        let extent = descriptor.validate(resource).map_err(|reason| {
            VelloPresenterError::InvalidNativeTexture {
                texture: resource.id,
                reason,
            }
        })?;
        let registration_generation = self
            .next_registration_generation
            .checked_add(1)
            .ok_or(VelloPresenterError::NativeTextureGenerationExhausted)?;
        operations.register_texture(resource.id, extent, resource.sampling)?;
        operations.publish(resource.id)?;
        let _ = self.entries.insert(
            resource.id,
            NativeTextureEntry {
                registration_generation,
                content_revision,
                extent,
                sampling: resource.sampling,
            },
        );
        self.next_registration_generation = registration_generation;
        Ok(VelloNativeTextureRegistration {
            scope: scope.clone(),
            texture: resource.id,
            registration_generation,
        })
    }

    fn update(
        &mut self,
        scope: &PresenterDeviceScope,
        registration: &VelloNativeTextureRegistration,
        content_revision: u64,
        operations: &mut dyn NativeTextureOperations,
    ) -> Result<VelloNativeTextureUpdateOutcome, VelloPresenterError> {
        if registration.scope != *scope {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        if !self.entries.contains_key(&registration.texture) {
            return Err(VelloPresenterError::NativeTextureNotRegistered {
                texture: registration.texture,
            });
        }
        let entry = self
            .entries
            .get_mut(&registration.texture)
            .expect("entry existence was checked");
        if entry.registration_generation != registration.registration_generation {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        if content_revision == entry.content_revision {
            return Ok(VelloNativeTextureUpdateOutcome::Unchanged);
        }
        if content_revision < entry.content_revision {
            return Err(VelloPresenterError::NativeTextureRevisionRegressed {
                texture: registration.texture,
                current: entry.content_revision,
                requested: content_revision,
            });
        }
        operations.invalidate(registration.texture)?;
        operations.mark_override_image_dirty(registration.texture)?;
        operations.publish(registration.texture)?;
        entry.content_revision = content_revision;
        Ok(VelloNativeTextureUpdateOutcome::MarkedDirty)
    }

    fn replace(
        &mut self,
        scope: &PresenterDeviceScope,
        registration: &VelloNativeTextureRegistration,
        resource: &TextureResource,
        descriptor: NativeTextureDescriptor,
        content_revision: u64,
        operations: &mut dyn NativeTextureOperations,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> {
        if registration.scope != *scope {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        if !self.entries.contains_key(&registration.texture) {
            return Err(VelloPresenterError::NativeTextureNotRegistered {
                texture: registration.texture,
            });
        }
        let entry = self
            .entries
            .get_mut(&registration.texture)
            .expect("entry existence was checked");
        if entry.registration_generation != registration.registration_generation {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        if resource.id != registration.texture {
            return Err(VelloPresenterError::InvalidNativeTexture {
                texture: registration.texture,
                reason: VelloNativeTextureValidationError::ResourceIdMismatch,
            });
        }
        let extent = descriptor.validate(resource).map_err(|reason| {
            VelloPresenterError::InvalidNativeTexture {
                texture: registration.texture,
                reason,
            }
        })?;
        if content_revision < entry.content_revision {
            return Err(VelloPresenterError::NativeTextureRevisionRegressed {
                texture: registration.texture,
                current: entry.content_revision,
                requested: content_revision,
            });
        }
        let registration_generation = self
            .next_registration_generation
            .checked_add(1)
            .ok_or(VelloPresenterError::NativeTextureGenerationExhausted)?;
        operations.invalidate(registration.texture)?;
        if extent == entry.extent {
            operations.override_image(registration.texture, extent, resource.sampling)?;
        } else {
            operations.unregister_texture(registration.texture)?;
            operations.register_texture(registration.texture, extent, resource.sampling)?;
        }
        operations.publish(registration.texture)?;
        entry.registration_generation = registration_generation;
        entry.content_revision = content_revision;
        entry.extent = extent;
        entry.sampling = resource.sampling;
        self.next_registration_generation = registration_generation;
        Ok(VelloNativeTextureRegistration {
            scope: scope.clone(),
            texture: registration.texture,
            registration_generation,
        })
    }

    fn remove(
        &mut self,
        scope: &PresenterDeviceScope,
        registration: &VelloNativeTextureRegistration,
        operations: &mut dyn NativeTextureOperations,
    ) -> Result<(), VelloPresenterError> {
        if registration.scope != *scope {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        if !self.entries.contains_key(&registration.texture) {
            return Err(VelloPresenterError::NativeTextureNotRegistered {
                texture: registration.texture,
            });
        }
        let entry = self
            .entries
            .get(&registration.texture)
            .expect("entry existence was checked");
        if entry.registration_generation != registration.registration_generation {
            return Err(VelloPresenterError::StaleNativeTextureRegistration {
                texture: registration.texture,
            });
        }
        operations.invalidate(registration.texture)?;
        operations.unregister_texture(registration.texture)?;
        let _ = self.entries.remove(&registration.texture);
        Ok(())
    }

    pub(super) fn invalidate_all(&mut self, operations: &mut dyn NativeTextureOperations) {
        operations.clear();
        self.entries = BTreeMap::new();
    }
}

impl VelloWindowPresenter {
    /// Registers a same-device native texture for one neutral texture resource.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable or stale device state, invalid metadata,
    /// duplicate registration, exhausted generations, or backend mutation failure.
    pub fn register_native_texture(
        &mut self,
        scope: &PresenterDeviceScope,
        resource: &TextureResource,
        texture: &wgpu::Texture,
        content_revision: u64,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> {
        self.poll_device_events()?;
        let current_scope = self.control.validated_native_scope(scope)?;
        let descriptor = NativeTextureDescriptor::from_texture(texture);
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let mut operations = RealNativeTextureOperations {
            registry: &mut gpu.native_registry,
            scope: &gpu.native_scope,
            renderer: &mut gpu.renderer,
            source_texture: Some(texture),
        };
        NativeTextureMutationDriver::register(
            &mut self.native_textures,
            &current_scope,
            resource,
            descriptor,
            content_revision,
            &mut operations,
        )
    }

    /// Marks newer producer content dirty for the next presentation.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable or stale device state, a missing or stale
    /// registration, a regressed revision, or backend mutation failure.
    pub fn update_native_texture(
        &mut self,
        registration: &VelloNativeTextureRegistration,
        content_revision: u64,
    ) -> Result<VelloNativeTextureUpdateOutcome, VelloPresenterError> {
        self.poll_device_events()?;
        let current_scope = self.control.current_native_scope()?;
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let mut operations = RealNativeTextureOperations {
            registry: &mut gpu.native_registry,
            scope: &gpu.native_scope,
            renderer: &mut gpu.renderer,
            source_texture: None,
        };
        NativeTextureMutationDriver::update(
            &mut self.native_textures,
            &current_scope,
            registration,
            content_revision,
            &mut operations,
        )
    }

    /// Replaces a native texture object and publishes a new registration token.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable or stale device state, invalid metadata,
    /// an invalid registration or revision, exhausted generations, or backend failure.
    pub fn replace_native_texture(
        &mut self,
        registration: &VelloNativeTextureRegistration,
        resource: &TextureResource,
        texture: &wgpu::Texture,
        content_revision: u64,
    ) -> Result<VelloNativeTextureRegistration, VelloPresenterError> {
        self.poll_device_events()?;
        let current_scope = self.control.current_native_scope()?;
        let descriptor = NativeTextureDescriptor::from_texture(texture);
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let mut operations = RealNativeTextureOperations {
            registry: &mut gpu.native_registry,
            scope: &gpu.native_scope,
            renderer: &mut gpu.renderer,
            source_texture: Some(texture),
        };
        NativeTextureMutationDriver::replace(
            &mut self.native_textures,
            &current_scope,
            registration,
            resource,
            descriptor,
            content_revision,
            &mut operations,
        )
    }

    /// Invalidates and unregisters a native texture registration.
    ///
    /// # Errors
    ///
    /// Returns an error for unavailable or stale device state, an invalid
    /// registration, or backend mutation failure.
    pub fn remove_native_texture(
        &mut self,
        registration: &VelloNativeTextureRegistration,
    ) -> Result<(), VelloPresenterError> {
        self.poll_device_events()?;
        let current_scope = self.control.current_native_scope()?;
        let gpu = self
            .gpu
            .as_mut()
            .ok_or(VelloPresenterError::DeviceUnavailable)?;
        let mut operations = RealNativeTextureOperations {
            registry: &mut gpu.native_registry,
            scope: &gpu.native_scope,
            renderer: &mut gpu.renderer,
            source_texture: None,
        };
        NativeTextureMutationDriver::remove(
            &mut self.native_textures,
            &current_scope,
            registration,
            &mut operations,
        )
    }
}

#[cfg(test)]
#[derive(Default)]
pub(crate) struct TestLifetimeOperations {
    clear_calls: usize,
}

#[cfg(test)]
impl NativeTextureOperations for TestLifetimeOperations {
    fn invalidate(&mut self, _texture: TextureId) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn register_texture(
        &mut self,
        _texture_id: TextureId,
        _extent: [u32; 2],
        _sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn mark_override_image_dirty(
        &mut self,
        _texture: TextureId,
    ) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn override_image(
        &mut self,
        _texture_id: TextureId,
        _extent: [u32; 2],
        _sampling: RenderImageSampling,
    ) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn unregister_texture(&mut self, _texture: TextureId) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn publish(&mut self, _texture: TextureId) -> Result<(), VelloPresenterError> {
        Ok(())
    }

    fn clear(&mut self) {
        self.clear_calls += 1;
    }
}

#[cfg(test)]
pub(crate) fn test_native_lifetime_clear() -> (usize, bool) {
    let texture = TextureId::from_raw(88_001);
    let mut driver = NativeTextureMutationDriver::new();
    let _ = driver.entries.insert(
        texture,
        NativeTextureEntry {
            registration_generation: 1,
            content_revision: 1,
            extent: [1, 1],
            sampling: RenderImageSampling::Pixelated,
        },
    );
    let mut operations = TestLifetimeOperations::default();
    NativeTextureMutationDriver::invalidate_all(&mut driver, &mut operations);
    (operations.clear_calls, driver.entries.is_empty())
}

#[cfg(test)]
pub(crate) fn test_same_device_native_lifetime_preserved() -> bool {
    let texture = TextureId::from_raw(88_002);
    let mut driver = NativeTextureMutationDriver::new();
    let _ = driver.entries.insert(
        texture,
        NativeTextureEntry {
            registration_generation: 1,
            content_revision: 1,
            extent: [1, 1],
            sampling: RenderImageSampling::Pixelated,
        },
    );
    driver.entries.contains_key(&texture)
}

#[cfg(test)]
mod tests;
