//! Explicit showcase review artifact dumps.

use std::fmt::Write as _;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use kinetik_ui::core::Size;

use crate::app::{ShowcaseApp, ShowcasePage};
use crate::raster::{rasterize, write_bmp};

/// Default root for manually inspectable showcase review dumps.
pub const DEFAULT_REVIEW_DUMP_ROOT: &str =
    "target/kinetik-ui-artifacts/kinetik-ui-showcase/review-dumps";

/// Request for a showcase review artifact dump.
#[derive(Debug, Clone, PartialEq)]
pub struct ReviewDumpRequest {
    /// Stable label used to create a dump directory under the default root.
    pub label: String,
    /// Raster width in physical pixels.
    pub width: usize,
    /// Raster height in physical pixels.
    pub height: usize,
    /// Logical viewport size used to build showcase primitives.
    pub logical_size: Size,
    /// Optional selected page. When absent, every showcase page is dumped.
    pub page: Option<ShowcasePage>,
}

impl ReviewDumpRequest {
    /// Creates a dump request using physical dimensions as logical dimensions.
    #[must_use]
    pub fn new(label: impl Into<String>, width: usize, height: usize) -> Self {
        Self {
            label: label.into(),
            width,
            height,
            logical_size: Size::new(pixel_to_f32(width), pixel_to_f32(height)),
            page: None,
        }
    }

    /// Selects a single showcase page.
    #[must_use]
    pub const fn with_page(mut self, page: ShowcasePage) -> Self {
        self.page = Some(page);
        self
    }

    /// Sets the logical viewport size used to build primitives.
    #[must_use]
    pub const fn with_logical_size(mut self, logical_size: Size) -> Self {
        self.logical_size = logical_size;
        self
    }
}

/// Metadata for a written review dump.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDump {
    /// Directory containing the dump artifacts.
    pub directory: PathBuf,
    /// Human-readable manifest path.
    pub manifest_path: PathBuf,
    /// Per-page artifact metadata.
    pub frames: Vec<ReviewDumpFrame>,
}

/// Metadata for one dumped page frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReviewDumpFrame {
    /// Showcase page name.
    pub page_name: &'static str,
    /// Number of generated primitives.
    pub primitive_count: usize,
    /// Number of frame warnings.
    pub warning_count: usize,
    /// Written CPU raster BMP path.
    pub bmp_path: PathBuf,
}

/// Returns the deterministic default dump root.
#[must_use]
pub fn default_review_dump_root() -> PathBuf {
    PathBuf::from("target")
        .join("kinetik-ui-artifacts")
        .join("kinetik-ui-showcase")
        .join("review-dumps")
}

/// Returns the default output directory for a label.
#[must_use]
pub fn default_review_dump_dir(label: &str) -> PathBuf {
    default_review_dump_root().join(stable_label(label))
}

/// Writes a showcase review artifact dump under the default root.
///
/// # Errors
///
/// Returns an I/O error when the directory, manifest, or BMP files cannot be
/// written.
pub fn dump_review_artifacts(request: &ReviewDumpRequest) -> io::Result<ReviewDump> {
    dump_review_artifacts_to_dir(request, default_review_dump_dir(&request.label))
}

/// Writes a showcase review artifact dump to an explicit directory.
///
/// Tests use this to keep deterministic assertions isolated. CLI callers should
/// prefer [`dump_review_artifacts`] so outputs stay below the default target
/// root.
///
/// # Errors
///
/// Returns an I/O error when the directory, manifest, or BMP files cannot be
/// written.
pub fn dump_review_artifacts_to_dir(
    request: &ReviewDumpRequest,
    directory: impl Into<PathBuf>,
) -> io::Result<ReviewDump> {
    let directory = directory.into();
    fs::create_dir_all(&directory)?;

    let pages = selected_pages(request.page);
    let mut frames = Vec::with_capacity(pages.len());
    for page in pages {
        let mut app = ShowcaseApp::new();
        app.set_viewport_size(request.logical_size);
        app.set_page(page);

        let page_name = page_name(page);
        let bmp_path = directory.join(format!("{page_name}.bmp"));
        let frame = rasterize(&app.output().primitives, request.width, request.height);
        write_bmp(&frame, &bmp_path)?;

        frames.push(ReviewDumpFrame {
            page_name,
            primitive_count: app.output().primitives.len(),
            warning_count: app.output().warnings.len(),
            bmp_path,
        });
    }

    let manifest_path = directory.join("manifest.txt");
    fs::write(
        &manifest_path,
        manifest_text(request, &directory, &manifest_path, &frames),
    )?;

    Ok(ReviewDump {
        directory,
        manifest_path,
        frames,
    })
}

fn selected_pages(page: Option<ShowcasePage>) -> Vec<ShowcasePage> {
    page.map_or_else(all_pages, |page| vec![page])
}

fn all_pages() -> Vec<ShowcasePage> {
    vec![
        ShowcasePage::Editor,
        ShowcasePage::Components,
        ShowcasePage::Layout,
        ShowcasePage::Viewport,
        ShowcasePage::Systems,
    ]
}

/// Stable lowercase page name used by review artifacts.
#[must_use]
pub const fn page_name(page: ShowcasePage) -> &'static str {
    match page {
        ShowcasePage::Editor => "editor",
        ShowcasePage::Components => "components",
        ShowcasePage::Layout => "layout",
        ShowcasePage::Viewport => "viewport",
        ShowcasePage::Systems => "systems",
    }
}

fn manifest_text(
    request: &ReviewDumpRequest,
    directory: &Path,
    manifest_path: &Path,
    frames: &[ReviewDumpFrame],
) -> String {
    let mut text = String::new();
    writeln!(text, "Kinetik UI Showcase Review Dump").expect("write manifest text");
    writeln!(text, "label: {}", request.label).expect("write manifest text");
    writeln!(text, "directory: {}", directory.display()).expect("write manifest text");
    writeln!(text, "manifest: {}", manifest_path.display()).expect("write manifest text");
    writeln!(
        text,
        "raster_dimensions: {}x{}",
        request.width, request.height
    )
    .expect("write manifest text");
    writeln!(
        text,
        "logical_dimensions: {:.3}x{:.3}",
        request.logical_size.width, request.logical_size.height
    )
    .expect("write manifest text");
    writeln!(text, "frame_count: {}", frames.len()).expect("write manifest text");
    text.push('\n');

    for frame in frames {
        writeln!(text, "page: {}", frame.page_name).expect("write manifest text");
        writeln!(text, "primitive_count: {}", frame.primitive_count).expect("write manifest text");
        writeln!(text, "warning_count: {}", frame.warning_count).expect("write manifest text");
        writeln!(text, "artifact: {}", frame.bmp_path.display()).expect("write manifest text");
        text.push('\n');
    }

    text
}

fn stable_label(label: &str) -> String {
    let mut output = String::new();
    for character in label.trim().chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('-') {
            output.push('-');
        }
    }
    let output = output.trim_matches('-');
    if output.is_empty() {
        "review".to_owned()
    } else {
        output.to_owned()
    }
}

fn pixel_to_f32(value: usize) -> f32 {
    let value = u16::try_from(value).unwrap_or(u16::MAX);
    f32::from(value)
}

#[cfg(test)]
mod tests {
    use super::{ReviewDumpRequest, default_review_dump_dir, dump_review_artifacts_to_dir};
    use crate::app::ShowcasePage;
    use kinetik_ui::core::Size;

    #[test]
    fn default_review_dump_root_is_under_target() {
        let root = super::default_review_dump_root();

        let components = root
            .iter()
            .map(|component| component.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            components,
            [
                "target",
                "kinetik-ui-artifacts",
                "kinetik-ui-showcase",
                "review-dumps"
            ]
        );
        assert!(root.starts_with("target"));
        let directory = default_review_dump_dir("S8 12C/Review");
        assert!(directory.starts_with(root));
        assert_eq!(directory.file_name().unwrap(), "s8-12c-review");
    }

    #[test]
    fn dump_writes_manifest_and_selected_page_bmp() {
        let directory = std::env::temp_dir().join(format!(
            "kinetik-ui-showcase-review-dump-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&directory);
        let request = ReviewDumpRequest::new("test dump", 320, 200)
            .with_logical_size(Size::new(320.0, 200.0))
            .with_page(ShowcasePage::Components);

        let dump = dump_review_artifacts_to_dir(&request, &directory).expect("dump artifacts");

        assert_eq!(dump.directory, directory);
        assert_eq!(dump.frames.len(), 1);
        assert_eq!(dump.frames[0].page_name, "components");
        assert_eq!(
            dump.frames[0].bmp_path.file_name().unwrap(),
            "components.bmp"
        );
        assert!(std::fs::metadata(&dump.manifest_path).unwrap().len() > 0);
        assert!(std::fs::metadata(&dump.frames[0].bmp_path).unwrap().len() > 54);

        let manifest = std::fs::read_to_string(&dump.manifest_path).expect("manifest");
        assert!(manifest.contains("page: components"));
        assert!(manifest.contains("raster_dimensions: 320x200"));
        assert!(manifest.contains("logical_dimensions: 320.000x200.000"));
        assert!(manifest.contains("primitive_count: "));
        assert!(manifest.contains("warning_count: "));
        assert!(manifest.contains("components.bmp"));

        let _ = std::fs::remove_dir_all(&dump.directory);
    }
}
