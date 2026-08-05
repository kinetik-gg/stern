//! Dock workspace story: frames, tab strips, seams, and overflowing rows.

use stern::core::{Axis, Rect, WidgetId};
use stern::widgets::dock::{DockScene, DockSceneConfig};
use stern::widgets::{Dock, DockNode, Frame, FrameId, Panel, PanelId, Ui};

use crate::story::{Story, StoryKind};

const ASSETS_PANEL: PanelId = PanelId::from_raw(1);
const VIEWPORT_PANEL: PanelId = PanelId::from_raw(2);
const INSPECTOR_PANEL: PanelId = PanelId::from_raw(3);
const TIMELINE_PANEL: PanelId = PanelId::from_raw(4);

/// Dock-with-seams: a four-frame dock composed full-window. Every internal
/// seam is visible here — the composition rung where the #941 duplicated
/// panel borders live. The Assets panel overflows two-line rows into
/// fixed-height slots on purpose.
#[must_use]
pub fn with_seams() -> Story {
    Story {
        id: "workspace/dock-with-seams",
        title: "Dock workspace with frames and seams",
        kind: StoryKind::Workspace,
        compose,
    }
}

fn story_dock() -> Dock {
    let assets = frame(1, ASSETS_PANEL, "Assets");
    let viewport = frame(2, VIEWPORT_PANEL, "Viewport");
    let inspector = frame(3, INSPECTOR_PANEL, "Inspector");
    let timeline = frame(4, TIMELINE_PANEL, "Timeline");
    let upper = split(Axis::Horizontal, 0.62, viewport, inspector);
    let right = split(Axis::Vertical, 0.7, upper, timeline);
    let mut dock = Dock::new(split(Axis::Horizontal, 0.24, assets, right));
    let _ = dock.set_active_frame(FrameId::from_raw(2));
    dock
}

fn frame(id: u64, panel: PanelId, title: &str) -> DockNode {
    DockNode::Frame(Frame::new(FrameId::from_raw(id), vec![Panel::new(
        panel, title,
    )]))
}

fn split(axis: Axis, ratio: f32, first: DockNode, second: DockNode) -> DockNode {
    DockNode::Split {
        axis,
        ratio,
        min_first: 120.0,
        min_second: 120.0,
        first: Box::new(first),
        second: Box::new(second),
    }
}

fn compose(ui: &mut Ui<'_>, rect: Rect) {
    let dock = story_dock();
    let scene = DockScene::new(
        DockSceneConfig::new(WidgetId::from_key("story-dock"), rect),
        &dock,
    );
    let _ = ui.dock_scene(&scene, |ui, panel| {
        let body = panel.rect.inset(8.0);
        match panel.panel {
            ASSETS_PANEL => assets_content(ui, body),
            INSPECTOR_PANEL => inspector_content(ui, body),
            TIMELINE_PANEL => timeline_content(ui, body),
            _ => viewport_content(ui, body),
        }
    });
}

fn assets_content(ui: &mut Ui<'_>, body: Rect) {
    // Two-line asset descriptions composed into fixed 24px rows: the honest
    // reproduction of the #941 row-clipping defect.
    let names = [
        "Granite cliff scan\nphotogrammetry, 8k diffuse",
        "Harbor crane rig\nanimation set, 42 clips",
        "Signal decal pack\nvector, 36 variants",
        "Night sky HDRI\n32-bit panorama",
    ];
    for (index, name) in names.iter().enumerate() {
        let row = Rect::new(body.x, 24.0f32.mul_add(index_f32(index), body.y), body.width, 24.0);
        let _ = ui.list_row(("asset-row", index), row, *name, index == 1, false);
    }
}

fn inspector_content(ui: &mut Ui<'_>, body: Rect) {
    ui.label(Rect::new(body.x, body.y, body.width, 16.0), "Transform");
    let labels = ["Position  12.0  4.5  -3.2", "Rotation  0.0  90.0  0.0"];
    for (index, text) in labels.iter().enumerate() {
        let row = Rect::new(
            body.x,
            24.0f32.mul_add(index_f32(index), body.y + 24.0),
            body.width,
            24.0,
        );
        ui.label_keyed(("inspector-line", index), row, *text);
    }
}

fn timeline_content(ui: &mut Ui<'_>, body: Rect) {
    ui.label(
        Rect::new(body.x, body.y, body.width, 16.0),
        "Timeline  0:00 / 4:12",
    );
    ui.separator(Rect::new(body.x, body.y + 24.0, body.width, 1.0));
}

fn viewport_content(ui: &mut Ui<'_>, body: Rect) {
    ui.label(
        Rect::new(body.x, body.y, body.width, 16.0),
        "Viewport surface placeholder",
    );
}

#[allow(clippy::cast_precision_loss)]
fn index_f32(index: usize) -> f32 {
    index as f32
}
