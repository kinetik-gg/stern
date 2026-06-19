//! Backend-independent renderer contract for Kinetik UI.
//!
//! This crate owns frame submission types, resource registries, image payloads,
//! and renderer diagnostics that are shared by renderer backends. Concrete
//! backends such as Vello consume this contract and keep backend-specific
//! encoding details in their own crates.

use std::collections::HashMap;

use kinetik_ui_core::{ImageId, Primitive, Size, TextLayoutId, TextureId, ViewportInfo};
use kinetik_ui_text::{ShapedTextLayout, StoredTextLayout};

/// Static image resource known by a renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct ImageResource {
    /// Image handle from core primitives.
    pub id: ImageId,
    /// Image size in physical pixels.
    pub size: Size,
    /// Sampling hint to use when drawing the image.
    pub sampling: RenderImageSampling,
    /// Optional CPU pixel data to draw.
    pub pixels: Option<RenderImage>,
}

/// Dynamic texture resource known by a renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct TextureResource {
    /// Texture handle from core primitives.
    pub id: TextureId,
    /// Texture size in physical pixels.
    pub size: Size,
    /// Sampling hint to use when drawing texture snapshots.
    pub sampling: RenderImageSampling,
    /// Optional CPU snapshot for renderers that consume image data.
    pub snapshot: Option<RenderImage>,
}

/// Sampling intent for image-like renderer resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenderImageSampling {
    /// Preserve crisp texels. Best for icons, UI snapshots, and editor/game surfaces.
    #[default]
    Pixelated,
    /// Smooth resampling for photographic or heavily scaled content.
    Smooth,
}

/// CPU image data accepted by renderer boundaries.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderImage {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Pixel bytes.
    pub data: Vec<u8>,
    /// Pixel format.
    pub format: RenderImageFormat,
    /// Alpha representation.
    pub alpha: RenderImageAlpha,
}

impl RenderImage {
    /// Creates an RGBA8 image after validating byte length.
    #[must_use]
    pub fn rgba8(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        Self::new(
            width,
            height,
            data,
            RenderImageFormat::Rgba8,
            RenderImageAlpha::Alpha,
        )
    }

    /// Creates a BGRA8 image after validating byte length.
    #[must_use]
    pub fn bgra8(width: u32, height: u32, data: Vec<u8>) -> Option<Self> {
        Self::new(
            width,
            height,
            data,
            RenderImageFormat::Bgra8,
            RenderImageAlpha::Alpha,
        )
    }

    /// Creates image data after validating byte length.
    #[must_use]
    pub fn new(
        width: u32,
        height: u32,
        data: Vec<u8>,
        format: RenderImageFormat,
        alpha: RenderImageAlpha,
    ) -> Option<Self> {
        let expected_len = format.byte_len(width, height)?;
        (data.len() == expected_len).then_some(Self {
            width,
            height,
            data,
            format,
            alpha,
        })
    }
}

/// CPU image pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderImageFormat {
    /// 32-bit RGBA with 8-bit channels.
    Rgba8,
    /// 32-bit BGRA with 8-bit channels.
    Bgra8,
}

impl RenderImageFormat {
    fn byte_len(self, width: u32, height: u32) -> Option<usize> {
        match self {
            Self::Rgba8 | Self::Bgra8 => 4_usize
                .checked_mul(usize::try_from(width).ok()?)
                .and_then(|bytes| bytes.checked_mul(usize::try_from(height).ok()?)),
        }
    }
}

/// CPU image alpha representation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderImageAlpha {
    /// Straight alpha.
    Alpha,
    /// Premultiplied alpha.
    Premultiplied,
}

/// Shaped text layout resource known by a renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayoutResource {
    /// Text layout handle from core primitives.
    pub id: TextLayoutId,
    /// Owned shaped text layout.
    pub layout: ShapedTextLayout,
}

/// Resource registry used during frame translation and encoding.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderResources {
    images: HashMap<ImageId, ImageResource>,
    textures: HashMap<TextureId, TextureResource>,
    text_layouts: HashMap<TextLayoutId, ShapedTextLayout>,
}

impl RenderResources {
    /// Creates an empty resource registry.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers an image resource.
    pub fn register_image(&mut self, image: ImageResource) {
        self.images.insert(image.id, image);
    }

    /// Registers a texture resource.
    pub fn register_texture(&mut self, texture: TextureResource) {
        self.textures.insert(texture.id, texture);
    }

    /// Registers a shaped text layout resource.
    pub fn register_text_layout(&mut self, text: TextLayoutResource) {
        self.text_layouts.insert(text.id, text.layout);
    }

    /// Registers a borrowed shaped text layout resource.
    pub fn register_text_layout_ref(&mut self, id: TextLayoutId, layout: &ShapedTextLayout) {
        self.text_layouts.insert(id, layout.clone());
    }

    /// Registers shaped text layouts exported by a text layout store.
    pub fn register_text_layouts<'a>(
        &mut self,
        layouts: impl IntoIterator<Item = StoredTextLayout<'a>>,
    ) {
        for layout in layouts {
            self.register_text_layout_ref(layout.id, layout.layout);
        }
    }

    /// Returns true when an image is registered.
    #[must_use]
    pub fn has_image(&self, image: ImageId) -> bool {
        self.images.contains_key(&image)
    }

    /// Returns true when a texture is registered.
    #[must_use]
    pub fn has_texture(&self, texture: TextureId) -> bool {
        self.textures.contains_key(&texture)
    }

    /// Returns a registered image resource.
    #[must_use]
    pub fn image(&self, image: ImageId) -> Option<&ImageResource> {
        self.images.get(&image)
    }

    /// Returns a registered texture resource.
    #[must_use]
    pub fn texture(&self, texture: TextureId) -> Option<&TextureResource> {
        self.textures.get(&texture)
    }

    /// Returns true when a shaped text layout is registered.
    #[must_use]
    pub fn has_text_layout(&self, layout: TextLayoutId) -> bool {
        self.text_layouts.contains_key(&layout)
    }

    /// Returns a registered shaped text layout.
    #[must_use]
    pub fn text_layout(&self, layout: TextLayoutId) -> Option<&ShapedTextLayout> {
        self.text_layouts.get(&layout)
    }

    /// Builds a deterministic resource inventory for tests and diagnostics.
    #[must_use]
    pub fn snapshot(&self) -> RenderResourceSnapshot {
        RenderResourceSnapshot::from_resources(self)
    }
}

/// Deterministic resource inventory used by renderer snapshot tests.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RenderResourceSnapshot {
    /// Image resources sorted by handle.
    pub images: Vec<ImageResourceSnapshot>,
    /// Texture resources sorted by handle.
    pub textures: Vec<TextureResourceSnapshot>,
    /// Shaped text layout resources sorted by handle.
    pub text_layouts: Vec<TextLayoutResourceSnapshot>,
}

impl RenderResourceSnapshot {
    fn from_resources(resources: &RenderResources) -> Self {
        let mut images = resources
            .images
            .values()
            .map(ImageResourceSnapshot::from_resource)
            .collect::<Vec<_>>();
        let mut textures = resources
            .textures
            .values()
            .map(TextureResourceSnapshot::from_resource)
            .collect::<Vec<_>>();
        let mut text_layouts = resources
            .text_layouts
            .iter()
            .map(|(id, layout)| TextLayoutResourceSnapshot::from_layout(*id, layout))
            .collect::<Vec<_>>();

        images.sort_by_key(|resource| resource.id);
        textures.sort_by_key(|resource| resource.id);
        text_layouts.sort_by_key(|resource| resource.id);

        Self {
            images,
            textures,
            text_layouts,
        }
    }

    /// Renders the resource inventory as stable line-oriented text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut lines = Vec::new();
        lines.push("resources:".to_owned());
        for image in &self.images {
            lines.push(format!(
                "  image#{id} size={width}x{height} pixels={pixels}",
                id = image.id,
                width = format_f32(image.width),
                height = format_f32(image.height),
                pixels = image.has_pixels,
            ));
        }
        for texture in &self.textures {
            lines.push(format!(
                "  texture#{id} size={width}x{height} snapshot={snapshot}",
                id = texture.id,
                width = format_f32(texture.width),
                height = format_f32(texture.height),
                snapshot = texture.has_snapshot,
            ));
        }
        for layout in &self.text_layouts {
            lines.push(format!(
                "  text_layout#{id} size={width}x{height} lines={lines_count} glyphs={glyphs}",
                id = layout.id,
                width = format_f32(layout.width),
                height = format_f32(layout.height),
                lines_count = layout.line_count,
                glyphs = layout.glyph_count,
            ));
        }
        lines.join("\n")
    }
}

/// Snapshot of one image resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageResourceSnapshot {
    /// Raw image handle.
    pub id: u64,
    /// Image width.
    pub width: OrderedF32,
    /// Image height.
    pub height: OrderedF32,
    /// Whether drawable pixel data is present.
    pub has_pixels: bool,
}

impl ImageResourceSnapshot {
    fn from_resource(resource: &ImageResource) -> Self {
        Self {
            id: resource.id.raw(),
            width: OrderedF32::new(resource.size.width),
            height: OrderedF32::new(resource.size.height),
            has_pixels: resource.pixels.is_some(),
        }
    }
}

/// Snapshot of one texture resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextureResourceSnapshot {
    /// Raw texture handle.
    pub id: u64,
    /// Texture width.
    pub width: OrderedF32,
    /// Texture height.
    pub height: OrderedF32,
    /// Whether a drawable CPU snapshot is present.
    pub has_snapshot: bool,
}

impl TextureResourceSnapshot {
    fn from_resource(resource: &TextureResource) -> Self {
        Self {
            id: resource.id.raw(),
            width: OrderedF32::new(resource.size.width),
            height: OrderedF32::new(resource.size.height),
            has_snapshot: resource.snapshot.is_some(),
        }
    }
}

/// Snapshot of one shaped text layout resource.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLayoutResourceSnapshot {
    /// Raw text layout handle.
    pub id: u64,
    /// Layout width.
    pub width: OrderedF32,
    /// Layout height.
    pub height: OrderedF32,
    /// Number of visual lines.
    pub line_count: usize,
    /// Number of shaped glyphs.
    pub glyph_count: usize,
}

impl TextLayoutResourceSnapshot {
    fn from_layout(id: TextLayoutId, layout: &ShapedTextLayout) -> Self {
        Self {
            id: id.raw(),
            width: OrderedF32::new(layout.size.width),
            height: OrderedF32::new(layout.size.height),
            line_count: layout.line_count,
            glyph_count: layout.glyph_count(),
        }
    }
}

/// Float wrapper with equality based on raw bits after snapshot sanitization.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderedF32(u32);

impl OrderedF32 {
    /// Creates a stable float snapshot value.
    #[must_use]
    pub fn new(value: f32) -> Self {
        let value = if value.is_finite() { value } else { 0.0 };
        Self(normalize_zero(value).to_bits())
    }

    /// Returns the wrapped finite float.
    #[must_use]
    pub fn get(self) -> f32 {
        f32::from_bits(self.0)
    }
}

fn format_f32(value: OrderedF32) -> String {
    format!("{:.3}", value.get())
}

fn normalize_zero(value: f32) -> f32 {
    if value == 0.0 { 0.0 } else { value }
}

/// Input submitted to a renderer for one frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RenderFrameInput<'a> {
    /// Viewport for the frame.
    pub viewport: ViewportInfo,
    /// Primitive sequence to draw in order.
    pub primitives: &'a [Primitive],
    /// Image, texture, and text resources available to this frame.
    pub resources: &'a RenderResources,
}

/// Output produced by renderer frame submission.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderFrameOutput {
    /// Number of primitives submitted.
    pub primitive_count: usize,
    /// Recoverable renderer diagnostics.
    pub diagnostics: Vec<RenderDiagnostic>,
}

/// Backend-neutral renderer contract.
///
/// Fatal submission failures are returned as `Self::Error`; recoverable issues
/// such as missing optional resources should be reported through
/// [`RenderFrameOutput::diagnostics`].
pub trait RendererBackend {
    /// Fatal renderer submission error.
    type Error;

    /// Submits one frame to the renderer backend.
    ///
    /// # Errors
    ///
    /// Returns `Self::Error` when the backend cannot submit the frame at all.
    /// Recoverable primitive/resource issues should be returned as diagnostics.
    fn render_frame(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error>;
}

/// Renderer diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RenderDiagnostic {
    /// Text layout resource was referenced but not registered.
    MissingTextLayout(TextLayoutId),
    /// Image resource was referenced but not registered.
    MissingImage(ImageId),
    /// Image resource was registered but does not include drawable pixels.
    MissingImagePixels(ImageId),
    /// Texture resource was referenced but not registered.
    MissingTexture(TextureId),
    /// Texture resource was registered but does not include a drawable snapshot.
    MissingTextureSnapshot(TextureId),
    /// Primitive kind is intentionally represented but not yet translated.
    UnsupportedPrimitive(&'static str),
    /// Primitive contained non-finite or non-positive geometry and was sanitized or skipped.
    InvalidGeometry(&'static str),
}

/// Result of deterministic primitive translation.
#[derive(Debug, Clone, PartialEq)]
pub struct Translation<T> {
    /// Deterministic backend command stream.
    pub commands: Vec<T>,
    /// Translation diagnostics.
    pub diagnostics: Vec<RenderDiagnostic>,
}

/// Returns the crate name.
#[must_use]
pub const fn crate_name() -> &'static str {
    "kinetik-ui-render"
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use super::{
        ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput, RenderImage,
        RenderImageSampling, RenderResources, RendererBackend, TextLayoutResource, TextureResource,
    };
    use kinetik_ui_core::{
        ImageId, PhysicalSize, ScaleFactor, Size, TextLayoutId, TextureId, ViewportInfo,
    };
    use kinetik_ui_text::{CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextStyle};

    #[derive(Default)]
    struct RecordingRenderer {
        submitted_frames: usize,
    }

    impl RendererBackend for RecordingRenderer {
        type Error = Infallible;

        fn render_frame(
            &mut self,
            input: RenderFrameInput<'_>,
        ) -> Result<RenderFrameOutput, Self::Error> {
            self.submitted_frames += 1;
            Ok(RenderFrameOutput {
                primitive_count: input.primitives.len(),
                diagnostics: vec![RenderDiagnostic::MissingTexture(TextureId::from_raw(7))],
            })
        }
    }

    fn render_once(
        renderer: &mut impl RendererBackend<Error = Infallible>,
        input: RenderFrameInput<'_>,
    ) -> RenderFrameOutput {
        match renderer.render_frame(input) {
            Ok(output) => output,
            Err(error) => match error {},
        }
    }

    #[test]
    fn render_image_validates_pixel_byte_length() {
        assert!(RenderImage::rgba8(2, 2, vec![0; 16]).is_some());
        assert!(RenderImage::rgba8(2, 2, vec![0; 15]).is_none());
    }

    #[test]
    fn resources_register_images_textures_and_text_layouts() {
        let mut resources = RenderResources::new();
        let image = ImageId::from_raw(1);
        let texture = TextureId::from_raw(2);
        let text = TextLayoutId::from_raw(3);
        let mut engine = CosmicTextEngine::new();
        let layout = engine.shape_text(&TextLayoutKey::new(
            "Label",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ));

        resources.register_image(ImageResource {
            id: image,
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
        });
        resources.register_texture(TextureResource {
            id: texture,
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            snapshot: None,
        });
        resources.register_text_layout(TextLayoutResource { id: text, layout });

        assert!(resources.has_image(image));
        assert!(resources.has_texture(texture));
        assert!(resources.has_text_layout(text));
        assert!(resources.image(image).is_some());
        assert!(resources.texture(texture).is_some());
        assert!(resources.text_layout(text).is_some());
    }

    #[test]
    fn resource_snapshot_is_sorted_and_stable() {
        let mut resources = RenderResources::new();
        let layout = ShapedTextLayout {
            size: Size::new(20.0, 10.0),
            line_count: 2,
            lines: Vec::new(),
            runs: Vec::new(),
        };

        resources.register_texture(TextureResource {
            id: TextureId::from_raw(9),
            size: Size::new(12.0, 8.0),
            sampling: RenderImageSampling::default(),
            snapshot: None,
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(2),
            size: Size::new(4.0, 3.0),
            sampling: RenderImageSampling::default(),
            pixels: Some(RenderImage::rgba8(1, 1, vec![255; 4]).expect("valid image")),
        });
        resources.register_text_layout(TextLayoutResource {
            id: TextLayoutId::from_raw(5),
            layout,
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(2.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
        });

        assert_eq!(
            resources.snapshot().to_text(),
            "resources:\n  image#1 size=2.000x1.000 pixels=false\n  image#2 size=4.000x3.000 pixels=true\n  texture#9 size=12.000x8.000 snapshot=false\n  text_layout#5 size=20.000x10.000 lines=2 glyphs=0"
        );
    }

    #[test]
    fn frame_input_and_diagnostics_are_backend_neutral() {
        let resources = RenderResources::new();
        let primitives = [];
        let input = RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 50.0),
                PhysicalSize::new(200, 100),
                ScaleFactor::new(2.0),
            ),
            primitives: &primitives,
            resources: &resources,
        };

        assert_eq!(input.primitives.len(), 0);
        assert_eq!(
            RenderDiagnostic::MissingImage(ImageId::from_raw(9)),
            RenderDiagnostic::MissingImage(ImageId::from_raw(9))
        );
        assert_eq!(
            RenderDiagnostic::MissingImagePixels(ImageId::from_raw(9)),
            RenderDiagnostic::MissingImagePixels(ImageId::from_raw(9))
        );
        assert_eq!(
            RenderDiagnostic::MissingTextureSnapshot(TextureId::from_raw(8)),
            RenderDiagnostic::MissingTextureSnapshot(TextureId::from_raw(8))
        );
    }

    #[test]
    fn renderer_backend_contract_separates_output_diagnostics_from_fatal_errors() {
        let resources = RenderResources::new();
        let primitives = [];
        let input = RenderFrameInput {
            viewport: ViewportInfo::new(
                Size::new(100.0, 50.0),
                PhysicalSize::new(200, 100),
                ScaleFactor::new(2.0),
            ),
            primitives: &primitives,
            resources: &resources,
        };
        let mut renderer = RecordingRenderer::default();

        let output = render_once(&mut renderer, input);

        assert_eq!(renderer.submitted_frames, 1);
        assert_eq!(output.primitive_count, 0);
        assert_eq!(
            output.diagnostics,
            vec![RenderDiagnostic::MissingTexture(TextureId::from_raw(7))]
        );
    }
}
