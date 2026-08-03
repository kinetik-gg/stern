//! Minimal stern-app application: one window, a few widgets, one action.
//!
//! Compare with `stern-vello-winit/examples/one_window.rs`, the manual
//! application-owned event loop this runner replaces for ordinary apps.

use stern_app::{App, AppConfig, AppError, ShellCtx, run};
use stern_core::{ActionContext, ActionDescriptor, ActionQueue, Rect, Size};
use stern_widgets::Ui;

const QUIT_ACTION: &str = "app.quit";

struct Hello {
    clicks: u32,
    quit: ActionDescriptor,
}

impl App for Hello {
    fn frame(&mut self, ui: &mut Ui<'_>, _shell: &mut ShellCtx) {
        ui.label(
            Rect::new(24.0, 24.0, 360.0, 24.0),
            format!("Button clicks: {}", self.clicks),
        );
        let counted = ui.button(
            "count",
            Rect::new(24.0, 64.0, 180.0, 30.0),
            "Count a click",
            false,
        );
        if counted.clicked {
            self.clicks += 1;
        }
        let quit = self.quit.clone();
        let _ = ui.action_button(
            QUIT_ACTION,
            Rect::new(24.0, 106.0, 180.0, 30.0),
            &quit,
            ActionContext::Global,
        );
    }

    fn on_actions(&mut self, actions: &mut ActionQueue, shell: &mut ShellCtx) {
        for invocation in actions.drain() {
            if invocation.action_id.as_str() == QUIT_ACTION {
                shell.request_close();
            }
        }
    }
}

fn main() -> Result<(), AppError> {
    run(
        AppConfig::new("Hello Stern").with_initial_size(Size::new(480.0, 220.0)),
        Hello {
            clicks: 0,
            quit: ActionDescriptor::new(QUIT_ACTION, "Quit"),
        },
    )
}
