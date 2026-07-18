//! Structural guard against private crates and substitute control painting.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

fn rust_sources(path: &Path, sources: &mut Vec<PathBuf>) {
    for entry in fs::read_dir(path).expect("source directory") {
        let path = entry.expect("source entry").path();
        if path.is_dir() {
            rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn dependency_name(dependency: &str) -> &str {
    dependency
        .split_once("\"name\":\"")
        .and_then(|(_, tail)| tail.split_once('"'))
        .map(|(name, _)| name)
        .expect("dependency name")
}

fn import_root(line: &str) -> Option<&str> {
    let mut words = line.split_whitespace();
    let first = words.next()?;
    if first == "pub" {
        if words.next() != Some("use") {
            return None;
        }
    } else if first != "use" {
        return None;
    }
    let import = words.next()?;
    import.split([':', '{', ';']).next()
}

#[test]
#[allow(clippy::too_many_lines)]
fn prohibited_techniques_are_absent_from_public_consumer() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    rust_sources(&root.join("src"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "demo source tree must not be empty");
    let sources = paths
        .iter()
        .map(|path| (path, fs::read_to_string(path).expect("source")))
        .collect::<Vec<_>>();
    let source = sources
        .iter()
        .map(|(_, source)| source.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();

    let output = Command::new(env!("CARGO"))
        .args(["metadata", "--no-deps", "--format-version", "1"])
        .current_dir(&root)
        .output()
        .expect("cargo metadata");
    assert!(output.status.success(), "cargo metadata failed");
    let metadata = String::from_utf8(output.stdout).expect("metadata utf-8");
    let package_name = "\"name\":\"stern-demo\"";
    let name = metadata.find(package_name).expect("stern-demo metadata");
    let package = &metadata[metadata[..name].rfind("{\"name\":").expect("package start")..];
    let dependencies = package
        .split_once("\"dependencies\":[")
        .and_then(|(_, tail)| tail.split_once("],\"targets\""))
        .map(|(dependencies, _)| dependencies)
        .expect("dependency metadata");
    let dependency_entries = dependencies.split("},{").collect::<Vec<_>>();
    let normal_dependencies = dependency_entries
        .iter()
        .filter(|dependency| dependency.contains("\"kind\":null"))
        .map(|dependency| dependency_name(dependency))
        .collect::<Vec<_>>();
    assert_eq!(
        normal_dependencies
            .iter()
            .filter(|name| **name == "stern")
            .count(),
        1
    );
    for dependency in dependency_entries {
        let name = dependency_name(dependency);
        assert!(["stern", "winit", "pollster"].contains(&name), "{name}");
        assert!(
            dependency.contains("\"rename\":null"),
            "renamed dependency: {name}"
        );
    }

    for private_dependency in [
        "stern_core",
        "stern_render",
        "stern_text",
        "stern_vello",
        "stern_widgets",
        "stern_winit",
    ] {
        assert!(!source.contains(private_dependency), "{private_dependency}");
    }
    for substitute in [
        "Primitive",
        "SemanticNode",
        ".primitive(",
        "push_primitive",
        "fixtures_paint",
        "fnpaint_",
        "fndraw_",
        "fnrender_widget",
        "fnrender_control",
        "fnrender_component",
        "fnrender_overlay",
        "fnrender_scene",
        "fnrender_primitive",
        "fnhit_test",
        "fnpointer_",
        "fnfocus_",
        "structDemoWidget",
        "structDemoControl",
        "structDemoTheme",
        "structDemoRenderer",
        "structDemoFramework",
        "modwidgets",
        "modcontrols",
        "modtheme",
        "modrenderer",
        "unsafe",
        "externcrate",
        "#[path",
        "include!",
        "include_str!",
    ] {
        assert!(!compact.contains(substitute), "{substitute}");
    }
    assert!(
        !compact.contains(".extend(") || !compact.contains(".primitives"),
        "primitive extension"
    );
    for (path, source) in sources {
        for line in source.lines() {
            if let Some(import) = import_root(line.trim_start()) {
                let bootstrap = path.ends_with(Path::new("src/bin/native_shell.rs"));
                let allowed = ["std", "stern", "stern_demo"].contains(&import)
                    || (bootstrap && ["winit", "pollster"].contains(&import));
                assert!(allowed, "{}: {line}", path.display());
            }
        }
    }
}
