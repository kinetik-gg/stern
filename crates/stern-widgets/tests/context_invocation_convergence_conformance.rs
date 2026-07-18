//! Public context-menu invocation convergence conformance.

use std::time::Duration;

use stern_core::{
    ActionDescriptor, ActionId, ActionInvocation, ActionSource, FrameContext, FrameOutput, Key,
    KeyEvent, KeyState, KeyboardInput, Modifiers, PhysicalSize, Point, PointerButtonState,
    PointerInput, PointerOrder, Rect, ScaleFactor, Size, TimeInfo, UiInput, UiMemory, ViewportInfo,
    default_dark_theme,
};
use stern_widgets::asset_browser::{
    AssetBrowserConfig, AssetBrowserItem, AssetBrowserLayout, AssetBrowserModel,
    AssetBrowserRequest, AssetBrowserState, AssetBrowserViewMode,
};
use stern_widgets::outliner::{
    OutlinerConfig, OutlinerRequest, OutlinerSelectionMode, OutlinerState,
};
use stern_widgets::{
    CollectionContextTarget, GridColumns, GridLayout, ItemId, ListLayout, OutlinerItem,
    OutlinerModel, Ui,
};

const BOUNDS: Rect = Rect::new(0.0, 0.0, 240.0, 100.0);

#[derive(Clone, Copy)]
enum Entry {
    Pointer,
    Menu,
    ShiftF10,
}

#[derive(Clone, Debug, PartialEq)]
struct Request {
    action: ActionId,
    target: CollectionContextTarget,
    ids: Vec<ItemId>,
}

struct Run {
    target_point: Point,
    opened: Option<CollectionContextTarget>,
    requests: Vec<Request>,
    frame: FrameOutput,
}

fn id(raw: u64) -> ItemId {
    ItemId::from_raw(raw)
}

fn context(input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            Size::new(320.0, 200.0),
            PhysicalSize::new(320, 200),
            ScaleFactor::ONE,
        ),
        input,
        TimeInfo::new(Duration::from_millis(1), Duration::from_millis(16), 1),
    )
}

fn key(key: Key, state: KeyState, modifiers: Modifiers) -> UiInput {
    UiInput {
        keyboard: KeyboardInput {
            modifiers,
            events: vec![KeyEvent::new(key, state, modifiers, false)],
        },
        ..UiInput::default()
    }
}

fn pointer(point: Point, primary: bool, down: bool) -> UiInput {
    let button = PointerButtonState::new(down, down, !down);
    let mut pointer = PointerInput {
        position: Some(point),
        ..PointerInput::default()
    };
    if primary {
        pointer.primary = button;
    } else {
        pointer.secondary = button;
    }
    UiInput {
        pointer,
        ..UiInput::default()
    }
}

fn position_only(point: Point) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn commands(prefix: &str) -> Vec<ActionDescriptor> {
    let mut remove = ActionDescriptor::new(format!("{prefix}.remove"), "Remove");
    remove.state.enabled = false;
    vec![
        ActionDescriptor::new(format!("{prefix}.open"), "Open"),
        remove,
    ]
}

fn menu_evidence(frame: &FrameOutput) -> (Vec<(&str, bool)>, Point) {
    let nodes = frame.semantics.nodes();
    let inventory = nodes
        .iter()
        .filter_map(|node| match node.label.as_deref() {
            Some(label @ ("Open" | "Remove")) => Some((label, node.state.disabled)),
            _ => None,
        })
        .collect();
    let center = nodes
        .iter()
        .find(|node| node.label.as_deref() == Some("Open"))
        .expect("open command")
        .bounds
        .center();
    (inventory, center)
}

#[rustfmt::skip]
fn asset_run(model: &AssetBrowserModel, state: &mut AssetBrowserState, memory: &mut UiMemory,
    input: UiInput, disabled: bool) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let layout = AssetBrowserLayout::new(
        AssetBrowserViewMode::List,
        GridLayout {
            columns: GridColumns::Fixed(1),
            item_size: Size::new(72.0, 72.0),
            gap: 4.0,
        },
        ListLayout::new(28.0),
    );
    let scene = ui.prepare_asset_browser(
        "assets", AssetBrowserConfig::new(BOUNDS, layout).disabled(disabled), model, state)
        .expect("asset scene");
    let target_point = scene.layout().items[0].rect.center();
    ui.resolve_pointer_targets(|plan| { scene.declare_pointer_targets(plan, PointerOrder::new(100), state); })
        .expect("asset targets");
    let output = ui.asset_browser(&scene, state, |_item, _draft| None, |_| commands("asset"));
    let requests = output
        .requests
        .into_iter()
        .filter_map(|request| match request {
            AssetBrowserRequest::Context(request) => Some(Request { action: request.action_id,
                target: request.target, ids: request.target_ids }),
            _ => None,
        })
        .collect();
    Run { target_point, opened: output.context_opened, requests, frame: ui.finish_output() }
}

#[rustfmt::skip]
fn outliner_run(model: &OutlinerModel, state: &mut OutlinerState, memory: &mut UiMemory,
    input: UiInput, disabled: bool) -> Run {
    let theme = default_dark_theme();
    let mut ui = Ui::begin_frame(context(input), memory, &theme);
    let scene = ui.prepare_outliner("outliner", OutlinerConfig::new(BOUNDS, 20.0, 16.0)
        .selection_mode(OutlinerSelectionMode::Multiple).disabled(disabled), model, state)
        .expect("outliner scene");
    let target_point = scene.rows()[0].label_rect.center();
    ui.resolve_pointer_targets(|plan| { scene.declare_pointer_targets(plan, PointerOrder::new(100), state); })
        .expect("outliner targets");
    let output = ui.outliner(&scene, state, |_| commands("scene"));
    let requests = output
        .requests
        .into_iter()
        .filter_map(|request| match request {
            OutlinerRequest::Context(request) => Some(Request { action: request.action_id,
                target: request.target, ids: request.target_ids }),
            _ => None,
        })
        .collect();
    Run { target_point, opened: output.context_opened, requests, frame: ui.finish_output() }
}

fn entry_input(entry: Entry) -> UiInput {
    match entry {
        Entry::Menu => key(Key::ContextMenu, KeyState::Pressed, Modifiers::default()),
        Entry::ShiftF10 => key(
            Key::Function(10),
            KeyState::Pressed,
            Modifiers::new(true, false, false, false),
        ),
        Entry::Pointer => unreachable!(),
    }
}

#[rustfmt::skip]
fn prove_convergence<S>(mut fresh: impl FnMut() -> S, mut selected: impl FnMut(&S) -> Vec<ItemId>,
    mut run: impl FnMut(&mut S, &mut UiMemory, UiInput, bool) -> Run,
    target: &CollectionContextTarget) {
    let mut expected: Option<(Request, ActionInvocation)> = None;
    for entry in [Entry::Menu, Entry::ShiftF10, Entry::Pointer] {
        let mut state = fresh();
        let mut memory = UiMemory::new();
        let seed = run(&mut state, &mut memory, UiInput::default(), false);
        let _ = run(&mut state, &mut memory, pointer(seed.target_point, true, true), false);
        let _ = run(&mut state, &mut memory, pointer(seed.target_point, true, false), false);
        let opened = if matches!(entry, Entry::Pointer) {
            let _ = run(&mut state, &mut memory, pointer(seed.target_point, false, true), false);
            run(&mut state, &mut memory, pointer(seed.target_point, false, false), false)
        } else {
            run(&mut state, &mut memory, entry_input(entry), false)
        };
        assert_eq!(opened.opened.as_ref(), Some(target));
        assert_eq!(selected(&state), vec![id(1)]);
        assert!(opened.requests.is_empty() && opened.frame.actions.is_empty());
        let shown = run(&mut state, &mut memory, UiInput::default(), false);
        let (inventory, center) = menu_evidence(&shown.frame);
        assert_eq!(inventory, [("Open", false), ("Remove", true)]);
        let neutral = run(&mut state, &mut memory, position_only(center), false);
        assert!(neutral.requests.is_empty() && neutral.frame.actions.is_empty());
        let press = run(&mut state, &mut memory, pointer(center, true, true), false);
        assert!(press.requests.is_empty() && press.frame.actions.is_empty());
        let mut invoked = run(&mut state, &mut memory, pointer(center, true, false), false);
        assert_eq!(invoked.requests.len(), 1);
        let evidence = (invoked.requests.pop().expect("typed request"),
            invoked.frame.actions.pop_front().expect("menu invocation"));
        assert_eq!(evidence.1.source, ActionSource::Menu);
        assert_eq!(expected.get_or_insert_with(|| evidence.clone()), &evidence);
        assert!(invoked.requests.is_empty() && invoked.frame.actions.is_empty());
    }
    for (focused, disabled, state) in [(true, false, KeyState::Released),
        (false, false, KeyState::Pressed), (true, true, KeyState::Pressed)] {
        let mut widget_state = fresh();
        let mut memory = UiMemory::new();
        let seed = run(&mut widget_state, &mut memory, UiInput::default(), false);
        if focused {
            let _ = run(&mut widget_state, &mut memory,
                pointer(seed.target_point, true, true), false);
            let _ = run(&mut widget_state, &mut memory,
                pointer(seed.target_point, true, false), false);
        }
        let closed = run(&mut widget_state, &mut memory,
            key(Key::ContextMenu, state, Modifiers::default()), disabled);
        assert!(closed.opened.is_none() && closed.requests.is_empty()
            && closed.frame.actions.is_empty());
    }
}

#[test]
fn asset_browser_pointer_menu_key_and_shift_f10_converge() {
    let model = AssetBrowserModel::new(vec![AssetBrowserItem::new(id(1), "One", "mesh")]);
    let target = CollectionContextTarget::selection([id(1)]).expect("selection target");
    prove_convergence(
        || {
            let mut state = AssetBrowserState::new();
            state.selection.replace(id(1));
            state
        },
        |state| state.selection.selected(),
        |state, memory, input, disabled| asset_run(&model, state, memory, input, disabled),
        &target,
    );
}

#[test]
fn outliner_pointer_menu_key_and_shift_f10_converge() {
    let model = OutlinerModel::new(vec![OutlinerItem::new(id(1), "One")]);
    let target = CollectionContextTarget::selection([id(1)]).expect("selection target");
    prove_convergence(
        || {
            let mut state = OutlinerState::new();
            state.selection.replace(id(1));
            state
        },
        |state| state.selection.selected(),
        |state, memory, input, disabled| outliner_run(&model, state, memory, input, disabled),
        &target,
    );
}
