//! Backend-neutral resource snapshot conformance tests.

use std::{convert::Infallible, fs, path::Component, sync::Arc};

use kinetik_ui_core::{
    ImageId, PhysicalSize, Rect, ScaleFactor, Size, TextLayoutId, TextureId, ViewportInfo,
};
use kinetik_ui_render::{
    ImageAtlasRegion, ImageResource, RenderDiagnostic, RenderFrameInput, RenderFrameOutput,
    RenderImage, RenderImageSampling, RenderResources, RendererBackend, TextLayoutResource,
    TextureResource,
};
use kinetik_ui_text::{ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle};

mod support;

use support::resource_snapshot_artifacts::{
    artifact_paths, assert_snapshot_text, emit_snapshot_artifacts,
};

fn empty_layout(width: f32, height: f32, line_count: usize) -> Arc<ShapedTextLayout> {
    Arc::new(ShapedTextLayout {
        size: Size::new(width, height),
        line_count,
        lines: Vec::new(),
        runs: Vec::new(),
    })
}

#[derive(Default)]
struct DiagnosticRenderer;

impl RendererBackend for DiagnosticRenderer {
    type Error = Infallible;

    fn render_frame(
        &mut self,
        input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error> {
        Ok(RenderFrameOutput {
            primitive_count: input.primitives.len(),
            diagnostics: vec![
                RenderDiagnostic::MissingImage(ImageId::from_raw(11)),
                RenderDiagnostic::MissingTexture(TextureId::from_raw(12)),
            ],
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FatalRendererError;

struct FatalRenderer;

impl RendererBackend for FatalRenderer {
    type Error = FatalRendererError;

    fn render_frame(
        &mut self,
        _input: RenderFrameInput<'_>,
    ) -> Result<RenderFrameOutput, Self::Error> {
        Err(FatalRendererError)
    }
}

fn empty_frame_input(resources: &RenderResources) -> RenderFrameInput<'_> {
    RenderFrameInput {
        viewport: ViewportInfo::new(
            Size::new(100.0, 50.0),
            PhysicalSize::new(200, 100),
            ScaleFactor::new(2.0),
        ),
        primitives: &[],
        resources,
    }
}

#[test]
fn resource_snapshot_conformance_sorts_resources_by_handle() {
    let mut resources = RenderResources::new();

    resources.register_texture(TextureResource {
        id: TextureId::from_raw(40),
        size: Size::new(32.0, 16.0),
        sampling: RenderImageSampling::HighQuality,
        snapshot: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(9),
        size: Size::new(8.0, 8.0),
        sampling: RenderImageSampling::Smooth,
        pixels: None,
        atlas_region: None,
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(12),
        key: TextLayoutKey::new(
            "Later",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(30.0, 16.0, 1),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(4),
        size: Size::new(4.0, 2.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: Some(RenderImage::rgba8(4, 2, vec![255; 32]).expect("valid texture snapshot")),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(1),
        size: Size::new(2.0, 1.0),
        sampling: RenderImageSampling::UiIcon,
        pixels: Some(RenderImage::rgba8(2, 1, vec![128; 8]).expect("valid image pixels")),
        atlas_region: None,
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(3),
        key: TextLayoutKey::new(
            "First",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(10.0, 16.0, 1),
    });

    assert_snapshot_text(
        "resource_snapshot_conformance_sorts_resources_by_handle",
        "resources:\n  image#1 size=2.000x1.000 sampling=ui_icon pixels=true atlas=none\n  image#9 size=8.000x8.000 sampling=smooth pixels=false atlas=none\n  texture#4 size=4.000x2.000 sampling=pixelated snapshot=true\n  texture#40 size=32.000x16.000 sampling=high_quality snapshot=false\n  text_layout#3 size=10.000x16.000 lines=1 glyphs=0\n  text_layout#12 size=30.000x16.000 lines=1 glyphs=0",
        &resources.snapshot().to_text(),
    );
}

#[test]
fn resource_snapshot_conformance_omits_raw_payloads_and_backend_objects() {
    let mut resources = RenderResources::new();

    resources.register_image(ImageResource {
        id: ImageId::from_raw(5),
        size: Size::new(f32::NAN, -0.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(RenderImage::rgba8(1, 1, vec![1, 2, 3, 4]).expect("valid image pixels")),
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(2),
            source: Rect::new(1.0, 2.0, f32::INFINITY, -0.0),
        }),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(6),
        size: Size::new(-0.0, f32::NEG_INFINITY),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![5, 6, 7, 8]).expect("valid snapshot")),
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(7),
        key: TextLayoutKey::new(
            "Bytes stay out of snapshots",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(f32::INFINITY, -0.0, 2),
    });

    let snapshot = resources.snapshot().to_text();

    assert_snapshot_text(
        "resource_snapshot_conformance_omits_raw_payloads_and_backend_objects",
        "resources:\n  image#5 size=0.000x0.000 sampling=pixelated pixels=true atlas=2:(1.000,2.000,0.000,0.000)\n  texture#6 size=0.000x0.000 sampling=smooth snapshot=true\n  text_layout#7 size=0.000x0.000 lines=2 glyphs=0",
        &snapshot,
    );
    assert!(!snapshot.contains("1, 2, 3, 4"));
    assert!(!snapshot.contains("5, 6, 7, 8"));
    assert!(!snapshot.contains("RenderImage"));
    assert!(!snapshot.contains("Arc"));
}

#[test]
fn resource_snapshot_conformance_exposes_missing_payload_metadata() {
    let mut resources = RenderResources::new();

    resources.register_image(ImageResource {
        id: ImageId::from_raw(21),
        size: Size::new(1920.0, 1080.0),
        sampling: RenderImageSampling::HighQuality,
        pixels: None,
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(22),
        size: Size::new(640.0, 360.0),
        sampling: RenderImageSampling::Smooth,
        snapshot: None,
    });

    let snapshot = resources.snapshot();

    assert_eq!(snapshot.len(), 2);
    assert!(!snapshot.is_empty());
    assert_eq!(snapshot.images[0].id, 21);
    assert!(!snapshot.images[0].has_pixels);
    assert_eq!(snapshot.textures[0].id, 22);
    assert!(!snapshot.textures[0].has_snapshot);
    assert_snapshot_text(
        "resource_snapshot_conformance_exposes_missing_payload_metadata",
        "resources:\n  image#21 size=1920.000x1080.000 sampling=high_quality pixels=false atlas=none\n  texture#22 size=640.000x360.000 sampling=smooth snapshot=false",
        &snapshot.to_text(),
    );
}

#[test]
fn resource_snapshot_conformance_normalizes_atlas_region_metadata() {
    let mut resources = RenderResources::new();

    resources.register_image(ImageResource {
        id: ImageId::from_raw(4),
        size: Size::new(16.0, 16.0),
        sampling: RenderImageSampling::UiIcon,
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(2),
            source: Rect::new(-0.0, f32::NAN, f32::INFINITY, 7.8912),
        }),
    });

    let snapshot = resources.snapshot().to_text();

    assert_snapshot_text(
        "resource_snapshot_conformance_normalizes_atlas_region_metadata",
        "resources:\n  image#4 size=16.000x16.000 sampling=ui_icon pixels=false atlas=2:(0.000,0.000,0.000,7.891)",
        &snapshot,
    );
    assert!(!snapshot.contains("NaN"));
    assert!(!snapshot.contains("inf"));
    assert!(!snapshot.contains("-0.000"));
}

#[test]
fn resource_snapshot_conformance_registers_shaped_text_store_exports() {
    let mut store = TextLayoutStore::new();
    let key = TextLayoutKey::new(
        "Glyph payload stays hidden",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    );
    let id = store.layout_id(key);
    let layout = store.layout(id).expect("layout is shaped");
    let expected_line_count = layout.line_count;
    let expected_glyph_count = layout.glyph_count();
    let mut resources = RenderResources::new();

    resources.register_text_layouts(store.layouts());

    let snapshot = resources.snapshot();
    assert_eq!(snapshot.text_layouts.len(), 1);
    assert_eq!(snapshot.text_layouts[0].id, id.raw());
    assert_eq!(snapshot.text_layouts[0].line_count, expected_line_count);
    assert_eq!(snapshot.text_layouts[0].glyph_count, expected_glyph_count);
    assert!(snapshot.text_layouts[0].glyph_count > 0);

    let snapshot_text = snapshot.to_text();
    assert_snapshot_text(
        "resource_snapshot_conformance_registers_shaped_text_store_exports",
        &format!(
            "resources:\n  text_layout#{id} size={width:.3}x{height:.3} lines={lines} glyphs={glyphs}",
            id = id.raw(),
            width = snapshot.text_layouts[0].width.get(),
            height = snapshot.text_layouts[0].height.get(),
            lines = expected_line_count,
            glyphs = expected_glyph_count,
        ),
        &snapshot_text,
    );
    assert!(!snapshot_text.contains("Glyph payload stays hidden"));
    assert!(!snapshot_text.contains("ShapedGlyph"));
    assert!(!snapshot_text.contains("PenikoFont"));
    assert!(!snapshot_text.contains("Inter"));
}

#[test]
fn resource_snapshot_conformance_preserves_shaped_text_layout_sharing() {
    let mut store = TextLayoutStore::new();
    let first = store.layout_id(TextLayoutKey::new(
        "Shared layout",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    ));
    let second = store.layout_id(TextLayoutKey::new(
        "Shared layout\nSecond line",
        TextStyle::new("sans-serif", 12.0, 16.0),
        120.0,
        true,
    ));
    let exported = store.layouts().collect::<Vec<_>>();
    let first_export = exported
        .iter()
        .find(|layout| layout.id == first)
        .expect("first layout export");
    let second_export = exported
        .iter()
        .find(|layout| layout.id == second)
        .expect("second layout export");
    let first_layout = Arc::clone(&first_export.layout);
    let second_layout = Arc::clone(&second_export.layout);
    let mut resources = RenderResources::new();

    resources.register_text_layouts(exported);

    assert!(Arc::ptr_eq(
        &first_layout,
        &resources
            .text_layout_resource(first)
            .expect("first registered layout")
            .layout
    ));
    assert!(Arc::ptr_eq(
        &second_layout,
        &resources
            .text_layout_resource(second)
            .expect("second registered layout")
            .layout
    ));
    assert_eq!(resources.snapshot().text_layouts.len(), 2);
}

#[test]
fn renderer_backend_contract_distinguishes_diagnostics_from_fatal_errors() {
    let resources = RenderResources::new();
    let input = empty_frame_input(&resources);
    let mut diagnostic_renderer = DiagnosticRenderer;

    let output = diagnostic_renderer
        .render_frame(input)
        .expect("diagnostic renderer should submit");

    assert_eq!(output.primitive_count, 0);
    assert_eq!(
        output.diagnostics,
        vec![
            RenderDiagnostic::MissingImage(ImageId::from_raw(11)),
            RenderDiagnostic::MissingTexture(TextureId::from_raw(12)),
        ]
    );

    let mut fatal_renderer = FatalRenderer;
    let error = fatal_renderer
        .render_frame(empty_frame_input(&resources))
        .expect_err("fatal renderer should return its error type");

    assert_eq!(error, FatalRendererError);
}

#[test]
fn resource_snapshot_conformance_keeps_mixed_inventory_stable_and_payload_free() {
    let mut resources = RenderResources::new();

    resources.register_texture(TextureResource {
        id: TextureId::from_raw(102),
        size: Size::new(320.0, 180.0),
        sampling: RenderImageSampling::HighQuality,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![9, 8, 7, 6]).expect("valid snapshot")),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(24),
        size: Size::new(64.0, 64.0),
        sampling: RenderImageSampling::Smooth,
        pixels: None,
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(7),
            source: Rect::new(4.0, 8.0, 16.0, 12.0),
        }),
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(88),
        key: TextLayoutKey::new(
            "Hidden snapshot payload",
            TextStyle::new("sans-serif", 13.0, 18.0),
            140.0,
            true,
        ),
        layout: empty_layout(92.0, 36.0, 2),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(3),
        size: Size::new(8.0, 8.0),
        sampling: RenderImageSampling::Pixelated,
        snapshot: None,
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(7),
        size: Size::new(128.0, 128.0),
        sampling: RenderImageSampling::UiIcon,
        pixels: Some(RenderImage::rgba8(1, 1, vec![1, 3, 5, 7]).expect("valid image pixels")),
        atlas_region: None,
    });
    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(2),
        key: TextLayoutKey::new(
            "First hidden payload",
            TextStyle::new("monospace", 11.0, 15.0),
            80.0,
            false,
        ),
        layout: empty_layout(48.0, 15.0, 1),
    });

    let snapshot = resources.snapshot().to_text();

    assert_snapshot_text(
        "resource_snapshot_conformance_keeps_mixed_inventory_stable_and_payload_free",
        "resources:\n  image#7 size=128.000x128.000 sampling=ui_icon pixels=true atlas=none\n  image#24 size=64.000x64.000 sampling=smooth pixels=false atlas=7:(4.000,8.000,16.000,12.000)\n  texture#3 size=8.000x8.000 sampling=pixelated snapshot=false\n  texture#102 size=320.000x180.000 sampling=high_quality snapshot=true\n  text_layout#2 size=48.000x15.000 lines=1 glyphs=0\n  text_layout#88 size=92.000x36.000 lines=2 glyphs=0",
        &snapshot,
    );
    assert_eq!(snapshot, resources.snapshot().to_text());
    assert!(!snapshot.contains("Hidden snapshot payload"));
    assert!(!snapshot.contains("First hidden payload"));
    assert!(!snapshot.contains("1, 3, 5, 7"));
    assert!(!snapshot.contains("9, 8, 7, 6"));
    assert!(!snapshot.contains("RenderImage"));
}

#[test]
fn resource_snapshot_conformance_sanitizes_and_rounds_unstable_values() {
    let mut resources = RenderResources::new();

    resources.register_text_layout(TextLayoutResource {
        id: TextLayoutId::from_raw(8),
        key: TextLayoutKey::new(
            "Text payload stays out",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
        layout: empty_layout(12.3456, -0.0, 1),
    });
    resources.register_image(ImageResource {
        id: ImageId::from_raw(3),
        size: Size::new(f32::NAN, 2.3456),
        sampling: RenderImageSampling::Smooth,
        pixels: Some(RenderImage::rgba8(1, 1, vec![10, 20, 30, 40]).expect("valid pixels")),
        atlas_region: Some(ImageAtlasRegion {
            atlas: ImageId::from_raw(1),
            source: Rect::new(-0.0, f32::INFINITY, 3.4567, f32::NEG_INFINITY),
        }),
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(2),
        size: Size::new(-0.0, 4.5678),
        sampling: RenderImageSampling::HighQuality,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![50, 60, 70, 80]).expect("valid snapshot")),
    });

    let snapshot = resources.snapshot().to_text();

    assert_snapshot_text(
        "resource_snapshot_conformance_sanitizes_and_rounds_unstable_values",
        "resources:\n  image#3 size=0.000x2.346 sampling=smooth pixels=true atlas=1:(0.000,0.000,3.457,0.000)\n  texture#2 size=0.000x4.568 sampling=high_quality snapshot=true\n  text_layout#8 size=12.346x0.000 lines=1 glyphs=0",
        &snapshot,
    );
    assert_eq!(snapshot, resources.snapshot().to_text());
    assert!(!snapshot.contains("Text payload stays out"));
    assert!(!snapshot.contains("10, 20, 30, 40"));
    assert!(!snapshot.contains("50, 60, 70, 80"));
    assert!(!snapshot.contains("NaN"));
    assert!(!snapshot.contains("inf"));
    assert!(!snapshot.contains("-0.000"));
}

#[test]
fn resource_snapshot_artifact_helper_formats_paths_under_target() {
    let paths = artifact_paths("Resource Snapshot: Example/Case");
    let directory = paths.directory.to_string_lossy().replace('\\', "/");

    assert!(paths.directory.is_absolute());
    assert!(
        paths
            .directory
            .components()
            .all(|component| { !matches!(component, Component::ParentDir | Component::CurDir) })
    );
    assert!(directory.ends_with(
        "target/kinetik-ui-artifacts/kinetik-ui-render/resource-snapshots/resource-snapshot-example-case"
    ));
    assert!(paths.expected.ends_with("expected.txt"));
    assert!(paths.actual.ends_with("actual.txt"));
    assert!(paths.diff.ends_with("diff.txt"));
}

#[test]
fn resource_snapshot_artifact_helper_writes_explicit_artifacts_without_panicking() {
    let snapshot_name = "explicit artifact emission";
    let paths = artifact_paths(snapshot_name);
    let _ = fs::remove_dir_all(&paths.directory);

    let paths = emit_snapshot_artifacts(
        snapshot_name,
        "resources:\n  image#1 size=1.000x1.000",
        "resources:\n  image#2 size=2.000x2.000",
    )
    .expect("artifact emission should succeed");

    assert_eq!(
        fs::read_to_string(&paths.expected).expect("expected artifact should be readable"),
        "resources:\n  image#1 size=1.000x1.000"
    );
    assert_eq!(
        fs::read_to_string(&paths.actual).expect("actual artifact should be readable"),
        "resources:\n  image#2 size=2.000x2.000"
    );

    let diff = fs::read_to_string(&paths.diff).expect("diff artifact should be readable");
    assert!(diff.contains("--- expected"));
    assert!(diff.contains("+++ actual"));
    assert!(diff.contains("-   image#1 size=1.000x1.000"));
    assert!(diff.contains("+   image#2 size=2.000x2.000"));
}

#[test]
fn resource_snapshot_artifact_helper_does_not_write_matching_artifacts() {
    let snapshot_name = "matching comparisons write no artifacts";
    let paths = artifact_paths(snapshot_name);
    let _ = fs::remove_dir_all(&paths.directory);

    assert_snapshot_text(snapshot_name, "resources:", "resources:");

    assert!(!paths.directory.exists());
}
