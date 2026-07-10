use super::evidence::COMPONENT_EVIDENCE;
use super::types::{
    ComponentCapabilityAxis, ComponentCapabilityEvidence, ComponentConformanceMatrixRow,
    ComponentConformanceStatus, ComponentEvidenceProof, ComponentMetadata,
};

/// Deterministic validation error for a component capability claim.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentConformanceValidationError {
    /// The claim does not declare any axes that Stable would need to prove.
    NoRequiredAxes,
    /// The claim declares the same required axis more than once.
    DuplicateRequiredAxis(ComponentCapabilityAxis),
    /// A capability mapping references an evidence descriptor that is not registered.
    UnknownEvidenceId(&'static str),
    /// A capability mapping references evidence that is not attached to the claim.
    CapabilityEvidenceNotAttached(&'static str),
    /// A capability mapping targets an axis the claim does not require.
    EvidenceForNonRequiredAxis(ComponentCapabilityAxis),
    /// A metadata-only descriptor was mapped as behavioral proof.
    MetadataOnlyEvidence {
        /// Axis the invalid mapping attempted to prove.
        axis: ComponentCapabilityAxis,
        /// Metadata-only evidence descriptor used by the mapping.
        evidence_id: &'static str,
    },
    /// A Stable claim has no accepted behavioral proof for a required axis.
    MissingRequiredAxis(ComponentCapabilityAxis),
}

/// Validates a raw component capability claim against the global evidence registry.
///
/// Every mapping must reference attached behavioral evidence for a declared axis.
/// Stable claims additionally need at least one such mapping for every required
/// axis; Experimental and Planned claims may remain incomplete.
///
/// # Errors
///
/// Returns [`ComponentConformanceValidationError`] when required axes are empty
/// or duplicated, a mapping is invalid, or a Stable claim lacks behavioral
/// evidence for any required axis.
pub fn validate_component_capability_claim(
    status: ComponentConformanceStatus,
    required_axes: &[ComponentCapabilityAxis],
    evidence_ids: &[&'static str],
    capability_evidence: &[ComponentCapabilityEvidence],
) -> Result<(), ComponentConformanceValidationError> {
    if required_axes.is_empty() {
        return Err(ComponentConformanceValidationError::NoRequiredAxes);
    }

    for (index, axis) in required_axes.iter().copied().enumerate() {
        if required_axes[..index].contains(&axis) {
            return Err(ComponentConformanceValidationError::DuplicateRequiredAxis(
                axis,
            ));
        }
    }

    for mapping in capability_evidence {
        let evidence = COMPONENT_EVIDENCE
            .iter()
            .find(|evidence| evidence.id == mapping.evidence_id)
            .ok_or(ComponentConformanceValidationError::UnknownEvidenceId(
                mapping.evidence_id,
            ))?;

        if !evidence_ids.contains(&mapping.evidence_id) {
            return Err(
                ComponentConformanceValidationError::CapabilityEvidenceNotAttached(
                    mapping.evidence_id,
                ),
            );
        }
        if !required_axes.contains(&mapping.axis) {
            return Err(
                ComponentConformanceValidationError::EvidenceForNonRequiredAxis(mapping.axis),
            );
        }
        if evidence.proof == ComponentEvidenceProof::MetadataOnly {
            return Err(ComponentConformanceValidationError::MetadataOnlyEvidence {
                axis: mapping.axis,
                evidence_id: mapping.evidence_id,
            });
        }
    }

    if status.is_stable() {
        for axis in required_axes {
            if !capability_evidence
                .iter()
                .any(|mapping| mapping.axis == *axis)
            {
                return Err(ComponentConformanceValidationError::MissingRequiredAxis(
                    *axis,
                ));
            }
        }
    }

    Ok(())
}

/// Validates the capability claim carried by a component metadata row.
///
/// # Errors
///
/// Returns [`ComponentConformanceValidationError`] when the row violates the
/// raw capability-claim invariants.
pub fn validate_component_metadata(
    metadata: &ComponentMetadata,
) -> Result<(), ComponentConformanceValidationError> {
    validate_component_capability_claim(
        metadata.status,
        metadata.required_axes,
        metadata.evidence_ids,
        metadata.capability_evidence,
    )
}

/// Validates the capability claim carried by a conformance matrix row.
///
/// # Errors
///
/// Returns [`ComponentConformanceValidationError`] when the row violates the
/// raw capability-claim invariants.
pub fn validate_component_conformance_matrix_row(
    row: &ComponentConformanceMatrixRow,
) -> Result<(), ComponentConformanceValidationError> {
    validate_component_capability_claim(
        row.status,
        row.required_axes,
        row.evidence_ids,
        row.capability_evidence,
    )
}
