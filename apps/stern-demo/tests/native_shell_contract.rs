//! Source boundary for the public native integration-demo host.

use std::fs;
use std::path::PathBuf;

#[test]
fn native_shell_source_uses_only_public_facade_and_winit_bootstrap() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("manifest");
    let source = fs::read_to_string(root.join("src/bin/native_shell.rs")).expect("source");

    for dependency in [
        "stern-core =",
        "stern-render =",
        "stern-vello =",
        "stern-vello-winit =",
        "stern-widgets =",
        "stern-winit =",
        "vello =",
        "wgpu =",
    ] {
        assert!(!manifest.contains(dependency), "{dependency}");
    }
    for substitute in [
        "stern_core",
        "stern_render",
        "stern_vello",
        "stern_widgets",
        "stern_winit",
        "RectPrimitive",
        "TextPrimitive",
        "Primitive::",
        "SemanticNode",
        "push_primitive",
        "push_semantic_node",
        ".primitive(",
        "fn paint_",
        "DockScene",
        "ChromeScene",
        "build_shell_frame",
        "shell_dock",
        "ui.chrome_scene",
        "ui.dock_scene",
    ] {
        assert!(!source.contains(substitute), "{substitute}");
    }
    assert!(manifest.contains("pollster = \"0.4.0\""));
    assert!(manifest.contains("winit = \"0.30.12\""));
    assert!(source.contains("use stern::"));
    assert!(source.contains("use stern_demo::{DEMO_TITLE, DemoApp};"));
    assert!(source.contains("use winit::"));
    assert!(source.contains("app: DemoApp"));
    assert!(source.contains("self.app.frame(context)"));
    assert!(source.contains("self.app.render_resources()"));
    assert!(source.contains("VelloWindowPresenter"));
}

#[test]
fn native_shell_success_exit_is_guarded_by_smoke_and_present_status() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let source = fs::read_to_string(root.join("src/bin/native_shell.rs")).expect("source");

    let guard = source
        .find("if should_terminate_successful_smoke(self.smoke, present_status)")
        .expect("smoke success guard");
    let success_exit = source
        .find("std::process::exit(0);")
        .expect("smoke success exit");
    assert!(guard < success_exit);
    assert_eq!(source.matches("std::process::exit(0);").count(), 1);
    assert!(source.contains("assert!(!should_terminate_successful_smoke(false, status));"));
    assert!(source.contains("VelloPresentStatus::PresentedSuboptimal"));
}
