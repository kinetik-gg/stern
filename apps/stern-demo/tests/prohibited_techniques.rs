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

fn contains_root_path(source: &str, root: &str) -> bool {
    source.match_indices(root).any(|(index, _)| {
        let before = source[..index].chars().next_back();
        let after = &source[index + root.len()..];
        before.is_none_or(|character| !character.is_ascii_alphanumeric() && character != '_')
            && after.trim_start().starts_with("::")
    })
}

fn app_owned_path(path: &Path) -> bool {
    ["src/lib.rs", "src/app_model.rs", "src/bin/native_shell.rs"]
        .iter()
        .any(|suffix| path.ends_with(Path::new(suffix)))
        || path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem.ends_with("_workspace"))
}

fn source_violates_policy(path: &Path, source: &str) -> bool {
    let compact = source
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect::<String>();
    if ["::primitive(", "push_semantic_node("]
        .iter()
        .any(|prohibited| compact.contains(prohibited))
    {
        return true;
    }
    if compact.contains(".primitives")
        && (compact.contains(".extend(") || compact.contains("::extend("))
    {
        return true;
    }
    let bootstrap = path.ends_with(Path::new("src/bin/native_shell.rs"));
    if !bootstrap
        && ["winit", "pollster"]
            .iter()
            .any(|root| contains_root_path(source, root))
    {
        return true;
    }
    let tokens = source
        .split(|character: char| !character.is_ascii_alphanumeric() && character != '_')
        .filter(|token| !token.is_empty())
        .collect::<Vec<_>>();
    for (index, token) in tokens.iter().enumerate() {
        let name = tokens.get(index + 1).copied().unwrap_or_default();
        let lower = name.to_ascii_lowercase();
        if *token == "fn"
            && ("pointer hover pressed drag click keyboard control_state"
                .split_whitespace()
                .any(|term| lower.contains(term))
                || (lower.contains("focus") && lower != "focused"))
        {
            return true;
        }
        if ["struct", "enum", "trait"].contains(token) {
            if !app_owned_path(path) {
                return true;
            }
            if "control widget primitive semantic theme renderer framework"
                .split_whitespace()
                .any(|term| lower.contains(term))
            {
                return true;
            }
        } else if *token == "impl" && !app_owned_path(path) {
            return true;
        }
    }
    false
}

#[test]
#[allow(clippy::too_many_lines)]
fn demo_sources_use_only_public_stern_components() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut paths = Vec::new();
    rust_sources(&root.join("src"), &mut paths);
    paths.sort();
    assert!(!paths.is_empty(), "demo source tree must not be empty");
    let local_modules = paths
        .iter()
        .filter_map(|path| {
            path.parent()
                .is_some_and(|parent| parent == root.join("src"))
                .then(|| path.file_stem()?.to_str().map(str::to_owned))
                .flatten()
        })
        .collect::<Vec<_>>();
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
    assert_eq!(
        normal_dependencies
            .iter()
            .filter(|name| **name == "stern-icons-phosphor")
            .count(),
        1
    );
    for dependency in dependency_entries {
        let name = dependency_name(dependency);
        assert!(
            ["stern", "stern-icons-phosphor", "winit", "pollster"].contains(&name),
            "{name}"
        );
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
    for (path, source) in &sources {
        for line in source.lines() {
            if let Some(import) = import_root(line.trim_start()) {
                let bootstrap = path.ends_with(Path::new("src/bin/native_shell.rs"));
                let allowed = ["std", "stern", "stern_demo", "stern_icons_phosphor"]
                    .contains(&import)
                    || local_modules.iter().any(|module| module == import)
                    || (bootstrap && ["winit", "pollster"].contains(&import));
                assert!(allowed, "{}: {line}", path.display());
            }
        }
    }
    for (path, source) in &sources {
        assert!(!source_violates_policy(path, source), "{}", path.display());
    }
    let reject = |path: &str, source: &str| {
        assert!(
            source_violates_policy(Path::new(path), source),
            "accepted {source}"
        );
    };
    for probe in [
        "stern::widgets::Ui :: primitive(&mut ui, frame.primitives.remove(0));",
        "stern::widgets::Ui :: extend(&mut ui, frame.primitives);",
        "ui.push_semantic_node(stern::widgets::button_semantics(id, rect, label, false));",
    ] {
        reject("src/lib.rs", probe);
    }
    reject("src/main.rs", "use\nwinit::window::Window;");
    reject("src/main.rs", "pub(crate) use pollster::block_on;");
    reject("src/lib.rs", "struct CustomControl;");
    reject("src/main.rs", "enum LocalState { Ready }");
    for helper in "route_pointer update_hover set_pressed move_focus begin_drag dispatch_click route_keyboard update_control_state".split_whitespace() {
        let probe = format!("fn {helper}() {{}}");
        reject("src/lib.rs", &probe);
    }
    assert!(
        !source_violates_policy(
            Path::new("src/bin/native_shell.rs"),
            "use winit::window::Window; use pollster::block_on; struct NativeShell; impl NativeShell {}",
        ),
        "rejected approved native shell bootstrap"
    );
}
