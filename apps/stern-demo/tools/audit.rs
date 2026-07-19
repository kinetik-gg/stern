use std::{
    fs,
    path::{Path, PathBuf},
    process::Command,
};

use super::json::{Json as Value, json};

pub(super) fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

pub(super) fn git(root: &Path, args: &[&str]) -> Result<String, String> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .output()
        .map_err(|e| e.to_string())?;
    if !output.status.success() {
        return Err(String::from_utf8_lossy(&output.stderr).trim().to_owned());
    }
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}

pub(super) fn public_consumer_audit(root: &Path) -> Result<Value, String> {
    let demo = root.join("apps/stern-demo");
    let manifest = read(&demo.join("Cargo.toml"))?;
    let paths = rust_paths(&demo.join("src"))?;
    let sources = paths
        .iter()
        .map(|path| read(path))
        .collect::<Result<Vec<_>, _>>()?;
    let combined = sources.join("\n");
    let private = [
        "stern-core",
        "stern-render",
        "stern-text",
        "stern-vello",
        "stern-widgets",
    ]
    .into_iter()
    .filter(|name| manifest.contains(&format!("{name} =")))
    .collect::<Vec<_>>();
    let forbidden = [
        "stern_core",
        "stern_render",
        "stern_widgets",
        "RectPrimitive",
        "TextPrimitive",
        "SemanticNode::new",
        "push_semantic_node",
        ".primitive(",
        "push_primitive",
        "fn paint_",
    ]
    .into_iter()
    .filter(|pattern| combined.contains(pattern))
    .collect::<Vec<_>>();
    let imports = combined
        .lines()
        .filter(|line| line.trim_start().starts_with("use stern::"))
        .map(str::trim)
        .collect::<Vec<_>>();
    Ok(json!({
        "passed": private.is_empty() && forbidden.is_empty() && !imports.is_empty(),
        "manifestPath": "apps/stern-demo/Cargo.toml", "publicFacadeDependency": manifest.contains("stern = {"),
        "privateSternDependencies": private, "publicFacadeImports": imports,
        "forbiddenSourceMatches": forbidden,
        "auditedSourcePaths": paths.iter().map(|path| relative(root, path)).collect::<Vec<_>>(),
    }))
}

pub(super) fn primitive_allowlist(root: &Path) -> Result<Value, String> {
    let entries = [
        (
            "frame-output-consumption",
            [".primitives", "translate_primitives"].as_slice(),
        ),
        (
            "viewport-content-surface",
            ["ViewportSurface", "TextureResource", "RenderImage"].as_slice(),
        ),
        (
            "native-render-attachment",
            ["RenderFrameInput", "render_resources"].as_slice(),
        ),
    ];
    let paths = rust_paths(&root.join("apps/stern-demo/src"))?;
    let sources = paths
        .iter()
        .map(|path| Ok((path, read(path)?)))
        .collect::<Result<Vec<_>, String>>()?;
    Ok(Value::Array(entries.into_iter().map(|(id, patterns)| {
        let matches = sources.iter().filter(|(_, source)| patterns.iter().any(|pattern| source.contains(pattern)))
            .map(|(path, _)| relative(root, path)).collect::<Vec<_>>();
        json!({"id": id, "allowedPatterns": patterns, "matchedSourcePaths": matches, "reason": "public output translation or declared content-surface integration; no control painting"})
    }).collect()))
}

fn read(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))
}

fn rust_paths(root: &Path) -> Result<Vec<PathBuf>, String> {
    let mut pending = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(dir) = pending.pop() {
        for entry in fs::read_dir(&dir).map_err(|e| format!("{}: {e}", dir.display()))? {
            let path = entry.map_err(|e| e.to_string())?.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                paths.push(path);
            }
        }
    }
    paths.sort();
    Ok(paths)
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/")
}
