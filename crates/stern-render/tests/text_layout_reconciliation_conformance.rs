//! Source-bound incremental text resource reconciliation conformance.

use std::sync::Arc;

use stern_core::{ImageId, Size, TextLayoutId, TextureId};
use stern_render::{
    ImageResource, RenderImage, RenderImageSampling, RenderResources, TextLayoutResourceSync,
    TextLayoutResourceSyncKind, TextLayoutResourceSyncReport, TextureResource,
};
use stern_text::{TextFeatureSet, TextLayoutKey, TextLayoutStore, TextOverflow, TextStyle};

const MAX_TEXT_PAYLOAD_BYTES: usize = 32 * 1024 * 1024;

fn key(text: impl Into<String>) -> TextLayoutKey {
    TextLayoutKey::new(text, TextStyle::new("Inter", 12.0, 16.0), 120.0, false)
}

fn static_resources() -> RenderResources {
    let mut resources = RenderResources::new();
    resources.register_image(ImageResource {
        id: ImageId::from_raw(7),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Pixelated,
        pixels: Some(RenderImage::rgba8(1, 1, vec![1, 2, 3, 255]).expect("image")),
        atlas_region: None,
    });
    resources.register_texture(TextureResource {
        id: TextureId::from_raw(9),
        size: Size::new(1.0, 1.0),
        sampling: RenderImageSampling::Smooth,
        snapshot: Some(RenderImage::rgba8(1, 1, vec![4, 5, 6, 255]).expect("texture")),
    });
    resources
}

fn text_ids(resources: &RenderResources) -> Vec<u64> {
    resources
        .snapshot()
        .text_layouts
        .into_iter()
        .map(|resource| resource.id)
        .collect()
}

fn store_ids(store: &TextLayoutStore) -> Vec<u64> {
    store.layouts().map(|entry| entry.id.raw()).collect()
}

#[test]
fn initial_full_then_noop_is_exact_and_preserves_allocations() {
    let mut store = TextLayoutStore::new();
    let first = store.layout_id(key("first"));
    let second = store.layout_id(key("second 😀"));
    let mut resources = static_resources();
    let mut sync = TextLayoutResourceSync::new();

    assert!(!sync.is_initialized());
    assert_eq!(
        resources.reconcile_text_layouts(&store, &mut sync),
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Full,
            processed_changes: 0,
            added: 2,
            updated: 0,
            removed: 0,
            retained: 2,
        }
    );
    assert!(sync.is_initialized());
    assert_eq!(text_ids(&resources), store_ids(&store));
    assert_eq!(
        resources.retained_text_layout_payload_bytes(),
        Some(store.retained_payload_bytes())
    );

    let first_resource = resources.text_layout_resource(first).expect("first");
    let text_pointer = first_resource.key.text.as_ptr();
    let family_pointer = first_resource.key.style.family.as_ptr();
    let layout = Arc::clone(&first_resource.layout);
    let snapshot = resources.snapshot();
    let bytes = resources.retained_text_layout_payload_bytes();

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(
        report,
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Incremental,
            processed_changes: 0,
            added: 0,
            updated: 0,
            removed: 0,
            retained: 2,
        }
    );
    assert!(report.is_noop());
    let first_resource = resources.text_layout_resource(first).expect("first");
    assert_eq!(first_resource.key.text.as_ptr(), text_pointer);
    assert_eq!(first_resource.key.style.family.as_ptr(), family_pointer);
    assert!(Arc::ptr_eq(&first_resource.layout, &layout));
    assert_eq!(resources.snapshot(), snapshot);
    assert_eq!(resources.retained_text_layout_payload_bytes(), bytes);
    assert!(resources.has_text_layout(second));
}

#[test]
fn reconciliation_retains_distinct_feature_bearing_layout_resources() {
    let default_key = key("12038475");
    let numeric_key = TextLayoutKey::new(
        "12038475",
        TextStyle::new("Inter", 12.0, 16.0).with_features(TextFeatureSet::TABULAR_NUMBERS),
        120.0,
        false,
    );
    let mut store = TextLayoutStore::new();
    let default_id = store.layout_id(default_key);
    let numeric_id = store.layout_id(numeric_key);
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.kind, TextLayoutResourceSyncKind::Full);
    assert_eq!((report.added, report.retained), (2, 2));
    assert_ne!(default_id, numeric_id);

    let default_resource = resources.text_layout_resource(default_id).unwrap();
    let default_layout = Arc::clone(&default_resource.layout);
    assert_eq!(default_resource.key.style.features, TextFeatureSet::NONE);
    let numeric_resource = resources.text_layout_resource(numeric_id).unwrap();
    let numeric_layout = Arc::clone(&numeric_resource.layout);
    assert_eq!(
        numeric_resource.key.style.features,
        TextFeatureSet::TABULAR_NUMBERS
    );
    assert_eq!(
        resources.retained_text_layout_payload_bytes(),
        Some(store.retained_payload_bytes())
    );

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert!(report.is_noop());
    assert!(Arc::ptr_eq(
        &resources.text_layout_resource(default_id).unwrap().layout,
        &default_layout
    ));
    assert!(Arc::ptr_eq(
        &resources.text_layout_resource(numeric_id).unwrap().layout,
        &numeric_layout
    ));
}

#[test]
fn reconciliation_preserves_complete_source_and_overflow_policy() {
    let source = "Renderer resources keep this complete source even when its presentation elides";
    let visible_key = key(source);
    let ellipsized_key = visible_key.clone().with_overflow(TextOverflow::EndEllipsis);
    let mut store = TextLayoutStore::new();
    let visible_id = store.layout_id(visible_key);
    let ellipsized_id = store.layout_id(ellipsized_key);
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.kind, TextLayoutResourceSyncKind::Full);
    assert_eq!((report.added, report.retained), (2, 2));
    assert_ne!(visible_id, ellipsized_id);

    let visible = resources.text_layout_resource(visible_id).unwrap();
    assert_eq!(visible.key.text, source);
    assert_eq!(visible.key.overflow, TextOverflow::Visible);
    assert!(!visible.layout.is_elided());
    let visible_layout = Arc::clone(&visible.layout);

    let ellipsized = resources.text_layout_resource(ellipsized_id).unwrap();
    assert_eq!(ellipsized.key.text, source);
    assert_eq!(ellipsized.key.overflow, TextOverflow::EndEllipsis);
    assert!(ellipsized.layout.is_elided());
    let ellipsized_layout = Arc::clone(&ellipsized.layout);
    assert_eq!(
        resources.retained_text_layout_payload_bytes(),
        Some(store.retained_payload_bytes())
    );

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert!(report.is_noop());
    assert!(Arc::ptr_eq(
        &resources.text_layout_resource(visible_id).unwrap().layout,
        &visible_layout
    ));
    assert!(Arc::ptr_eq(
        &resources
            .text_layout_resource(ellipsized_id)
            .unwrap()
            .layout,
        &ellipsized_layout
    ));
}

#[test]
fn incremental_add_and_remove_follow_final_store_presence() {
    let mut store = TextLayoutStore::new();
    let hot = store.layout_id(key("hot"));
    let cold = store.layout_id(key("cold"));
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&store, &mut sync);

    let added = store.layout_id(key("added"));
    assert_eq!(
        resources.reconcile_text_layouts(&store, &mut sync),
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Incremental,
            processed_changes: 1,
            added: 1,
            updated: 0,
            removed: 0,
            retained: 3,
        }
    );

    for _ in 0..121 {
        store.advance_generation();
        assert!(store.touch_layout(hot));
        assert!(store.touch_layout(added));
    }
    assert!(store.layout(cold).is_none());
    assert_eq!(
        resources.reconcile_text_layouts(&store, &mut sync),
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Incremental,
            processed_changes: 1,
            added: 0,
            updated: 0,
            removed: 1,
            retained: 2,
        }
    );
    assert_eq!(text_ids(&resources), store_ids(&store));
}

#[test]
fn duplicate_dirty_id_records_use_final_absence_and_are_not_noop() {
    let mut store = TextLayoutStore::new();
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&store, &mut sync);

    let transient = store.layout_id(key("transient"));
    for _ in 0..121 {
        store.advance_generation();
    }
    assert!(store.layout(transient).is_none());

    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.kind, TextLayoutResourceSyncKind::Incremental);
    assert_eq!(report.processed_changes, 2, "addition then removal");
    assert_eq!((report.added, report.updated, report.removed), (0, 0, 0));
    assert_eq!(report.retained, 0);
    assert!(!report.is_noop(), "dirty-but-idempotent is not a no-op");
}

#[test]
fn store_and_manual_resets_rebuild_only_text_and_preserve_static_arcs() {
    let mut store = TextLayoutStore::new();
    let first = store.layout_id(key("first"));
    let mut resources = static_resources();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&store, &mut sync);
    let image = resources
        .image(ImageId::from_raw(7))
        .and_then(|resource| resource.pixels.as_ref())
        .map(|image| Arc::clone(&image.data))
        .expect("image payload");
    let texture = resources
        .texture(TextureId::from_raw(9))
        .and_then(|resource| resource.snapshot.as_ref())
        .map(|image| Arc::clone(&image.data))
        .expect("texture payload");

    store.clear();
    let replacement = store.layout_id(key("replacement"));
    assert_eq!(
        resources.reconcile_text_layouts(&store, &mut sync),
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Full,
            processed_changes: 0,
            added: 1,
            updated: 0,
            removed: 1,
            retained: 1,
        }
    );
    assert!(!resources.has_text_layout(first));
    assert!(resources.has_text_layout(replacement));
    assert!(Arc::ptr_eq(
        &image,
        &resources
            .image(ImageId::from_raw(7))
            .unwrap()
            .pixels
            .as_ref()
            .unwrap()
            .data
    ));
    assert!(Arc::ptr_eq(
        &texture,
        &resources
            .texture(TextureId::from_raw(9))
            .unwrap()
            .snapshot
            .as_ref()
            .unwrap()
            .data
    ));

    sync.reset();
    assert!(!sync.is_initialized());
    assert_eq!(
        resources.reconcile_text_layouts(&store, &mut sync),
        TextLayoutResourceSyncReport {
            kind: TextLayoutResourceSyncKind::Full,
            processed_changes: 0,
            added: 1,
            updated: 0,
            removed: 1,
            retained: 1,
        }
    );
    assert!(sync.is_initialized());
    let resource = resources.text_layout_resource(replacement).unwrap();
    let text_pointer = resource.key.text.as_ptr();
    let layout = Arc::clone(&resource.layout);
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert!(report.is_noop());
    let resource = resources.text_layout_resource(replacement).unwrap();
    assert_eq!(resource.key.text.as_ptr(), text_pointer);
    assert!(Arc::ptr_eq(&resource.layout, &layout));
}

#[test]
fn foreign_store_and_delayed_consumers_force_or_apply_the_right_path() {
    let mut first_store = TextLayoutStore::new();
    let first = first_store.layout_id(key("first store"));
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&first_store, &mut sync);

    let mut second_store = TextLayoutStore::new();
    let second = second_store.layout_id(key("second store"));
    let report = resources.reconcile_text_layouts(&second_store, &mut sync);
    assert_eq!(report.kind, TextLayoutResourceSyncKind::Full);
    assert_eq!((report.removed, report.added, report.retained), (1, 1, 1));
    assert!(!resources.has_text_layout(first));
    assert!(resources.has_text_layout(second));

    let mut current_resources = RenderResources::new();
    let mut delayed_resources = RenderResources::new();
    let mut current_sync = TextLayoutResourceSync::new();
    let mut delayed_sync = TextLayoutResourceSync::new();
    let _ = current_resources.reconcile_text_layouts(&second_store, &mut current_sync);
    let _ = delayed_resources.reconcile_text_layouts(&second_store, &mut delayed_sync);
    second_store.layout_id(key("third"));
    let current_report = current_resources.reconcile_text_layouts(&second_store, &mut current_sync);
    assert_eq!(current_report.processed_changes, 1);
    second_store.layout_id(key("fourth"));
    let delayed_report = delayed_resources.reconcile_text_layouts(&second_store, &mut delayed_sync);
    assert_eq!(delayed_report.processed_changes, 2);
    let _ = current_resources.reconcile_text_layouts(&second_store, &mut current_sync);
    assert_eq!(current_resources.snapshot(), delayed_resources.snapshot());
}

#[test]
fn resource_arc_lifetime_ends_on_reconciliation_but_external_owner_survives() {
    let mut store = TextLayoutStore::new();
    let resource_only = store.layout_id(key("resource only"));
    let externally_owned = store.layout_id(key("externally owned"));
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&store, &mut sync);
    let resource_only_weak = Arc::downgrade(
        &resources
            .text_layout_resource(resource_only)
            .expect("resource")
            .layout,
    );
    let external = Arc::clone(
        &resources
            .text_layout_resource(externally_owned)
            .expect("resource")
            .layout,
    );
    let external_weak = Arc::downgrade(&external);

    for _ in 0..121 {
        store.advance_generation();
    }
    assert!(resource_only_weak.upgrade().is_some());
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.removed, 2);
    assert!(resource_only_weak.upgrade().is_none());
    assert!(external_weak.upgrade().is_some());
    drop(external);
    assert!(external_weak.upgrade().is_none());
}

#[test]
fn one_thousand_dynamic_generations_keep_registry_equal_and_bounded() {
    let mut store = TextLayoutStore::new();
    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let _ = resources.reconcile_text_layouts(&store, &mut sync);

    for generation in 0..1_000 {
        store.advance_generation();
        let text = format!("dynamic-{generation}-e\u{301}-😀");
        assert!(store.try_layout_id(key(text)).is_some());
        let _ = resources.reconcile_text_layouts(&store, &mut sync);
        assert_eq!(text_ids(&resources), store_ids(&store));
        assert_eq!(
            resources.retained_text_layout_payload_bytes(),
            Some(store.retained_payload_bytes())
        );
        assert!(store.retained_payload_bytes() <= MAX_TEXT_PAYLOAD_BYTES);
    }
    assert!(resources.text_layout_count() <= 121);
}

#[test]
fn same_generation_saturation_exports_every_accepted_id_and_no_rejection() {
    let mut store = TextLayoutStore::new();
    let mut accepted = Vec::new();
    let mut rejected = 0;
    for index in 0..72 {
        let request = TextLayoutKey::new(
            format!("saturation-{index}"),
            TextStyle::new(
                format!("saturation-{index}-{}", "x".repeat(512 * 1024)),
                12.0,
                16.0,
            ),
            120.0,
            false,
        );
        if let Some(id) = store.try_layout_id(request) {
            accepted.push(id);
        } else {
            rejected += 1;
        }
    }
    assert!(!accepted.is_empty());
    assert!(rejected > 0);
    assert!(accepted.iter().all(|id| id.raw() != 0));

    let mut resources = RenderResources::new();
    let mut sync = TextLayoutResourceSync::new();
    let report = resources.reconcile_text_layouts(&store, &mut sync);
    assert_eq!(report.added, accepted.len());
    assert_eq!(report.retained, accepted.len());
    assert!(accepted.iter().all(|id| resources.has_text_layout(*id)));
    assert_eq!(resources.text_layout_count(), store.len());
    assert_eq!(
        resources.retained_text_layout_payload_bytes(),
        Some(store.retained_payload_bytes())
    );
    assert!(store.retained_payload_bytes() <= MAX_TEXT_PAYLOAD_BYTES);
    assert!(!resources.has_text_layout(TextLayoutId::from_raw(0)));
}
