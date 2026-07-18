import { readFileSync } from "node:fs";

const fixture = JSON.parse(readFileSync(new URL("../tests/fixtures/v1-contract-baseline.json", import.meta.url), "utf8"));
const expected = {
  requiredComponentIds: ["button", "text-field", "dropdown", "selection-controls", "value-controls", "progress-feedback", "overlay-system", "virtual-list", "editor-frame", "workspace-chrome", "dock", "inspector-collections", "node-graph", "timeline", "viewport", "color-picker", "gradient-editor", "content-structure-components", "icon-shortcut-components", "toolbar-components", "menu-components", "command-palette-components", "advanced-editor-fields", "choice-value-components", "feedback-status-components", "overlay-components", "navigation-surface-components", "collection-components", "inspector-components", "editor-chrome-components", "color-components", "timeline-components", "node-components", "viewport-components"],
  workspaces: [
    { id: "edit-workspace", requiredVisibleComponentIds: ["button", "text-field", "dropdown", "selection-controls", "value-controls", "progress-feedback", "virtual-list", "editor-frame", "workspace-chrome", "dock", "inspector-collections", "timeline", "viewport", "content-structure-components", "icon-shortcut-components", "toolbar-components", "advanced-editor-fields", "choice-value-components", "feedback-status-components", "navigation-surface-components", "collection-components", "inspector-components", "editor-chrome-components", "timeline-components", "viewport-components"], requiredInvokedComponentIds: ["overlay-system", "menu-components", "command-palette-components", "overlay-components", "color-picker", "gradient-editor", "color-components"] },
    { id: "graph-workspace", requiredVisibleComponentIds: ["editor-frame", "workspace-chrome", "dock", "node-graph", "node-components", "viewport", "viewport-components", "inspector-components", "toolbar-components", "navigation-surface-components"], requiredInvokedComponentIds: ["menu-components", "command-palette-components", "overlay-system", "overlay-components"] },
  ],
  journeys: [
    { id: "workspace-boot-and-traversal", workspaceId: "edit-workspace", requiredComponentIds: ["editor-frame", "workspace-chrome", "dock", "editor-chrome-components", "navigation-surface-components", "content-structure-components"] },
    { id: "shared-action-projection", workspaceId: "edit-workspace", requiredComponentIds: ["button", "toolbar-components", "menu-components", "command-palette-components", "icon-shortcut-components"] },
    { id: "collection-to-inspector-edit", workspaceId: "edit-workspace", requiredComponentIds: ["virtual-list", "collection-components", "inspector-collections", "inspector-components", "text-field", "advanced-editor-fields", "dropdown", "selection-controls", "value-controls", "choice-value-components"] },
    { id: "timeline-and-viewport-edit", workspaceId: "edit-workspace", requiredComponentIds: ["timeline", "timeline-components", "viewport", "viewport-components", "progress-feedback", "feedback-status-components"] },
    { id: "color-and-gradient-edit", workspaceId: "edit-workspace", requiredComponentIds: ["color-picker", "gradient-editor", "color-components", "overlay-system", "overlay-components"] },
    { id: "graph-connection-edit", workspaceId: "graph-workspace", requiredComponentIds: ["node-graph", "node-components", "dock", "inspector-components", "viewport-components"] },
    { id: "overlay-and-failure-recovery", workspaceId: "edit-workspace", requiredComponentIds: ["overlay-system", "overlay-components", "menu-components", "command-palette-components", "feedback-status-components"] },
  ],
  gateIds: ["public-consumer-boundary", "canonical-component-composition", "complete-component-coverage", "deterministic-user-journeys", "semantic-structure", "renderer-and-scale-quality", "platform-integration", "honest-evidence"],
};

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertStringArray(value, label) {
  assert(Array.isArray(value), `${label} must be an array`);
  assert(value.every((item) => typeof item === "string"), `${label} must contain strings`);
  assert(new Set(value).size === value.length, `${label} must contain unique IDs`);
}

function assertExact(actual, wanted, label) {
  assert(JSON.stringify(actual) === JSON.stringify(wanted), `${label} differs from the pinned v1 contract`);
}

assert(fixture && typeof fixture === "object" && !Array.isArray(fixture), "fixture must be an object");
assertExact(Object.keys(fixture).sort(), ["acceptedComponentIds", "acceptedJourneyIds", "coverageSource", "formatVersion", "gateIds", "journeys", "passedGateIds", "requiredComponentIds", "specificationSha256", "status", "sternVersion", "workspaces"].sort(), "top-level keys");
assert(fixture.formatVersion === 1, "formatVersion must be 1");
assert(fixture.sternVersion === "1.0.0-rc.2.dev", "unexpected Stern version");
assert(fixture.specificationSha256 === "f1d489f6f28b613c0bcfa4490b7855da341457ee20c66c892dc37ebff2d024ed", "unexpected specification hash");
assert(fixture.status === "incomplete", "phase-zero status must remain incomplete");
assert(fixture.coverageSource === "none", "phase-zero coverage source must remain none");
assertStringArray(fixture.requiredComponentIds, "requiredComponentIds");
assertStringArray(fixture.gateIds, "gateIds");
assertExact(fixture.requiredComponentIds, expected.requiredComponentIds, "requiredComponentIds");
assertExact(fixture.workspaces, expected.workspaces, "workspaces");
assertExact(fixture.journeys, expected.journeys, "journeys");
assertExact(fixture.gateIds, expected.gateIds, "gateIds");

const required = new Set(fixture.requiredComponentIds);
const workspaceIds = new Set(fixture.workspaces.map(({ id }) => id));
const journeyIds = new Set(fixture.journeys.map(({ id }) => id));
for (const workspace of fixture.workspaces) {
  assertStringArray(workspace.requiredVisibleComponentIds, `${workspace.id}.requiredVisibleComponentIds`);
  assertStringArray(workspace.requiredInvokedComponentIds, `${workspace.id}.requiredInvokedComponentIds`);
  assert([...workspace.requiredVisibleComponentIds, ...workspace.requiredInvokedComponentIds].every((id) => required.has(id)), `${workspace.id} references an unknown component`);
}
for (const journey of fixture.journeys) {
  assert(workspaceIds.has(journey.workspaceId), `${journey.id} references an unknown workspace`);
  assertStringArray(journey.requiredComponentIds, `${journey.id}.requiredComponentIds`);
  assert(journey.requiredComponentIds.every((id) => required.has(id)), `${journey.id} references an unknown component`);
}
for (const [field, authority] of [["acceptedComponentIds", required], ["acceptedJourneyIds", journeyIds], ["passedGateIds", new Set(fixture.gateIds)]]) {
  assertStringArray(fixture[field], field);
  assert(fixture[field].every((id) => authority.has(id)), `${field} references an unknown ID`);
  assert(fixture[field].length === 0, `${field} cannot claim phase-zero acceptance`);
}
assert(fixture.requiredComponentIds.length === 34, "expected 34 component IDs");
assert(fixture.workspaces.length === 2, "expected two workspaces");
assert(fixture.journeys.length === 7, "expected seven journeys");
assert(fixture.gateIds.length === 8, "expected eight gates");

console.log("demo v1 contract baseline: PASS (34 components, 2 workspaces, 7 journeys, 8 gates; zero accepted)");
