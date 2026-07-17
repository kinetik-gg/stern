//! Backend-independent renderer contract for Stern.
//!
//! This crate owns frame submission types, resource registries, image payloads,
//! and renderer diagnostics that are shared by renderer backends. Concrete
//! backends such as Vello consume this contract and keep backend-specific
//! encoding details in their own crates.

use std::{collections::HashMap, mem::size_of, sync::Arc};

use stern_core::{ImageId, Primitive, Rect, Size, TextLayoutId, TextureId, ViewportInfo};
use stern_text::{
    ShapedGlyph, ShapedGlyphRun, ShapedTextLayout, ShapedTextLine, StoredTextLayout,
    TextLayoutChangeCursor, TextLayoutKey, TextLayoutStore,
};

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
    /// Optional source rectangle into another image resource.
    pub atlas_region: Option<ImageAtlasRegion>,
}

/// Source rectangle inside an atlas image resource.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ImageAtlasRegion {
    /// Atlas image handle.
    pub atlas: ImageId,
    /// Source rectangle in atlas pixels.
    pub source: Rect,
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
    /// Preserve crisp UI icon edges while allowing renderers to use icon-specific policies.
    UiIcon,
    /// Smooth resampling for photographic or heavily scaled content.
    Smooth,
    /// Prioritize quality for previews or photographic content where scaling artifacts matter.
    HighQuality,
}

/// CPU image data accepted by renderer boundaries.
///
/// RGB bytes are sRGB-encoded values. [`RenderImageAlpha`] declares whether
/// those bytes carry straight or already-premultiplied alpha; callers are
/// responsible for supplying bytes that match the selected representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderImage {
    /// Pixel width.
    pub width: u32,
    /// Pixel height.
    pub height: u32,
    /// Pixel bytes.
    pub data: Arc<[u8]>,
    /// Pixel byte order and channel format.
    pub format: RenderImageFormat,
    /// Alpha representation of the supplied payload.
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
            data: data.into(),
            format,
            alpha,
        })
    }
}

/// CPU sRGB image pixel format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderImageFormat {
    /// 32-bit RGBA byte order with 8-bit sRGB color channels.
    Rgba8,
    /// 32-bit BGRA byte order with 8-bit sRGB color channels.
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

/// CPU image alpha representation declared by the caller.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RenderImageAlpha {
    /// Straight (unpremultiplied) alpha.
    Alpha,
    /// Already-premultiplied alpha.
    ///
    /// Renderers trust this metadata and do not scan payload bytes to validate
    /// the premultiplication invariant.
    Premultiplied,
}

/// Shaped text layout resource known by a renderer.
#[derive(Debug, Clone, PartialEq)]
pub struct TextLayoutResource {
    /// Text layout handle from core primitives.
    pub id: TextLayoutId,
    /// Layout request used to shape the text.
    pub key: TextLayoutKey,
    /// Shared shaped text layout.
    pub layout: Arc<ShapedTextLayout>,
}

/// Reconciliation path used for one renderer text-resource update.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextLayoutResourceSyncKind {
    /// The complete text namespace was cleared and rebuilt.
    Full,
    /// Dirty IDs were reconciled against final store presence.
    Incremental,
}

/// Deterministic result of reconciling one text layout store into resources.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLayoutResourceSyncReport {
    /// Reconciliation path used for this update.
    pub kind: TextLayoutResourceSyncKind,
    /// Number of bounded journal records inspected.
    pub processed_changes: usize,
    /// Number of resources inserted into absent IDs.
    pub added: usize,
    /// Number of present IDs replaced with different key/layout data.
    pub updated: usize,
    /// Number of present IDs removed.
    pub removed: usize,
    /// Final number of retained text resources.
    pub retained: usize,
}

impl TextLayoutResourceSyncReport {
    /// Returns true only for an empty incremental batch with no mutations.
    #[must_use]
    pub const fn is_noop(&self) -> bool {
        matches!(self.kind, TextLayoutResourceSyncKind::Incremental)
            && self.processed_changes == 0
            && self.added == 0
            && self.updated == 0
            && self.removed == 0
    }
}

/// Cursor state pairing one text layout store consumer with one resource registry.
///
/// This state is intentionally not cloneable: its cursor is valid only for the
/// exact registry state advanced alongside it. A separate or cloned registry
/// must start with a fresh sync state and perform a full reconciliation.
///
/// ```compile_fail
/// use stern_render::TextLayoutResourceSync;
///
/// let sync = TextLayoutResourceSync::new();
/// let _invalid_fork = sync.clone();
/// ```
#[derive(Debug, Default, PartialEq, Eq)]
pub struct TextLayoutResourceSync {
    cursor: Option<TextLayoutChangeCursor>,
}

impl TextLayoutResourceSync {
    /// Creates an uninitialized resource sync state.
    #[must_use]
    pub const fn new() -> Self {
        Self { cursor: None }
    }

    /// Forces the next reconciliation to rebuild the complete text namespace.
    pub fn reset(&mut self) {
        self.cursor = None;
    }

    /// Returns whether this sync state has reconciled a registry snapshot.
    #[must_use]
    pub const fn is_initialized(&self) -> bool {
        self.cursor.is_some()
    }
}

/// Resource registry used during frame translation and encoding.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RenderResources {
    images: HashMap<ImageId, ImageResource>,
    textures: HashMap<TextureId, TextureResource>,
    text_layouts: HashMap<TextLayoutId, TextLayoutResource>,
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
        self.text_layouts.insert(text.id, text);
    }

    /// Registers a borrowed shaped text layout resource.
    pub fn register_text_layout_ref(
        &mut self,
        id: TextLayoutId,
        key: &TextLayoutKey,
        layout: &ShapedTextLayout,
    ) {
        self.text_layouts.insert(
            id,
            TextLayoutResource {
                id,
                key: key.clone(),
                layout: Arc::new(layout.clone()),
            },
        );
    }

    /// Registers shaped text layouts exported by a text layout store.
    pub fn register_text_layouts<'a>(
        &mut self,
        layouts: impl IntoIterator<Item = StoredTextLayout<'a>>,
    ) {
        for layout in layouts {
            self.text_layouts.insert(
                layout.id,
                TextLayoutResource {
                    id: layout.id,
                    key: layout.key.clone(),
                    layout: layout.layout,
                },
            );
        }
    }

    /// Reconciles one retained text store into this registry's text namespace.
    ///
    /// The sync state must remain paired with this exact registry. Managed
    /// reconciliation owns all text-layout IDs in the registry; manually
    /// registering text layouts between managed calls is unsupported. Images
    /// and textures remain independently owned and are never cleared here.
    pub fn reconcile_text_layouts(
        &mut self,
        store: &TextLayoutStore,
        sync: &mut TextLayoutResourceSync,
    ) -> TextLayoutResourceSyncReport {
        let Some(cursor) = sync.cursor else {
            return self.rebuild_text_layouts(store, sync);
        };
        let changes = store.changes_since(cursor);
        if changes.requires_reset() {
            return self.rebuild_text_layouts(store, sync);
        }

        let next_cursor = changes.cursor();
        let mut report = TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Incremental,
            processed_changes: 0,
            added: 0,
            updated: 0,
            removed: 0,
            retained: self.text_layouts.len(),
        };
        for change in changes.iter() {
            report.processed_changes += 1;
            if let Some(stored) = store.stored_layout(change.id()) {
                match self.upsert_stored_text_layout(stored) {
                    TextLayoutMutation::Added => report.added += 1,
                    TextLayoutMutation::Updated => report.updated += 1,
                    TextLayoutMutation::Unchanged => {}
                }
            } else if self.text_layouts.remove(&change.id()).is_some() {
                report.removed += 1;
            }
        }
        report.retained = self.text_layouts.len();
        sync.cursor = Some(next_cursor);
        report
    }

    /// Returns the number of retained shaped text resources.
    #[must_use]
    pub fn text_layout_count(&self) -> usize {
        self.text_layouts.len()
    }

    /// Returns checked owned-key and reachable shaped-layout payload bytes.
    ///
    /// This metric excludes registry buckets, allocator and `Arc` headers,
    /// shared font data, renderer caches, shaping-engine internals, and external
    /// `Arc` owners. Because shaped layouts are shared with the text store, this
    /// reachability metric and the store metric are not additive process RSS.
    #[must_use]
    pub fn retained_text_layout_payload_bytes(&self) -> Option<usize> {
        self.text_layouts
            .values()
            .try_fold(0_usize, |total, resource| {
                total.checked_add(text_layout_resource_payload_bytes(resource)?)
            })
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
        self.text_layout_resource(layout)
            .map(|resource| resource.layout.as_ref())
    }

    /// Returns a registered shaped text layout resource.
    #[must_use]
    pub fn text_layout_resource(&self, layout: TextLayoutId) -> Option<&TextLayoutResource> {
        self.text_layouts.get(&layout)
    }

    /// Builds a deterministic resource inventory for tests and diagnostics.
    #[must_use]
    pub fn snapshot(&self) -> RenderResourceSnapshot {
        RenderResourceSnapshot::from_resources(self)
    }

    fn rebuild_text_layouts(
        &mut self,
        store: &TextLayoutStore,
        sync: &mut TextLayoutResourceSync,
    ) -> TextLayoutResourceSyncReport {
        let removed = self.text_layouts.len();
        self.text_layouts.clear();
        let mut added = 0;
        for stored in store.layouts() {
            let mutation = self.upsert_stored_text_layout(stored);
            debug_assert!(matches!(mutation, TextLayoutMutation::Added));
            added += 1;
        }
        sync.cursor = Some(store.change_cursor());
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Full,
            processed_changes: 0,
            added,
            updated: 0,
            removed,
            retained: self.text_layouts.len(),
        }
    }

    fn upsert_stored_text_layout(&mut self, stored: StoredTextLayout<'_>) -> TextLayoutMutation {
        if let Some(existing) = self.text_layouts.get(&stored.id)
            && existing.key.eq(stored.key)
            && Arc::ptr_eq(&existing.layout, &stored.layout)
        {
            return TextLayoutMutation::Unchanged;
        }

        let mutation = if self.text_layouts.contains_key(&stored.id) {
            TextLayoutMutation::Updated
        } else {
            TextLayoutMutation::Added
        };
        self.text_layouts.insert(
            stored.id,
            TextLayoutResource {
                id: stored.id,
                key: stored.key.clone(),
                layout: stored.layout,
            },
        );
        mutation
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TextLayoutMutation {
    Added,
    Updated,
    Unchanged,
}

fn text_layout_resource_payload_bytes(resource: &TextLayoutResource) -> Option<usize> {
    let layout = resource.layout.as_ref();
    let mut bytes = checked_payload_sum([
        size_of::<TextLayoutKey>(),
        resource.key.text.capacity(),
        resource.key.style.family.capacity(),
        size_of::<ShapedTextLayout>(),
        checked_payload_product(layout.lines.capacity(), size_of::<ShapedTextLine>())?,
        checked_payload_product(layout.runs.capacity(), size_of::<ShapedGlyphRun>())?,
    ])?;
    for run in &layout.runs {
        bytes = bytes.checked_add(checked_payload_product(
            run.normalized_coords.capacity(),
            size_of::<i16>(),
        )?)?;
        bytes = bytes.checked_add(checked_payload_product(
            run.glyphs.capacity(),
            size_of::<ShapedGlyph>(),
        )?)?;
    }
    Some(bytes)
}

fn checked_payload_product(count: usize, item_size: usize) -> Option<usize> {
    count.checked_mul(item_size)
}

fn checked_payload_sum(parts: impl IntoIterator<Item = usize>) -> Option<usize> {
    parts.into_iter().try_fold(0_usize, usize::checked_add)
}

/// Deterministic, payload-free resource inventory used by renderer snapshot tests.
///
/// Snapshots are sorted by raw handle within each resource class and contain
/// metadata only. Raw image pixels, font bytes, shaped glyph payloads, and
/// backend-native objects are intentionally excluded.
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
            .values()
            .map(TextLayoutResourceSnapshot::from_resource)
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

    /// Returns the number of resources included in this snapshot.
    #[must_use]
    pub fn len(&self) -> usize {
        self.images.len() + self.textures.len() + self.text_layouts.len()
    }

    /// Returns true when the snapshot contains no resources.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Renders the resource inventory as stable line-oriented text.
    #[must_use]
    pub fn to_text(&self) -> String {
        let mut lines = Vec::with_capacity(self.len() + 1);
        lines.push("resources:".to_owned());
        for image in &self.images {
            lines.push(format!(
                "  image#{id} size={width}x{height} sampling={sampling} pixels={pixels} atlas={atlas}",
                id = image.id,
                width = format_f32(image.width),
                height = format_f32(image.height),
                sampling = format_sampling(image.sampling),
                pixels = image.has_pixels,
                atlas = format_optional_atlas(image.atlas),
            ));
        }
        for texture in &self.textures {
            lines.push(format!(
                "  texture#{id} size={width}x{height} sampling={sampling} snapshot={snapshot}",
                id = texture.id,
                width = format_f32(texture.width),
                height = format_f32(texture.height),
                sampling = format_sampling(texture.sampling),
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
    /// Sampling intent.
    pub sampling: RenderImageSampling,
    /// Atlas source when this resource is a region.
    pub atlas: Option<ImageAtlasRegionSnapshot>,
}

impl ImageResourceSnapshot {
    fn from_resource(resource: &ImageResource) -> Self {
        Self {
            id: resource.id.raw(),
            width: OrderedF32::new(resource.size.width),
            height: OrderedF32::new(resource.size.height),
            has_pixels: resource.pixels.is_some(),
            sampling: resource.sampling,
            atlas: resource
                .atlas_region
                .map(ImageAtlasRegionSnapshot::from_region),
        }
    }
}

/// Snapshot of one atlas region.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageAtlasRegionSnapshot {
    /// Raw atlas image handle.
    pub atlas: u64,
    /// Source x coordinate.
    pub x: OrderedF32,
    /// Source y coordinate.
    pub y: OrderedF32,
    /// Source width.
    pub width: OrderedF32,
    /// Source height.
    pub height: OrderedF32,
}

impl ImageAtlasRegionSnapshot {
    fn from_region(region: ImageAtlasRegion) -> Self {
        Self {
            atlas: region.atlas.raw(),
            x: OrderedF32::new(region.source.x),
            y: OrderedF32::new(region.source.y),
            width: OrderedF32::new(region.source.width),
            height: OrderedF32::new(region.source.height),
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
    /// Sampling intent.
    pub sampling: RenderImageSampling,
}

impl TextureResourceSnapshot {
    fn from_resource(resource: &TextureResource) -> Self {
        Self {
            id: resource.id.raw(),
            width: OrderedF32::new(resource.size.width),
            height: OrderedF32::new(resource.size.height),
            has_snapshot: resource.snapshot.is_some(),
            sampling: resource.sampling,
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
    fn from_resource(resource: &TextLayoutResource) -> Self {
        Self {
            id: resource.id.raw(),
            width: OrderedF32::new(resource.layout.size.width),
            height: OrderedF32::new(resource.layout.size.height),
            line_count: resource.layout.line_count,
            glyph_count: resource.layout.glyph_count(),
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

fn format_optional_atlas(atlas: Option<ImageAtlasRegionSnapshot>) -> String {
    atlas.map_or_else(
        || "none".to_owned(),
        |atlas| {
            format!(
                "{}:({},{},{},{})",
                atlas.atlas,
                format_f32(atlas.x),
                format_f32(atlas.y),
                format_f32(atlas.width),
                format_f32(atlas.height)
            )
        },
    )
}

fn format_sampling(sampling: RenderImageSampling) -> &'static str {
    match sampling {
        RenderImageSampling::Pixelated => "pixelated",
        RenderImageSampling::UiIcon => "ui_icon",
        RenderImageSampling::Smooth => "smooth",
        RenderImageSampling::HighQuality => "high_quality",
    }
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
/// [`RenderFrameOutput::diagnostics`]. Backend crates should choose a stable,
/// concrete error type for `Self::Error` so future device, resource upload, or
/// presentation failures can be added without changing the backend trait shape.
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
    /// Primitive contained invalid geometry or color and was sanitized or skipped.
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
    "stern-render"
}

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, mem::size_of, sync::Arc};

    use super::{
        ImageAtlasRegion, ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput,
        RenderImage, RenderImageSampling, RenderResources, RendererBackend, TextLayoutMutation,
        TextLayoutResource, TextureResource, checked_payload_product, checked_payload_sum,
    };
    use stern_core::{
        ImageId, PhysicalSize, ScaleFactor, Size, TextLayoutId, TextureId, ViewportInfo,
    };
    use stern_text::{
        CosmicTextEngine, ShapedGlyph, ShapedGlyphRun, ShapedTextLayout, ShapedTextLine,
        TextLayoutKey, TextLayoutStore, TextStyle,
    };

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
    fn render_image_clones_share_pixel_payloads() {
        let image = RenderImage::rgba8(2, 2, vec![1; 16]).expect("valid image");
        let clone = image.clone();

        assert!(std::sync::Arc::ptr_eq(&image.data, &clone.data));
    }

    #[test]
    fn resources_register_images_textures_and_text_layouts() {
        let mut resources = RenderResources::new();
        let image = ImageId::from_raw(1);
        let texture = TextureId::from_raw(2);
        let text = TextLayoutId::from_raw(3);
        let mut engine = CosmicTextEngine::new();
        let key = TextLayoutKey::new(
            "Label",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        );
        let layout = engine.shape_text(&key);

        resources.register_image(ImageResource {
            id: image,
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: None,
        });
        resources.register_texture(TextureResource {
            id: texture,
            size: Size::new(1.0, 1.0),
            sampling: RenderImageSampling::default(),
            snapshot: None,
        });
        resources.register_text_layout(TextLayoutResource {
            id: text,
            key,
            layout: Arc::new(layout),
        });

        assert!(resources.has_image(image));
        assert!(resources.has_texture(texture));
        assert!(resources.has_text_layout(text));
        assert!(resources.image(image).is_some());
        assert!(resources.texture(texture).is_some());
        assert!(resources.text_layout(text).is_some());
    }

    #[test]
    fn resources_preserve_shared_text_layout_exports() {
        let mut store = TextLayoutStore::new();
        let id = store.layout_id(TextLayoutKey::new(
            "Shared",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ));
        let exported = store
            .layouts()
            .find(|layout| layout.id == id)
            .expect("stored layout");
        let exported_layout = Arc::clone(&exported.layout);
        let mut resources = RenderResources::new();

        resources.register_text_layouts([exported]);

        assert!(Arc::ptr_eq(
            &exported_layout,
            &resources
                .text_layout_resource(id)
                .expect("resource layout")
                .layout
        ));
    }

    #[test]
    fn dirty_upsert_distinguishes_update_from_identity_and_payload_sum_is_checked() {
        let mut store = TextLayoutStore::new();
        let id = store.layout_id(TextLayoutKey::new(
            "canonical",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ));
        let mut resources = RenderResources::new();
        resources.register_text_layout(TextLayoutResource {
            id,
            key: TextLayoutKey::new(
                "manual conflict",
                TextStyle::new("serif", 9.0, 11.0),
                80.0,
                true,
            ),
            layout: Arc::new(ShapedTextLayout {
                size: Size::new(1.0, 1.0),
                line_count: 1,
                lines: Vec::new(),
                runs: Vec::new(),
            }),
        });

        assert_eq!(
            resources.upsert_stored_text_layout(store.stored_layout(id).expect("stored")),
            TextLayoutMutation::Updated
        );
        assert_eq!(
            resources.upsert_stored_text_layout(store.stored_layout(id).expect("stored")),
            TextLayoutMutation::Unchanged
        );
        assert!(checked_payload_sum([usize::MAX, 1]).is_none());
    }

    #[test]
    fn text_layout_payload_metric_counts_spare_capacities_and_checks_multiplication() {
        let mut text = String::with_capacity(128);
        text.push_str("capacity metric");
        let mut family = String::with_capacity(96);
        family.push_str("sans-serif");
        let key = TextLayoutKey::new(text, TextStyle::new(family, 12.0, 16.0), 200.0, false);
        let mut engine = CosmicTextEngine::new();
        let mut layout = engine.shape_text(&key);
        layout.lines.reserve_exact(9);
        layout.runs.reserve_exact(7);
        for run in &mut layout.runs {
            run.glyphs.reserve_exact(19);
        }

        assert!(key.text.capacity() > key.text.len());
        assert!(key.style.family.capacity() > key.style.family.len());
        assert!(layout.lines.capacity() > layout.lines.len());
        assert!(layout.runs.capacity() > layout.runs.len());
        assert!(
            layout
                .runs
                .iter()
                .all(|run| run.glyphs.capacity() > run.glyphs.len())
        );

        let mut expected = size_of::<TextLayoutKey>()
            + key.text.capacity()
            + key.style.family.capacity()
            + size_of::<ShapedTextLayout>()
            + layout.lines.capacity() * size_of::<ShapedTextLine>()
            + layout.runs.capacity() * size_of::<ShapedGlyphRun>();
        for run in &layout.runs {
            expected += run.normalized_coords.capacity() * size_of::<i16>();
            expected += run.glyphs.capacity() * size_of::<ShapedGlyph>();
        }

        let mut resources = RenderResources::new();
        resources.register_text_layout(TextLayoutResource {
            id: TextLayoutId::from_raw(91),
            key,
            layout: Arc::new(layout),
        });

        assert_eq!(
            resources.retained_text_layout_payload_bytes(),
            Some(expected)
        );
        assert!(checked_payload_product(usize::MAX, 2).is_none());
        assert!(checked_payload_sum([usize::MAX, 1]).is_none());
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
            atlas_region: None,
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(3),
            size: Size::new(2.0, 2.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: Some(ImageAtlasRegion {
                atlas: ImageId::from_raw(2),
                source: stern_core::Rect::new(1.0, 0.0, 2.0, 2.0),
            }),
        });
        resources.register_text_layout(TextLayoutResource {
            id: TextLayoutId::from_raw(5),
            key: TextLayoutKey::new(
                "Label",
                TextStyle::new("sans-serif", 12.0, 16.0),
                200.0,
                false,
            ),
            layout: Arc::new(layout),
        });
        resources.register_image(ImageResource {
            id: ImageId::from_raw(1),
            size: Size::new(2.0, 1.0),
            sampling: RenderImageSampling::default(),
            pixels: None,
            atlas_region: None,
        });

        assert_eq!(
            resources.snapshot().to_text(),
            "resources:\n  image#1 size=2.000x1.000 sampling=pixelated pixels=false atlas=none\n  image#2 size=4.000x3.000 sampling=pixelated pixels=true atlas=none\n  image#3 size=2.000x2.000 sampling=pixelated pixels=false atlas=2:(1.000,0.000,2.000,2.000)\n  texture#9 size=12.000x8.000 sampling=pixelated snapshot=false\n  text_layout#5 size=20.000x10.000 lines=2 glyphs=0"
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
