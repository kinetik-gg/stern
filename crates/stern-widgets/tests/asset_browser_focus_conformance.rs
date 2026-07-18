//! Asset-browser inward focus ownership and composition conformance tests.

#![allow(clippy::float_cmp)]

use std::time::Duration;

use stern_core::{
    ActionContext, ActionDescriptor, ActionId, ActionInvocation, ActionSource, Brush, Color,
    ComponentState, FrameContext, ImageId, Key, KeyEvent, KeyState, KeyboardInput, Modifiers,
    PathElement, PhysicalSize, Point, PointerButtonState, PointerInput, PointerOrder, Primitive,
    Rect, RepaintRequest, ScaleFactor, SemanticActionKind, SemanticNode, Size, TimeInfo, UiInput,
    UiMemory, Vec2, ViewportInfo, WidgetId, default_dark_theme,
};
use stern_widgets::asset_browser::{
    AssetBrowserConfig, AssetBrowserContextMenuConfig, AssetBrowserDropTargetKind,
    AssetBrowserItem, AssetBrowserItemRect, AssetBrowserLayout, AssetBrowserModel,
    AssetBrowserOutput, AssetBrowserRequest, AssetBrowserSort, AssetBrowserSortKey,
    AssetBrowserState, AssetBrowserViewMode, AssetIconFallback,
};
use stern_widgets::{
    CollectionContextActionRequest, CollectionContextTarget, GridColumns, GridLayout,
    InlineEditCancelReason, InlineEditCommitReason, InlineEditRequest, ItemId, ListLayout,
    SortDirection, Ui,
};

const BOUNDS: Rect = Rect::new(10.25, 20.5, 240.0, 112.0);

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn asset(raw: u64, name: impl Into<String>, kind: impl Into<String>) -> AssetBrowserItem {
    AssetBrowserItem::new(id(raw), name, kind)
}

fn layout(view_mode: AssetBrowserViewMode) -> AssetBrowserLayout {
    AssetBrowserLayout::new(
        view_mode,
        GridLayout {
            columns: GridColumns::Fixed(3),
            item_size: Size::new(72.0, 72.0),
            gap: 4.0,
        },
        ListLayout::new(28.0),
    )
    .with_overscan(1)
}

fn config(view_mode: AssetBrowserViewMode) -> AssetBrowserConfig {
    AssetBrowserConfig::new(BOUNDS, layout(view_mode))
        .label("Project assets")
        .selection_mode(stern_widgets::asset_browser::AssetBrowserSelectionMode::Multiple)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 240.0),
            PhysicalSize::new(320, 240),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(500), Duration::from_millis(16), 1),
    )
}

fn pointer_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            click_count: u8::from(released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn secondary_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            secondary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn move_input(point: Point, delta: Vec2) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            delta,
            primary: PointerButtonState::new(true, false, false),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn key_input(key: Key) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![KeyEvent::new(
                key,
                KeyState::Pressed,
                Modifiers::default(),
                false,
            )],
        },
        ..UiInput::default()
    }
}

fn typed_input(text: &str) -> UiInput {
    let event = KeyEvent::new(
        Key::Character(text.to_owned()),
        KeyState::Pressed,
        Modifiers::default(),
        false,
    )
    .with_text(text);
    UiInput {
        keyboard: KeyboardInput {
            modifiers: Modifiers::default(),
            events: vec![event],
        },
        ..UiInput::default()
    }
}

#[derive(Debug)]
struct Run {
    root: WidgetId,
    outside: WidgetId,
    items: Vec<AssetBrowserItemRect>,
    projected: Vec<ItemId>,
    output: AssetBrowserOutput,
    frame: stern_core::FrameOutput,
}

fn run_frame(
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
    input: UiInput,
) -> Run {
    run_frame_with_options(model, config, state, memory, input, false, false)
}

#[allow(clippy::too_many_arguments)]
fn run_frame_with_options(
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
    input: UiInput,
    reject_rename: bool,
    context_actions: bool,
) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let scene = ui
        .prepare_asset_browser("focus-assets", config, model, state)
        .expect("valid asset browser scene");
    let root = scene.widget_id();
    let outside = ui.make_id("outside-focus");
    ui.register_id(outside);
    let items = scene.layout().items.clone();
    let projected = scene.projection().visible_ids();
    ui.resolve_pointer_targets(|plan| {
        scene.declare_pointer_targets(plan, PointerOrder::new(100), state);
    })
    .expect("valid shared pointer plan");
    let output = ui.asset_browser(
        &scene,
        state,
        |_target, _draft| reject_rename.then(|| "name already exists".to_owned()),
        |target| {
            if !context_actions {
                return Vec::new();
            }
            match target {
                CollectionContextTarget::Background(_) => {
                    vec![ActionDescriptor::new("asset.create", "Create")]
                }
                CollectionContextTarget::Item(_) => {
                    vec![ActionDescriptor::new("asset.inspect", "Inspect")]
                }
                CollectionContextTarget::Selection(_) => {
                    vec![ActionDescriptor::new("asset.delete", "Delete")]
                }
            }
        },
    );
    let frame = ui.finish_output();
    Run {
        root,
        outside,
        items,
        projected,
        output,
        frame,
    }
}

fn context_click(
    point: Point,
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame_with_options(
        model,
        config.clone(),
        state,
        memory,
        secondary_input(point, true, true, false),
        false,
        true,
    );
    run_frame_with_options(
        model,
        config,
        state,
        memory,
        secondary_input(point, false, false, true),
        false,
        true,
    )
}

fn click(
    point: Point,
    model: &AssetBrowserModel,
    config: AssetBrowserConfig,
    state: &mut AssetBrowserState,
    memory: &mut UiMemory,
) -> Run {
    let _ = run_frame(
        model,
        config.clone(),
        state,
        memory,
        pointer_input(point, true, true, false),
    );
    run_frame(
        model,
        config,
        state,
        memory,
        pointer_input(point, false, false, true),
    )
}

fn path_bounds(elements: &[PathElement]) -> Rect {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for point in elements.iter().flat_map(|element| match *element {
        PathElement::MoveTo(point) | PathElement::LineTo(point) => vec![point],
        PathElement::QuadTo { ctrl, to } => vec![ctrl, to],
        PathElement::CubicTo { ctrl1, ctrl2, to } => vec![ctrl1, ctrl2, to],
        PathElement::Close => Vec::new(),
    }) {
        min_x = min_x.min(point.x);
        min_y = min_y.min(point.y);
        max_x = max_x.max(point.x);
        max_y = max_y.max(point.y);
    }
    Rect::new(min_x, min_y, max_x - min_x, max_y - min_y)
}

fn item_response(run: &Run, target: ItemId) -> stern_core::Response {
    run.output
        .responses
        .iter()
        .find(|response| response.item == target)
        .unwrap_or_else(|| panic!("missing response for item {}", target.raw()))
        .response
}

fn item_rect(run: &Run, target: ItemId) -> &AssetBrowserItemRect {
    run.items
        .iter()
        .find(|item| item.item.id == target)
        .unwrap_or_else(|| panic!("missing geometry for item {}", target.raw()))
}

fn item_geometry(run: &Run) -> Vec<(ItemId, Rect, Rect, Rect, Rect)> {
    run.items
        .iter()
        .map(|item| {
            (
                item.item.id,
                item.rect,
                item.preview_rect,
                item.name_rect,
                item.kind_rect,
            )
        })
        .collect()
}

fn assert_item_focus(run: &Run, target: ItemId) -> [Primitive; 2] {
    let theme = default_dark_theme();
    let item = item_rect(run, target);
    let response = item_response(run, target);
    assert!(response.state.focused);
    assert!(!response.state.disabled);
    let state = ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: response.state.disabled,
        selected: response.state.selected,
    };
    let recipe = theme.row(state);
    let base_index = run
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == item.rect))
        .expect("asset item base");
    let Primitive::Rect(base) = &run.frame.primitives[base_index] else {
        unreachable!()
    };
    assert_eq!(base.fill, Some(recipe.background));
    assert_eq!(base.stroke, Some(recipe.border));
    assert_eq!(base.radius, recipe.radius);

    let expected = theme
        .focus_ring(true)
        .expect("focus recipe")
        .inward_annulus_primitives(item.rect, recipe.radius, recipe.border.width);
    assert_eq!(run.frame.primitives[base_index + 1], expected[0]);
    assert_eq!(run.frame.primitives[base_index + 2], expected[1]);
    for primitive in &run.frame.primitives[base_index + 1..=base_index + 2] {
        let Primitive::Path(path) = primitive else {
            panic!("asset focus must remain a compound path");
        };
        assert_eq!(path.elements.len(), 20);
        assert_eq!(path.stroke, None);
        let bounds = path_bounds(&path.elements);
        assert!(
            [
                bounds.x,
                bounds.y,
                bounds.width,
                bounds.height,
                bounds.max_x(),
                bounds.max_y(),
            ]
            .into_iter()
            .all(f32::is_finite)
        );
        assert!(item.rect.contains_rect(bounds));
    }

    assert!(matches!(
        run.frame.primitives[base_index + 3],
        Primitive::Rect(preview) if preview.rect == item.preview_rect
    ));
    let content_index = base_index + 4;
    if item.item.thumbnail.is_some() {
        assert!(matches!(
            run.frame.primitives[content_index],
            Primitive::Image(image) if image.rect == item.preview_rect
        ));
    } else {
        assert!(matches!(
            run.frame.primitives[content_index],
            Primitive::Text(ref text) if text.text == item.item.fallback.label
        ));
    }
    assert!(matches!(
        run.frame.primitives[content_index + 1],
        Primitive::Text(ref text) if text.text == item.item.name
    ));
    assert!(matches!(
        run.frame.primitives[content_index + 2],
        Primitive::Text(ref text) if text.text == item.item.kind
    ));

    [
        run.frame.primitives[base_index + 1].clone(),
        run.frame.primitives[base_index + 2].clone(),
    ]
}

fn primitives_without_focus_paths(run: &Run) -> Vec<Primitive> {
    run.frame
        .primitives
        .iter()
        .filter(|primitive| !matches!(primitive, Primitive::Path(_)))
        .cloned()
        .collect()
}

fn output_without_focus(mut output: AssetBrowserOutput) -> AssetBrowserOutput {
    for response in &mut output.responses {
        response.response.state.focused = false;
    }
    output
}

fn semantics_without_focus(run: &Run) -> Vec<SemanticNode> {
    run.frame
        .semantics
        .nodes()
        .iter()
        .cloned()
        .map(|mut node| {
            node.state.focused = false;
            node
        })
        .collect()
}

fn assert_focus_only_transition(focused: &Run, unfocused: &Run) {
    assert_eq!(focused.items, unfocused.items);
    assert_eq!(focused.projected, unfocused.projected);
    assert_eq!(focused.frame.repaint, unfocused.frame.repaint);
    assert_eq!(
        output_without_focus(focused.output.clone()),
        unfocused.output
    );
    assert_eq!(
        primitives_without_focus_paths(focused),
        unfocused.frame.primitives
    );
    assert_eq!(
        semantics_without_focus(focused),
        unfocused.frame.semantics.nodes()
    );
    assert_eq!(
        focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        2
    );
    assert!(
        focused
            .frame
            .primitives
            .iter()
            .all(|primitive| !matches!(primitive, Primitive::TransformBegin { .. }))
    );
}

#[test]
fn grid_and_list_thumbnail_fallback_selected_pairs_add_only_exact_owned_annuli() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "Thumbnail", "image").with_thumbnail(ImageId::from_raw(77)),
        asset(2, "Fallback", "material").with_fallback(AssetIconFallback::new("material", "MAT")),
    ]);

    for view_mode in [AssetBrowserViewMode::Grid, AssetBrowserViewMode::List] {
        let seed = run_frame(
            &model,
            config(view_mode),
            &mut AssetBrowserState::new(),
            &mut UiMemory::new(),
            UiInput::default(),
        );
        for target in [id(1), id(2)] {
            for selected in [false, true] {
                let mut unfocused_state = AssetBrowserState::new();
                if selected {
                    unfocused_state.selection.replace(target);
                }
                let unfocused = run_frame(
                    &model,
                    config(view_mode),
                    &mut unfocused_state,
                    &mut UiMemory::new(),
                    UiInput::default(),
                );

                let mut focused_state = AssetBrowserState::new();
                if selected {
                    focused_state.selection.replace(target);
                }
                let mut focused_memory = UiMemory::new();
                focused_memory.focus(seed.root.child(("asset-browser-item", target.raw())));
                let focused = run_frame(
                    &model,
                    config(view_mode),
                    &mut focused_state,
                    &mut focused_memory,
                    UiInput::default(),
                );

                assert_focus_only_transition(&focused, &unfocused);
                assert_eq!(item_response(&focused, target).state.selected, selected);
                assert_item_focus(&focused, target);
                let semantic = focused
                    .frame
                    .semantics
                    .get(seed.root.child(("asset-browser-item", target.raw())))
                    .expect("focused asset semantic");
                assert!(semantic.state.focused);
                assert_eq!(semantic.state.selected, selected);
                assert_eq!(semantic.bounds, item_rect(&focused, target).rect);
            }
        }
    }
}

#[test]
fn hover_press_selection_and_focus_combinations_preserve_the_exact_annuli() {
    let model = AssetBrowserModel::new(vec![asset(1, "State target", "mesh")]);

    for view_mode in [AssetBrowserViewMode::Grid, AssetBrowserViewMode::List] {
        let seed = run_frame(
            &model,
            config(view_mode),
            &mut AssetBrowserState::new(),
            &mut UiMemory::new(),
            UiInput::default(),
        );
        let point = seed.items[0].rect.center();
        let target_widget = seed.root.child(("asset-browser-item", 1_u64));
        let mut baseline = None;
        for (hovered, pressed, selected) in [
            (false, false, false),
            (true, false, false),
            (false, true, false),
            (false, false, true),
            (true, false, true),
            (false, true, true),
        ] {
            let input = if pressed {
                pointer_input(point, true, true, false)
            } else if hovered {
                pointer_input(point, false, false, false)
            } else {
                UiInput::default()
            };
            let mut unfocused_state = AssetBrowserState::new();
            if selected {
                unfocused_state.selection.replace(id(1));
            }
            let unfocused = run_frame(
                &model,
                config(view_mode),
                &mut unfocused_state,
                &mut UiMemory::new(),
                input.clone(),
            );

            let mut focused_state = AssetBrowserState::new();
            if selected {
                focused_state.selection.replace(id(1));
            }
            let mut focused_memory = UiMemory::new();
            focused_memory.focus(target_widget);
            let focused = run_frame(
                &model,
                config(view_mode),
                &mut focused_state,
                &mut focused_memory,
                input,
            );

            assert_focus_only_transition(&focused, &unfocused);
            let response = item_response(&focused, id(1));
            assert_eq!(response.state.hovered, hovered || pressed);
            assert_eq!(response.state.pressed, pressed);
            assert_eq!(response.state.selected, selected);
            let annuli = assert_item_focus(&focused, id(1));
            if let Some(baseline) = &baseline {
                assert_eq!(&annuli, baseline);
            } else {
                baseline = Some(annuli);
            }
        }
    }
}

#[test]
fn stable_id_focus_cursor_and_selection_survive_filter_sort_reorder_scroll_and_view_changes() {
    let original = AssetBrowserModel::new(vec![
        asset(1, "Zeta", "scene"),
        asset(2, "Alpha", "mesh"),
        asset(3, "Beta", "material"),
        asset(4, "Gamma", "image"),
        asset(5, "Delta", "mesh"),
        asset(6, "Epsilon", "audio"),
    ]);
    let sorted = config(AssetBrowserViewMode::List).sort(Some(AssetBrowserSort::new(
        AssetBrowserSortKey::Name,
        SortDirection::Ascending,
    )));
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let seed = run_frame(
        &original,
        sorted.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(
        seed.projected,
        vec![id(2), id(3), id(5), id(6), id(4), id(1)]
    );
    let target_point = item_rect(&seed, id(3)).rect.center();
    let selected = click(target_point, &original, sorted, &mut state, &mut memory);
    let target_widget = selected.root.child(("asset-browser-item", 3_u64));
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert_eq!(state.selection.selected(), vec![id(3)]);
    assert!(memory.is_focused(target_widget));
    assert_item_focus(&selected, id(3));

    let filtered_grid = run_frame(
        &original,
        config(AssetBrowserViewMode::Grid)
            .query("a")
            .sort(Some(AssetBrowserSort::new(
                AssetBrowserSortKey::Name,
                SortDirection::Descending,
            ))),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(
        filtered_grid.projected,
        vec![id(1), id(4), id(6), id(5), id(3), id(2)]
    );
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert_eq!(state.selection.selected(), vec![id(3)]);
    assert!(memory.is_focused(target_widget));
    assert!(item_response(&filtered_grid, id(3)).state.selected);
    assert_item_focus(&filtered_grid, id(3));

    let reordered = AssetBrowserModel::new(vec![
        asset(6, "Epsilon", "audio"),
        asset(5, "Delta", "mesh"),
        asset(4, "Gamma", "image"),
        asset(3, "Beta", "material"),
        asset(2, "Alpha", "mesh"),
        asset(1, "Zeta", "scene"),
    ]);
    memory.set_scroll_offset(selected.root, Vec2::new(0.0, 28.5));
    let scrolled_list = run_frame(
        &reordered,
        config(AssetBrowserViewMode::List),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let target_rect = item_rect(&scrolled_list, id(3)).rect;
    assert_eq!(target_rect.y.to_bits(), 76.0_f32.to_bits());
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert_eq!(state.selection.selected(), vec![id(3)]);
    assert!(memory.is_focused(target_widget));
    assert!(item_response(&scrolled_list, id(3)).state.selected);
    assert_item_focus(&scrolled_list, id(3));
    assert_eq!(scrolled_list.output.visible_range, 1..6);
    assert_eq!(scrolled_list.output.materialized_range, 0..6);
}

fn assert_only_item_content_while_editing(run: &Run, target: ItemId) {
    let item = item_rect(run, target);
    let base_index = run
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == item.rect))
        .expect("editing item base");
    assert!(matches!(
        run.frame.primitives[base_index + 1],
        Primitive::Rect(preview) if preview.rect == item.preview_rect
    ));
    assert!(matches!(
        run.frame.primitives[base_index + 2],
        Primitive::Text(ref text) if text.text == item.item.fallback.label
    ));
    assert!(matches!(
        run.frame.primitives[base_index + 3],
        Primitive::Text(ref text) if text.text == item.item.kind
    ));
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
    assert!(
        run.frame
            .semantics
            .get(run.root.child(("asset-browser-item", target.raw())))
            .is_none()
    );
}

fn assert_no_item_annuli(run: &Run) {
    assert_eq!(
        run.frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
}

fn item_base_index(run: &Run, target: ItemId) -> usize {
    let rect = item_rect(run, target).rect;
    run.frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Rect(base) if base.rect == rect))
        .expect("asset item base")
}

fn text_color(run: &Run, text: &str) -> Color {
    run.frame
        .primitives
        .iter()
        .find_map(|primitive| match primitive {
            Primitive::Text(primitive) if primitive.text == text => match primitive.brush {
                Brush::Solid(color) => Some(color),
                Brush::LinearGradient(_) => None,
            },
            _ => None,
        })
        .unwrap_or_else(|| panic!("missing text primitive {text}"))
}

fn linear_channel(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

fn contrast_ratio(foreground: Color, background: Color) -> f32 {
    let luminance = |color: Color| {
        0.2126 * linear_channel(color.r)
            + 0.7152 * linear_channel(color.g)
            + 0.0722 * linear_channel(color.b)
    };
    let foreground = luminance(foreground);
    let background = luminance(background);
    (foreground.max(background) + 0.05) / (foreground.min(background) + 0.05)
}

#[test]
fn fractional_scroll_clips_first_middle_last_and_overscan_annuli_without_transforming_geometry() {
    let model = AssetBrowserModel::new(
        (1..=12)
            .map(|raw| asset(raw, format!("Asset {raw}"), "mesh"))
            .collect::<Vec<_>>(),
    );

    for (view_mode, offset) in [
        (AssetBrowserViewMode::Grid, 18.25_f32),
        (AssetBrowserViewMode::List, 14.25_f32),
    ] {
        let seed = run_frame(
            &model,
            config(view_mode),
            &mut AssetBrowserState::new(),
            &mut UiMemory::new(),
            UiInput::default(),
        );
        let mut probe_memory = UiMemory::new();
        probe_memory.set_scroll_offset(seed.root, Vec2::new(0.0, offset));
        let probe = run_frame(
            &model,
            config(view_mode),
            &mut AssetBrowserState::new(),
            &mut probe_memory,
            UiInput::default(),
        );
        assert!(probe.output.materialized_range.len() > probe.output.visible_range.len());
        assert!(
            probe
                .items
                .first()
                .is_some_and(|item| item.rect.y < BOUNDS.y && item.rect.max_y() > BOUNDS.y)
        );
        assert!(probe.items.iter().any(|item| {
            item.rect.y < BOUNDS.max_y()
                && item.rect.max_y() > BOUNDS.max_y()
                && item.rect.intersection(BOUNDS).is_some()
        }));
        assert!(
            probe
                .items
                .last()
                .is_some_and(|item| item.rect.intersection(BOUNDS).is_none())
        );

        let targets = [
            probe.items[0].item.id,
            probe.items[probe.items.len() / 2].item.id,
            probe.items[probe.items.len() - 1].item.id,
        ];
        for target in targets {
            let mut memory = UiMemory::new();
            memory.set_scroll_offset(seed.root, Vec2::new(0.0, offset));
            memory.focus(seed.root.child(("asset-browser-item", target.raw())));
            let focused = run_frame(
                &model,
                config(view_mode),
                &mut AssetBrowserState::new(),
                &mut memory,
                UiInput::default(),
            );
            assert_item_focus(&focused, target);
            let clip_begin = focused
                .frame
                .primitives
                .iter()
                .position(|primitive| {
                    matches!(primitive, Primitive::ClipBegin { rect, .. } if *rect == BOUNDS)
                })
                .expect("asset browser clip begins");
            let clip_end = focused
                .frame
                .primitives
                .iter()
                .rposition(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
                .expect("asset browser clip ends");
            let item_base = item_base_index(&focused, target);
            assert!(clip_begin < item_base && item_base + 2 < clip_end);
            assert!(
                focused
                    .frame
                    .primitives
                    .iter()
                    .all(|primitive| !matches!(primitive, Primitive::TransformBegin { .. }))
            );
            assert_eq!(focused.output.visible_range, probe.output.visible_range);
            assert_eq!(
                focused.output.materialized_range,
                probe.output.materialized_range
            );
            assert_eq!(
                item_rect(&focused, target).rect,
                item_rect(&probe, target).rect
            );
        }
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn retained_disabled_focus_is_suppressed_while_read_only_focus_remains_visible_and_inert() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "Enabled", "mesh"),
        asset(2, "Item disabled", "mesh").disabled(true),
        asset(3, "Read only", "mesh").read_only(true),
    ]);

    for view_mode in [AssetBrowserViewMode::Grid, AssetBrowserViewMode::List] {
        let cfg = config(view_mode);
        let mut state = AssetBrowserState::new();
        let mut memory = UiMemory::new();
        let seed = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        let enabled = click(
            item_rect(&seed, id(1)).rect.center(),
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
        );
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        assert_item_focus(&enabled, id(1));

        let expected_cursor = state.cursor.clone();
        let expected_selection = state.selection.clone();
        let mut expected_semantics = enabled.frame.semantics.nodes().to_vec();
        expected_semantics[0].state.disabled = true;
        for semantic in expected_semantics.iter_mut().skip(1) {
            semantic.state.disabled = true;
            semantic.focusable = false;
            semantic.actions.clear();
        }
        let expected_text = enabled
            .frame
            .primitives
            .iter()
            .filter_map(|primitive| match primitive {
                Primitive::Text(text) => Some(text.text.clone()),
                _ => None,
            })
            .collect::<Vec<_>>();

        let globally_disabled = run_frame(
            &model,
            cfg.clone().disabled(true),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        assert_eq!(state.cursor, expected_cursor);
        assert_eq!(state.selection, expected_selection);
        assert_eq!(item_geometry(&globally_disabled), item_geometry(&enabled));
        assert_eq!(globally_disabled.projected, enabled.projected);
        assert_eq!(
            globally_disabled.frame.semantics.nodes(),
            expected_semantics
        );
        assert_eq!(
            globally_disabled.output.visible_range,
            enabled.output.visible_range
        );
        assert_eq!(
            globally_disabled.output.materialized_range,
            enabled.output.materialized_range
        );
        assert!(globally_disabled.output.requests.is_empty());
        assert_eq!(
            globally_disabled
                .frame
                .primitives
                .iter()
                .filter_map(|primitive| match primitive {
                    Primitive::Text(text) => Some(text.text.clone()),
                    _ => None,
                })
                .collect::<Vec<_>>(),
            expected_text
        );
        assert!(
            globally_disabled
                .output
                .responses
                .iter()
                .all(|response| response.response.state.disabled)
        );
        assert!(item_response(&globally_disabled, id(1)).state.focused);
        assert_no_item_annuli(&globally_disabled);
        for item in &globally_disabled.items {
            let semantic = globally_disabled
                .frame
                .semantics
                .get(seed.root.child(("asset-browser-item", item.item.id.raw())))
                .expect("global-disabled item semantic");
            assert!(semantic.state.disabled);
            assert!(!semantic.focusable);
            assert!(semantic.actions.is_empty());
            assert_eq!(semantic.bounds, item.rect);
        }

        let mut item_disabled_state = AssetBrowserState::new();
        item_disabled_state.selection.replace(id(2));
        let expected_item_selection = item_disabled_state.selection.clone();
        let expected_item_cursor = item_disabled_state.cursor.clone();
        let mut item_disabled_memory = UiMemory::new();
        item_disabled_memory.focus(seed.root.child(("asset-browser-item", 2_u64)));
        let item_disabled = run_frame(
            &model,
            cfg.clone(),
            &mut item_disabled_state,
            &mut item_disabled_memory,
            UiInput::default(),
        );
        let response = item_response(&item_disabled, id(2));
        assert!(response.state.focused);
        assert!(response.state.selected);
        assert!(response.state.disabled);
        assert_eq!(item_disabled_state.cursor, expected_item_cursor);
        assert_eq!(item_disabled_state.selection, expected_item_selection);
        assert_eq!(item_geometry(&item_disabled), item_geometry(&seed));
        assert_eq!(item_disabled.projected, seed.projected);
        assert!(item_disabled.output.requests.is_empty());
        assert_no_item_annuli(&item_disabled);
        let semantic = item_disabled
            .frame
            .semantics
            .get(seed.root.child(("asset-browser-item", 2_u64)))
            .expect("item-disabled semantic");
        assert!(semantic.state.focused);
        assert!(semantic.state.selected);
        assert!(semantic.state.disabled);
        assert!(!semantic.focusable);
        assert!(semantic.actions.is_empty());
        assert_eq!(semantic.bounds, item_rect(&item_disabled, id(2)).rect);

        let mut read_only_state = AssetBrowserState::new();
        let mut read_only_memory = UiMemory::new();
        let read_only = click(
            item_rect(&seed, id(3)).rect.center(),
            &model,
            cfg.clone(),
            &mut read_only_state,
            &mut read_only_memory,
        );
        assert_eq!(read_only_state.cursor.active(), Some(id(3)));
        assert_eq!(read_only_state.selection.selected(), vec![id(3)]);
        let response = item_response(&read_only, id(3));
        assert!(response.state.focused);
        assert!(response.state.selected);
        assert!(!response.state.disabled);
        assert_item_focus(&read_only, id(3));
        let semantic = read_only
            .frame
            .semantics
            .get(seed.root.child(("asset-browser-item", 3_u64)))
            .expect("read-only semantic");
        assert!(semantic.state.focused);
        assert!(semantic.state.selected);
        assert!(!semantic.state.disabled);
        assert!(semantic.focusable);
        assert_eq!(
            semantic
                .actions
                .iter()
                .filter(|action| {
                    matches!(
                        action.kind,
                        SemanticActionKind::Focus | SemanticActionKind::Invoke
                    )
                })
                .count(),
            2
        );
        assert_eq!(semantic.actions.len(), 2);
        assert!(semantic.actions.iter().all(|action| !matches!(
            &action.kind,
            SemanticActionKind::Custom(action) if action == "rename"
        )));
        let resolved = item_rect(&read_only, id(3));
        assert!(
            resolved
                .item
                .inline_rename_begin_request(seed.root)
                .is_none()
        );
        assert!(
            resolved
                .item
                .drag_source(&read_only_state.selection)
                .is_none()
        );
        assert!(read_only.output.requests.is_empty());

        let read_only_f2 = run_frame(
            &model,
            cfg,
            &mut read_only_state,
            &mut read_only_memory,
            key_input(Key::Function(2)),
        );
        assert_eq!(read_only_state.cursor.active(), Some(id(3)));
        assert_eq!(read_only_state.selection.selected(), vec![id(3)]);
        assert!(item_response(&read_only_f2, id(3)).state.focused);
        assert_item_focus(&read_only_f2, id(3));
        assert!(read_only_f2.output.requests.is_empty());
        assert!(read_only_memory.drag_source().is_none());
    }
}

#[test]
#[allow(clippy::too_many_lines)]
fn rename_transfers_focus_omits_item_annuli_and_restores_them_after_all_terminal_paths() {
    let model = AssetBrowserModel::new(vec![asset(1, "Stone", "material")]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let focused = click(
        idle.items[0].rect.center(),
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
    );
    let item_widget = focused.root.child(("asset-browser-item", 1_u64));
    let rename_widget = focused.root.child(("inline-edit", 1_u64));
    assert_item_focus(&focused, id(1));

    let begin = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Function(2)),
    );
    assert!(matches!(
        begin.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Begin(request))]
            if request.target == id(1)
    ));
    assert_eq!(state.rename_target(), Some(id(1)));
    assert!(memory.is_focused(rename_widget));
    assert!(!item_response(&begin, id(1)).state.focused);
    assert_no_item_annuli(&begin);
    assert!(
        begin
            .frame
            .primitives
            .iter()
            .any(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "Stone"))
    );

    let drafted = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        typed_input("X"),
    );
    assert!(matches!(
        drafted.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::DraftEdit(draft))]
            if draft.target == id(1) && draft.draft_text == "StoneX"
    ));
    assert_only_item_content_while_editing(&drafted, id(1));

    let conflict = run_frame_with_options(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Enter),
        true,
        false,
    );
    assert_eq!(
        conflict
            .output
            .rename_conflict
            .as_ref()
            .map(|conflict| conflict.message.as_str()),
        Some("name already exists")
    );
    assert_eq!(state.rename_target(), Some(id(1)));
    assert!(memory.is_focused(rename_widget));
    assert_only_item_content_while_editing(&conflict, id(1));

    let committed = run_frame_with_options(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Enter),
        false,
        false,
    );
    assert!(matches!(
        committed.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Commit(commit))]
            if commit.target == id(1)
                && commit.draft_text == "StoneX"
                && commit.reason == InlineEditCommitReason::Enter
    ));
    assert_eq!(state.rename_target(), None);
    assert!(memory.is_focused(item_widget));
    assert_only_item_content_while_editing(&committed, id(1));
    let after_commit = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_item_focus(&after_commit, id(1));

    let cancel_begin = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Function(2)),
    );
    assert_no_item_annuli(&cancel_begin);
    let cancelled = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Escape),
    );
    assert!(matches!(
        cancelled.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Cancel(cancel))]
            if cancel.target == id(1) && cancel.reason == InlineEditCancelReason::Escape
    ));
    assert_only_item_content_while_editing(&cancelled, id(1));
    let after_cancel = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_item_focus(&after_cancel, id(1));

    let focus_loss_begin = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Function(2)),
    );
    assert_no_item_annuli(&focus_loss_begin);
    let _ = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        typed_input("Y"),
    );
    memory.focus(focus_loss_begin.outside);
    let focus_lost = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert!(matches!(
        focus_lost.output.requests.as_slice(),
        [AssetBrowserRequest::Rename(InlineEditRequest::Commit(commit))]
            if commit.target == id(1)
                && commit.draft_text == "StoneY"
                && commit.reason == InlineEditCommitReason::FocusLost
    ));
    assert_only_item_content_while_editing(&focus_lost, id(1));
    let after_focus_loss = run_frame(&model, cfg, &mut state, &mut memory, UiInput::default());
    assert_item_focus(&after_focus_loss, id(1));
}

#[test]
#[allow(clippy::too_many_lines)]
fn drop_background_and_context_owners_never_create_asset_item_annuli() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
    ]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let source = idle.items[0].rect.center();
    let target = idle.items[2].rect.center();
    let focused = click(source, &model, cfg.clone(), &mut state, &mut memory);
    let source_widget = focused.root.child(("asset-browser-item", 1_u64));
    let baseline_annuli = assert_item_focus(&focused, id(1));
    assert_eq!(state.cursor.active(), Some(id(1)));
    assert_eq!(state.selection.selected(), vec![id(1)]);

    let pressed = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        pointer_input(source, true, true, false),
    );
    assert_eq!(assert_item_focus(&pressed, id(1)), baseline_annuli);
    let dragging = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        move_input(target, Vec2::new(target.x - source.x, target.y - source.y)),
    );
    assert_eq!(assert_item_focus(&dragging, id(1)), baseline_annuli);
    assert_eq!(
        dragging
            .output
            .drag_payload
            .as_ref()
            .map(|source| source.items.clone()),
        Some(vec![id(1)])
    );
    assert!(matches!(
        dragging
            .output
            .drop_preview
            .as_ref()
            .map(|preview| preview.kind),
        Some(AssetBrowserDropTargetKind::Item { target }) if target == id(3)
    ));
    let clip_end = dragging
        .frame
        .primitives
        .iter()
        .rposition(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
        .expect("asset clip end");
    assert!(matches!(
        dragging.frame.primitives[clip_end - 1],
        Primitive::Rect(preview) if preview.rect == item_rect(&dragging, id(3)).rect
    ));
    let dropped = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        pointer_input(target, false, false, true),
    );
    assert!(matches!(
        dropped.output.requests.as_slice(),
        [AssetBrowserRequest::Drop(drop)]
            if drop.source.items == vec![id(1)]
                && drop.kind == AssetBrowserDropTargetKind::Item { target: id(3) }
    ));
    assert_eq!(state.cursor.active(), Some(id(1)));
    assert_eq!(state.selection.selected(), vec![id(1)]);
    assert!(memory.is_focused(source_widget));

    for derived_owner in [
        source_widget.child("drop"),
        focused.root.child("background"),
        focused.root.child("background").child("drop"),
    ] {
        memory.focus(derived_owner);
        let derived = run_frame(
            &model,
            cfg.clone(),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        assert!(
            derived
                .output
                .responses
                .iter()
                .all(|response| !response.response.state.focused)
        );
        assert_eq!(
            derived
                .frame
                .primitives
                .iter()
                .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                .count(),
            0
        );
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
    }

    memory.focus(source_widget);
    let context_opened = context_click(target, &model, cfg.clone(), &mut state, &mut memory);
    assert_eq!(
        context_opened.output.context_opened,
        Some(CollectionContextTarget::item(id(3)))
    );
    assert_eq!(state.cursor.active(), Some(id(1)));
    assert_eq!(state.selection.selected(), vec![id(1)]);
    let context_trigger = focused.root.child(("asset-browser-item", 3_u64));
    assert_eq!(memory.focused(), Some(context_trigger));
    let menu = run_frame_with_options(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    assert!(!item_response(&menu, id(1)).state.focused);
    assert!(item_response(&menu, id(3)).state.focused);
    assert_item_focus(&menu, id(3));
    let inspect = menu
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Inspect"))
        .expect("context overlay row");
    let inspect_id = inspect.id;
    let clip_end = menu
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::ClipEnd { .. }))
        .expect("asset clip end");
    let inspect_text = menu
        .frame
        .primitives
        .iter()
        .position(|primitive| matches!(primitive, Primitive::Text(text) if text.text == "Inspect"))
        .expect("context text paint");
    assert!(inspect_text > clip_end);

    memory.focus(inspect_id);
    let overlay_focused = run_frame_with_options(
        &model,
        cfg,
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    assert!(
        overlay_focused
            .output
            .responses
            .iter()
            .all(|response| !response.response.state.focused)
    );
    assert_eq!(
        overlay_focused
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
    assert_eq!(state.cursor.active(), Some(id(1)));
    assert_eq!(state.selection.selected(), vec![id(1)]);
}

#[test]
fn context_escape_dismissal_restores_asset_trigger_focus_without_mutating_selection() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
    ]);
    let cfg = config(AssetBrowserViewMode::List);
    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let idle = run_frame(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    let trigger_point = item_rect(&idle, id(2)).rect.center();
    let selected = click(trigger_point, &model, cfg.clone(), &mut state, &mut memory);
    let trigger = selected.root.child(("asset-browser-item", 2_u64));
    let expected_cursor = state.cursor.active();
    let expected_selection = state.selection.selected();
    assert_eq!(expected_cursor, Some(id(2)));
    assert_eq!(expected_selection, vec![id(2)]);
    assert_eq!(memory.focused(), Some(trigger));

    let opened = context_click(trigger_point, &model, cfg.clone(), &mut state, &mut memory);
    assert_eq!(
        opened.output.context_opened,
        state.context_target().cloned()
    );
    assert_eq!(memory.focused(), Some(trigger));
    let menu = run_frame_with_options(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    let action = menu
        .frame
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Delete"))
        .expect("selected asset context action")
        .id;
    memory.focus(action);
    assert_eq!(memory.focused(), Some(action));

    let dismissed = run_frame_with_options(
        &model,
        cfg.clone(),
        &mut state,
        &mut memory,
        key_input(Key::Escape),
        false,
        true,
    );
    assert_eq!(state.context_target(), None);
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.cursor.active(), expected_cursor);
    assert_eq!(state.selection.selected(), expected_selection);
    assert!(dismissed.output.requests.is_empty());
    assert!(dismissed.frame.actions.is_empty());
    assert_eq!(dismissed.frame.repaint, RepaintRequest::NextFrame);

    let settled = run_frame_with_options(
        &model,
        cfg,
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.cursor.active(), expected_cursor);
    assert_eq!(state.selection.selected(), expected_selection);
    assert!(
        settled
            .frame
            .semantics
            .nodes()
            .iter()
            .all(|node| node.label.as_deref() != Some("Delete"))
    );
}

#[test]
#[allow(clippy::too_many_lines)]
fn invalid_asset_context_reconciles_focus_without_selection_or_action() {
    fn open_focused_context(
        model: &AssetBrowserModel,
        cfg: AssetBrowserConfig,
        state: &mut AssetBrowserState,
        memory: &mut UiMemory,
        trigger: ItemId,
        extra_selection: Option<ItemId>,
    ) -> (WidgetId, stern_widgets::Selection, Point) {
        let idle = run_frame(model, cfg.clone(), state, memory, UiInput::default());
        let point = item_rect(&idle, trigger).rect.center();
        let selected = click(point, model, cfg.clone(), state, memory);
        if let Some(extra) = extra_selection {
            state.selection.toggle(extra);
        }
        let opened = context_click(point, model, cfg.clone(), state, memory);
        assert!(matches!(
            opened.output.context_opened,
            Some(CollectionContextTarget::Selection(_))
        ));
        let menu =
            run_frame_with_options(model, cfg, state, memory, UiInput::default(), false, true);
        let command = menu
            .frame
            .semantics
            .nodes()
            .iter()
            .find(|node| node.label.as_deref() == Some("Delete"))
            .expect("real asset context command");
        memory.focus(command.id);
        assert_eq!(memory.focused(), Some(command.id));
        (
            selected.root.child(("asset-browser-item", trigger.raw())),
            state.selection.clone(),
            command.bounds.center(),
        )
    }

    let initial = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
    ]);
    let cfg = config(AssetBrowserViewMode::List);

    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let (trigger, selection, stale_command) = open_focused_context(
        &initial,
        cfg.clone(),
        &mut state,
        &mut memory,
        id(1),
        Some(id(2)),
    );
    let without_non_trigger =
        AssetBrowserModel::new(vec![asset(1, "One", "mesh"), asset(3, "Three", "mesh")]);
    let invalidated = run_frame_with_options(
        &without_non_trigger,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput {
            pointer: PointerInput {
                position: Some(stale_command),
                ..PointerInput::default()
            },
            ..UiInput::default()
        },
        false,
        true,
    );
    assert_eq!(state.context_target(), None);
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.selection, selection);
    assert!(invalidated.output.requests.is_empty());
    assert!(invalidated.frame.actions.is_empty());
    assert_eq!(invalidated.frame.repaint, RepaintRequest::NextFrame);
    assert!(
        invalidated
            .frame
            .semantics
            .nodes()
            .iter()
            .all(|node| node.label.as_deref() != Some("Delete"))
    );
    let settled = run_frame_with_options(
        &without_non_trigger,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    assert_eq!(memory.focused(), Some(trigger));
    assert_eq!(state.selection, selection);
    assert!(settled.output.requests.is_empty() && settled.frame.actions.is_empty());

    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let (_, selection, _) = open_focused_context(
        &initial,
        cfg.clone(),
        &mut state,
        &mut memory,
        id(2),
        Some(id(1)),
    );
    let removed_trigger = run_frame_with_options(
        &without_non_trigger,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    let fallback = removed_trigger.root.child(("asset-browser-item", 3_u64));
    assert_eq!(state.cursor.active(), Some(id(3)));
    assert_eq!(memory.focused(), Some(fallback));
    assert_eq!(state.selection, selection);
    assert!(removed_trigger.output.requests.is_empty() && removed_trigger.frame.actions.is_empty());
    let _ = run_frame(
        &without_non_trigger,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    );
    assert_eq!(memory.focused(), Some(fallback));

    let single = AssetBrowserModel::new(vec![asset(1, "One", "mesh")]);
    for (owner, disabled) in [
        (AssetBrowserModel::new(Vec::new()), false),
        (single.clone(), true),
    ] {
        let mut state = AssetBrowserState::new();
        let mut memory = UiMemory::new();
        let (_, selection, _) =
            open_focused_context(&single, cfg.clone(), &mut state, &mut memory, id(1), None);
        let closed = run_frame_with_options(
            &owner,
            cfg.clone().disabled(disabled),
            &mut state,
            &mut memory,
            UiInput::default(),
            false,
            true,
        );
        assert_eq!(state.context_target(), None);
        assert_eq!(memory.focused(), None);
        assert_eq!(state.selection, selection);
        assert!(closed.output.requests.is_empty() && closed.frame.actions.is_empty());
        assert_eq!(closed.frame.repaint, RepaintRequest::NextFrame);
        let _ = run_frame(
            &owner,
            cfg.clone().disabled(disabled),
            &mut state,
            &mut memory,
            UiInput::default(),
        );
        assert_eq!(memory.focused(), None);
    }

    let mut state = AssetBrowserState::new();
    let mut memory = UiMemory::new();
    let _ = open_focused_context(&single, cfg.clone(), &mut state, &mut memory, id(1), None);
    let external = run_frame(
        &single,
        cfg.clone(),
        &mut state,
        &mut memory,
        UiInput::default(),
    )
    .outside;
    memory.focus(external);
    let unrelated = run_frame_with_options(
        &AssetBrowserModel::new(Vec::new()),
        cfg,
        &mut state,
        &mut memory,
        UiInput::default(),
        false,
        true,
    );
    assert_eq!(state.context_target(), None);
    assert_eq!(memory.focused(), Some(external));
    assert!(unrelated.output.requests.is_empty() && unrelated.frame.actions.is_empty());
}

#[test]
#[allow(clippy::too_many_lines)]
fn context_outside_release_and_focused_command_restore_asset_trigger_without_click_through() {
    let model = AssetBrowserModel::new(vec![
        asset(1, "One", "mesh"),
        asset(2, "Two", "mesh"),
        asset(3, "Three", "mesh"),
        asset(4, "Four", "mesh"),
    ]);
    let cfg = config(AssetBrowserViewMode::List).context_menu(AssetBrowserContextMenuConfig {
        size: Size::new(110.0, 40.0),
        offset: 4.0,
    });
    let captured_target =
        CollectionContextTarget::selection([id(1)]).expect("captured asset selection");
    let frame = |state: &mut AssetBrowserState, memory: &mut UiMemory, input| {
        run_frame_with_options(&model, cfg.clone(), state, memory, input, false, true)
    };

    for invoke_command in [false, true] {
        let mut state = AssetBrowserState::new();
        let mut memory = UiMemory::new();
        let seed = frame(&mut state, &mut memory, UiInput::default());
        let trigger_point = item_rect(&seed, id(1)).rect.center();
        let selected = click(trigger_point, &model, cfg.clone(), &mut state, &mut memory);
        let trigger = selected.root.child(("asset-browser-item", 1_u64));
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        assert_eq!(memory.focused(), Some(trigger));

        let opened = context_click(trigger_point, &model, cfg.clone(), &mut state, &mut memory);
        assert_eq!(opened.output.context_opened, Some(captured_target.clone()));
        let shown = frame(&mut state, &mut memory, UiInput::default());
        let menu_bounds = shown
            .frame
            .semantics
            .nodes()
            .iter()
            .find(|node| node.label.as_deref() == Some("Asset actions"))
            .expect("asset context menu surface")
            .bounds;
        let command = shown
            .frame
            .semantics
            .nodes()
            .iter()
            .find(|node| node.label.as_deref() == Some("Delete"))
            .expect("asset context command")
            .id;

        let closed = if invoke_command {
            memory.focus(command);
            let focused = frame(&mut state, &mut memory, UiInput::default());
            assert!(
                focused
                    .frame
                    .semantics
                    .get(command)
                    .expect("focused asset command")
                    .state
                    .focused
            );
            let mut invoked = frame(&mut state, &mut memory, key_input(Key::Enter));
            let expected_action = ActionId::new("asset.delete");
            assert_eq!(
                invoked.output.requests,
                vec![AssetBrowserRequest::Context(
                    CollectionContextActionRequest::new(expected_action.clone(), &captured_target)
                )]
            );
            assert_eq!(
                invoked.frame.actions.drain().collect::<Vec<_>>(),
                vec![ActionInvocation::new(
                    expected_action,
                    ActionSource::Menu,
                    ActionContext::Widget(invoked.root),
                )]
            );
            invoked
        } else {
            let outside_point = item_rect(&shown, id(4)).rect.center();
            assert!(!menu_bounds.contains_point(outside_point));
            assert!(!item_response(&shown, id(4)).state.disabled);
            let pressed = frame(
                &mut state,
                &mut memory,
                pointer_input(outside_point, true, true, false),
            );
            assert_eq!(state.context_target(), Some(&captured_target));
            assert_eq!(memory.focused(), Some(trigger));
            assert_eq!(state.cursor.active(), Some(id(1)));
            assert_eq!(state.selection.selected(), vec![id(1)]);
            assert!(!item_response(&pressed, id(4)).clicked);
            assert!(!item_response(&pressed, id(4)).state.pressed);
            assert!(pressed.output.requests.is_empty() && pressed.frame.actions.is_empty());
            let dismissed = frame(
                &mut state,
                &mut memory,
                pointer_input(outside_point, false, false, true),
            );
            assert!(!item_response(&dismissed, id(4)).clicked);
            assert!(dismissed.output.requests.is_empty() && dismissed.frame.actions.is_empty());
            dismissed
        };

        assert_eq!(state.context_target(), None);
        assert_eq!(memory.focused(), Some(trigger));
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        assert_eq!(closed.frame.repaint, RepaintRequest::NextFrame);
        assert!(closed.frame.actions.is_empty());
        let settled = frame(&mut state, &mut memory, UiInput::default());
        assert_eq!(state.context_target(), None);
        assert_eq!(memory.focused(), Some(trigger));
        assert_eq!(state.cursor.active(), Some(id(1)));
        assert_eq!(state.selection.selected(), vec![id(1)]);
        assert!(settled.output.requests.is_empty() && settled.frame.actions.is_empty());
        assert!(
            settled
                .frame
                .semantics
                .get(trigger)
                .expect("settled asset trigger")
                .state
                .focused
        );
        assert!(
            settled
                .frame
                .semantics
                .nodes()
                .iter()
                .all(|node| !matches!(node.label.as_deref(), Some("Asset actions" | "Delete")))
        );
    }
}
#[test]
#[allow(clippy::too_many_lines)]
fn selected_names_inventory_only_the_named_exception_while_muted_kind_remains_nonconforming() {
    let theme = default_dark_theme();
    let model = AssetBrowserModel::new(vec![asset(1, "Selected asset", "Selected kind")]);

    for view_mode in [AssetBrowserViewMode::Grid, AssetBrowserViewMode::List] {
        let seed = run_frame(
            &model,
            config(view_mode),
            &mut AssetBrowserState::new(),
            &mut UiMemory::new(),
            UiInput::default(),
        );
        let target_widget = seed.root.child(("asset-browser-item", 1_u64));
        let point = seed.items[0].rect.center();
        for (focused, hovered, pressed) in [
            (false, false, false),
            (false, true, false),
            (false, false, true),
            (true, false, false),
            (true, true, false),
            (true, false, true),
        ] {
            let input = if pressed {
                pointer_input(point, true, true, false)
            } else if hovered {
                pointer_input(point, false, false, false)
            } else {
                UiInput::default()
            };
            let mut state = AssetBrowserState::new();
            state.selection.replace(id(1));
            let mut memory = UiMemory::new();
            if focused {
                memory.focus(target_widget);
            }
            let run = run_frame(&model, config(view_mode), &mut state, &mut memory, input);
            let Primitive::Rect(base) = &run.frame.primitives[item_base_index(&run, id(1))] else {
                unreachable!()
            };
            assert_eq!(
                base.fill,
                Some(Brush::Solid(theme.colors.selection.background))
            );
            let name = text_color(&run, "Selected asset");
            let kind = text_color(&run, "Selected kind");
            assert_eq!(name, theme.colors.selection.foreground);
            assert_eq!(kind, theme.colors.content.muted);
            let name_ratio = contrast_ratio(name, theme.colors.selection.background);
            let kind_ratio = contrast_ratio(kind, theme.colors.selection.background);
            assert!((3.52..3.54).contains(&name_ratio));
            assert!(name_ratio < 4.5);
            assert!((1.23..1.25).contains(&kind_ratio));
            assert!(kind_ratio < 3.0);
            if focused {
                assert_item_focus(&run, id(1));
            } else {
                assert_eq!(
                    run.frame
                        .primitives
                        .iter()
                        .filter(|primitive| matches!(primitive, Primitive::Path(_)))
                        .count(),
                    0
                );
            }
        }
    }

    let disabled_model = AssetBrowserModel::new(vec![
        asset(1, "Disabled selected", "Disabled kind").disabled(true),
    ]);
    let mut disabled_state = AssetBrowserState::new();
    disabled_state.selection.replace(id(1));
    let seed = run_frame(
        &disabled_model,
        config(AssetBrowserViewMode::List),
        &mut AssetBrowserState::new(),
        &mut UiMemory::new(),
        UiInput::default(),
    );
    let mut disabled_memory = UiMemory::new();
    disabled_memory.focus(seed.root.child(("asset-browser-item", 1_u64)));
    let disabled = run_frame(
        &disabled_model,
        config(AssetBrowserViewMode::List),
        &mut disabled_state,
        &mut disabled_memory,
        UiInput::default(),
    );
    let response = item_response(&disabled, id(1));
    let recipe = theme.row(ComponentState {
        hovered: response.state.hovered,
        pressed: response.state.pressed,
        focused: response.state.focused,
        disabled: response.state.disabled,
        selected: response.state.selected,
    });
    let Primitive::Rect(base) = &disabled.frame.primitives[item_base_index(&disabled, id(1))]
    else {
        unreachable!()
    };
    assert_eq!(base.fill, Some(recipe.background));
    assert_eq!(
        text_color(&disabled, "Disabled selected"),
        recipe.foreground
    );
    assert_ne!(recipe.foreground, theme.colors.selection.foreground);
    assert_ne!(
        base.fill,
        Some(Brush::Solid(theme.colors.selection.background))
    );
    assert_eq!(
        disabled
            .frame
            .primitives
            .iter()
            .filter(|primitive| matches!(primitive, Primitive::Path(_)))
            .count(),
        0
    );
}
