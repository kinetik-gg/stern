import { spawn } from "node:child_process";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { dirname, resolve } from "node:path";
import { pathToFileURL } from "node:url";

export const PRESENTATION_EVIDENCE = "native-shell-smoke=pass status=Presented";

const PLATFORM_CONTRACT = Object.freeze({
  windows: { backend: "dx12", runnerOs: "Windows" },
  macos: { backend: "metal", runnerOs: "macOS" },
  linux: { backend: "gl", runnerOs: "Linux" },
});
const PLATFORM_NAMES = Object.freeze(Object.keys(PLATFORM_CONTRACT));
const RECORD_KEYS = Object.freeze([
  "commit",
  "exitCode",
  "expectedBackend",
  "formatVersion",
  "platform",
  "presentationEvidence",
  "runnerArch",
  "runnerOs",
  "timedOut",
]);

function assert(condition, message) {
  if (!condition) throw new Error(message);
}

function assertCommit(commit, label = "commit") {
  assert(typeof commit === "string" && /^[0-9a-f]{40}$/u.test(commit), `${label} must be a 40-character lowercase Git SHA`);
}

export function verifyRecord(record, expectedCommit) {
  assert(record && typeof record === "object" && !Array.isArray(record), "record must be an object");
  assert(JSON.stringify(Object.keys(record).sort()) === JSON.stringify([...RECORD_KEYS].sort()), "record keys do not match the platform-smoke contract");
  assert(record.formatVersion === 1, "record formatVersion must be 1");
  assertCommit(record.commit, "record commit");
  assert(record.commit === expectedCommit, `wrong commit for ${record.platform ?? "unknown platform"}`);

  const contract = PLATFORM_CONTRACT[record.platform];
  assert(contract, `unknown platform: ${record.platform}`);
  assert(record.runnerOs === contract.runnerOs, `wrong runner OS for ${record.platform}`);
  assert(typeof record.runnerArch === "string" && record.runnerArch.length > 0, `missing runner architecture for ${record.platform}`);
  assert(record.expectedBackend === contract.backend, `wrong backend for ${record.platform}`);
  assert(record.timedOut === false, `native shell timed out on ${record.platform}`);
  assert(record.exitCode === 0, `native shell exited nonzero on ${record.platform}`);
  assert(record.presentationEvidence === PRESENTATION_EVIDENCE, `wrong presentation evidence for ${record.platform}`);
}

export function verifyRecords(records, expectedCommit) {
  assertCommit(expectedCommit, "expected commit");
  assert(Array.isArray(records), "records must be an array");

  const counts = new Map();
  for (const record of records) {
    const platform = record?.platform;
    counts.set(platform, (counts.get(platform) ?? 0) + 1);
  }
  for (const [platform, count] of counts) {
    assert(count === 1, `duplicate platform record: ${platform}`);
  }
  for (const platform of PLATFORM_NAMES) {
    assert(counts.get(platform) === 1, `missing platform record: ${platform}`);
  }
  assert(records.length === PLATFORM_NAMES.length, `expected exactly ${PLATFORM_NAMES.length} platform records`);

  for (const record of records) verifyRecord(record, expectedCommit);
  return [...records].sort((left, right) => PLATFORM_NAMES.indexOf(left.platform) - PLATFORM_NAMES.indexOf(right.platform));
}

function appendBounded(current, chunk) {
  const limit = 64 * 1024;
  if (current.length >= limit) return current;
  return (current + chunk.toString()).slice(0, limit);
}

function execute(command, args, timeoutMs) {
  return new Promise((resolveResult) => {
    let stdout = "";
    let stderr = "";
    let finished = false;
    let timedOut = false;
    const child = spawn(command, args, { env: process.env, shell: false, windowsHide: true });
    const timer = setTimeout(() => {
      timedOut = true;
      child.kill("SIGKILL");
    }, timeoutMs);

    const finish = (exitCode) => {
      if (finished) return;
      finished = true;
      clearTimeout(timer);
      resolveResult({ exitCode: timedOut ? 124 : exitCode, stderr, stdout, timedOut });
    };

    child.stdout.on("data", (chunk) => { stdout = appendBounded(stdout, chunk); });
    child.stderr.on("data", (chunk) => { stderr = appendBounded(stderr, chunk); });
    child.on("error", (error) => {
      stderr = appendBounded(stderr, error.message);
      finish(1);
    });
    child.on("close", (code) => finish(code ?? 1));
  });
}

function parseOptions(args) {
  const options = new Map();
  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    assert(key?.startsWith("--") && value !== undefined, `invalid option sequence near ${key ?? "end of arguments"}`);
    if (key === "--input") {
      options.set(key, [...(options.get(key) ?? []), value]);
    } else {
      assert(!options.has(key), `duplicate option: ${key}`);
      options.set(key, value);
    }
  }
  return options;
}

function required(options, name) {
  const value = options.get(name);
  assert(typeof value === "string" && value.length > 0, `missing ${name}`);
  return value;
}

function writeJson(path, value) {
  const absolute = resolve(path);
  mkdirSync(dirname(absolute), { recursive: true });
  writeFileSync(absolute, `${JSON.stringify(value)}\n`, "utf8");
}

async function recordCommand(args) {
  const separator = args.indexOf("--");
  assert(separator >= 0 && separator < args.length - 1, "record requires a command after --");
  const options = parseOptions(args.slice(0, separator));
  const command = args[separator + 1];
  const commandArgs = args.slice(separator + 2);
  const timeoutMs = Number(required(options, "--timeout-ms"));
  assert(Number.isSafeInteger(timeoutMs) && timeoutMs > 0, "--timeout-ms must be a positive integer");

  const platform = required(options, "--platform");
  const commit = required(options, "--commit");
  assertCommit(commit);
  const result = await execute(command, commandArgs, timeoutMs);
  const evidenceLines = result.stdout.split(/\r?\n/u).map((line) => line.trim()).filter((line) => line === PRESENTATION_EVIDENCE);
  const record = {
    formatVersion: 1,
    platform,
    commit,
    runnerOs: required(options, "--runner-os"),
    runnerArch: required(options, "--runner-arch"),
    expectedBackend: required(options, "--backend"),
    exitCode: result.exitCode,
    timedOut: result.timedOut,
    presentationEvidence: evidenceLines.length === 1 ? PRESENTATION_EVIDENCE : null,
  };
  writeJson(required(options, "--output"), record);
  process.stdout.write(result.stdout);
  process.stderr.write(result.stderr);
  verifyRecord(record, commit);
  console.log(`platform smoke record: PASS (${platform}, ${record.expectedBackend}, ${commit})`);
}

function verifyCommand(args) {
  const options = parseOptions(args);
  const commit = required(options, "--commit");
  const inputs = options.get("--input");
  assert(Array.isArray(inputs) && inputs.length > 0, "verify requires at least one --input");
  const records = inputs.map((path) => JSON.parse(readFileSync(resolve(path), "utf8")));
  const verified = verifyRecords(records, commit);
  const output = options.get("--output");
  if (output) writeJson(output, { formatVersion: 1, commit, status: "pass", records: verified });
  console.log(`platform smoke aggregate: PASS (${PLATFORM_NAMES.join(", ")}; ${commit})`);
}

export async function main(args = process.argv.slice(2)) {
  const [command, ...rest] = args;
  if (command === "record") return recordCommand(rest);
  if (command === "verify") return verifyCommand(rest);
  throw new Error("usage: platform-smoke-record.mjs <record|verify> [options]");
}

const invokedPath = process.argv[1] ? pathToFileURL(resolve(process.argv[1])).href : "";
if (invokedPath === import.meta.url) {
  main().catch((error) => {
    console.error(`platform smoke: FAIL: ${error.message}`);
    process.exitCode = 1;
  });
}
