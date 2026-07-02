use super::common::text_layout_resource;
use crate::{
    MAX_CACHED_TEXT_LAYOUTS, RenderFrameInput, RenderResources, ShapedTextCache, VelloRenderer,
};
use kinetik_ui_core::{
    Brush, Color, Point, Primitive, ScaleFactor, Size, TextLayoutId, TextPrimitive, ViewportInfo,
};
use kinetik_ui_text::{
    CosmicTextEngine, ShapedTextLayout, TextLayoutKey, TextLayoutStore, TextStyle,
};

#[test]
fn repeated_registered_text_reuses_cached_physical_layout() {
    let layout = TextLayoutId::from_raw(48);
    let mut resources = RenderResources::new();
    resources.register_text_layout(text_layout_resource(layout, "Label"));
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: Some(layout),
        origin: Point::new(4.0, 16.0),
        text: "Label".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let viewport = ViewportInfo::new(
        Size::new(100.0, 100.0),
        kinetik_ui_core::PhysicalSize::new(125, 125),
        ScaleFactor::new(1.25),
    );
    let mut renderer = VelloRenderer::new();

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn shaped_text_cache_evicts_least_recent_entry_at_capacity() {
    let first = TextLayoutKey::new(
        "layout 1",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    );
    let second = TextLayoutKey::new(
        "layout 2",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    );
    let dummy_layout = std::sync::Arc::new(ShapedTextLayout {
        size: Size::new(0.0, 0.0),
        line_count: 0,
        lines: Vec::new(),
        runs: Vec::new(),
    });
    let mut cache = ShapedTextCache::default();

    for index in 1..=MAX_CACHED_TEXT_LAYOUTS {
        let key = TextLayoutKey::new(
            format!("layout {index}"),
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        );
        cache.layout_order.push_back(key.clone());
        cache
            .layouts
            .insert(key, std::sync::Arc::clone(&dummy_layout));
    }

    let mut engine = CosmicTextEngine::new();
    cache.layout(&mut engine, first.clone());
    cache.layout(
        &mut engine,
        TextLayoutKey::new(
            "layout overflow",
            TextStyle::new("sans-serif", 12.0, 16.0),
            200.0,
            false,
        ),
    );

    assert_eq!(cache.layouts.len(), MAX_CACHED_TEXT_LAYOUTS);
    assert!(cache.layouts.contains_key(&first));
    assert!(!cache.layouts.contains_key(&second));
}

#[test]
fn repeated_fallback_text_reuses_cached_physical_layout() {
    let resources = RenderResources::new();
    let primitives = vec![Primitive::Text(TextPrimitive {
        layout: None,
        origin: Point::new(4.0, 16.0),
        text: "Fallback".to_owned(),
        family: "sans-serif".to_owned(),
        size: 12.0,
        line_height: 16.0,
        brush: Brush::Solid(Color::WHITE),
    })];
    let viewport = ViewportInfo::new(
        Size::new(100.0, 100.0),
        kinetik_ui_core::PhysicalSize::new(125, 125),
        ScaleFactor::new(1.25),
    );
    let mut renderer = VelloRenderer::new();

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);

    renderer.submit_frame(RenderFrameInput {
        viewport,
        primitives: &primitives,
        resources: &resources,
    });
    assert_eq!(renderer.cached_text_layout_count(), 1);
}

#[test]
fn render_resources_register_text_layout_store_entries() {
    let mut store = TextLayoutStore::new();
    let id = store.layout_id(TextLayoutKey::new(
        "Label",
        TextStyle::new("sans-serif", 12.0, 16.0),
        200.0,
        false,
    ));
    let mut resources = RenderResources::new();

    resources.register_text_layouts(store.layouts());

    assert!(resources.has_text_layout(id));
    assert_eq!(
        resources.text_layout(id).map(ShapedTextLayout::glyph_count),
        store.layout(id).map(ShapedTextLayout::glyph_count)
    );
}
