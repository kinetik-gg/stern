//! Window-free contracts for the story harness itself.

use std::path::PathBuf;

use stern_stories::diff::{DiffOutcome, RgbaImage, diff_images};
use stern_stories::manifest::{ManifestEntry, fnv1a64, manifest_files, manifest_json};
use stern_stories::ops::{
    DiffStatus, MANIFEST_FILE, bless_directories, diff_directories, render_stories,
};
use stern_stories::story::{StoryKind, registry, story_matches_filter};

#[test]
fn registry_ids_are_unique_kebab_case_and_titled() {
    let stories = registry();
    assert!(!stories.is_empty());
    let mut seen = std::collections::BTreeSet::new();
    for story in &stories {
        assert!(seen.insert(story.id), "duplicate story id {}", story.id);
        assert!(!story.title.trim().is_empty(), "untitled story {}", story.id);
        assert!(
            story
                .id
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-' || c == '/'),
            "story id {} is not kebab-case",
            story.id
        );
        assert!(
            story.id.contains('/'),
            "story id {} must be family/name",
            story.id
        );
    }
}

#[test]
fn registry_covers_component_composition_and_workspace_rungs() {
    let stories = registry();
    for kind in [
        StoryKind::Component,
        StoryKind::Composition,
        StoryKind::Workspace,
    ] {
        assert!(
            stories.iter().any(|story| story.kind == kind),
            "registry has no {kind:?} story"
        );
    }
}

#[test]
fn variant_matrix_includes_required_scales_and_4k_workspace() {
    for story in registry() {
        let variants = story.variants();
        let has_scale = |scale: f32| variants.iter().any(|v| (v.scale - scale).abs() < f32::EPSILON);
        assert!(has_scale(1.0), "{} missing 1.0 scale", story.id);
        assert!(has_scale(2.0), "{} missing 2.0 scale", story.id);
        if story.kind == StoryKind::Workspace {
            assert!(
                variants
                    .iter()
                    .any(|v| v.logical.width >= 3840.0 && v.logical.height >= 2160.0),
                "{} missing 4K-logical full-window variant",
                story.id
            );
            assert!(
                variants
                    .iter()
                    .map(|v| (v.logical.width.to_bits(), v.logical.height.to_bits()))
                    .collect::<std::collections::BTreeSet<_>>()
                    .len()
                    >= 3,
                "{} must cover at least three window sizes",
                story.id
            );
        }
    }
}

#[test]
fn filter_matching_is_case_insensitive_substring() {
    assert!(story_matches_filter("basic-controls/sheet", ""));
    assert!(story_matches_filter("basic-controls/sheet", "BASIC"));
    assert!(story_matches_filter("basic-controls/sheet", "controls/sh"));
    assert!(!story_matches_filter("basic-controls/sheet", "dock"));
}

#[test]
fn fnv1a64_matches_known_vectors() {
    assert_eq!(fnv1a64(b""), 0xcbf2_9ce4_8422_2325);
    assert_eq!(fnv1a64(b"a"), 0xaf63_dc4c_8601_ec8c);
}

#[test]
fn manifest_roundtrip_preserves_files_and_hashes() {
    let entries = vec![
        ManifestEntry {
            story_id: "a/b".into(),
            file: "a-b_10x10@1.00.png".into(),
            logical_width: 10,
            logical_height: 10,
            scale_percent: 100,
            device_width: 10,
            device_height: 10,
            pixel_hash: 0xdead_beef,
        },
        ManifestEntry {
            story_id: "c/d".into(),
            file: "c-d_20x20@2.00.png".into(),
            logical_width: 20,
            logical_height: 20,
            scale_percent: 200,
            device_width: 40,
            device_height: 40,
            pixel_hash: 1,
        },
    ];
    let json = manifest_json(&entries);
    let files = manifest_files(&json);
    assert_eq!(files, vec![
        ("a-b_10x10@1.00.png".to_owned(), 0xdead_beef),
        ("c-d_20x20@2.00.png".to_owned(), 1),
    ]);
}

fn solid_image(width: u32, height: u32, rgba: [u8; 4]) -> RgbaImage {
    let pixels = rgba
        .iter()
        .copied()
        .cycle()
        .take((width * height * 4) as usize)
        .collect();
    RgbaImage::new(width, height, pixels).expect("valid image")
}

#[test]
fn diff_detects_identical_changed_and_resized_images() {
    let base = solid_image(8, 8, [10, 20, 30, 255]);
    assert!(diff_images(&base, &base).is_match());

    let mut changed = base.clone();
    changed.pixels[0] = 255;
    match diff_images(&changed, &base) {
        DiffOutcome::PixelsDiffer {
            differing_pixels,
            max_channel_delta,
            diff,
        } => {
            assert_eq!(differing_pixels, 1);
            assert_eq!(max_channel_delta, 245);
            assert_eq!((diff.width, diff.height), (8, 8));
            assert_eq!(&diff.pixels[0..4], &[255, 0, 200, 255]);
        }
        other => panic!("expected pixel diff, got {other:?}"),
    }

    let resized = solid_image(4, 8, [10, 20, 30, 255]);
    assert!(matches!(
        diff_images(&resized, &base),
        DiffOutcome::DimensionsDiffer {
            current: (4, 8),
            golden: (8, 8),
        }
    ));
}

fn scratch_dir(name: &str) -> PathBuf {
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(name);
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

#[test]
fn render_is_deterministic_and_bless_then_diff_is_clean() {
    let stories = registry();
    let filter = "basic-controls";
    let first = scratch_dir("render-first");
    let second = scratch_dir("render-second");
    let report_first =
        render_stories(&stories, filter, &first).expect("first render succeeds");
    let report_second =
        render_stories(&stories, filter, &second).expect("second render succeeds");
    assert_eq!(report_first.files, report_second.files);
    assert!(!report_first.files.is_empty());

    let manifest_first =
        std::fs::read_to_string(first.join(MANIFEST_FILE)).expect("first manifest");
    let manifest_second =
        std::fs::read_to_string(second.join(MANIFEST_FILE)).expect("second manifest");
    assert_eq!(
        manifest_first, manifest_second,
        "two renders of the same tree must produce identical manifests"
    );

    // Without goldens, every file reports as missing a golden — never a match.
    let goldens = scratch_dir("goldens");
    let diff_out = scratch_dir("diff-out");
    let statuses = diff_directories(&first, &goldens, &diff_out, "")
        .expect("diff against empty goldens runs");
    assert!(
        statuses
            .iter()
            .all(|status| matches!(status, DiffStatus::MissingGolden(_))),
        "expected only missing-golden rows, got {statuses:?}"
    );

    // Bless is explicit; after blessing, the second render diffs clean.
    let blessed = bless_directories(&first, &goldens, "").expect("bless succeeds");
    assert_eq!(blessed.len(), report_first.files.len());
    let statuses =
        diff_directories(&second, &goldens, &diff_out, "").expect("diff after bless runs");
    assert!(
        statuses.iter().all(DiffStatus::is_match),
        "expected clean diff after bless, got {statuses:?}"
    );
}
