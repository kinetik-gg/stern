//! Public exact spacing token and semantic-role conformance.

#![allow(clippy::float_cmp)]

use std::{fs, path::Path};

use stern_core::{
    Color, ControlMetrics, CornerRadius, DurationScale, ElevationScale, FontToken, OpacityScale,
    RadiusScale, SpacingRole, SpacingScale, SpacingStep, StrokeScale, TypographyScale,
    default_dark_theme,
};

const EXPECTED_STEPS: [SpacingStep; 9] = [
    SpacingStep::Zero,
    SpacingStep::One,
    SpacingStep::Two,
    SpacingStep::Three,
    SpacingStep::Four,
    SpacingStep::Five,
    SpacingStep::Six,
    SpacingStep::Seven,
    SpacingStep::Eight,
];

const EXPECTED_ROLES: [SpacingRole; 9] = [
    SpacingRole::IconLabelGap,
    SpacingRole::TightControlGap,
    SpacingRole::CompactInlineControlPadding,
    SpacingRole::DefaultInlineControlPadding,
    SpacingRole::BlockControlPadding,
    SpacingRole::InspectorLabelValueGap,
    SpacingRole::GroupGap,
    SpacingRole::PanelPadding,
    SpacingRole::SectionGap,
];

const LEGACY_GENERIC_FIELD_NAMES: [&str; 5] = ["xs", "sm", "md", "lg", "xl"];

const SENTINELS: SpacingScale = SpacingScale::new(
    101.0, 103.0, 107.0, 109.0, 113.0, 127.0, 131.0, 137.0, 139.0,
);

#[test]
fn default_scale_exposes_the_exact_nine_step_ladder() {
    let spacing = default_dark_theme().spacing;

    assert_eq!(SpacingStep::ALL, EXPECTED_STEPS.as_slice());
    assert_eq!(
        [
            spacing.zero,
            spacing.one,
            spacing.two,
            spacing.three,
            spacing.four,
            spacing.five,
            spacing.six,
            spacing.seven,
            spacing.eight,
        ],
        [0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0]
    );
    assert_eq!(
        SpacingStep::ALL
            .iter()
            .copied()
            .map(|step| spacing.get(step))
            .collect::<Vec<_>>(),
        vec![0.0, 2.0, 4.0, 6.0, 8.0, 12.0, 16.0, 24.0, 32.0]
    );
}

#[test]
fn typed_step_lookup_routes_nine_independent_sentinels() {
    assert_eq!(
        SpacingStep::ALL
            .iter()
            .copied()
            .map(|step| SENTINELS.get(step))
            .collect::<Vec<_>>(),
        vec![
            101.0, 103.0, 107.0, 109.0, 113.0, 127.0, 131.0, 137.0, 139.0
        ]
    );
}

#[test]
fn every_semantic_role_resolves_through_its_configured_step() {
    let expected = [
        (SpacingRole::IconLabelGap, SpacingStep::Two, 107.0),
        (SpacingRole::TightControlGap, SpacingStep::Two, 107.0),
        (
            SpacingRole::CompactInlineControlPadding,
            SpacingStep::Three,
            109.0,
        ),
        (
            SpacingRole::DefaultInlineControlPadding,
            SpacingStep::Four,
            113.0,
        ),
        (SpacingRole::BlockControlPadding, SpacingStep::Two, 107.0),
        (
            SpacingRole::InspectorLabelValueGap,
            SpacingStep::Four,
            113.0,
        ),
        (SpacingRole::GroupGap, SpacingStep::Four, 113.0),
        (SpacingRole::PanelPadding, SpacingStep::Four, 113.0),
        (SpacingRole::SectionGap, SpacingStep::Six, 131.0),
    ];

    assert_eq!(SpacingRole::ALL, EXPECTED_ROLES.as_slice());
    assert_eq!(expected.len(), SpacingRole::ALL.len());
    for (role, step, value) in expected {
        assert_eq!(role.step(), step);
        assert_eq!(SENTINELS.resolve(role), value);
        assert_eq!(SENTINELS.resolve(role), SENTINELS.get(role.step()));
    }
}

#[test]
fn spacing_replacement_preserves_every_non_spacing_theme_field() {
    let defaults = default_dark_theme();
    let mut base = defaults;
    base.colors.surface.application = Color::rgb8(1, 2, 3);
    base.radii = RadiusScale {
        none: CornerRadius::all(201.0),
        sm: CornerRadius::all(202.0),
        md: CornerRadius::all(203.0),
        lg: CornerRadius::all(204.0),
        full: CornerRadius::all(205.0),
    };
    base.strokes = StrokeScale::from_values(301.0, 302.0, 303.0, 304.0, 305.0);
    base.typography = TypographyScale {
        body: FontToken::new("spacing-isolation-body", 401.0, 402.0),
        label: FontToken::new("spacing-isolation-label", 403.0, 404.0),
        caption: FontToken::new("spacing-isolation-caption", 405.0, 406.0),
        title: FontToken::new("spacing-isolation-title", 407.0, 408.0),
        monospace: FontToken::new("spacing-isolation-monospace", 409.0, 410.0),
    };
    base.opacity = OpacityScale {
        disabled: 0.11,
        hover: 0.12,
        pressed: 0.13,
        selection: 0.14,
        overlay_scrim: 0.15,
    };
    base.elevation = ElevationScale::new(501.0, 502.0, 503.0, 504.0);
    base.duration = DurationScale {
        instant: 601.0,
        fast: 602.0,
        normal: 603.0,
        slow: 604.0,
    };
    base.controls = ControlMetrics {
        control_height: 701.0,
        compact_control_height: 702.0,
        padding_x: 705.0,
        padding_y: 706.0,
    };
    base.radius = CornerRadius::all(801.0);
    base.border_width = 802.0;
    base.text_size = 803.0;

    assert_ne!(base.colors, defaults.colors);
    assert_ne!(base.radii, defaults.radii);
    assert_ne!(base.strokes, defaults.strokes);
    assert_ne!(base.typography, defaults.typography);
    assert_ne!(base.opacity, defaults.opacity);
    assert_ne!(base.elevation, defaults.elevation);
    assert_ne!(base.duration, defaults.duration);
    assert_ne!(base.controls, defaults.controls);
    assert_ne!(base.radius, defaults.radius);
    assert_ne!(base.border_width, defaults.border_width);
    assert_ne!(base.text_size, defaults.text_size);

    let customized = base.with_spacing(SENTINELS);

    assert_eq!(customized.spacing, SENTINELS);
    assert_eq!(customized.colors, base.colors);
    assert_eq!(customized.radii, base.radii);
    assert_eq!(customized.strokes, base.strokes);
    assert_eq!(customized.typography, base.typography);
    assert_eq!(customized.opacity, base.opacity);
    assert_eq!(customized.elevation, base.elevation);
    assert_eq!(customized.duration, base.duration);
    assert_eq!(customized.controls, base.controls);
    assert_eq!(customized.radius, base.radius);
    assert_eq!(customized.border_width, base.border_width);
    assert_eq!(customized.text_size, base.text_size);

    for role in SpacingRole::ALL {
        assert_eq!(customized.spacing.resolve(*role), SENTINELS.resolve(*role));
        assert_ne!(
            customized.spacing.resolve(*role),
            base.spacing.resolve(*role)
        );
    }
}

#[test]
fn legacy_generic_field_audit_is_scoped_to_spacing_scale() {
    let unrelated_size_fields = r"
        pub struct ControlSizeScale {
            pub xs: f32,
            pub sm: f32,
            pub md: f32,
            pub lg: f32,
            pub xl: f32,
        }

        pub struct SpacingScale {
            pub zero: f32,
            pub one: f32,
        }
    ";
    assert!(legacy_generic_spacing_fields(unrelated_size_fields).is_empty());

    let mutated_spacing_scale = r"
        pub struct ControlSizeScale {
            pub xs: f32,
        }

        pub struct SpacingScale {
            pub zero: f32,
            pub xs : core::primitive::f32,
            pub sm: f32,
            pub md: f32,
            pub lg: f32,
            pub xl: f32,
        }
    ";
    assert_eq!(
        legacy_generic_spacing_fields(mutated_spacing_scale),
        vec!["xs", "sm", "md", "lg", "xl"]
    );
}

#[test]
fn production_source_contains_no_legacy_fields_or_five_value_constructors() {
    let crate_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workspace_root = crate_root
        .parent()
        .and_then(Path::parent)
        .expect("stern-core must live under the workspace crates directory");
    let mut sources = Vec::new();
    for production_root in [workspace_root.join("crates"), workspace_root.join("apps")] {
        collect_production_rust_sources(&production_root, &mut sources);
    }
    assert!(
        sources
            .iter()
            .any(|path| path.starts_with(workspace_root.join("crates"))),
        "workspace crate production sources must be audited"
    );
    assert!(
        sources
            .iter()
            .any(|path| path.starts_with(workspace_root.join("apps"))),
        "workspace application production sources must be audited"
    );

    let legacy_accesses = [
        ".spacing.xs",
        ".spacing.sm",
        ".spacing.md",
        ".spacing.lg",
        ".spacing.xl",
        "spacing.xs",
        "spacing.sm",
        "spacing.md",
        "spacing.lg",
        "spacing.xl",
    ];

    for path in &sources {
        let source = fs::read_to_string(path).expect("production Rust source must be readable");
        for legacy in legacy_accesses {
            assert!(
                !source.contains(legacy),
                "legacy spacing surface {legacy:?} remains in {}",
                path.display()
            );
        }
        for count in spacing_constructor_argument_counts(&source) {
            assert_eq!(
                count,
                9,
                "SpacingScale::new in {} must take exactly nine values",
                path.display()
            );
        }
    }

    let tokens_path = crate_root.join("src/theme/tokens.rs");
    let tokens_source = fs::read_to_string(&tokens_path).expect("theme tokens must be readable");
    let legacy_fields = legacy_generic_spacing_fields(&tokens_source);
    assert!(
        legacy_fields.is_empty(),
        "legacy generic fields {legacy_fields:?} remain in the SpacingScale declaration"
    );
}

fn legacy_generic_spacing_fields(source: &str) -> Vec<&'static str> {
    let compact_body: String = spacing_scale_body(source)
        .chars()
        .filter(|character| !character.is_whitespace())
        .collect();
    LEGACY_GENERIC_FIELD_NAMES
        .into_iter()
        .filter(|field| compact_body.contains(&format!("pub{field}:")))
        .collect()
}

fn spacing_scale_body(source: &str) -> &str {
    const DECLARATION: &str = "pub struct SpacingScale";
    let declaration_start = source
        .find(DECLARATION)
        .expect("SpacingScale declaration must exist");
    let opening_brace = source[declaration_start..]
        .find('{')
        .map(|offset| declaration_start + offset)
        .expect("SpacingScale declaration must have a body");
    let body_start = opening_brace + 1;
    let mut depth = 1_usize;
    for (offset, character) in source[body_start..].char_indices() {
        match character {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    return &source[body_start..body_start + offset];
                }
            }
            _ => {}
        }
    }
    panic!("SpacingScale declaration body must close");
}

fn collect_production_rust_sources(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("production source directory must be readable") {
        let path = entry.expect("source entry must be readable").path();
        if path.is_dir() {
            if !is_nonproduction_directory(&path) {
                collect_production_rust_sources(&path, output);
            }
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs")
            && !is_nonproduction_rust_file(&path)
        {
            output.push(path);
        }
    }
}

fn is_nonproduction_directory(path: &Path) -> bool {
    matches!(
        path.file_name().and_then(|name| name.to_str()),
        Some(
            "tests"
                | "test"
                | "testdata"
                | "test-data"
                | "test_data"
                | "benches"
                | "benchmarks"
                | "examples"
                | "fixtures"
                | "snapshots"
                | "goldens"
                | "target"
                | ".runway"
        )
    )
}

fn is_nonproduction_rust_file(path: &Path) -> bool {
    matches!(
        path.file_stem().and_then(|name| name.to_str()),
        Some("test" | "tests")
    )
}

fn spacing_constructor_argument_counts(source: &str) -> Vec<usize> {
    const NEEDLE: &str = "SpacingScale::new(";
    let mut counts = Vec::new();
    let mut remainder = source;

    while let Some(start) = remainder.find(NEEDLE) {
        let arguments = &remainder[start + NEEDLE.len()..];
        let mut depth = 0_usize;
        let mut commas = 0_usize;
        let mut end = None;
        for (index, byte) in arguments.bytes().enumerate() {
            match byte {
                b'(' | b'[' | b'{' => depth += 1,
                b')' if depth == 0 => {
                    end = Some(index);
                    break;
                }
                b')' | b']' | b'}' => depth -= 1,
                b',' if depth == 0 => commas += 1,
                _ => {}
            }
        }
        let end = end.expect("SpacingScale::new call must close");
        let values = &arguments[..end];
        counts.push(if values.trim().is_empty() {
            0
        } else {
            commas + usize::from(!values.trim_end().ends_with(','))
        });
        remainder = &arguments[end + 1..];
    }

    counts
}
