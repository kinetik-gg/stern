pub(super) const VERSION: &str = "1.0.0-rc.2.dev";
pub(super) const SPEC_SHA256: &str =
    "f1d489f6f28b613c0bcfa4490b7855da341457ee20c66c892dc37ebff2d024ed";
pub(super) const COMPONENTS: [&str; 34] = [
    "button",
    "text-field",
    "dropdown",
    "selection-controls",
    "value-controls",
    "progress-feedback",
    "overlay-system",
    "virtual-list",
    "editor-frame",
    "workspace-chrome",
    "dock",
    "inspector-collections",
    "node-graph",
    "timeline",
    "viewport",
    "color-picker",
    "gradient-editor",
    "content-structure-components",
    "icon-shortcut-components",
    "toolbar-components",
    "menu-components",
    "command-palette-components",
    "advanced-editor-fields",
    "choice-value-components",
    "feedback-status-components",
    "overlay-components",
    "navigation-surface-components",
    "collection-components",
    "inspector-components",
    "editor-chrome-components",
    "color-components",
    "timeline-components",
    "node-components",
    "viewport-components",
];
pub(super) const JOURNEYS: [(&str, &str); 7] = [
    ("workspace-boot-and-traversal", "edit-workspace"),
    ("shared-action-projection", "edit-workspace"),
    ("collection-to-inspector-edit", "edit-workspace"),
    ("timeline-and-viewport-edit", "edit-workspace"),
    ("color-and-gradient-edit", "edit-workspace"),
    ("graph-connection-edit", "graph-workspace"),
    ("overlay-and-failure-recovery", "edit-workspace"),
];
pub(super) const GATES: [&str; 8] = [
    "public-consumer-boundary",
    "canonical-component-composition",
    "complete-component-coverage",
    "deterministic-user-journeys",
    "semantic-structure",
    "renderer-and-scale-quality",
    "platform-integration",
    "honest-evidence",
];

pub(super) fn gate_refs(id: &str) -> Vec<&'static str> {
    match id {
        "public-consumer-boundary" => {
            vec![
                "#/publicConsumerAudit",
                "#/primitiveContentSurfaceAllowlist",
            ]
        }
        "canonical-component-composition" => vec!["#/runtime/components", "#/semanticSnapshots"],
        "complete-component-coverage" => vec!["#/runtime/components"],
        "deterministic-user-journeys" => vec!["#/runtime/journeys", "#/logs"],
        "semantic-structure" => vec![
            "#/semanticSnapshots",
            "#/traversalTraces",
            "#/focusRestorationTraces",
        ],
        "renderer-and-scale-quality" => vec!["#/rendererEvidence"],
        "platform-integration" => vec!["#/platformEvidence"],
        _ => vec!["#/source", "#/knownGaps", "#/gates"],
    }
}

pub(super) fn component_workspaces(id: &str) -> Vec<&'static str> {
    let graph = [
        "workspace-chrome",
        "dock",
        "node-graph",
        "toolbar-components",
        "navigation-surface-components",
        "inspector-components",
        "node-components",
    ];
    if ["node-graph", "node-components"].contains(&id) {
        vec!["graph-workspace"]
    } else if graph.contains(&id) {
        vec!["edit-workspace", "graph-workspace"]
    } else {
        vec!["edit-workspace"]
    }
}

pub(super) fn component_refs(id: &str) -> Vec<&'static str> {
    if ["color-picker", "gradient-editor", "color-components"].contains(&id) {
        vec!["#/logs/stateTransitions/5", "#/logs/stateTransitions/6"]
    } else if ["node-graph", "node-components"].contains(&id) {
        vec!["#/semanticSnapshots/1", "#/logs/stateTransitions/4"]
    } else {
        vec!["#/semanticSnapshots/0", "#/logs"]
    }
}

pub(super) fn journey_refs(id: &str) -> Vec<&'static str> {
    match id {
        "workspace-boot-and-traversal" => vec!["#/semanticSnapshots/0", "#/traversalTraces/0"],
        "shared-action-projection" => vec!["#/logs/actions", "#/focusRestorationTraces/0"],
        "collection-to-inspector-edit" => vec![
            "#/logs/stateTransitions/0",
            "#/logs/stateTransitions/1",
            "#/logs/stateTransitions/2",
        ],
        "timeline-and-viewport-edit" => vec!["#/logs/stateTransitions/3", "#/logs/failurePaths/2"],
        "graph-connection-edit" => vec![
            "#/logs/stateTransitions/4",
            "#/logs/failurePaths/0",
            "#/logs/failurePaths/1",
        ],
        "color-and-gradient-edit" => vec![
            "#/logs/stateTransitions/5",
            "#/logs/stateTransitions/6",
            "#/logs/stateTransitions/7",
            "#/focusRestorationTraces/2",
        ],
        "overlay-and-failure-recovery" => vec![
            "#/logs/failurePaths/3",
            "#/logs/stateTransitions/8",
            "#/logs/stateTransitions/9",
            "#/focusRestorationTraces/3",
            "#/focusRestorationTraces/4",
        ],
        _ => vec!["#/source"],
    }
}
