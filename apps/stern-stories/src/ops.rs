//! Render / diff / bless orchestration over directories.
//!
//! `render` writes PNGs, a contact sheet, and a deterministic manifest.
//! `diff` compares a render directory against a goldens directory and writes
//! visual diff images. `bless` copies a reviewed render into the goldens
//! directory — it is a separate human-invoked command; neither `render` nor
//! `diff` ever writes into the goldens directory.

use std::path::{Path, PathBuf};

use stern::core::Color;

use crate::contact_sheet::{SheetCell, build_contact_sheet};
use crate::diff::{DiffOutcome, RgbaImage, diff_images};
use crate::frame::compose_story_frame;
use crate::manifest::{ManifestEntry, fnv1a64, manifest_files, manifest_json};
use crate::raster::rasterize;
use crate::story::{Story, story_matches_filter};

/// Manifest file name inside a render or goldens directory.
pub const MANIFEST_FILE: &str = "manifest.json";
/// Contact sheet file name inside a render directory.
pub const CONTACT_SHEET_FILE: &str = "contact-sheet.png";

/// Background color used behind every story.
#[must_use]
pub fn story_background() -> Color {
    Color::rgb8(0x0B, 0x0B, 0x0B)
}

/// Summary of one `render` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderReport {
    /// Rendered variant files, registry order.
    pub files: Vec<String>,
    /// Diagnostics per file (empty when clean).
    pub diagnostics: Vec<(String, Vec<String>)>,
}

/// Renders every registry story matching `filter` into `out_dir`.
///
/// # Errors
///
/// Returns an error when composition, rasterization, encoding, or file IO
/// fails.
pub fn render_stories(
    stories: &[Story],
    filter: &str,
    out_dir: &Path,
) -> Result<RenderReport, String> {
    std::fs::create_dir_all(out_dir)
        .map_err(|error| format!("create {}: {error}", out_dir.display()))?;
    let mut entries = Vec::new();
    let mut files = Vec::new();
    let mut diagnostics = Vec::new();
    let mut cells = Vec::new();
    for story in stories {
        if !story_matches_filter(story.id, filter) {
            continue;
        }
        for variant in story.variants() {
            let composed = compose_story_frame(story, variant);
            let frame = rasterize(
                &composed.output.primitives,
                &composed.resources,
                variant.logical,
                variant.scale,
                story_background(),
            )
            .ok_or_else(|| format!("rasterize failed for {}", story.id))?;
            let file = format!("{}.png", variant.file_stem(story.id));
            write_png(&out_dir.join(&file), &frame.image)?;
            entries.push(manifest_entry(story.id, &file, variant, &frame.image));
            if !frame.diagnostics.is_empty() {
                diagnostics.push((file.clone(), frame.diagnostics.clone()));
            }
            cells.push(SheetCell {
                label: format!(
                    "{} {}x{}@{:.2}",
                    story.id, variant.logical.width, variant.logical.height, variant.scale
                ),
                image: frame.image,
            });
            files.push(file);
        }
    }
    if files.is_empty() {
        return Err(format!("no stories match filter {filter:?}"));
    }
    let sheet = build_contact_sheet(&cells).ok_or("contact sheet rasterization failed")?;
    write_png(&out_dir.join(CONTACT_SHEET_FILE), &sheet.image)?;
    std::fs::write(out_dir.join(MANIFEST_FILE), manifest_json(&entries))
        .map_err(|error| format!("write manifest: {error}"))?;
    Ok(RenderReport { files, diagnostics })
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn manifest_entry(
    story_id: &str,
    file: &str,
    variant: crate::story::StoryVariant,
    image: &RgbaImage,
) -> ManifestEntry {
    ManifestEntry {
        story_id: story_id.to_owned(),
        file: file.to_owned(),
        logical_width: variant.logical.width as u32,
        logical_height: variant.logical.height as u32,
        scale_percent: (variant.scale * 100.0).round() as u32,
        device_width: image.width,
        device_height: image.height,
        pixel_hash: fnv1a64(&image.pixels),
    }
}

/// One comparison row from a `diff` run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffStatus {
    /// Pixels are identical.
    Match(String),
    /// Pixels or dimensions differ; the diff image path is recorded when one
    /// could be written.
    Changed(String, Option<PathBuf>),
    /// Present in current render, absent from goldens.
    MissingGolden(String),
    /// Present in goldens, absent from current render.
    MissingCurrent(String),
}

impl DiffStatus {
    /// Reports whether this row is a clean match.
    #[must_use]
    pub const fn is_match(&self) -> bool {
        matches!(self, Self::Match(_))
    }
}

/// Compares the render in `current_dir` against `goldens_dir`.
///
/// # Errors
///
/// Returns an error when the current manifest is missing or unreadable, or
/// when diff images cannot be written.
pub fn diff_directories(
    current_dir: &Path,
    goldens_dir: &Path,
    diff_dir: &Path,
    filter: &str,
) -> Result<Vec<DiffStatus>, String> {
    let current_manifest = read_manifest(current_dir)?;
    let golden_manifest = read_manifest(goldens_dir).unwrap_or_default();
    std::fs::create_dir_all(diff_dir)
        .map_err(|error| format!("create {}: {error}", diff_dir.display()))?;

    let mut statuses = Vec::new();
    for (file, _) in &current_manifest {
        if !file_matches_filter(file, filter) {
            continue;
        }
        let golden_path = goldens_dir.join(file);
        if !golden_path.is_file() {
            statuses.push(DiffStatus::MissingGolden(file.clone()));
            continue;
        }
        let current = read_png(&current_dir.join(file))?;
        let golden = read_png(&golden_path)?;
        match diff_images(&current, &golden) {
            DiffOutcome::Match => statuses.push(DiffStatus::Match(file.clone())),
            DiffOutcome::PixelsDiffer { diff, .. } => {
                let diff_path = diff_dir.join(format!("{file}.diff.png"));
                write_png(&diff_path, &diff)?;
                statuses.push(DiffStatus::Changed(file.clone(), Some(diff_path)));
            }
            DiffOutcome::DimensionsDiffer { .. } => {
                statuses.push(DiffStatus::Changed(file.clone(), None));
            }
        }
    }
    for (file, _) in &golden_manifest {
        if file_matches_filter(file, filter) && !current_dir.join(file).is_file() {
            statuses.push(DiffStatus::MissingCurrent(file.clone()));
        }
    }
    Ok(statuses)
}

/// Copies the reviewed render from `current_dir` into `goldens_dir`.
///
/// Blessing is human-initiated: this function is only reachable through the
/// explicit `bless` CLI command.
///
/// # Errors
///
/// Returns an error when the current render has no manifest or files cannot
/// be copied.
pub fn bless_directories(
    current_dir: &Path,
    goldens_dir: &Path,
    filter: &str,
) -> Result<Vec<String>, String> {
    let manifest = read_manifest(current_dir)?;
    std::fs::create_dir_all(goldens_dir)
        .map_err(|error| format!("create {}: {error}", goldens_dir.display()))?;
    let mut blessed = Vec::new();
    for (file, _) in &manifest {
        if !file_matches_filter(file, filter) {
            continue;
        }
        std::fs::copy(current_dir.join(file), goldens_dir.join(file))
            .map_err(|error| format!("copy {file}: {error}"))?;
        blessed.push(file.clone());
    }
    if filter.is_empty() {
        std::fs::copy(
            current_dir.join(MANIFEST_FILE),
            goldens_dir.join(MANIFEST_FILE),
        )
        .map_err(|error| format!("copy manifest: {error}"))?;
    }
    Ok(blessed)
}

fn file_matches_filter(file: &str, filter: &str) -> bool {
    filter.is_empty()
        || file
            .to_ascii_lowercase()
            .contains(&filter.to_ascii_lowercase())
}

fn read_manifest(dir: &Path) -> Result<Vec<(String, u64)>, String> {
    let path = dir.join(MANIFEST_FILE);
    let text = std::fs::read_to_string(&path)
        .map_err(|error| format!("read {}: {error}", path.display()))?;
    Ok(manifest_files(&text))
}

/// Writes an RGBA image as a PNG file.
///
/// # Errors
///
/// Returns an error when encoding or writing fails.
pub fn write_png(path: &Path, image: &RgbaImage) -> Result<(), String> {
    let mut pixmap = tiny_skia::Pixmap::new(image.width, image.height)
        .ok_or_else(|| format!("invalid dimensions for {}", path.display()))?;
    for (pixel, source) in pixmap
        .pixels_mut()
        .iter_mut()
        .zip(image.pixels.chunks_exact(4))
    {
        *pixel = premultiply(source);
    }
    let bytes = pixmap
        .encode_png()
        .map_err(|error| format!("encode {}: {error}", path.display()))?;
    std::fs::write(path, bytes).map_err(|error| format!("write {}: {error}", path.display()))
}

/// Reads a PNG file into a straight-alpha RGBA image.
///
/// # Errors
///
/// Returns an error when the file cannot be read or decoded.
pub fn read_png(path: &Path) -> Result<RgbaImage, String> {
    let bytes = std::fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let pixmap = tiny_skia::Pixmap::decode_png(&bytes)
        .map_err(|error| format!("decode {}: {error}", path.display()))?;
    let pixels = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let demultiplied = pixel.demultiply();
            [
                demultiplied.red(),
                demultiplied.green(),
                demultiplied.blue(),
                demultiplied.alpha(),
            ]
        })
        .collect();
    RgbaImage::new(pixmap.width(), pixmap.height(), pixels)
        .ok_or_else(|| format!("decoded size mismatch for {}", path.display()))
}

fn premultiply(rgba: &[u8]) -> tiny_skia::PremultipliedColorU8 {
    tiny_skia::ColorU8::from_rgba(rgba[0], rgba[1], rgba[2], rgba[3]).premultiply()
}
