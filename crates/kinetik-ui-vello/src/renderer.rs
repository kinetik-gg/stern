use kinetik_ui_render::{RenderFrameInput, RenderFrameOutput, RendererBackend};
use kinetik_ui_text::TextLayoutStore;
use vello::Scene;

use crate::{
    VelloNativeTextureRegistry, VelloNativeTextureScope,
    encoding::{encode_scene, encode_scene_with_native},
    geometry::viewport_device_scale,
    image::ImageDataCache,
    translation::{translate_primitives, translate_primitives_with_native},
};

/// Vello renderer boundary.
pub struct VelloRenderer {
    scene: Scene,
    fallback_text_layouts: TextLayoutStore,
    image_cache: ImageDataCache,
}

/// Fatal error returned by [`VelloRenderer`] frame submission.
///
/// The current Vello backend translates primitives and encodes a CPU-side
/// [`Scene`], so it has no fatal submission failures today. Recoverable
/// primitive, geometry, and resource issues are still reported as
/// [`RenderDiagnostic`](crate::RenderDiagnostic) values in
/// [`RenderFrameOutput::diagnostics`]. This
/// non-exhaustive type reserves the backend contract for future GPU/device
/// submission failures without changing `RendererBackend::Error`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum VelloRendererError {}

impl std::fmt::Display for VelloRendererError {
    fn fmt(&self, _formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match *self {}
    }
}

impl std::error::Error for VelloRendererError {}

impl VelloRenderer {
    /// Creates a renderer boundary with an empty Vello scene.
    #[must_use]
    pub fn new() -> Self {
        Self {
            scene: Scene::new(),
            fallback_text_layouts: TextLayoutStore::new(),
            image_cache: ImageDataCache::default(),
        }
    }

    /// Returns the current Vello scene.
    #[must_use]
    pub const fn scene(&self) -> &Scene {
        &self.scene
    }

    #[cfg(test)]
    pub(crate) fn cached_image_count(&self) -> usize {
        self.image_cache.image_len()
    }

    #[cfg(test)]
    pub(crate) fn cached_texture_count(&self) -> usize {
        self.image_cache.texture_len()
    }

    #[cfg(test)]
    pub(crate) fn cached_text_layout_count(&self) -> usize {
        self.fallback_text_layouts.len()
    }

    #[cfg(test)]
    pub(crate) const fn cached_text_layout_payload_bytes(&self) -> usize {
        self.fallback_text_layouts.retained_payload_bytes()
    }

    #[cfg(test)]
    pub(crate) const fn cached_text_layout_generation(&self) -> u64 {
        self.fallback_text_layouts.generation()
    }

    /// Submits a frame for translation.
    pub fn submit_frame(&mut self, input: RenderFrameInput<'_>) -> RenderFrameOutput {
        self.fallback_text_layouts.advance_generation();
        let translated = translate_primitives(input.primitives, input.resources);
        self.scene.reset();
        encode_scene(
            &mut self.scene,
            &translated.commands,
            input.resources,
            &mut self.fallback_text_layouts,
            &mut self.image_cache,
            viewport_device_scale(input.viewport),
        );
        RenderFrameOutput {
            primitive_count: input.primitives.len(),
            diagnostics: translated.diagnostics,
        }
    }

    /// Submits a frame with native textures owned by this renderer.
    ///
    /// This API requires the same renderer instance.
    #[doc(hidden)]
    pub fn submit_frame_with_native_textures(
        &mut self,
        input: RenderFrameInput<'_>,
        registry: &VelloNativeTextureRegistry,
        scope: &VelloNativeTextureScope,
    ) -> RenderFrameOutput {
        self.fallback_text_layouts.advance_generation();
        let translated = translate_primitives_with_native(
            input.primitives,
            input.resources,
            Some((registry, scope)),
        );
        self.scene.reset();
        encode_scene_with_native(
            &mut self.scene,
            &translated.commands,
            input.resources,
            &mut self.fallback_text_layouts,
            &mut self.image_cache,
            viewport_device_scale(input.viewport),
            Some((registry, scope)),
        );
        RenderFrameOutput {
            primitive_count: input.primitives.len(),
            diagnostics: translated.diagnostics,
        }
    }
}

impl Default for VelloRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl RendererBackend for VelloRenderer {
    type Error = VelloRendererError;

    fn render_frame(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error> {
        Ok(self.submit_frame(input))
    }
}
