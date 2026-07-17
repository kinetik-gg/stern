//! Public exact typography-foundation token conformance.

#![allow(clippy::float_cmp)]

use std::{fs, path::Path};

use stern_core::{
    FontFeatureScale, FontFeatureToken, FontLineHeightScale, FontLineHeightToken, FontSizeScale,
    FontSizeToken, FontWeightScale, FontWeightToken, TextRole, TypographyScale, default_dark_theme,
};

const EXPECTED_SIZE_TOKENS: [FontSizeToken; 6] = [
    FontSizeToken::Ui,
    FontSizeToken::Dense,
    FontSizeToken::Metadata,
    FontSizeToken::Section,
    FontSizeToken::Dialog,
    FontSizeToken::Heading,
];

const EXPECTED_LINE_HEIGHT_TOKENS: [FontLineHeightToken; 3] = [
    FontLineHeightToken::Ui,
    FontLineHeightToken::Dense,
    FontLineHeightToken::Metadata,
];

const EXPECTED_WEIGHT_TOKENS: [FontWeightToken; 4] = [
    FontWeightToken::Regular,
    FontWeightToken::Medium,
    FontWeightToken::Semibold,
    FontWeightToken::Bold,
];

const EXPECTED_FEATURE_TOKENS: [FontFeatureToken; 1] = [FontFeatureToken::Numeric];

const TEXT_ROLES: [TextRole; 5] = [
    TextRole::Body,
    TextRole::Label,
    TextRole::Caption,
    TextRole::Title,
    TextRole::Monospace,
];

#[test]
fn token_inventories_have_exact_normative_order() {
    assert_eq!(FontSizeToken::ALL, EXPECTED_SIZE_TOKENS.as_slice());
    assert_eq!(
        FontLineHeightToken::ALL,
        EXPECTED_LINE_HEIGHT_TOKENS.as_slice()
    );
    assert_eq!(FontWeightToken::ALL, EXPECTED_WEIGHT_TOKENS.as_slice());
    assert_eq!(FontFeatureToken::ALL, EXPECTED_FEATURE_TOKENS.as_slice());
}

#[test]
fn default_foundation_values_and_storage_types_are_exact() {
    let typography = default_dark_theme().typography;

    let _: f32 = typography.sizes.ui;
    let _: f32 = typography.line_heights.ui;
    let _: u16 = typography.weights.regular;
    let _: &'static str = typography.features.numeric;

    assert_eq!(typography.sizes.get(FontSizeToken::Ui), 12.0);
    assert_eq!(typography.sizes.get(FontSizeToken::Dense), 11.0);
    assert_eq!(typography.sizes.get(FontSizeToken::Metadata), 10.0);
    assert_eq!(typography.sizes.get(FontSizeToken::Section), 14.0);
    assert_eq!(typography.sizes.get(FontSizeToken::Dialog), 16.0);
    assert_eq!(typography.sizes.get(FontSizeToken::Heading), 20.0);

    assert_eq!(typography.line_heights.get(FontLineHeightToken::Ui), 16.0);
    assert_eq!(
        typography.line_heights.get(FontLineHeightToken::Dense),
        15.0
    );
    assert_eq!(
        typography.line_heights.get(FontLineHeightToken::Metadata),
        14.0
    );

    assert_eq!(typography.weights.get(FontWeightToken::Regular), 400);
    assert_eq!(typography.weights.get(FontWeightToken::Medium), 500);
    assert_eq!(typography.weights.get(FontWeightToken::Semibold), 600);
    assert_eq!(typography.weights.get(FontWeightToken::Bold), 700);
    assert_eq!(
        typography.features.get(FontFeatureToken::Numeric),
        "tabular-nums"
    );
}

#[test]
fn typed_lookups_route_every_independent_sentinel() {
    let sizes = FontSizeScale::new(101.0, 103.0, 107.0, 109.0, 113.0, 127.0);
    assert_eq!(sizes.get(FontSizeToken::Ui), 101.0);
    assert_eq!(sizes.get(FontSizeToken::Dense), 103.0);
    assert_eq!(sizes.get(FontSizeToken::Metadata), 107.0);
    assert_eq!(sizes.get(FontSizeToken::Section), 109.0);
    assert_eq!(sizes.get(FontSizeToken::Dialog), 113.0);
    assert_eq!(sizes.get(FontSizeToken::Heading), 127.0);

    let line_heights = FontLineHeightScale::new(131.0, 137.0, 139.0);
    assert_eq!(line_heights.get(FontLineHeightToken::Ui), 131.0);
    assert_eq!(line_heights.get(FontLineHeightToken::Dense), 137.0);
    assert_eq!(line_heights.get(FontLineHeightToken::Metadata), 139.0);

    let weights = FontWeightScale::new(601, 607, 613, 617);
    assert_eq!(weights.get(FontWeightToken::Regular), 601);
    assert_eq!(weights.get(FontWeightToken::Medium), 607);
    assert_eq!(weights.get(FontWeightToken::Semibold), 613);
    assert_eq!(weights.get(FontWeightToken::Bold), 617);

    let features = FontFeatureScale::new("sentinel-tabular-numeric");
    assert_eq!(
        features.get(FontFeatureToken::Numeric),
        "sentinel-tabular-numeric"
    );
}

#[test]
fn replacing_any_foundation_scale_preserves_theme_and_resolved_text_roles() {
    let base_theme = default_dark_theme();
    let base = base_theme.typography;
    let sizes = FontSizeScale::new(201.0, 203.0, 207.0, 209.0, 211.0, 223.0);
    let line_heights = FontLineHeightScale::new(227.0, 229.0, 233.0);
    let weights = FontWeightScale::new(701, 709, 719, 727);
    let features = FontFeatureScale::new("replacement-numeric");
    let variants = [
        TypographyScale { sizes, ..base },
        TypographyScale {
            line_heights,
            ..base
        },
        TypographyScale { weights, ..base },
        TypographyScale { features, ..base },
        TypographyScale {
            sizes,
            line_heights,
            weights,
            features,
            ..base
        },
    ];

    for typography in variants {
        let customized = base_theme.with_typography(typography);

        assert_ne!(typography, base);
        assert_eq!(typography.families, base.families);
        for role in TEXT_ROLES {
            assert_eq!(typography.metrics(role), base.metrics(role));
            assert_eq!(customized.font(role), base_theme.font(role));
        }

        assert_eq!(customized.text_size, base_theme.text_size);
        assert_eq!(customized.colors, base_theme.colors);
        assert_eq!(customized.spacing, base_theme.spacing);
        assert_eq!(customized.sizes, base_theme.sizes);
        assert_eq!(customized.radii, base_theme.radii);
        assert_eq!(customized.strokes, base_theme.strokes);
        assert_eq!(customized.opacity, base_theme.opacity);
        assert_eq!(customized.elevation, base_theme.elevation);
        assert_eq!(customized.duration, base_theme.duration);
        assert_eq!(customized.controls, base_theme.controls);
        assert_eq!(customized.radius, base_theme.radius);
        assert_eq!(customized.border_width, base_theme.border_width);
    }
}

#[test]
fn typography_scale_stores_each_foundation_authority_once() {
    let source = include_str!("../src/theme/tokens.rs");
    let declaration = struct_declaration(source, "TypographyScale");

    for field in [
        "pub sizes: FontSizeScale",
        "pub line_heights: FontLineHeightScale",
        "pub weights: FontWeightScale",
        "pub features: FontFeatureScale",
    ] {
        assert_eq!(
            declaration.matches(field).count(),
            1,
            "expected one storage authority for {field}"
        );
    }
}

#[test]
fn foundation_metadata_does_not_expand_core_resolved_or_primitive_shapes() {
    let token_source = include_str!("../src/theme/tokens.rs");
    let render_source = include_str!("../src/render.rs");
    let declarations = [
        ("FontToken", struct_declaration(token_source, "FontToken")),
        (
            "TextRoleMetrics",
            struct_declaration(token_source, "TextRoleMetrics"),
        ),
        (
            "TextPrimitive",
            struct_declaration(render_source, "TextPrimitive"),
        ),
    ];

    for (name, declaration) in declarations {
        for forbidden in [
            "FontSizeScale",
            "FontSizeToken",
            "FontLineHeightScale",
            "FontLineHeightToken",
            "FontWeightScale",
            "FontWeightToken",
            "FontFeatureScale",
            "FontFeatureToken",
            "pub weight:",
            "pub feature:",
            "pub weights:",
            "pub features:",
        ] {
            assert!(
                !declaration.contains(forbidden),
                "{name} must not transport foundation metadata through {forbidden}"
            );
        }
    }
}

#[test]
fn text_style_transports_exactly_the_bounded_low_level_weight_and_feature_set() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let source = fs::read_to_string(workspace.join("crates/stern-text/src/style.rs"))
        .expect("read stern-text TextStyle source");
    let declaration = struct_declaration(&source, "TextStyle");

    assert_eq!(declaration.matches("pub weight:").count(), 1);
    assert_eq!(declaration.matches("pub weight: u16").count(), 1);
    assert_eq!(declaration.matches("pub features:").count(), 1);
    assert_eq!(
        declaration.matches("pub features: TextFeatureSet").count(),
        1
    );
    for forbidden in [
        "FontSizeScale",
        "FontSizeToken",
        "FontLineHeightScale",
        "FontLineHeightToken",
        "FontWeightScale",
        "FontWeightToken",
        "FontFeatureScale",
        "FontFeatureToken",
        "pub weights:",
        "pub feature:",
    ] {
        assert!(
            !declaration.contains(forbidden),
            "TextStyle must not transport foundation metadata through {forbidden}"
        );
    }
}

#[test]
fn production_numeric_feature_adoption_is_narrow_and_semantically_resolved() {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let roots = [
        workspace.join("crates/stern-text/src"),
        workspace.join("crates/stern-render/src"),
        workspace.join("crates/stern-vello/src"),
        workspace.join("crates/stern-widgets/src"),
        workspace.join("apps/stern-demo/src"),
    ];
    let mut sources = Vec::new();
    for root in roots {
        collect_rust_sources(&root, &mut sources);
    }

    let mut violations = Vec::new();
    for path in sources {
        let source = fs::read_to_string(&path).expect("read production Rust source");
        let relative = path
            .strip_prefix(&workspace)
            .expect("workspace production path")
            .to_string_lossy()
            .replace('\\', "/");
        for forbidden in [
            "tabular-nums",
            "FontWeightScale",
            "FontFeatureScale",
            "FontWeightToken",
            "FontFeatureToken",
            "TextFeatureSet::TABULAR_NUMBERS",
        ] {
            let allowed = match forbidden {
                "tabular-nums" | "FontFeatureScale" => relative == "crates/stern-text/src/style.rs",
                "FontFeatureToken" => matches!(
                    relative.as_str(),
                    "crates/stern-text/src/style.rs"
                        | "crates/stern-widgets/src/components.rs"
                        | "crates/stern-widgets/src/components/numeric_inputs.rs"
                ),
                "TextFeatureSet::TABULAR_NUMBERS" => {
                    relative.starts_with("crates/stern-text/src/")
                        || relative == "crates/stern-vello/src/tests/text_layouts.rs"
                }
                _ => false,
            };
            if source.contains(forbidden) && !allowed {
                violations.push(format!("{relative} contains {forbidden}"));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "numeric feature adoption exceeded its bounded semantic path:\n{}",
        violations.join("\n")
    );

    let numeric =
        fs::read_to_string(workspace.join("crates/stern-widgets/src/components/numeric_inputs.rs"))
            .expect("read numeric component source");
    assert_eq!(
        numeric
            .matches("resolve_semantic(theme.typography.features, FontFeatureToken::Numeric)")
            .count(),
        1,
        "numeric components must resolve the semantic feature exactly once through one helper"
    );
    let fields =
        fs::read_to_string(workspace.join("crates/stern-widgets/src/components/text_fields.rs"))
            .expect("read canonical text-field source");
    let geometry =
        fs::read_to_string(workspace.join("crates/stern-widgets/src/components/text_geometry.rs"))
            .expect("read text-field geometry source");
    assert_eq!(fields.matches(".with_features(features)").count(), 1);
    assert_eq!(geometry.matches(".with_features(features)").count(), 1);
    for required in [
        "build_transient_with_features",
        "resolve_text_navigation",
        "build_with_features",
    ] {
        assert!(
            fields.contains(required),
            "canonical text fields must carry features through {required}"
        );
    }
}

fn struct_declaration<'a>(source: &'a str, name: &str) -> &'a str {
    let marker = format!("pub struct {name} {{");
    let start = source.find(&marker).expect("public struct declaration");
    let declaration = &source[start..];
    let end = declaration
        .find("\n}")
        .expect("public struct declaration end");
    &declaration[..end + 2]
}

fn collect_rust_sources(root: &Path, sources: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(root).expect("read production source directory") {
        let path = entry.expect("read production source entry").path();
        if path.is_dir() {
            collect_rust_sources(&path, sources);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            sources.push(path);
        }
    }
}
