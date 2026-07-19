import assert from "node:assert/strict";
import test from "node:test";

import { PRESENTATION_EVIDENCE, verifyRecords } from "./platform-smoke-record.mjs";

const COMMIT = "0123456789abcdef0123456789abcdef01234567";

function record(platform, overrides = {}) {
  const contracts = {
    windows: { backend: "dx12", runnerOs: "Windows" },
    macos: { backend: "metal", runnerOs: "macOS" },
    linux: { backend: "gl", runnerOs: "Linux" },
  };
  return {
    formatVersion: 1,
    platform,
    commit: COMMIT,
    runnerOs: contracts[platform].runnerOs,
    runnerArch: "X64",
    expectedBackend: contracts[platform].backend,
    exitCode: 0,
    timedOut: false,
    presentationEvidence: PRESENTATION_EVIDENCE,
    ...overrides,
  };
}

function validRecords() {
  return [record("windows"), record("macos"), record("linux")];
}

test("accepts exactly Windows DX12, macOS Metal, and Linux GL for one commit", () => {
  assert.deepEqual(verifyRecords(validRecords(), COMMIT).map(({ platform }) => platform), ["windows", "macos", "linux"]);
});

test("rejects a missing platform", () => {
  assert.throws(() => verifyRecords(validRecords().slice(0, 2), COMMIT), /missing platform record: linux/u);
});

test("rejects a duplicate platform", () => {
  const records = validRecords();
  records[2] = record("windows");
  assert.throws(() => verifyRecords(records, COMMIT), /duplicate platform record: windows/u);
});

test("rejects a record from the wrong commit", () => {
  const records = validRecords();
  records[1] = record("macos", { commit: "fedcba9876543210fedcba9876543210fedcba98" });
  assert.throws(() => verifyRecords(records, COMMIT), /wrong commit for macos/u);
});

test("rejects a backend that does not match its platform", () => {
  const records = validRecords();
  records[0] = record("windows", { expectedBackend: "gl" });
  assert.throws(() => verifyRecords(records, COMMIT), /wrong backend for windows/u);
});

test("rejects a nonzero native-shell exit", () => {
  const records = validRecords();
  records[2] = record("linux", { exitCode: 1 });
  assert.throws(() => verifyRecords(records, COMMIT), /native shell exited nonzero on linux/u);
});

test("rejects presentation evidence other than exact Presented output", () => {
  const records = validRecords();
  records[1] = record("macos", { presentationEvidence: "native-shell-smoke=pass status=Deferred" });
  assert.throws(() => verifyRecords(records, COMMIT), /wrong presentation evidence for macos/u);
});
