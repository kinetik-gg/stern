//! Interactive story browser built on `stern-app`.
//!
//! Minimal but honest: a story sidebar (click or ArrowUp/ArrowDown to
//! select) and a live story canvas receiving real input. The canvas
//! composes the selected story exactly as `render` mode does.

use stern::app::{App, AppConfig, AppError, ShellCtx, run};
use stern::core::{Key, KeyState, Rect, UiInputEvent};
use stern::widgets::Ui;

use crate::story::{Story, registry, story_matches_filter};

const SIDEBAR_WIDTH: f32 = 260.0;

/// Runs the interactive browser over registry stories matching `filter`.
///
/// # Errors
///
/// Returns an error when no story matches or the shell fails to start.
pub fn run_browser(filter: &str) -> Result<(), String> {
    let stories: Vec<Story> = registry()
        .into_iter()
        .filter(|story| story_matches_filter(story.id, filter))
        .collect();
    if stories.is_empty() {
        return Err(format!("no stories match filter {filter:?}"));
    }
    run(AppConfig::new("Stern Stories"), Browser {
        stories,
        selected: 0,
    })
    .map_err(|error: AppError| error.to_string())
}

struct Browser {
    stories: Vec<Story>,
    selected: usize,
}

impl App for Browser {
    fn frame(&mut self, ui: &mut Ui<'_>, _shell: &mut ShellCtx) {
        let logical = ui.viewport().logical_size;
        self.apply_keyboard(ui);

        let sidebar = Rect::new(0.0, 0.0, SIDEBAR_WIDTH.min(logical.width), logical.height);
        ui.panel(sidebar);
        ui.label(
            Rect::new(sidebar.x + 12.0, 12.0, sidebar.width - 24.0, 16.0),
            "Stories",
        );
        let mut clicked = None;
        for (index, story) in self.stories.iter().enumerate() {
            let row = Rect::new(
                sidebar.x + 8.0,
                28.0f32.mul_add(index_f32(index), 40.0),
                sidebar.width - 16.0,
                24.0,
            );
            let response = ui.list_row(
                ("story-row", index),
                row,
                story.title,
                index == self.selected,
                false,
            );
            if response.clicked {
                clicked = Some(index);
            }
        }
        if let Some(index) = clicked {
            self.selected = index;
        }

        let canvas = Rect::new(
            sidebar.width,
            0.0,
            (logical.width - sidebar.width).max(0.0),
            logical.height,
        );
        if let Some(story) = self.stories.get(self.selected) {
            let compose = story.compose;
            ui.scope(("story-canvas", story.id), |ui| compose(ui, canvas));
        }
    }
}

impl Browser {
    fn apply_keyboard(&mut self, ui: &Ui<'_>) {
        for event in &ui.input().events {
            let UiInputEvent::Key(key_event) = event else {
                continue;
            };
            if key_event.state != KeyState::Pressed {
                continue;
            }
            match key_event.key {
                Key::ArrowDown => {
                    self.selected = (self.selected + 1).min(self.stories.len().saturating_sub(1));
                }
                Key::ArrowUp => {
                    self.selected = self.selected.saturating_sub(1);
                }
                _ => {}
            }
        }
    }
}

#[allow(clippy::cast_precision_loss)]
fn index_f32(index: usize) -> f32 {
    index as f32
}
