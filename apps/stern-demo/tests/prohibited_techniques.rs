//! Structural guard against private crates and substitute control painting.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use syn::visit::{self, Visit};
use syn::{
    Attribute, Expr, ExprCall, ExprMethodCall, File, Ident, ImplItemFn, ItemEnum, ItemExternCrate,
    ItemFn, ItemForeignMod, ItemImpl, ItemMacro, ItemStruct, ItemTrait, ItemType, ItemUnion,
    ItemUse, Local, Macro, Path as SynPath, UseTree,
};

const PRIVATE_ROOTS: [&str; 6] = [
    "stern_core",
    "stern_render",
    "stern_text",
    "stern_vello",
    "stern_widgets",
    "stern_winit",
];

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

fn ident_name(ident: &Ident) -> String {
    ident.to_string().trim_start_matches("r#").to_owned()
}

fn path_names(path: &SynPath) -> Vec<String> {
    path.segments
        .iter()
        .map(|segment| ident_name(&segment.ident))
        .collect()
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

fn is_bootstrap(path: &Path) -> bool {
    path.ends_with(Path::new("src/bin/native_shell.rs"))
}

fn prohibited_declaration_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    "control widget primitive semantic theme renderer framework"
        .split_whitespace()
        .any(|term| lower.contains(term))
}

fn prohibited_function_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    "pointer hover pressed drag click keyboard control_state hit_test"
        .split_whitespace()
        .any(|term| lower.contains(term))
        || (lower.contains("focus") && lower != "focused")
        || [
            "paint_",
            "draw_",
            "render_widget",
            "render_control",
            "render_component",
            "render_overlay",
            "render_scene",
            "render_primitive",
        ]
        .iter()
        .any(|term| lower.starts_with(term))
}

fn use_imports(
    tree: &UseTree,
    prefix: &mut Vec<String>,
    imports: &mut Vec<(Vec<String>, Option<String>)>,
) {
    match tree {
        UseTree::Path(path) => {
            prefix.push(ident_name(&path.ident));
            use_imports(&path.tree, prefix, imports);
            prefix.pop();
        }
        UseTree::Name(name) => {
            prefix.push(ident_name(&name.ident));
            imports.push((prefix.clone(), None));
            prefix.pop();
        }
        UseTree::Rename(rename) => {
            prefix.push(ident_name(&rename.ident));
            imports.push((prefix.clone(), Some(ident_name(&rename.rename))));
            prefix.pop();
        }
        UseTree::Glob(_) => imports.push((prefix.clone(), Some("*".to_owned()))),
        UseTree::Group(group) => {
            for item in &group.items {
                use_imports(item, prefix, imports);
            }
        }
    }
}

fn expr_mentions_primitives(expr: &Expr) -> bool {
    struct PrimitiveFieldVisitor(bool);

    impl<'ast> Visit<'ast> for PrimitiveFieldVisitor {
        fn visit_member(&mut self, member: &'ast syn::Member) {
            if let syn::Member::Named(ident) = member {
                self.0 |= ident_name(ident) == "primitives";
            }
            visit::visit_member(self, member);
        }
    }

    let mut visitor = PrimitiveFieldVisitor(false);
    visitor.visit_expr(expr);
    visitor.0
}

struct PurityVisitor<'a> {
    source_path: &'a Path,
    local_modules: &'a [String],
    violations: Vec<String>,
}

struct BindingVisitor<'a>(&'a mut Vec<String>);

impl<'ast> Visit<'ast> for BindingVisitor<'_> {
    fn visit_pat_ident(&mut self, pat: &'ast syn::PatIdent) {
        self.0.push(ident_name(&pat.ident));
        visit::visit_pat_ident(self, pat);
    }
}

impl PurityVisitor<'_> {
    fn reject(&mut self, message: impl Into<String>) {
        self.violations.push(message.into());
    }

    fn inspect_path(&mut self, path: &SynPath) {
        let names = path_names(path);
        if let Some(root) = names.first() {
            if PRIVATE_ROOTS.contains(&root.as_str()) {
                self.reject(format!("private crate path `{root}`"));
            }
            if !is_bootstrap(self.source_path) && ["winit", "pollster"].contains(&root.as_str()) {
                self.reject(format!(
                    "bootstrap dependency path `{root}` outside native shell"
                ));
            }
        }
        for name in &names {
            if ["Primitive", "SemanticNode"].contains(&name.as_str()) {
                self.reject(format!("raw output type `{name}`"));
            }
        }
    }

    fn inspect_function_name(&mut self, ident: &Ident) {
        let name = ident_name(ident);
        if prohibited_function_name(&name) {
            self.reject(format!("substitute behavior function `{name}`"));
        }
    }

    fn inspect_declaration_name(&mut self, ident: &Ident) {
        let name = ident_name(ident);
        if prohibited_declaration_name(&name) {
            self.reject(format!("substitute control declaration `{name}`"));
        }
    }
}

impl<'ast> Visit<'ast> for PurityVisitor<'_> {
    fn visit_item_use(&mut self, item: &'ast ItemUse) {
        let mut imports = Vec::new();
        use_imports(&item.tree, &mut Vec::new(), &mut imports);
        for (path, rename) in imports {
            let Some(root) = path.first() else {
                self.reject("empty import path");
                continue;
            };
            let bootstrap_dependency = ["winit", "pollster"].contains(&root.as_str());
            let allowed = [
                "std",
                "stern",
                "stern_demo",
                "stern_icons_phosphor",
                "self",
                "super",
                "crate",
            ]
            .contains(&root.as_str())
                || self.local_modules.contains(root)
                || (is_bootstrap(self.source_path) && bootstrap_dependency);
            if !allowed {
                self.reject(format!("disallowed import root `{root}`"));
            }
            if bootstrap_dependency && rename.is_some() {
                self.reject(format!("aliased bootstrap dependency `{root}`"));
            }
            if let Some(imported) = path.last()
                && ["Ui", "Primitive", "SemanticNode"].contains(&imported.as_str())
                && rename.as_deref().is_some_and(|alias| alias != "_")
            {
                self.reject(format!("aliased raw UI symbol `{imported}`"));
            }
        }
        visit::visit_item_use(self, item);
    }

    fn visit_path(&mut self, path: &'ast SynPath) {
        self.inspect_path(path);
        visit::visit_path(self, path);
    }

    fn visit_expr_call(&mut self, call: &'ast ExprCall) {
        if let Expr::Path(function) = call.func.as_ref()
            && let Some(name) = function
                .path
                .segments
                .last()
                .map(|segment| ident_name(&segment.ident))
        {
            if ["primitive", "push_primitive", "push_semantic_node"].contains(&name.as_str()) {
                self.reject(format!("raw UI call `{name}`"));
            }
            if name == "extend" && call.args.iter().any(expr_mentions_primitives) {
                self.reject("raw primitive extension");
            }
        }
        visit::visit_expr_call(self, call);
    }

    fn visit_expr_method_call(&mut self, call: &'ast ExprMethodCall) {
        let name = ident_name(&call.method);
        if ["primitive", "push_primitive", "push_semantic_node"].contains(&name.as_str()) {
            self.reject(format!("raw UI method `{name}`"));
        }
        if name == "extend"
            && (expr_mentions_primitives(&call.receiver)
                || call.args.iter().any(expr_mentions_primitives))
        {
            self.reject("raw primitive extension");
        }
        visit::visit_expr_method_call(self, call);
    }

    fn visit_item_fn(&mut self, item: &'ast ItemFn) {
        self.inspect_function_name(&item.sig.ident);
        if item.sig.unsafety.is_some() {
            self.reject("unsafe function");
        }
        visit::visit_item_fn(self, item);
    }

    fn visit_impl_item_fn(&mut self, item: &'ast ImplItemFn) {
        self.inspect_function_name(&item.sig.ident);
        if item.sig.unsafety.is_some() {
            self.reject("unsafe method");
        }
        visit::visit_impl_item_fn(self, item);
    }

    fn visit_item_struct(&mut self, item: &'ast ItemStruct) {
        self.inspect_declaration_name(&item.ident);
        if !app_owned_path(self.source_path) {
            self.reject(format!(
                "struct outside app-owned source `{}`",
                ident_name(&item.ident)
            ));
        }
        visit::visit_item_struct(self, item);
    }

    fn visit_item_enum(&mut self, item: &'ast ItemEnum) {
        self.inspect_declaration_name(&item.ident);
        if !app_owned_path(self.source_path) {
            self.reject(format!(
                "enum outside app-owned source `{}`",
                ident_name(&item.ident)
            ));
        }
        visit::visit_item_enum(self, item);
    }

    fn visit_item_trait(&mut self, item: &'ast ItemTrait) {
        self.inspect_declaration_name(&item.ident);
        if item.unsafety.is_some() {
            self.reject("unsafe trait");
        }
        if !app_owned_path(self.source_path) {
            self.reject(format!(
                "trait outside app-owned source `{}`",
                ident_name(&item.ident)
            ));
        }
        visit::visit_item_trait(self, item);
    }

    fn visit_item_union(&mut self, item: &'ast ItemUnion) {
        self.inspect_declaration_name(&item.ident);
        if !app_owned_path(self.source_path) {
            self.reject(format!(
                "union outside app-owned source `{}`",
                ident_name(&item.ident)
            ));
        }
        visit::visit_item_union(self, item);
    }

    fn visit_item_type(&mut self, item: &'ast ItemType) {
        self.inspect_declaration_name(&item.ident);
        visit::visit_item_type(self, item);
    }

    fn visit_item_impl(&mut self, item: &'ast ItemImpl) {
        if item.unsafety.is_some() {
            self.reject("unsafe impl");
        }
        if !app_owned_path(self.source_path) {
            self.reject("impl outside app-owned source");
        }
        visit::visit_item_impl(self, item);
    }

    fn visit_local(&mut self, local: &'ast Local) {
        if local
            .init
            .as_ref()
            .is_some_and(|init| matches!(init.expr.as_ref(), Expr::Closure(_)))
        {
            let mut bindings = Vec::new();
            BindingVisitor(&mut bindings).visit_pat(&local.pat);
            for binding in bindings {
                if prohibited_declaration_name(&binding) || prohibited_function_name(&binding) {
                    self.reject(format!("substitute control closure `{binding}`"));
                }
            }
        }
        visit::visit_local(self, local);
    }

    fn visit_item_extern_crate(&mut self, item: &'ast ItemExternCrate) {
        self.reject(format!("extern crate `{}`", ident_name(&item.ident)));
        visit::visit_item_extern_crate(self, item);
    }

    fn visit_item_foreign_mod(&mut self, item: &'ast ItemForeignMod) {
        self.reject("foreign module");
        visit::visit_item_foreign_mod(self, item);
    }

    fn visit_expr_unsafe(&mut self, expr: &'ast syn::ExprUnsafe) {
        self.reject("unsafe expression");
        visit::visit_expr_unsafe(self, expr);
    }

    fn visit_attribute(&mut self, attribute: &'ast Attribute) {
        if attribute.path().is_ident("path") {
            self.reject("path attribute");
        }
        visit::visit_attribute(self, attribute);
    }

    fn visit_macro(&mut self, mac: &'ast Macro) {
        if mac
            .path
            .segments
            .last()
            .map(|segment| ident_name(&segment.ident))
            .is_some_and(|name| ["include", "include_str"].contains(&name.as_str()))
        {
            self.reject("source inclusion macro");
        }
        visit::visit_macro(self, mac);
    }

    fn visit_item_macro(&mut self, item: &'ast ItemMacro) {
        if item.ident.is_some() {
            self.reject("local macro definition");
        }
        visit::visit_item_macro(self, item);
    }
}

fn structural_violations(path: &Path, source: &str, local_modules: &[String]) -> Vec<String> {
    let syntax: File = match syn::parse_file(source) {
        Ok(syntax) => syntax,
        Err(error) => return vec![format!("invalid Rust source: {error}")],
    };
    let mut visitor = PurityVisitor {
        source_path: path,
        local_modules,
        violations: Vec::new(),
    };
    visitor.visit_file(&syntax);
    visitor.violations.sort();
    visitor.violations.dedup();
    visitor.violations
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
            ["stern", "stern-icons-phosphor", "winit", "pollster", "syn"].contains(&name),
            "{name}"
        );
        if name == "syn" {
            assert!(
                dependency.contains("\"kind\":\"dev\""),
                "syn must remain test-only"
            );
        } else {
            assert!(
                dependency.contains("\"kind\":null"),
                "{name} must remain a normal dependency"
            );
        }
        assert!(
            dependency.contains("\"rename\":null"),
            "renamed dependency: {name}"
        );
    }

    for (path, source) in &sources {
        let violations = structural_violations(path, source, &local_modules);
        assert!(violations.is_empty(), "{}: {violations:?}", path.display());
    }
}

#[test]
fn structural_checker_rejects_material_bypass_forms() {
    let reject = |path: &str, source: &str| {
        let violations = structural_violations(Path::new(path), source, &[]);
        assert!(
            !violations.is_empty(),
            "accepted structural bypass: {source}"
        );
    };

    reject(
        "src/lib.rs",
        "use stern::widgets::Ui as r#Canvas; fn escape(ui: &mut r#Canvas<'_>) { r#Canvas::r#primitive(ui, todo!()); }",
    );
    reject("src/main.rs", "use\nwinit as r#windowing;");
    reject(
        "src/main.rs",
        "pub(crate) use\npollster::{block_on as run};",
    );
    reject("src/lib.rs", "use r#stern_core::WidgetId as Id;");
    reject(
        "src/lib.rs",
        "fn emit(ui: &mut stern::widgets::Ui<'_>) { ui.r#push_semantic_node(todo!()); }",
    );
    reject(
        "src/lib.rs",
        "fn configure() { let r#manual_control = || {}; }",
    );
    reject("src/lib.rs", "type r#ManualWidget<T> = Option<T>;");
    reject("src/lib.rs", "struct r#ManualControl<T>(T);");
    reject("src/main.rs", "struct Innocent<T>(T);");
    reject(
        "src/lib.rs",
        "macro_rules! escape { () => { stern::widgets::Ui::primitive(todo!(), todo!()) } }",
    );
    reject("src/lib.rs", "include_str!(\"hidden.rs\");");
}

#[test]
fn structural_checker_preserves_public_consumer_and_bootstrap_allowances() {
    let public_consumer = "use stern_icons_phosphor as phosphor; use stern::widgets::Button; fn icon() { let _ = phosphor::CHECK; }";
    assert!(structural_violations(Path::new("src/app_model.rs"), public_consumer, &[]).is_empty());

    let bootstrap = "use\nwinit::window::Window; use pollster::block_on; struct NativeShell; impl NativeShell {}";
    assert!(structural_violations(Path::new("src/bin/native_shell.rs"), bootstrap, &[]).is_empty());
}
