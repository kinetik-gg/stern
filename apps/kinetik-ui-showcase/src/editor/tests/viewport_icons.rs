#[test]
fn viewport_selection_overlay_uses_scaled_content_mapping() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let viewport_frame = editor_frame_rect(&editor, FRAME_VIEWPORT);
    let viewport_body = frame_body_rect(viewport_frame);
    let surface_bounds = Rect::new(
        viewport_body.x + 8.0,
        viewport_body.y + 36.0,
        (viewport_body.width - 16.0).max(1.0),
        (viewport_body.height - 66.0).max(1.0),
    );
    let surface = ViewportSurface {
        texture: super::VIEWPORT_TEXTURE,
        source_size: VIEWPORT_SIZE,
        bounds: surface_bounds,
        pan_zoom: editor.viewport_pan_zoom,
    };
    let scale = ScaleFactor::new(1.25);
    let expected = surface
        .content_rect_to_screen_at(Rect::new(720.0, 210.0, 210.0, 280.0), scale)
        .expect("selection rect");

    let mut ui = Ui::begin_frame(
        editor_test_context_scaled(UiInput::default(), scale),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let selection_fill = rgba(78, 142, 245, 0.12);
    let selection = output
            .primitives
            .iter()
            .find_map(|primitive| match primitive {
                Primitive::Rect(rect)
                    if matches!(&rect.fill, Some(Brush::Solid(color)) if *color == selection_fill) =>
                {
                    Some(rect.rect)
                }
                _ => None,
            })
            .expect("selection overlay rect");

    assert_eq!(selection, expected);
    let physical_x = f64::from(selection.x) * scale.value();
    let physical_width = f64::from(selection.width) * scale.value();
    assert!((physical_x - physical_x.round()).abs() < 0.001);
    assert!((physical_width - physical_width.round()).abs() < 0.001);
}

#[test]
fn scene_expander_flips_arrow_and_requests_repaint_same_frame() {
    let mut editor = EditorShowcase::new();
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();
    let expander = Point::new(38.0, super::workspace_top(&theme) + 100.0);

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(expander.x, expander.y, true, true, false)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let _ = ui.finish_output();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(expander.x, expander.y, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(output.repaint, RepaintRequest::NextFrame);
    assert!(!editor.scene_expansion.is_expanded(item_id(2)));
    assert!(
        output
            .primitives
            .iter()
            .any(|primitive| { matches!(primitive, Primitive::Text(text) if text.text == ">") })
    );
}

#[test]
fn outside_click_dismisses_menu_and_requests_repaint() {
    let mut editor = EditorShowcase::new();
    editor.open_menu = Some(EditorMenuKind::File);
    let mut memory = UiMemory::new();
    let theme = default_dark_theme();

    let mut ui = Ui::begin_frame(
        editor_test_context(pointer_input_at(900.0, 700.0, false, false, true)),
        &mut memory,
        &theme,
    );
    editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert_eq!(editor.open_menu, None);
    assert_eq!(output.repaint, RepaintRequest::NextFrame);
}

#[test]
fn icon_atlas_duplicates_edge_pixels_into_gutters() {
    let first = phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE)
        .expect("standard icon entry");
    let atlas = icon_atlas_image(first.physical_size).expect("atlas");
    let source = first.source;
    let left_gutter = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.x as u32 - phosphor_icons::ICON_ATLAS_PADDING,
        source.y as u32,
    );
    let first_inner = atlas_pixel(&atlas.data, atlas.width, source.x as u32, source.y as u32);
    let bottom_gutter = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.max_x() as u32,
        source.max_y() as u32,
    );
    let bottom_inner = atlas_pixel(
        &atlas.data,
        atlas.width,
        source.max_x() as u32 - 1,
        source.max_y() as u32 - 1,
    );
    let atlas_entry = phosphor_icons::ICON_ATLASES
        .iter()
        .find(|atlas| atlas.image == first.atlas)
        .expect("atlas entry");

    assert_eq!(atlas.width, atlas_entry.width);
    assert_eq!(atlas.height, atlas_entry.height);
    assert_eq!(left_gutter, first_inner);
    assert_eq!(bottom_gutter, bottom_inner);
}

#[test]
fn icon_manifest_entries_register_as_atlas_regions() {
    let mut resources = RenderResources::new();

    register_resources(&mut resources);

    for entry in phosphor_icons::ICON_ENTRIES {
        let resource = resources.image(entry.image).expect(entry.symbol);
        let region = resource.atlas_region.expect("icon atlas region");

        assert_eq!(
            resource.size,
            Size::new(entry.logical_size as f32, entry.logical_size as f32)
        );
        assert_eq!(
            resource.sampling,
            kinetik_ui::render::RenderImageSampling::UiIcon
        );
        assert_eq!(region.atlas, entry.atlas);
        assert_eq!(region.source, entry.source, "{}", entry.source_name);
    }
}

#[test]
fn icon_atlas_regions_target_inner_unpadded_cells() {
    let mut resources = RenderResources::new();

    register_resources(&mut resources);

    let entry = phosphor_icons::ICON_ENTRIES
        .iter()
        .find(|entry| {
            entry.icon == phosphor_icons::PhosphorIcon::Crosshair
                && entry.logical_size == phosphor_icons::STANDARD_ICON_LOGICAL_SIZE
                && entry.physical_size == 24
        })
        .expect("crosshair entry");
    let region = resources
        .image(entry.image)
        .and_then(|resource| resource.atlas_region)
        .expect("icon region");

    assert_eq!(region.source.width, entry.physical_size as f32);
    assert_eq!(region.source.height, entry.physical_size as f32);
    assert_eq!(region.source, entry.source);
    assert_eq!(entry.source_name, "crosshair");
}

#[test]
fn editor_structural_smoke_emits_dock_frame_panel_viewport_and_action_categories() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    let invocations = editor.render(&mut ui, 0);
    let output = ui.finish_output();

    assert!(invocations.is_empty());
    assert_eq!(output.warnings, Vec::new());
    assert!(output.primitives.len() > 200);
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Rect(_)
        )) > 100
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Text(_)
        )) > 50
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Image(_)
        )) >= 24
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Texture(_)
        )) >= 1
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::Line(_)
        )) >= 8
    );
    assert!(
        count_primitives(&output.primitives, |primitive| matches!(
            primitive,
            Primitive::ClipBegin { .. }
        )) >= 2
    );
    assert!(output.primitives.iter().any(|primitive| {
            matches!(primitive, Primitive::Texture(texture) if texture.texture == super::VIEWPORT_TEXTURE)
        }));
    assert!(output.primitives.iter().any(|primitive| {
        matches!(primitive, Primitive::Text(text) if text.text == "CameraPreview")
    }));

    assert_eq!(count_semantic_role(&output, &SemanticRole::Dock), 1);
    assert!(count_semantic_role(&output, &SemanticRole::Frame) >= 5);
    assert!(count_semantic_role(&output, &SemanticRole::Panel) >= 5);
    assert!(count_semantic_role(&output, &SemanticRole::Viewport) >= 1);
    assert!(count_semantic_role(&output, &SemanticRole::Tab) >= 6);
    assert!(count_semantic_role(&output, &SemanticRole::IconButton) >= 12);
    assert!(count_semantic_role(&output, &SemanticRole::Slider) >= 1);
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::IconButton
            && node.label.as_deref() == Some("Play")
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::Invoke)
    }));
    assert!(output.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Slider
            && node
                .actions
                .iter()
                .any(|action| action.kind == SemanticActionKind::SetValue)
    }));
}

#[test]
fn editor_uses_phosphor_atlas_primitives_for_visible_editor_icons() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let atlas_icon_count = output
        .primitives
        .iter()
        .filter(
            |primitive| matches!(primitive, Primitive::Image(image) if is_editor_icon(image.image)),
        )
        .count();

    assert!(
        atlas_icon_count >= 24,
        "visible Phosphor icon count was {atlas_icon_count}"
    );
}

#[test]
fn editor_toolbar_icons_use_tinted_bitmap_atlas() {
    let theme = default_dark_theme();
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let toolbar_bitmap_icons = output
        .primitives
        .iter()
        .filter_map(|primitive| match primitive {
            Primitive::Image(image)
                if is_editor_icon(image.image) && point_is_in_toolbar(image.rect.center()) =>
            {
                Some(image)
            }
            _ => None,
        })
        .collect::<Vec<_>>();

    assert!(toolbar_bitmap_icons.len() >= 12);
    assert!(
        toolbar_bitmap_icons
            .iter()
            .all(|image| image.tint.is_some())
    );
}

#[test]
fn editor_toolbar_atlas_icons_use_integer_logical_destinations() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let mut memory = UiMemory::new();
    let context = editor_test_context_scaled(UiInput::default(), ScaleFactor::new(1.25));
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let mut checked = 0;

    for primitive in &output.primitives {
        let Primitive::Image(image) = primitive else {
            continue;
        };
        if !is_editor_icon(image.image) || !point_is_in_toolbar(image.rect.center()) {
            continue;
        }
        assert_eq!(image.rect.x, image.rect.x.round());
        assert_eq!(image.rect.y, image.rect.y.round());
        assert_eq!(image.rect.width, chrome.toolbar_icon);
        assert_eq!(image.rect.height, chrome.toolbar_icon);
        checked += 1;
    }

    assert!(checked >= 12);
}

#[test]
fn editor_icons_pick_exact_physical_atlas_for_dpi_scale() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let dense = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Search,
        super::DENSE_ICON_SIZE,
        1.25,
    );
    let toolbar = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Cursor,
        chrome.toolbar_icon,
        1.5,
    );
    let dense_entry = icon_entry(dense);
    let toolbar_entry = icon_entry(toolbar);

    assert_eq!(dense_entry.logical_size, 16);
    assert_eq!(dense_entry.physical_size, 20);
    assert_eq!(toolbar_entry.logical_size, 16);
    assert_eq!(toolbar_entry.physical_size, 24);

    let fallback = phosphor_icons::icon_image(
        phosphor_icons::PhosphorIcon::Search,
        super::DENSE_ICON_SIZE,
        1.33,
    );
    assert_eq!(icon_entry(fallback).physical_size, 24);
}

#[test]
fn toolbar_icon_size_leaves_padding_inside_button() {
    let theme = default_dark_theme();
    let chrome = EditorChromeMetrics::from_theme(&theme);
    let mut memory = UiMemory::new();
    let context = editor_test_context(UiInput::default());
    let mut ui = Ui::begin_frame(context, &mut memory, &theme);
    let mut editor = EditorShowcase::new();
    let first_button = Rect::new(
        10.0,
        TOOLBAR_Y,
        chrome.toolbar_button,
        chrome.toolbar_button,
    );

    editor.render(&mut ui, 0);
    let output = ui.finish_output();
    let first_icon = output
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Image(image)
                if is_editor_icon(image.image)
                    && first_button.contains_point(image.rect.center()) =>
            {
                Some(image)
            }
            _ => None,
        })
        .expect("first toolbar icon");

    assert!(first_icon.rect.x >= first_button.x + 4.0);
    assert!(first_icon.rect.max_x() <= first_button.max_x() - 4.0);
    assert!(first_icon.rect.y >= first_button.y + 4.0);
    assert!(first_icon.rect.max_y() <= first_button.max_y() - 4.0);
}
