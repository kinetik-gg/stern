//! Viewport-driven relayout evidence (issue #944).
//!
//! Drives the demo through a resize sequence — 960x640 -> 1280x800 ->
//! 4K-logical (1920x1080 @ 2.0, physical 3840x2160) -> back to 960x640 —
//! and asserts on every step that the workspace layout is derived from the
//! current frame's viewport: chrome bands span the full width at their
//! token heights, the status bar stays pinned to the bottom edge, and the
//! dock band absorbs every remaining logical pixel with panels tiling it.
//! No stale band geometry, no dead space, no overlap.

#![allow(clippy::float_cmp)] // Band layout math is exact token arithmetic.

use stern::core::{
    FrameContext, FrameOutput, PhysicalSize, Point, PointerButtonState, PointerInput, Rect,
    ScaleFactor, SemanticNode, SemanticRole, Size, TimeInfo, UiInput, ViewportInfo,
    default_dark_theme,
};
use stern_demo::{DemoApp, DemoWorkspace};

/// Resize sequence exercised by every scenario, including a return to the
/// starting size to prove nothing sticks to the largest layout.
const RESIZE_SEQUENCE: [(f32, f32, f64); 4] = [
    (960.0, 640.0, 1.0),
    (1280.0, 800.0, 1.0),
    (1920.0, 1080.0, 2.0),
    (960.0, 640.0, 1.0),
];

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
fn context_at(logical: Size, scale: f64, input: UiInput) -> FrameContext {
    FrameContext::new(
        ViewportInfo::new(
            logical,
            PhysicalSize::new(
                (logical.width * scale as f32) as u32,
                (logical.height * scale as f32) as u32,
            ),
            ScaleFactor::new(scale),
        ),
        input,
        TimeInfo::default(),
    )
}

/// Finds the workspace-level chrome surface with `label`.
///
/// The gallery also composes a miniature chrome *specimen* with the same
/// surface labels inside its content band, so the workspace surface is the
/// one anchored to the viewport's left edge.
fn surface<'a>(output: &'a FrameOutput, label: &str) -> &'a SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.label.as_deref() == Some(label))
        .min_by(|a, b| a.bounds.x.total_cmp(&b.bounds.x))
        .unwrap_or_else(|| panic!("chrome surface: {label}"))
}

fn dock(output: &FrameOutput) -> &SemanticNode {
    output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.role == SemanticRole::Dock)
        .expect("dock semantic root")
}

fn frames(output: &FrameOutput) -> Vec<Rect> {
    output
        .semantics
        .nodes()
        .iter()
        .filter(|node| node.role == SemanticRole::Frame)
        .map(|node| node.bounds)
        .collect()
}

fn overlap_area(a: Rect, b: Rect) -> f32 {
    a.intersection(b)
        .map_or(0.0, |shared| shared.width * shared.height)
}

/// Asserts the composed frame's workspace layout tracks `logical` exactly.
///
/// Since #876 the shell's top band is the shared public application bar at
/// the `sizes.workspace_bar` token height: five 64px menu headings anchored
/// left and two 96px workspace tabs anchored right. The workspace-local
/// chrome below it keeps only the contextual toolbar, the dock content
/// band, and the status presentation.
#[allow(clippy::too_many_lines)]
fn assert_layout_tracks_viewport(output: &FrameOutput, logical: Size) {
    let theme = default_dark_theme();
    let bar = theme.sizes.workspace_bar;
    let menus = surface(output, "Application menu").bounds;
    let workspaces = surface(output, "Workspaces").bounds;
    let bar_root = output
        .semantics
        .nodes()
        .iter()
        .find(|node| node.label.as_deref() == Some("Application bar"))
        .expect("application bar root")
        .bounds;
    let toolbar = surface(output, "Application toolbar").bounds;
    let status = surface(output, "Application status").bounds;
    let dock = dock(output).bounds;

    // The bar spans the full width at the workspace_bar token height; its
    // menu headings anchor left and the workspace tabs anchor right.
    assert_eq!(bar_root, Rect::new(0.0, 0.0, logical.width, bar));
    assert_eq!(menus, Rect::new(0.0, 0.0, 5.0 * 64.0, bar));
    assert_eq!(
        workspaces,
        Rect::new(logical.width - 2.0 * 96.0, 0.0, 2.0 * 96.0, bar)
    );
    // The contextual toolbar sits directly under the bar, and the dock band
    // absorbs everything between the toolbar and the status bar pinned to
    // the bottom edge: panels fill the viewport.
    assert_eq!(
        toolbar,
        Rect::new(0.0, bar, logical.width, theme.sizes.control.lg)
    );
    assert_eq!(
        dock,
        Rect::new(
            0.0,
            toolbar.max_y(),
            logical.width,
            logical.height - toolbar.max_y() - theme.sizes.control.sm
        )
    );
    assert_eq!(
        status,
        Rect::new(0.0, dock.max_y(), logical.width, theme.sizes.control.sm)
    );
    assert_eq!(status.max_y(), logical.height, "status pins to the bottom");

    // Dock frames tile the dock band: each stays inside it, none overlap
    // beyond seam lines, and together they cover its full area.
    let frames = frames(output);
    assert!(!frames.is_empty(), "dock composes frames");
    let mut covered = 0.0;
    for (index, frame) in frames.iter().enumerate() {
        assert_eq!(
            frame.intersection(dock),
            Some(*frame),
            "frame {index} stays inside the dock band"
        );
        covered += frame.width * frame.height;
        for other in &frames[index + 1..] {
            assert!(
                overlap_area(*frame, *other) < 1.0,
                "frames must not overlap: {frame:?} vs {other:?}"
            );
        }
    }
    let dock_area = dock.width * dock.height;
    // Frames are separated by a few px of splitter seams, so allow seam
    // slack proportional to the band's extent while still rejecting any
    // fixed-size-island layout (the dock band equality above is the hard
    // guarantee; this catches frames not filling their own band).
    assert!(
        covered >= dock_area - 8.0 * (dock.width + dock.height),
        "frames fill the dock band: covered {covered} of {dock_area}"
    );
}

fn pointer(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
    UiInput {
        pointer: PointerInput {
            position: Some(point),
            primary: PointerButtonState::new(down, pressed, released),
            ..PointerInput::default()
        },
        ..UiInput::default()
    }
}

fn activate_tab(app: &mut DemoApp, logical: Size, label: &str, expected: DemoWorkspace) {
    let output = app.frame(context_at(logical, 1.0, UiInput::default()));
    let control = output
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.label.as_deref() == Some(label)
                && matches!(node.role, SemanticRole::Tab | SemanticRole::IconButton)
        })
        .unwrap_or_else(|| panic!("workspace switch control: {label}"))
        .bounds
        .center();
    let _ = app.frame(context_at(
        logical,
        1.0,
        pointer(control, true, true, false),
    ));
    let _ = app.frame(context_at(
        logical,
        1.0,
        pointer(control, false, false, true),
    ));
    assert_eq!(app.workspace(), expected);
}

#[test]
fn edit_workspace_relayout_tracks_resize_sequence() {
    let mut app = DemoApp::new();
    for (width, height, scale) in RESIZE_SEQUENCE {
        let logical = Size::new(width, height);
        let output = app.frame(context_at(logical, scale, UiInput::default()));
        assert_layout_tracks_viewport(&output, logical);
    }
}

#[test]
fn graph_workspace_relayout_tracks_resize_sequence_without_island() {
    let start = Size::new(960.0, 640.0);
    let mut app = DemoApp::new();
    activate_tab(&mut app, start, "Graph Workspace", DemoWorkspace::Graph);
    for (width, height, scale) in RESIZE_SEQUENCE {
        let logical = Size::new(width, height);
        let output = app.frame(context_at(logical, scale, UiInput::default()));
        assert_layout_tracks_viewport(&output, logical);
    }
}

#[test]
fn gallery_workspace_chrome_tracks_resize_sequence() {
    let theme = default_dark_theme();
    let start = Size::new(960.0, 640.0);
    let mut app = DemoApp::new();
    activate_tab(&mut app, start, "Gallery Workspace", DemoWorkspace::Gallery);
    for (width, height, scale) in RESIZE_SEQUENCE {
        let logical = Size::new(width, height);
        let output = app.frame(context_at(logical, scale, UiInput::default()));
        // The gallery has no dock; assert its chrome bands directly.
        let menu = surface(&output, "Application menu").bounds;
        let status = surface(&output, "Application status").bounds;
        assert_eq!(
            menu,
            Rect::new(0.0, 0.0, logical.width, theme.sizes.control.md)
        );
        assert_eq!(status.width, logical.width);
        assert_eq!(status.max_y(), logical.height, "status pins to the bottom");
    }
}
