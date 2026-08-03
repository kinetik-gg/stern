//! Public native host for the real Stern integration demo.
//!
//! The window, event loop, input, platform requests, repaint scheduling, and
//! GPU recovery are owned by the `stern::app` runner; this host keeps only
//! the demo application state and the CI smoke contract.
use std::io::{self, Write};

use stern::app::{App, AppConfig, ShellCtx, VelloPresentStatus, run};
use stern::core::{ActionQueue, Size};
use stern::render::RenderResources;
use stern::widgets::Ui;
use stern_demo::{DEMO_TITLE, DemoApp};

struct NativeShell {
    app: DemoApp,
    smoke: bool,
    presented: bool,
}

fn should_terminate_successful_smoke(smoke: bool, status: VelloPresentStatus) -> bool {
    smoke
        && matches!(
            status,
            VelloPresentStatus::Presented | VelloPresentStatus::PresentedSuboptimal
        )
}

impl App for NativeShell {
    fn frame(&mut self, ui: &mut Ui<'_>, shell: &mut ShellCtx) {
        if let Some(target) = self.app.compose(ui) {
            shell.request_widget_focus(target);
        }
    }

    fn on_actions(&mut self, actions: &mut ActionQueue, shell: &mut ShellCtx) {
        for invocation in actions.drain() {
            if let Some(target) = self.app.apply_action(&invocation) {
                shell.request_widget_focus(target);
            }
        }
    }

    fn register_resources(&mut self, resources: &mut RenderResources) {
        self.app.register_domain_resources(resources);
    }

    fn on_present(&mut self, status: VelloPresentStatus) {
        if matches!(
            status,
            VelloPresentStatus::Presented | VelloPresentStatus::PresentedSuboptimal
        ) {
            self.presented = true;
        }
        if should_terminate_successful_smoke(self.smoke, status) {
            let mut stdout = io::stdout().lock();
            if writeln!(stdout, "native-shell-smoke=pass status={status:?}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                std::process::exit(1);
            }
            std::process::exit(0);
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let smoke = std::env::args().any(|argument| argument == "--smoke-exit-after-present");
    let mut shell = NativeShell {
        app: DemoApp::new(),
        smoke,
        presented: false,
    };
    run(
        AppConfig::new(DEMO_TITLE).with_initial_size(Size::new(960.0, 640.0)),
        &mut shell,
    )?;
    if smoke && !shell.presented {
        return Err("native shell exited without a successful present".into());
    }
    Ok(())
}

#[cfg(test)]
fn test_context(input: stern::core::UiInput) -> stern::core::FrameContext {
    stern::core::FrameContext::new(
        stern::core::ViewportInfo::new(
            stern::core::Size::new(960.0, 640.0),
            stern::core::PhysicalSize::new(960, 640),
            stern::core::ScaleFactor::ONE,
        ),
        input,
        stern::core::TimeInfo::default(),
    )
}

#[cfg(test)]
#[test]
fn native_shell_hosts_real_edit_and_graph_workspaces() {
    use stern::core::{Point, PointerButtonState, PointerInput, SemanticRole, UiInput};
    use stern_demo::DemoWorkspace;

    fn workspace_input(point: Point, down: bool, pressed: bool, released: bool) -> UiInput {
        UiInput {
            pointer: PointerInput {
                position: Some(point),
                primary: PointerButtonState::new(down, pressed, released),
                ..PointerInput::default()
            },
            ..UiInput::default()
        }
    }

    let mut app = DemoApp::new();
    let edit = app.frame(test_context(UiInput::default()));
    assert_eq!(app.workspace(), DemoWorkspace::Edit);
    assert!(edit.semantics.nodes().iter().any(|node| {
        node.role == SemanticRole::Dock && node.label.as_deref() == Some("Editor dock")
    }));
    let graph = edit
        .semantics
        .nodes()
        .iter()
        .find(|node| {
            node.role == SemanticRole::IconButton
                && node.label.as_deref() == Some("Graph Workspace")
        })
        .expect("Graph workspace action")
        .bounds
        .center();
    let _ = app.frame(test_context(workspace_input(graph, true, true, false)));
    let switched = app.frame(test_context(workspace_input(graph, false, false, true)));
    assert_eq!(app.workspace(), DemoWorkspace::Graph);
    let mut actions = switched.actions.clone();
    assert!(
        actions
            .drain()
            .any(|invocation| invocation.action_id.as_str() == "workspace.graph")
    );
    let graph = app.frame(test_context(UiInput::default()));
    assert!(
        graph.semantics.nodes().iter().any(|node| {
            matches!(&node.role, SemanticRole::Custom(role) if role == "node-graph")
        })
    );
}

#[cfg(test)]
#[test]
fn native_shell_runner_config_is_valid_without_a_window() {
    let config = AppConfig::new(DEMO_TITLE).with_initial_size(Size::new(960.0, 640.0));

    assert_eq!(config.title(), DEMO_TITLE);
    assert_eq!(config.validate(), Ok(()));
}

#[cfg(test)]
#[test]
fn smoke_success_termination_requires_smoke_and_confirmed_presentation() {
    for status in [
        VelloPresentStatus::Presented,
        VelloPresentStatus::PresentedSuboptimal,
    ] {
        assert!(should_terminate_successful_smoke(true, status));
        assert!(!should_terminate_successful_smoke(false, status));
    }
    for status in [
        VelloPresentStatus::SurfaceRecoveryRequired,
        VelloPresentStatus::DeviceRecoveryRequired,
        VelloPresentStatus::SurfaceLost,
        VelloPresentStatus::Detached,
    ] {
        assert!(!should_terminate_successful_smoke(true, status));
    }
}
