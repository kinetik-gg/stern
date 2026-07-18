#!/usr/bin/env node

import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, lstatSync, readdirSync, readFileSync, realpathSync } from "node:fs";
import { isAbsolute, join, relative, resolve, sep } from "node:path";
import { TextDecoder } from "node:util";

const FORBIDDEN_MARKERS = Object.freeze([
  Object.freeze({
    normalizedLength: 7,
    sha256: "d1f8b80df0f04f9fb9d2301f8811562cc16010891492393f88ec8c3a4f2890a5",
  }),
  Object.freeze({
    normalizedLength: 9,
    sha256: "7c0ddb175d6f4cf0264ff2a86c84492826e61450889b5c78d108ffa6ecd1c563",
  }),
]);

const CANONICAL_PACKAGES = Object.freeze([
  Object.freeze({ name: "stern", crate: "stern", path: "crates/stern" }),
  Object.freeze({ name: "stern-core", crate: "stern_core", path: "crates/stern-core" }),
  Object.freeze({ name: "stern-icon-atlas", crate: "stern_icon_atlas", path: "crates/stern-icon-atlas" }),
  Object.freeze({ name: "stern-icons-phosphor", crate: "stern_icons_phosphor", path: "crates/stern-icons-phosphor" }),
  Object.freeze({ name: "stern-render", crate: "stern_render", path: "crates/stern-render" }),
  Object.freeze({ name: "stern-text", crate: "stern_text", path: "crates/stern-text" }),
  Object.freeze({ name: "stern-vello", crate: "stern_vello", path: "crates/stern-vello" }),
  Object.freeze({
    name: "stern-vello-winit",
    crate: "stern_vello_winit",
    path: "crates/stern-vello-winit",
  }),
  Object.freeze({ name: "stern-widgets", crate: "stern_widgets", path: "crates/stern-widgets" }),
  Object.freeze({ name: "stern-winit", crate: "stern_winit", path: "crates/stern-winit" }),
  Object.freeze({ name: "stern-demo", crate: "stern_demo", path: "apps/stern-demo" }),
]);

const UTF8 = new TextDecoder("utf-8", { fatal: true });

function parseArgs(argv) {
  const parsed = { metadata: [] };
  for (let index = 0; index < argv.length; index += 1) {
    const argument = argv[index];
    if (argument === "--self-test") {
      parsed.selfTest = true;
      continue;
    }
    if (!["--root", "--scope", "--path", "--metadata", "--configured-repository"].includes(argument)) {
      throw new Error("unknown argument");
    }
    const value = argv[index + 1];
    if (value === undefined) {
      throw new Error("missing argument value");
    }
    index += 1;
    if (argument === "--metadata") {
      parsed.metadata.push(value);
    } else {
      parsed[argument.slice(2).replace("-repository", "Repository")] = value;
    }
  }
  return parsed;
}

function asciiCaseFold(value) {
  return value.replace(/[A-Z]/g, (letter) => String.fromCharCode(letter.charCodeAt(0) + 32));
}

function sha256(value) {
  return createHash("sha256").update(value, "utf8").digest("hex");
}

function firstForbiddenMatch(value) {
  const folded = asciiCaseFold(value);
  const runs = folded.match(/[a-z0-9]+/g) ?? [];
  for (const run of runs) {
    for (const marker of FORBIDDEN_MARKERS) {
      if (run.length < marker.normalizedLength) {
        continue;
      }
      for (let offset = 0; offset <= run.length - marker.normalizedLength; offset += 1) {
        const digest = sha256(run.slice(offset, offset + marker.normalizedLength));
        if (digest === marker.sha256) {
          return marker;
        }
      }
    }
  }
  return undefined;
}

function escapeRegex(value) {
  return value.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
}

function parseConfiguredRepository(value) {
  const match = /^([A-Za-z0-9_.-]+)\/([A-Za-z0-9_.-]+)$/.exec(value ?? "");
  if (!match || match[2] !== "stern") {
    throw new Error("configured repository must name the canonical repository");
  }
  return { owner: match[1], slug: match[2], repository: value };
}

function maskOwner(value, owner) {
  return value.replaceAll(owner, "x".repeat(owner.length));
}

function maskCanonicalRepositoryUrls(value, configured) {
  const owner = escapeRegex(configured.owner);
  const slug = escapeRegex(configured.slug);
  const patterns = [
    new RegExp(`https://github\\.com/${owner}/${slug}(?=$|[/?#\\s\"'\\)\\]>,.;:])`, "g"),
    new RegExp(`git@github\\.com:${owner}/${slug}\\.git(?=$|[\\s\"'\\)\\]>,.;:])`, "g"),
    new RegExp(`ssh://git@github\\.com/${owner}/${slug}\\.git(?=$|[\\s\"'\\)\\]>,.;:])`, "g"),
  ];
  let masked = value;
  for (const pattern of patterns) {
    masked = masked.replace(pattern, (match) => maskOwner(match, configured.owner));
  }
  return masked;
}

function maskMetadataValue(key, value, configured) {
  if (key === "repository" && value === configured.repository) {
    return maskOwner(value, configured.owner);
  }
  const exactRemotes = new Set([
    `https://github.com/${configured.repository}.git`,
    `git@github.com:${configured.repository}.git`,
    `ssh://git@github.com/${configured.repository}.git`,
  ]);
  if (key === "origin" && exactRemotes.has(value)) {
    return maskOwner(value, configured.owner);
  }
  return value;
}

function safeSurface(kind, path) {
  if (firstForbiddenMatch(path)) {
    return { kind, pathSha256: sha256(path.replaceAll("\\", "/")) };
  }
  return { kind, path: path.replaceAll("\\", "/") };
}

function matchRecord(kind, path, marker) {
  return {
    ...safeSurface(kind, path),
    normalizedLength: marker.normalizedLength,
    markerHash: marker.sha256,
  };
}

function scanPath(path, kind = "path") {
  const marker = firstForbiddenMatch(path.replaceAll("\\", "/"));
  return marker ? matchRecord(kind, path, marker) : undefined;
}

function scanText(text, path, configured, kind = "content") {
  const masked = maskCanonicalRepositoryUrls(text, configured);
  const marker = firstForbiddenMatch(masked);
  return marker ? matchRecord(kind, path, marker) : undefined;
}

function decodeUtf8(bytes) {
  if (bytes.includes(0)) {
    return undefined;
  }
  try {
    return UTF8.decode(bytes);
  } catch {
    return undefined;
  }
}

function assertInsideRoot(root, path) {
  const resolved = resolve(path);
  if (resolved !== root && !resolved.startsWith(`${root}${sep}`)) {
    throw new Error("scan path escaped root");
  }
  return resolved;
}

function effectiveTrackedFiles(root) {
  const output = execFileSync("git", ["ls-files", "-co", "--exclude-standard", "-z"], {
    cwd: root,
    encoding: "utf8",
    windowsHide: true,
  });
  return [...new Set(output.split("\0").filter(Boolean))]
    .filter((path) => existsSync(join(root, path)) && lstatSync(join(root, path)).isFile())
    .sort((left, right) => left.localeCompare(right));
}

function filesystemEntries(root, start) {
  const entries = [];
  function visit(path) {
    const stat = lstatSync(path);
    entries.push({ path, isFile: stat.isFile() });
    if (!stat.isDirectory()) {
      return;
    }
    for (const name of readdirSync(path).sort((left, right) => left.localeCompare(right))) {
      visit(join(path, name));
    }
  }
  visit(assertInsideRoot(root, start));
  return entries;
}

function scanFiles(root, entries, configured, scope) {
  let textFileCount = 0;
  for (const entry of entries) {
    const absolutePath = isAbsolute(entry.path) ? entry.path : join(root, entry.path);
    const relativePath = relative(root, absolutePath) || ".";
    const pathFailure = scanPath(relativePath, `${scope}-path`);
    if (pathFailure) {
      return { failure: pathFailure, textFileCount };
    }
    if (!entry.isFile) {
      continue;
    }
    const text = decodeUtf8(readFileSync(absolutePath));
    if (text === undefined) {
      continue;
    }
    textFileCount += 1;
    const contentFailure = scanText(text, relativePath, configured, `${scope}-content`);
    if (contentFailure) {
      return { failure: contentFailure, textFileCount };
    }
  }
  return { textFileCount };
}

function scanMetadata(entries, configured) {
  for (const entry of entries) {
    const separator = entry.indexOf("=");
    if (separator <= 0) {
      throw new Error("metadata must use key=value");
    }
    const key = entry.slice(0, separator);
    const value = entry.slice(separator + 1);
    const masked = maskMetadataValue(key, value, configured);
    const marker = firstForbiddenMatch(masked);
    if (marker) {
      return {
        kind: "metadata",
        keySha256: sha256(key),
        normalizedLength: marker.normalizedLength,
        markerHash: marker.sha256,
      };
    }
  }
  return undefined;
}

function runInventoryTest(root) {
  const metadata = JSON.parse(execFileSync(
    "cargo",
    ["metadata", "--locked", "--no-deps", "--format-version", "1"],
    { cwd: root, encoding: "utf8", windowsHide: true },
  ));
  const actual = metadata.packages
    .map((pkg) => ({
      name: pkg.name,
      crate: pkg.targets.find((target) => target.kind.includes("lib"))?.name ?? pkg.targets[0].name,
      path: relative(root, resolve(pkg.manifest_path, "..")).replaceAll("\\", "/"),
    }))
    .sort((left, right) => left.path.localeCompare(right.path));
  const expected = [...CANONICAL_PACKAGES].sort((left, right) => left.path.localeCompare(right.path));
  assert.deepEqual(actual, expected);
}

function runUnitTests(root) {
  const base = String.fromCharCode(107, 105, 110, 101, 116, 105, 107);
  const suffix = String.fromCharCode(117, 105);
  const configured = parseConfiguredRepository(`${base}-gg/stern`);

  assert.equal(firstForbiddenMatch(base.toUpperCase())?.normalizedLength, 7);
  assert.equal(firstForbiddenMatch(`${base}-${suffix}`)?.normalizedLength, 7);
  const record = matchRecord("test", "clean", firstForbiddenMatch(base));
  assert.equal(JSON.stringify(record).includes(base), false);
  assert.equal(scanText(`https://github.com/${configured.repository}`, "clean", configured), undefined);
  assert.equal(scanText(`https://github.com/${configured.repository}/issues/1`, "clean", configured), undefined);
  assert.ok(scanText(`https://example.com/${configured.repository}`, "clean", configured));
  assert.ok(scanPath(`fixtures/${base.toUpperCase()}.txt`));
  assert.ok(scanText(UTF8.decode(Buffer.from(`prefix-${base}-suffix`, "utf8")), "clean", configured));
  assert.equal(scanText("Stern — UTF-8", "clean", configured), undefined);
  assert.equal(maskMetadataValue("repository", configured.repository, configured).includes(configured.owner), false);
  assert.ok(firstForbiddenMatch(maskMetadataValue("owner", configured.owner, configured)));
  runInventoryTest(root);
}

function main() {
  const args = parseArgs(process.argv.slice(2));
  const root = realpathSync(resolve(args.root ?? "."));
  if (args.selfTest) {
    runUnitTests(root);
    process.stdout.write(`${JSON.stringify({ status: "pass", suite: "identity-scan", tests: 11 })}\n`);
    return;
  }

  const configured = parseConfiguredRepository(args.configuredRepository);
  let result;
  if (args.scope === "tracked") {
    const entries = effectiveTrackedFiles(root).map((path) => ({ path, isFile: true }));
    result = scanFiles(root, entries, configured, "tracked");
  } else if (args.scope === "filesystem") {
    if (!args.path) {
      throw new Error("filesystem scope requires path");
    }
    const start = assertInsideRoot(root, join(root, args.path));
    result = scanFiles(root, filesystemEntries(root, start), configured, "filesystem");
  } else if (args.scope === "metadata") {
    const failure = scanMetadata(args.metadata, configured);
    result = failure ? { failure, textFileCount: 0 } : { textFileCount: 0 };
  } else {
    throw new Error("scope must be tracked, filesystem, or metadata");
  }

  if (result.failure) {
    process.stderr.write(`${JSON.stringify({ status: "fail", match: result.failure })}\n`);
    process.exitCode = 1;
    return;
  }
  process.stdout.write(`${JSON.stringify({ status: "pass", scope: args.scope, textFiles: result.textFileCount })}\n`);
}

try {
  main();
} catch (error) {
  process.stderr.write(`${JSON.stringify({ status: "error", message: error instanceof Error ? error.message : "scan failed" })}\n`);
  process.exitCode = 2;
}
