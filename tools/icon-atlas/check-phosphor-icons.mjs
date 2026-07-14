import { execFileSync } from "node:child_process";
import { mkdtempSync, readFileSync, readdirSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const TOOL_ROOT = dirname(fileURLToPath(import.meta.url));
const ACCEPTED_ROOT = resolve(TOOL_ROOT, "../../apps/stern-demo/assets/icons/phosphor");
const REQUIRED_RELEASE_SCALES = [1, 1.25, 1.5, 2];

function validateReleaseScaleCoverage(root) {
  const manifest = JSON.parse(readFileSync(join(root, "manifest.json"), "utf8"));
  const declaredScales = new Set(manifest.raster.scaleFactors);
  const atlasesById = new Map(manifest.atlases.map((atlas) => [atlas.idRaw, atlas]));

  for (const scale of REQUIRED_RELEASE_SCALES) {
    if (!declaredScales.has(scale)) {
      throw new Error(`icon manifest is missing required release scale ${scale}`);
    }
  }

  for (const icon of manifest.icons) {
    for (const scale of REQUIRED_RELEASE_SCALES) {
      const variants = icon.variants.filter((variant) => variant.scaleFactor === scale);
      for (const logicalSize of manifest.raster.logicalIconSizes) {
        const variant = variants.find((candidate) => candidate.logicalSize === logicalSize);
        if (!variant) {
          throw new Error(`${icon.variant} is missing ${logicalSize}@${scale}x`);
        }

        const requiredPhysicalSize = Math.ceil(logicalSize * scale);
        const atlas = atlasesById.get(variant.atlas);
        if (
          variant.physicalSize < requiredPhysicalSize ||
          variant.sourceRect.width < requiredPhysicalSize ||
          variant.sourceRect.height < requiredPhysicalSize ||
          !atlas ||
          atlas.physicalSize < requiredPhysicalSize
        ) {
          throw new Error(
            `${icon.variant} ${logicalSize}@${scale}x requires at least ${requiredPhysicalSize}px`,
          );
        }
      }
    }
  }
}

function filesUnder(root, directory = root) {
  const files = [];
  for (const entry of readdirSync(directory, { withFileTypes: true })) {
    const path = join(directory, entry.name);
    if (entry.isDirectory()) {
      files.push(...filesUnder(root, path));
    } else if (entry.isFile()) {
      files.push(relative(root, path).replaceAll("\\", "/"));
    }
  }
  return files.sort((left, right) => left.localeCompare(right));
}

const disposableRoot = mkdtempSync(join(tmpdir(), "stern-phosphor-check-"));
try {
  validateReleaseScaleCoverage(ACCEPTED_ROOT);
  execFileSync(
    process.execPath,
    [join(TOOL_ROOT, "generate-phosphor-icons.mjs"), "--output", disposableRoot],
    { cwd: TOOL_ROOT, stdio: "inherit", windowsHide: true },
  );
  validateReleaseScaleCoverage(disposableRoot);

  const acceptedFiles = filesUnder(ACCEPTED_ROOT);
  const generatedFiles = filesUnder(disposableRoot);
  if (JSON.stringify(generatedFiles) !== JSON.stringify(acceptedFiles)) {
    throw new Error("generated icon inventory drifted");
  }
  for (const path of acceptedFiles) {
    if (!readFileSync(join(ACCEPTED_ROOT, path)).equals(readFileSync(join(disposableRoot, path)))) {
      throw new Error(`generated icon content drifted: ${path}`);
    }
  }
  process.stdout.write(`Phosphor icon atlas check passed (${acceptedFiles.length} files)\n`);
} finally {
  rmSync(disposableRoot, { recursive: true, force: true });
}
