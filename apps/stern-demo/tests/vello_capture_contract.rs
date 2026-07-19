//! Source boundary for the public `DemoApp` Vello evidence harness.

use std::fs;
use std::path::PathBuf;

#[test]
fn capture_uses_real_demo_output_through_the_public_facade() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest = fs::read_to_string(root.join("Cargo.toml")).expect("manifest");
    let source = fs::read_to_string(root.join("examples/capture_vello_workspaces.rs"))
        .expect("capture source");

    for dependency in [
        "stern-core =",
        "stern-render =",
        "stern-vello =",
        "stern-vello-winit =",
        "stern-widgets =",
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
        "RectPrimitive",
        "TextPrimitive",
        "Primitive::",
        "scene_primitives",
        "push_primitive",
        "push_semantic_node",
        "alternate_scene",
        "manual_scene",
    ] {
        assert!(!source.contains(substitute), "{substitute}");
    }
    assert!(source.contains("use stern::"));
    assert!(source.contains("use stern_demo::{DemoApp, DemoWorkspace};"));
    assert!(source.contains("app.frame("));
    assert!(source.contains("app.render_resources()"));
    assert!(source.contains("toolkit_renderer.scene()"));
}
