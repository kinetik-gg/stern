import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import { resolve } from "node:path";

import { verifyEvidence as verifyRendererEvidence } from "../../../tools/capture-demo-vello.mjs";
import { verifyRecords as verifyPlatformRecords } from "./platform-smoke-record.mjs";

const SPEC_SHA256 = "f1d489f6f28b613c0bcfa4490b7855da341457ee20c66c892dc37ebff2d024ed";
const EXPECTED_PACKET_SHA256 = "c75b530b3016ed4509c2d290a30a9eabe16cbb1f1922ffcba517ea654127e71a";
const COMPONENTS = [
  "button", "text-field", "dropdown", "selection-controls", "value-controls",
  "progress-feedback", "overlay-system", "virtual-list", "editor-frame",
  "workspace-chrome", "dock", "inspector-collections", "node-graph", "timeline",
  "viewport", "color-picker", "gradient-editor", "content-structure-components",
  "icon-shortcut-components", "toolbar-components", "menu-components",
  "command-palette-components", "advanced-editor-fields", "choice-value-components",
  "feedback-status-components", "overlay-components", "navigation-surface-components",
  "collection-components", "inspector-components", "editor-chrome-components",
  "color-components", "timeline-components", "node-components", "viewport-components",
];
const JOURNEYS = [
  ["workspace-boot-and-traversal", "edit-workspace"],
  ["shared-action-projection", "edit-workspace"],
  ["collection-to-inspector-edit", "edit-workspace"],
  ["timeline-and-viewport-edit", "edit-workspace"],
  ["color-and-gradient-edit", "edit-workspace"],
  ["graph-connection-edit", "graph-workspace"],
  ["overlay-and-failure-recovery", "edit-workspace"],
];
const GATES = [
  "public-consumer-boundary", "canonical-component-composition",
  "complete-component-coverage", "deterministic-user-journeys", "semantic-structure",
  "renderer-and-scale-quality", "platform-integration", "honest-evidence",
];
const RENDERER_COMPATIBLE_DRIFT = [
  "apps/stern-demo/src/bin/native_shell.rs",
  "apps/stern-demo/tests/native_shell_contract.rs",
  "apps/stern-demo/tools/platform-smoke-record.mjs",
  "apps/stern-demo/tools/platform-smoke-record.test.mjs",
];
const CANDIDATE_EVIDENCE_DRIFT = [
  ".github/workflows/ci.yml",
  "apps/stern-demo/Cargo.toml",
  "apps/stern-demo/src/app_model.rs",
  "apps/stern-demo/src/edit_workspace.rs",
  "apps/stern-demo/src/graph_workspace.rs",
  "apps/stern-demo/src/lib.rs",
  "apps/stern-demo/src/overlay_workspace.rs",
  "apps/stern-demo/src/timeline_workspace.rs",
  "apps/stern-demo/tests/app_model_contract.rs",
  "apps/stern-demo/tests/edit_workspace_screen_contract.rs",
  "apps/stern-demo/tests/evidence/runtime-semantic-evidence.provisional.json",
  "apps/stern-demo/tests/graph_journey_contract.rs",
  "apps/stern-demo/tests/graph_workspace_screen_contract.rs",
  "apps/stern-demo/tests/overlay_recovery_journey_contract.rs",
  "apps/stern-demo/tests/public_consumer_contract.rs",
  "apps/stern-demo/tests/runtime_semantic_evidence.rs",
  "apps/stern-demo/tests/timeline_journey_contract.rs",
  "apps/stern-demo/tools/audit.rs",
  "apps/stern-demo/tools/check-runtime-semantic-evidence.mjs",
  "apps/stern-demo/tools/color-evidence.rs",
  "apps/stern-demo/tools/contract.rs",
  "apps/stern-demo/tools/json.rs",
  "apps/stern-demo/tools/runtime-semantic-evidence.rs",
  "crates/stern-widgets/src/chrome.rs",
  "crates/stern-widgets/src/chrome/application_bar.rs",
  "crates/stern-widgets/src/dock/controller.rs",
  "crates/stern-widgets/src/dock/layout.rs",
  "crates/stern-widgets/src/dock/model.rs",
  "crates/stern-widgets/src/dock/model/tree.rs",
  "crates/stern-widgets/src/dock/scene.rs",
  "crates/stern-widgets/src/lib.rs",
  "crates/stern-widgets/src/ui.rs",
  "crates/stern-widgets/src/ui/chrome.rs",
  "crates/stern-widgets/src/ui/dock.rs",
  "crates/stern-widgets/src/ui/frame.rs",
  "crates/stern-widgets/src/ui/viewport.rs",
  "crates/stern-widgets/src/ui/viewport_tools.rs",
  "crates/stern-widgets/src/viewport/tool_scene.rs",
  "crates/stern-widgets/src/viewport/widget.rs",
  "crates/stern-widgets/tests/application_bar_conformance.rs",
  "crates/stern-widgets/tests/dock_controller_conformance.rs",
  "crates/stern-widgets/tests/dock_layout_conformance.rs",
  "crates/stern-widgets/tests/dock_scene_conformance.rs",
  "crates/stern-widgets/tests/viewport_tool_scene_conformance.rs",
  "crates/stern-widgets/tests/viewport_widget_conformance.rs",
  "crates/stern/tests/application_bar_facade.rs",
  "crates/stern/tests/dock_facade.rs",
  "crates/stern/tests/viewport_facade.rs",
  "evidence/stern-demo-vello-845/manifest.json",
];
const PROVISIONAL_GRAPH_SOURCE_DRIFT = [
  "apps/stern-demo/src/app_model.rs",
  "apps/stern-demo/src/edit_workspace.rs",
  "apps/stern-demo/src/graph_workspace.rs",
  "apps/stern-demo/src/lib.rs",
  "apps/stern-demo/src/overlay_workspace.rs",
];
const PROVISIONAL_GRAPH_CONTRACT_DRIFT = [
  "apps/stern-demo/tests/graph_journey_contract.rs",
  "apps/stern-demo/tests/graph_workspace_screen_contract.rs",
  "apps/stern-demo/tests/public_consumer_contract.rs",
];
const PROVISIONAL_MODEL_COLOR_SOURCE_DRIFT = [
  "apps/stern-demo/src/app_model.rs",
  "apps/stern-demo/src/edit_workspace.rs",
  "apps/stern-demo/src/lib.rs",
];
const PROVISIONAL_MODEL_COLOR_CONTRACT_DRIFT = [
  "apps/stern-demo/tests/app_model_contract.rs",
  "apps/stern-demo/tests/edit_workspace_screen_contract.rs",
];
const PROVISIONAL_TIMELINE_SOURCE_DRIFT = [
  "apps/stern-demo/src/app_model.rs",
  "apps/stern-demo/src/edit_workspace.rs",
  "apps/stern-demo/src/lib.rs",
  "apps/stern-demo/src/timeline_workspace.rs",
];
const PROVISIONAL_TIMELINE_CONTRACT_DRIFT = [
  "apps/stern-demo/tests/timeline_journey_contract.rs",
];
const PROVISIONAL_OVERLAY_RECOVERY_SOURCE_DRIFT = [
  "apps/stern-demo/src/app_model.rs",
  "apps/stern-demo/src/edit_workspace.rs",
  "apps/stern-demo/src/lib.rs",
  "apps/stern-demo/src/overlay_workspace.rs",
];
const PROVISIONAL_OVERLAY_RECOVERY_CONTRACT_DRIFT = [
  "apps/stern-demo/tests/overlay_recovery_journey_contract.rs",
];

const options = parseArgs(process.argv.slice(2));
const evidencePath = resolve(options.evidence ?? fail("--evidence is required"));
const root = resolve(new URL("../../..", import.meta.url).pathname.replace(/^\/([A-Za-z]:)/, "$1"));
const evidenceBytes = readFileSync(evidencePath);
const packetSha256 = createHash("sha256").update(evidenceBytes).digest("hex");
assert(packetSha256 === EXPECTED_PACKET_SHA256,
  `packet integrity: SHA-256 mismatch (expected ${EXPECTED_PACKET_SHA256}, observed ${packetSha256})`);
const evidence = JSON.parse(evidenceBytes.toString("utf8"));
const sourceRef = options.sourceRef ?? evidence.source?.sourceRef;

assertExact(Object.keys(evidence).sort(), [
  "focusRestorationTraces", "formatVersion", "gates", "knownGaps", "logs",
  "platformEvidence", "primitiveContentSurfaceAllowlist", "publicConsumerAudit", "runtime",
  "rendererEvidence", "semanticSnapshots", "source", "specificationSha256", "status", "sternVersion",
  "traversalTraces",
].sort(), "top-level keys");
assert(evidence.formatVersion === 1, "formatVersion must be 1");
assert(evidence.sternVersion === "1.0.0-rc.2.dev", "unexpected Stern version");
assert(evidence.specificationSha256 === SPEC_SHA256, "specification hash mismatch");
assert(["incomplete", "final"].includes(evidence.status), "invalid status");

assert(/^[0-9a-f]{40}$/.test(evidence.source.sourceRef),
  "sourceRef must be one exact lowercase commit ID");
assert(sourceRef === evidence.source.sourceRef,
  "verification source ref must equal the packet's exact sourceRef");
const wantedCommit = git("rev-parse", `${sourceRef}^{commit}`);
const wantedTree = git("rev-parse", `${sourceRef}^{tree}`);
assert(evidence.source.commit === wantedCommit, "source commit is stale or mismatched");
assert(evidence.source.tree === wantedTree, "source tree is stale or mismatched");
assert(git("rev-parse", `${evidence.source.commit}^{tree}`) === evidence.source.tree,
  "recorded source commit does not own recorded tree");
assert(typeof evidence.source.generatedFromCleanWorktree === "boolean",
  "source cleanliness must be explicit");
assertExact(evidence.source.provisionalGraphSourceDrift, PROVISIONAL_GRAPH_SOURCE_DRIFT,
  "provisional Graph production drift");
assertExact(evidence.source.provisionalGraphContractDrift, PROVISIONAL_GRAPH_CONTRACT_DRIFT,
  "provisional Graph contract drift");
assertExact(evidence.source.provisionalModelColorSourceDrift, PROVISIONAL_MODEL_COLOR_SOURCE_DRIFT,
  "provisional model/color production drift");
assertExact(evidence.source.provisionalModelColorContractDrift, PROVISIONAL_MODEL_COLOR_CONTRACT_DRIFT,
  "provisional model/color contract drift");
assertExact(evidence.source.provisionalTimelineSourceDrift, PROVISIONAL_TIMELINE_SOURCE_DRIFT,
  "provisional timeline production drift");
assertExact(evidence.source.provisionalTimelineContractDrift, PROVISIONAL_TIMELINE_CONTRACT_DRIFT,
  "provisional timeline contract drift");
assertExact(evidence.source.provisionalOverlayRecoverySourceDrift,
  PROVISIONAL_OVERLAY_RECOVERY_SOURCE_DRIFT, "provisional overlay-recovery production drift");
assertExact(evidence.source.provisionalOverlayRecoveryContractDrift,
  PROVISIONAL_OVERLAY_RECOVERY_CONTRACT_DRIFT, "provisional overlay-recovery contract drift");
const candidateDrift = git(
  "diff", "--name-only", evidence.source.commit, git("rev-parse", "HEAD^{commit}"),
).split(/\r?\n/u).filter(Boolean);
assertExact(candidateDrift, CANDIDATE_EVIDENCE_DRIFT,
  "canonical-source-to-candidate evidence-only drift");

assertExact(evidence.runtime.components.map(({ id }) => id), COMPONENTS, "component IDs");
for (const component of evidence.runtime.components) {
  assertRecord(component, "component");
  assert(component.status === "passed", `${component.id} must be passed`);
  assertStringArray(component.workspaceIds, `${component.id}.workspaceIds`);
  assert(component.workspaceIds.every((id) => ["edit-workspace", "graph-workspace"].includes(id)),
    `${component.id} references unknown workspace`);
}
assertExact(evidence.runtime.workspaces.map(({ id }) => id),
  ["edit-workspace", "graph-workspace"], "workspace IDs");
assertExact(evidence.runtime.journeys.map(({ id, workspaceId }) => [id, workspaceId]),
  JOURNEYS, "journey contracts");
for (const journey of evidence.runtime.journeys) {
  assertRecord(journey, "journey");
  const expected = journey.id === "graph-connection-edit" ? "pending" : "passed";
  assert(journey.status === expected, `${journey.id} must be ${expected}`);
}

assert(evidence.semanticSnapshots.length === 2, "expected two semantic snapshots");
for (const [index, snapshot] of evidence.semanticSnapshots.entries()) {
  assert(snapshot.workspaceId === evidence.runtime.workspaces[index].id,
    "semantic snapshot workspace order mismatch");
  assert(Array.isArray(snapshot.nodes) && snapshot.nodes.length > 0, "empty semantic snapshot");
  const ids = new Set(snapshot.nodes.map(({ id }) => id));
  assert(ids.size === snapshot.nodes.length, "duplicate semantic node ID");
  assert(ids.has(snapshot.root), "semantic root missing from node set");
  assert(snapshot.focusOrder.every((id) => ids.has(id)), "focus order references missing node");
  for (const node of snapshot.nodes) {
    assert(node.parent === null || ids.has(node.parent), "semantic parent missing from node set");
    assert(node.children.every((id) => ids.has(id)), "semantic child missing from node set");
  }
}
assert(evidence.traversalTraces.length > 0, "missing traversal traces");
assert(evidence.traversalTraces.some(({ input }) => input === "Tab"), "missing Tab traversal trace");
assert(evidence.traversalTraces.every(({ status }) => status === "passed"),
  "every traversal trace must pass");
assert(evidence.focusRestorationTraces.length >= 2, "missing focus restoration traces");
assert(evidence.focusRestorationTraces.every(({ restored }) => restored === true),
  "every focus restoration trace must restore its owner");
const ownerCleanup = evidence.focusRestorationTraces.find(
  ({ interaction }) => interaction === "focus-owner removal cleanup",
) ?? fail("missing focus-owner removal cleanup trace");
assert(ownerCleanup.workspaceId === "edit-to-graph", "owner cleanup must use DemoApp workspaces");
assert(ownerCleanup.scope === "real DemoApp Edit-to-Graph shared-overlay transition",
  "owner cleanup must not use a runtime proxy");
assert(ownerCleanup.actionId === "workspace.graph" && ownerCleanup.actionSource === "Menu",
  "owner cleanup must invoke the shared Graph menu action");
assert(ownerCleanup.actionCount === 1, "owner cleanup must invoke Graph exactly once");
for (const field of ["workspaceChanged", "overlayClosed", "restoredFocusLive", "restored"]) {
  assert(ownerCleanup[field] === true, `focus-owner removal cleanup ${field} must be true`);
}
assert(ownerCleanup.oldOwnerLive === false, "removed Edit focus owner must not remain live");
assert(ownerCleanup.focusOwner !== ownerCleanup.restoredFocus,
  "owner cleanup must recover to a different live Graph focus owner");

const logs = [
  ...evidence.logs.actions,
  ...evidence.logs.stateTransitions,
  ...evidence.logs.failurePaths,
];
assertExact(evidence.logs.actions.map(({ id }) => id), [
  "pointer-apply", "keyboard-apply",
], "action log IDs");
assertExact(evidence.logs.stateTransitions.map(({ id }) => id), [
  "collection-pointer-selection", "collection-keyboard-traversal",
  "collection-keyboard-rename", "timeline-pointer-preview-commit",
  "graph-pointer-connection", "color-picker-cancel-apply",
  "gradient-stable-id-move-reverse", "color-style-explicit-srgb",
  "color-style-save-retry", "overlay-route-exclusivity",
], "state-transition log IDs");
assertExact(evidence.logs.failurePaths.map(({ id }) => id), [
  "graph-incompatible-target", "graph-escape-cancel", "preview-job-failure",
  "color-style-save-failure",
], "failure-path log IDs");
assert(logs.every(({ status }) => status === "passed"),
  "every action, state-transition, and failure-path log must pass");
assert(logs.some(({ input }) => input === "pointer"), "missing pointer log");
assert(logs.some(({ input }) => input === "keyboard"), "missing keyboard log");
assert(evidence.logs.failurePaths.length >= 3, "missing failure-path logs");
assert(evidence.logs.failurePaths.every(({ optimisticMutation }) => optimisticMutation === false),
  "failure paths must reject optimistic mutation explicitly");
const overlayRoute = evidence.logs.stateTransitions.find(
  ({ id }) => id === "overlay-route-exclusivity",
) ?? fail("missing overlay route exclusivity trace");
assertExact(overlayRoute.sequence, ["tooltip", "menu", "palette", "popover", "modal"],
  "shared overlay route sequence");
assert(overlayRoute.exclusive === true && overlayRoute.sharedRoute === true,
  "overlay surfaces must execute exclusively through the shared route");

assert(evidence.publicConsumerAudit.passed === true, "public-consumer audit failed");
assert(evidence.publicConsumerAudit.publicFacadeDependency === true, "public facade missing");
assertExact(evidence.publicConsumerAudit.privateSternDependencies, [], "private dependencies");
assertExact(evidence.publicConsumerAudit.forbiddenSourceMatches, [], "forbidden source matches");
assertExact(evidence.primitiveContentSurfaceAllowlist.map(({ id }) => id), [
  "frame-output-consumption", "viewport-content-surface", "native-render-attachment",
], "primitive/content-surface allowlist");
for (const entry of evidence.primitiveContentSurfaceAllowlist) {
  assertStringArray(entry.allowedPatterns, `${entry.id}.allowedPatterns`);
  assertStringArray(entry.matchedSourcePaths, `${entry.id}.matchedSourcePaths`);
  assert(entry.matchedSourcePaths.length > 0, `${entry.id} has no audited match`);
}

const renderer = verifyRendererEvidence("evidence/stern-demo-vello-845", "final", {
  checkSourceInputs: false,
  requiredReview: "approved",
});
assertExact(evidence.rendererEvidence, {
  issue: 845,
  manifestPath: "evidence/stern-demo-vello-845/manifest.json",
  captureStatus: "final",
  reviewStatus: "approved",
  artifactCount: 8,
  provenance: "prior-baseline",
  currentGraphLayoutStatus: "pending",
  sourceCompatibility: "Graph layout changed; approved bytes are not current candidate acceptance",
}, "renderer evidence record");
const rendererDrift = git(
  "diff", "--name-only", renderer.source.commit, evidence.source.commit,
  "--", ...renderer.source.guarded_paths,
).split(/\r?\n/u).filter(Boolean);
assertExact(rendererDrift, RENDERER_COMPATIBLE_DRIFT,
  "renderer capture-sensitive source compatibility");

const platformCommit = "50edc219ae5d013c242129adf2ec7a25942f5c28";
assertExact(evidence.platformEvidence, {
  issue: 848,
  runId: 29672838723,
  runUrl: "https://github.com/kinetik-gg/stern/actions/runs/29672838723",
  artifactName: "demo-platform-smoke-verified",
  commit: platformCommit,
  status: "pass",
  records: [
    { formatVersion: 1, platform: "windows", commit: platformCommit, runnerOs: "Windows", runnerArch: "X64", expectedBackend: "dx12", exitCode: 0, timedOut: false, presentationEvidence: "native-shell-smoke=pass status=Presented" },
    { formatVersion: 1, platform: "macos", commit: platformCommit, runnerOs: "macOS", runnerArch: "ARM64", expectedBackend: "metal", exitCode: 0, timedOut: false, presentationEvidence: "native-shell-smoke=pass status=Presented" },
    { formatVersion: 1, platform: "linux", commit: platformCommit, runnerOs: "Linux", runnerArch: "X64", expectedBackend: "vulkan", exitCode: 0, timedOut: false, presentationEvidence: "native-shell-smoke=pass status=Presented" },
  ],
}, "platform evidence record");
verifyPlatformRecords(evidence.platformEvidence.records, platformCommit);
assert(git("merge-base", platformCommit, evidence.source.commit) === platformCommit,
  "platform evidence commit is not an ancestor of the packet source");

assertExact(evidence.gates.map(({ id }) => id), GATES, "gate IDs");
for (const gate of evidence.gates) assertRecord(gate, "gate");
assert(gate("deterministic-user-journeys").status === "pending",
  "full-journey acceptance must remain pending");
assert(gate("renderer-and-scale-quality").status === "pending",
  "current Graph renderer/layout acceptance must remain pending");
assert(evidence.gates.filter(({ id }) =>
  !["deterministic-user-journeys", "renderer-and-scale-quality"].includes(id)
).every(({ status }) => status === "passed"), "unexpected non-passing gate");
for (const record of [...evidence.runtime.components, ...evidence.runtime.journeys, ...evidence.gates]) {
  for (const ref of record.evidenceRefs) assert(resolvePointer(evidence, ref) !== undefined,
    `${record.id} evidence link does not resolve: ${ref}`);
}
assertExact(evidence.knownGaps.map(({ id, issue, blocksGateIds }) =>
  ({ id, issue, blocksGateIds })), [
  {
    id: "graph-current-layout-renderer-acceptance",
    issue: 855,
    blocksGateIds: ["renderer-and-scale-quality"],
  },
  {
    id: "graph-full-journey-acceptance",
    issue: 856,
    blocksGateIds: ["deterministic-user-journeys"],
  },
], "provisional Graph known gaps");
for (const gap of evidence.knownGaps) {
  assert(Number.isInteger(gap.issue), "known gap needs issue number");
  assertStringArray(gap.blocksGateIds, `${gap.id}.blocksGateIds`);
  assertStringArray(gap.evidenceRefs, `${gap.id}.evidenceRefs`);
  for (const ref of gap.evidenceRefs) assert(resolvePointer(evidence, ref) !== undefined,
    `${gap.id} evidence link does not resolve: ${ref}`);
  for (const gateId of gap.blocksGateIds) {
    assert(GATES.includes(gateId), `${gap.id} blocks unknown gate`);
    assert(gate(gateId).status !== "passed", `${gateId} passed while ${gap.id} remains open`);
  }
}
const journeyGap = evidence.knownGaps.find(({ issue }) => issue === 856)
  ?? fail("missing issue #856 journey gap");
for (const ref of [
  "#/source/provisionalOverlayRecoverySourceDrift",
  "#/source/provisionalOverlayRecoveryContractDrift",
]) {
  assert(journeyGap.evidenceRefs.includes(ref), `journey gap missing overlay evidence ref: ${ref}`);
}

const allComponents = evidence.runtime.components.every(({ status }) => status === "passed");
const allJourneys = evidence.runtime.journeys.every(({ status }) => status === "passed");
const allGates = evidence.gates.every(({ status }) => status === "passed");
if (evidence.status === "final") {
  assert(evidence.source.generatedFromCleanWorktree, "final evidence needs a clean source worktree");
  assert(allComponents && allJourneys && allGates, "final evidence cannot retain incomplete claims");
  assert(evidence.knownGaps.length === 0, "final evidence cannot retain known gaps");
} else {
  assert(!(allComponents && allJourneys && allGates), "complete evidence must use final status");
  assert(evidence.knownGaps.length > 0, "provisional evidence must retain linked known gaps");
}

console.log(`runtime semantic evidence: PASS (${evidence.runtime.components.length} components, ${evidence.runtime.journeys.length} journeys, ${evidence.semanticSnapshots.length} snapshots; ${evidence.status})`);

function gate(id) {
  return evidence.gates.find((candidate) => candidate.id === id) ?? fail(`missing gate: ${id}`);
}

function assertRecord(record, label) {
  assert(record && typeof record === "object" && !Array.isArray(record), `${label} must be object`);
  assert(["passed", "failed", "notExecuted", "pending"].includes(record.status),
    `${record.id ?? label} has invalid status`);
  assertStringArray(record.evidenceRefs, `${record.id ?? label}.evidenceRefs`);
  assert(record.evidenceRefs.every((ref) => ref.startsWith("#/")),
    `${record.id ?? label} has invalid evidence link`);
}

function assertStringArray(value, label) {
  assert(Array.isArray(value) && value.every((item) => typeof item === "string"),
    `${label} must be a string array`);
  assert(new Set(value).size === value.length, `${label} must be unique`);
}

function resolvePointer(rootValue, pointer) {
  if (pointer === "#") return rootValue;
  return pointer.slice(2).split("/").reduce((value, token) => {
    if (value === undefined || value === null) return undefined;
    const key = token.replaceAll("~1", "/").replaceAll("~0", "~");
    return value[key];
  }, rootValue);
}

function assertExact(actual, expected, label) {
  assert(JSON.stringify(actual) === JSON.stringify(expected), `${label} mismatch`);
}

function assert(condition, message) {
  if (!condition) fail(message);
}

function fail(message) {
  throw new Error(message);
}

function git(...args) {
  return execFileSync("git", args, { cwd: root, encoding: "utf8" }).trim();
}

function parseArgs(args) {
  const parsed = {};
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === "--evidence") parsed.evidence = args[++index];
    else if (args[index] === "--source-ref") parsed.sourceRef = args[++index];
    else fail(`unknown argument: ${args[index]}`);
  }
  return parsed;
}
