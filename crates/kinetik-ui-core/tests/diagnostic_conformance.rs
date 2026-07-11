//! Structured diagnostic metadata conformance coverage.

use kinetik_ui_core::{
    ClipId, DiagnosticCategory, DiagnosticLocation, DiagnosticSeverity, FrameDiagnostic,
    FrameWarning, InputStreamConflict, Key, KeyEvent, KeyState, LayerId, Modifiers, Primitive,
    Rect, SemanticNode, SemanticRole, SemanticTreeError, TextInputEvent, Transform, UiInputEvent,
    UiTestHarness, WidgetId,
};

fn assert_warning_diagnostic(warning: FrameWarning, expected: FrameDiagnostic) {
    assert_eq!(warning.diagnostic(), expected);
}

#[test]
fn duplicate_widget_id_reports_identity_diagnostic() {
    let mut harness = UiTestHarness::new();

    let (id, output) = harness.run_frame(|ui| {
        let id = ui.id("duplicate");
        ui.register_id(id);
        id
    });

    assert_eq!(
        output.warnings,
        vec![FrameWarning::DuplicateWidgetId { id }]
    );
    assert_eq!(
        output.diagnostics(),
        vec![FrameDiagnostic {
            code: "identity.duplicate_widget_id",
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::Identity,
            location: DiagnosticLocation::Widget(id),
        }]
    );
}

#[test]
fn mixed_input_authorities_report_one_structured_input_diagnostic() {
    let mut harness = UiTestHarness::new();
    harness
        .input_mut()
        .push_event(UiInputEvent::Key(KeyEvent::new(
            Key::Character("a".to_owned()),
            KeyState::Pressed,
            Modifiers::default(),
            false,
        )));
    harness
        .input_mut()
        .text_events
        .push(TextInputEvent::Commit("legacy".to_owned()));

    let ((), output) = harness.run_frame(|_| {});

    assert_eq!(
        output.warnings,
        vec![FrameWarning::InputStreamConflict {
            conflict: InputStreamConflict::TextEvents,
        }]
    );
    assert_eq!(
        output.diagnostics(),
        vec![FrameDiagnostic {
            code: "input.stream_projection_conflict",
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::Input,
            location: DiagnosticLocation::InputStream,
        }]
    );
}

#[test]
fn invalid_semantic_tree_reports_semantic_tree_diagnostic() {
    let mut harness = UiTestHarness::new();
    let root = WidgetId::from_key("root");
    let missing = WidgetId::from_key("missing");
    let expected_error = SemanticTreeError::UnknownChild {
        parent: root,
        child: missing,
    };

    let ((), output) = harness.run_frame(|ui| {
        ui.push_semantic_node(
            SemanticNode::new(root, SemanticRole::Root, Rect::ZERO).with_children([missing]),
        );
    });

    assert_eq!(
        output.warnings,
        vec![FrameWarning::InvalidSemanticTree {
            error: expected_error
        }]
    );
    assert_eq!(
        output.diagnostics(),
        vec![FrameDiagnostic {
            code: "semantics.invalid_tree",
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::SemanticTree,
            location: DiagnosticLocation::SemanticTree,
        }]
    );
}

#[test]
fn primitive_stack_warnings_report_stack_diagnostics() {
    let mut harness = UiTestHarness::new();
    let wrong_clip = ClipId::from_raw(1);
    let open_clip = ClipId::from_raw(2);
    let wrong_layer = LayerId::from_raw(3);
    let open_layer = LayerId::from_raw(4);

    let ((), output) = harness.run_frame(|ui| {
        ui.extend_primitives([
            Primitive::ClipEnd { id: wrong_clip },
            Primitive::ClipBegin {
                id: open_clip,
                rect: Rect::ZERO,
            },
            Primitive::LayerEnd { id: wrong_layer },
            Primitive::LayerBegin { id: open_layer },
            Primitive::TransformEnd,
            Primitive::TransformBegin(Transform::IDENTITY),
        ]);
    });

    assert_eq!(
        output.warnings,
        vec![
            FrameWarning::UnmatchedClipEnd { id: wrong_clip },
            FrameWarning::UnmatchedLayerEnd { id: wrong_layer },
            FrameWarning::UnmatchedTransformEnd,
            FrameWarning::UnclosedClip { id: open_clip },
            FrameWarning::UnclosedLayer { id: open_layer },
            FrameWarning::UnclosedTransforms { count: 1 },
        ]
    );
    assert_eq!(
        output.diagnostics(),
        vec![
            FrameDiagnostic {
                code: "primitive_stack.unmatched_clip_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Clip(wrong_clip),
            },
            FrameDiagnostic {
                code: "primitive_stack.unmatched_layer_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Layer(wrong_layer),
            },
            FrameDiagnostic {
                code: "primitive_stack.unmatched_transform_end",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::TransformStack,
            },
            FrameDiagnostic {
                code: "primitive_stack.unclosed_clip",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Clip(open_clip),
            },
            FrameDiagnostic {
                code: "primitive_stack.unclosed_layer",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::Layer(open_layer),
            },
            FrameDiagnostic {
                code: "primitive_stack.unclosed_transforms",
                severity: DiagnosticSeverity::Warning,
                category: DiagnosticCategory::PrimitiveStack,
                location: DiagnosticLocation::TransformStack,
            },
        ]
    );
}

#[test]
fn frame_output_diagnostics_preserve_warning_order() {
    let mut output = kinetik_ui_core::FrameOutput::new();
    let widget = WidgetId::from_key("duplicate");
    let clip = ClipId::from_raw(7);
    let layer = LayerId::from_raw(8);

    output.push_warning(FrameWarning::DuplicateWidgetId { id: widget });
    output.push_warning(FrameWarning::UnmatchedClipEnd { id: clip });
    output.push_warning(FrameWarning::UnmatchedLayerEnd { id: layer });
    output.push_warning(FrameWarning::UnmatchedTransformEnd);

    let diagnostic_codes = output
        .diagnostics()
        .into_iter()
        .map(|diagnostic| diagnostic.code)
        .collect::<Vec<_>>();

    assert_eq!(
        diagnostic_codes,
        vec![
            "identity.duplicate_widget_id",
            "primitive_stack.unmatched_clip_end",
            "primitive_stack.unmatched_layer_end",
            "primitive_stack.unmatched_transform_end",
        ]
    );
}

#[test]
fn harness_warnings_remain_inspectable_with_diagnostic_metadata() {
    let mut harness = UiTestHarness::new();

    let (id, output) = harness.run_frame(|ui| {
        let id = ui.id("duplicate");
        ui.register_id(id);
        id
    });

    assert_eq!(harness.last_warnings(), Some(output.warnings.as_slice()));
    assert_warning_diagnostic(
        harness.last_warnings().expect("last warnings")[0],
        FrameDiagnostic {
            code: "identity.duplicate_widget_id",
            severity: DiagnosticSeverity::Warning,
            category: DiagnosticCategory::Identity,
            location: DiagnosticLocation::Widget(id),
        },
    );
}
