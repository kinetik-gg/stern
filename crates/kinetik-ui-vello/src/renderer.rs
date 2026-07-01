use kinetik_ui_render::{RenderFrameInput, RenderFrameOutput, RendererBackend};
use kinetik_ui_text::CosmicTextEngine;
use vello::Scene;

use crate::{
    encoding::encode_scene, geometry::viewport_device_scale, image::ImageDataCache,
    text::ShapedTextCache, translation::translate_primitives,
};

/// Vello renderer boundary.
pub struct VelloRenderer {
    scene: Scene,
    text_engine: CosmicTextEngine,
    text_cache: ShapedTextCache,
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
            text_engine: CosmicTextEngine::new(),
            text_cache: ShapedTextCache::default(),
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
        self.text_cache.len()
    }

    /// Submits a frame for translation.
    pub fn submit_frame(&mut self, input: RenderFrameInput<'_>) -> RenderFrameOutput {
        let translated = translate_primitives(input.primitives, input.resources);
        self.scene.reset();
        encode_scene(
            &mut self.scene,
            &translated.commands,
            input.resources,
            &mut self.text_engine,
            &mut self.text_cache,
            &mut self.image_cache,
            viewport_device_scale(input.viewport),
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
