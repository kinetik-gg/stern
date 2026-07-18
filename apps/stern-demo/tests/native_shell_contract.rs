//! Source boundary for the public native shell spike.

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
    ] {
        assert!(!source.contains(substitute), "{substitute}");
    }
    assert!(manifest.contains("pollster = \"0.4.0\""));
    assert!(manifest.contains("winit = \"0.30.12\""));
    assert!(source.contains("use stern::"));
    assert!(source.contains("use winit::"));
    assert!(source.contains("ui.chrome_scene(&chrome)"));
    assert!(source.contains("ui.dock_scene(&dock"));
    assert!(source.contains("VelloWindowPresenter"));
}
