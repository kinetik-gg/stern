use super::{
    COMPONENT_CONFORMANCE_MATRIX, COMPONENT_EVIDENCE, COMPONENT_METADATA, ComponentCapabilityAxis,
    ComponentCapabilityEvidence, ComponentConformanceStatus, ComponentConformanceValidationError,
    ComponentEvidenceProof, component_evidence, validate_component_capability_claim,
    validate_component_conformance_matrix_row, validate_component_metadata,
};

use ComponentCapabilityAxis::{Accessibility, Input, LiveWorkflow, Model, Paint, Platform};

const ALL_AXES: &[ComponentCapabilityAxis] =
    &[Model, Paint, Input, Accessibility, Platform, LiveWorkflow];
const BEHAVIORAL_EVIDENCE_IDS: &[&str] = &[
    "conformance.outliner-contracts",
    "conformance.asset-browser-contracts",
    "conformance.inline-edit-contracts",
    "conformance.collection-drag-context-contracts",
    "conformance.collection-projection-contracts",
    "conformance.timeline-contracts",
];
const ALL_AXIS_EVIDENCE: &[ComponentCapabilityEvidence] = &[
    ComponentCapabilityEvidence::new(Model, "conformance.outliner-contracts"),
    ComponentCapabilityEvidence::new(Paint, "conformance.asset-browser-contracts"),
    ComponentCapabilityEvidence::new(Input, "conformance.inline-edit-contracts"),
    ComponentCapabilityEvidence::new(
        Accessibility,
        "conformance.collection-drag-context-contracts",
    ),
    ComponentCapabilityEvidence::new(Platform, "conformance.collection-projection-contracts"),
    ComponentCapabilityEvidence::new(LiveWorkflow, "conformance.timeline-contracts"),
];

#[test]
fn stable_claim_requires_behavioral_proof_for_every_axis() {
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Stable,
            ALL_AXES,
            BEHAVIORAL_EVIDENCE_IDS,
            ALL_AXIS_EVIDENCE,
        ),
        Ok(())
    );

    for missing_axis in ALL_AXES {
        let mappings = ALL_AXIS_EVIDENCE
            .iter()
            .copied()
            .filter(|mapping| mapping.axis != *missing_axis)
            .collect::<Vec<_>>();

        assert_eq!(
            validate_component_capability_claim(
                ComponentConformanceStatus::Stable,
                ALL_AXES,
                BEHAVIORAL_EVIDENCE_IDS,
                &mappings,
            ),
            Err(ComponentConformanceValidationError::MissingRequiredAxis(
                *missing_axis,
            )),
            "missing {missing_axis:?}"
        );
    }
}

#[test]
fn metadata_only_evidence_cannot_prove_any_axis() {
    for axis in ALL_AXES {
        let mapping = [ComponentCapabilityEvidence::new(
            *axis,
            "showcase.metadata-only",
        )];

        assert_eq!(
            validate_component_capability_claim(
                ComponentConformanceStatus::Experimental,
                ALL_AXES,
                &["showcase.metadata-only"],
                &mapping,
            ),
            Err(ComponentConformanceValidationError::MetadataOnlyEvidence {
                axis: *axis,
                evidence_id: "showcase.metadata-only",
            }),
            "metadata-only mapping for {axis:?}"
        );
    }
}

#[test]
fn malformed_claims_report_specific_validation_errors() {
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            &[],
            &[],
            &[],
        ),
        Err(ComponentConformanceValidationError::NoRequiredAxes)
    );
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            &[Model, Paint, Model],
            &[],
            &[],
        ),
        Err(ComponentConformanceValidationError::DuplicateRequiredAxis(
            Model,
        ))
    );
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            &[Model],
            &["conformance.unknown-contracts"],
            &[ComponentCapabilityEvidence::new(
                Model,
                "conformance.unknown-contracts",
            )],
        ),
        Err(ComponentConformanceValidationError::UnknownEvidenceId(
            "conformance.unknown-contracts",
        ))
    );
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            &[Model],
            &["status.experimental-public-surface"],
            &[ComponentCapabilityEvidence::new(
                Model,
                "conformance.outliner-contracts",
            )],
        ),
        Err(
            ComponentConformanceValidationError::CapabilityEvidenceNotAttached(
                "conformance.outliner-contracts",
            ),
        )
    );
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            &[Model],
            &["conformance.asset-browser-contracts"],
            &[ComponentCapabilityEvidence::new(
                Paint,
                "conformance.asset-browser-contracts",
            )],
        ),
        Err(ComponentConformanceValidationError::EvidenceForNonRequiredAxis(Paint),)
    );
}

#[test]
fn incomplete_experimental_claim_is_valid_but_not_stable() {
    assert_eq!(
        validate_component_capability_claim(
            ComponentConformanceStatus::Experimental,
            ALL_AXES,
            &["conformance.outliner-contracts"],
            &[ComponentCapabilityEvidence::new(
                Model,
                "conformance.outliner-contracts",
            )],
        ),
        Ok(())
    );
    assert!(!ComponentConformanceStatus::Experimental.is_stable());
    assert!(ComponentConformanceStatus::Stable.is_stable());
    assert!(!ComponentConformanceStatus::Planned.is_stable());
}

#[test]
fn current_metadata_rows_are_valid_experimental_claims_with_explicit_profiles() {
    assert_eq!(COMPONENT_METADATA.len(), 55);
    assert_eq!(
        COMPONENT_METADATA
            .iter()
            .filter(|metadata| metadata.status.is_stable())
            .count(),
        0
    );

    let display = [
        "Label",
        "Image",
        "Separator",
        "Panel",
        "Tooltip",
        "Ruler",
        "ProgressIndicator",
    ];
    let shell_control = [
        "NumericInput",
        "NumericScrubInput",
        "TextField",
        "MultiLineTextField",
        "SearchField",
        "PathField",
    ];
    let workflow = [
        "Outliner",
        "AssetBrowser",
        "PropertyGrid",
        "PropertyAffordanceControls",
        "Vector2Field",
        "Vector3Field",
        "Vector4Field",
        "ColorField",
        "SelectField",
        "AssetSlotField",
        "Viewport",
        "ViewportTools",
        "ViewportActionRouting",
        "NodeGraph",
        "Timeline",
        "TransportControls",
        "StatusBar",
        "JobList",
        "DiagnosticStrip",
        "FeedbackStack",
    ];

    for metadata in COMPONENT_METADATA {
        assert_eq!(metadata.status, ComponentConformanceStatus::Experimental);
        assert_eq!(
            validate_component_metadata(metadata),
            Ok(()),
            "{metadata:?}"
        );

        let expected_axes: &[ComponentCapabilityAxis] = if display.contains(&metadata.name) {
            &[Model, Paint, Accessibility]
        } else if shell_control.contains(&metadata.name) {
            &[Model, Paint, Input, Accessibility, Platform]
        } else if workflow.contains(&metadata.name) {
            ALL_AXES
        } else {
            &[Model, Paint, Input, Accessibility]
        };
        assert_eq!(metadata.required_axes, expected_axes, "{}", metadata.name);
    }
}

#[test]
fn current_matrix_rows_require_all_axes_and_prove_model_only() {
    assert_eq!(COMPONENT_CONFORMANCE_MATRIX.len(), 18);
    assert_eq!(
        COMPONENT_CONFORMANCE_MATRIX
            .iter()
            .filter(|row| row.status.is_stable())
            .count(),
        0
    );

    for row in COMPONENT_CONFORMANCE_MATRIX {
        assert_eq!(row.status, ComponentConformanceStatus::Experimental);
        assert_eq!(row.required_axes, ALL_AXES, "{}", row.slug);
        assert_eq!(row.capability_evidence.len(), 1, "{}", row.slug);
        assert_eq!(row.capability_evidence[0].axis, Model, "{}", row.slug);
        assert!(
            row.evidence_ids
                .contains(&row.capability_evidence[0].evidence_id),
            "{}",
            row.slug
        );
        assert_eq!(
            component_evidence(row.capability_evidence[0].evidence_id)
                .map(|evidence| evidence.proof),
            Some(ComponentEvidenceProof::Behavioral),
            "{}",
            row.slug
        );
        assert_eq!(
            validate_component_conformance_matrix_row(row),
            Ok(()),
            "{row:?}"
        );
    }
}

#[test]
fn evidence_strength_and_status_vocabulary_are_explicit() {
    for evidence in COMPONENT_EVIDENCE {
        let expected = if evidence.id.starts_with("conformance.")
            && evidence.id != "conformance.component-taxonomy"
        {
            ComponentEvidenceProof::Behavioral
        } else {
            ComponentEvidenceProof::MetadataOnly
        };
        assert_eq!(evidence.proof, expected, "{}", evidence.id);
    }

    assert_eq!(
        [
            ComponentConformanceStatus::Stable.as_str(),
            ComponentConformanceStatus::Experimental.as_str(),
            ComponentConformanceStatus::Planned.as_str(),
        ],
        ["Stable", "Experimental", "Planned"]
    );
}
