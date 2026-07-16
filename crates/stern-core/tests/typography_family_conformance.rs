//! Public semantic font-family authority conformance.

#![allow(clippy::float_cmp)]

use std::{fs, path::Path};

use stern_core::{
    FontFamilyRole, FontFamilyScale, TextRole, TextRoleMetrics, TypographyScale, default_dark_theme,
};

const EXPECTED_FAMILY_ROLES: [FontFamilyRole; 3] = [
    FontFamilyRole::Ui,
    FontFamilyRole::Brand,
    FontFamilyRole::Mono,
];

const SENTINEL_FAMILIES: FontFamilyScale =
    FontFamilyScale::new("sentinel-ui", "sentinel-brand", "sentinel-mono");

const SENTINEL_TYPOGRAPHY: TypographyScale = TypographyScale {
    families: SENTINEL_FAMILIES,
    body: TextRoleMetrics::new(101.0, 103.0),
    label: TextRoleMetrics::new(107.0, 109.0),
    caption: TextRoleMetrics::new(113.0, 127.0),
    title: TextRoleMetrics::new(131.0, 137.0),
    monospace: TextRoleMetrics::new(139.0, 149.0),
};

const TEXT_ROLES: [TextRole; 5] = [
    TextRole::Body,
    TextRole::Label,
    TextRole::Caption,
    TextRole::Title,
    TextRole::Monospace,
];

#[test]
fn default_family_scale_has_exact_distinct_role_inventory() {
    let theme = default_dark_theme();

    assert_eq!(FontFamilyRole::ALL, EXPECTED_FAMILY_ROLES.as_slice());
    assert_eq!(theme.font_family(FontFamilyRole::Ui), "Inter");
    assert_eq!(theme.font_family(FontFamilyRole::Brand), "Space Grotesk");
    assert_eq!(theme.font_family(FontFamilyRole::Mono), "Space Mono");
    assert_ne!(
        theme.font_family(FontFamilyRole::Ui),
        theme.font_family(FontFamilyRole::Brand)
    );
    assert_ne!(
        theme.font_family(FontFamilyRole::Ui),
        theme.font_family(FontFamilyRole::Mono)
    );
    assert_ne!(
        theme.font_family(FontFamilyRole::Brand),
        theme.font_family(FontFamilyRole::Mono)
    );
}

#[test]
fn typed_family_lookup_routes_three_independent_sentinels() {
    assert_eq!(SENTINEL_FAMILIES.get(FontFamilyRole::Ui), "sentinel-ui");
    assert_eq!(
        SENTINEL_FAMILIES.get(FontFamilyRole::Brand),
        "sentinel-brand"
    );
    assert_eq!(SENTINEL_FAMILIES.get(FontFamilyRole::Mono), "sentinel-mono");
}

#[test]
fn every_text_role_resolves_one_family_and_its_independent_metrics() {
    let expected = [
        (TextRole::Body, "sentinel-ui", 101.0, 103.0),
        (TextRole::Label, "sentinel-ui", 107.0, 109.0),
        (TextRole::Caption, "sentinel-ui", 113.0, 127.0),
        (TextRole::Title, "sentinel-ui", 131.0, 137.0),
        (TextRole::Monospace, "sentinel-mono", 139.0, 149.0),
    ];

    for (role, family, size, line_height) in expected {
        let token = SENTINEL_TYPOGRAPHY.get(role);
        assert_eq!(token.family, family, "wrong family for {role:?}");
        assert_eq!(token.size, size, "wrong size for {role:?}");
        assert_eq!(
            token.line_height, line_height,
            "wrong line height for {role:?}"
        );
    }
}

#[test]
fn family_customization_preserves_all_text_role_metrics() {
    let defaults = default_dark_theme().typography;
    let customized = TypographyScale {
        families: SENTINEL_FAMILIES,
        ..defaults
    };

    for role in TEXT_ROLES {
        assert_eq!(customized.metrics(role), defaults.metrics(role));
    }
    assert_eq!(customized.get(TextRole::Body).family, "sentinel-ui");
    assert_eq!(customized.get(TextRole::Label).family, "sentinel-ui");
    assert_eq!(customized.get(TextRole::Caption).family, "sentinel-ui");
    assert_eq!(customized.get(TextRole::Title).family, "sentinel-ui");
    assert_eq!(customized.get(TextRole::Monospace).family, "sentinel-mono");
    assert_eq!(customized.family(FontFamilyRole::Brand), "sentinel-brand");
}

#[test]
fn metric_customization_preserves_all_semantic_family_authority() {
    let families = default_dark_theme().typography.families;
    let customized = TypographyScale {
        families,
        ..SENTINEL_TYPOGRAPHY
    };

    assert_eq!(customized.family(FontFamilyRole::Ui), "Inter");
    assert_eq!(customized.family(FontFamilyRole::Brand), "Space Grotesk");
    assert_eq!(customized.family(FontFamilyRole::Mono), "Space Mono");
    assert_eq!(customized.get(TextRole::Body).family, "Inter");
    assert_eq!(customized.get(TextRole::Title).family, "Inter");
    assert_eq!(customized.get(TextRole::Monospace).family, "Space Mono");

    for role in TEXT_ROLES {
        assert_eq!(customized.metrics(role), SENTINEL_TYPOGRAPHY.metrics(role));
    }
}

#[test]
fn typography_replacement_preserves_unrelated_theme_fields_and_body_mirror() {
    let base = default_dark_theme();
    let customized = base.with_typography(SENTINEL_TYPOGRAPHY);

    assert_eq!(customized.typography, SENTINEL_TYPOGRAPHY);
    assert_eq!(customized.text_size, SENTINEL_TYPOGRAPHY.body.size);
    assert_eq!(customized.colors, base.colors);
    assert_eq!(customized.spacing, base.spacing);
    assert_eq!(customized.sizes, base.sizes);
    assert_eq!(customized.radii, base.radii);
    assert_eq!(customized.strokes, base.strokes);
    assert_eq!(customized.opacity, base.opacity);
    assert_eq!(customized.elevation, base.elevation);
    assert_eq!(customized.duration, base.duration);
    assert_eq!(customized.controls, base.controls);
    assert_eq!(customized.radius, base.radius);
    assert_eq!(customized.border_width, base.border_width);
}

#[test]
fn typography_scale_stores_metrics_without_resolved_family_strings() {
    let source = include_str!("../src/theme/tokens.rs");
    let start = source
        .find("pub struct TypographyScale {")
        .expect("TypographyScale declaration");
    let declaration = &source[start..];
    let end = declaration
        .find("\n}\n\nimpl TypographyScale")
        .expect("TypographyScale declaration end");
    let declaration = &declaration[..end + 2];

    assert!(declaration.contains("pub families: FontFamilyScale"));
    assert_eq!(declaration.matches("TextRoleMetrics").count(), 5);
    assert!(!declaration.contains("FontToken"));
    assert!(!declaration.contains("&'static str"));
}

#[test]
fn default_theme_does_not_restore_the_removed_geist_family() {
    let defaults = include_str!("../src/theme/defaults.rs");

    assert!(!defaults.contains("Geist Mono"));
    assert!(
        defaults.contains("FontFamilyScale::new(\"Inter\", \"Space Grotesk\", \"Space Mono\")")
    );
}

#[test]
fn production_widget_and_demo_sources_do_not_embed_normative_family_literals() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let roots = [
        workspace.join("crates/stern-widgets/src"),
        workspace.join("apps/stern-demo/src"),
    ];
    let mut sources = Vec::new();
    for root in roots {
        collect_rust_sources(&root, &mut sources);
    }

    let mut violations = Vec::new();
    for path in sources {
        if is_test_source(&path) {
            continue;
        }
        let source = fs::read_to_string(&path).expect("read production Rust source");
        for literal in ["\"Inter\"", "\"Space Grotesk\"", "\"Space Mono\""] {
            if source.contains(literal) {
                violations.push(format!("{} contains {literal}", path.display()));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "normative families must resolve through the theme:\n{}",
        violations.join("\n")
    );
}

fn collect_rust_sources(root: &Path, sources: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("read source directory") {
        let path = entry.expect("read source entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}

fn is_test_source(path: &Path) -> bool {
    path.file_name().is_some_and(|name| name == "tests.rs")
        || path
            .components()
            .any(|component| component.as_os_str() == "tests")
}
