//! Public exact spacing token and semantic-role conformance.

#![allow(clippy::float_cmp)]

use std::{fs, path::Path};

use stern_core::{SpacingRole, SpacingScale, SpacingStep, default_dark_theme};

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
    let base = default_dark_theme();
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
fn production_source_contains_no_legacy_fields_or_five_value_constructors() {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let mut sources = Vec::new();
    collect_rust_sources(&source_root, &mut sources);

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
        "pub xs: f32",
        "pub sm: f32",
        "pub md: f32",
        "pub lg: f32",
        "pub xl: f32",
    ];

    for path in sources {
        let source = fs::read_to_string(&path).expect("production Rust source must be readable");
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
}

fn collect_rust_sources(directory: &Path, output: &mut Vec<std::path::PathBuf>) {
    for entry in fs::read_dir(directory).expect("production source directory must be readable") {
        let path = entry.expect("source entry must be readable").path();
        if path.is_dir() {
            collect_rust_sources(&path, output);
        } else if path.extension().and_then(|extension| extension.to_str()) == Some("rs") {
            output.push(path);
        }
    }
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
