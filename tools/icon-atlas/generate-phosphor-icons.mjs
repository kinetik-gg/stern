import { execFileSync } from "node:child_process";
import { mkdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { tmpdir } from "node:os";
import { fileURLToPath } from "node:url";

const TOOL_ROOT = dirname(fileURLToPath(import.meta.url));
const PADDING = 1;
const COLUMNS = 7;
const ATLAS_ID_BASE = 70_000;
const ICON_ID_BASE = 71_000;
const LOGICAL_ICON_SIZES = [16, 24];
const SCALE_FACTORS = [1, 1.25, 1.5, 1.75, 2];
const PHYSICAL_ICON_SIZES = Array.from(
  new Set(LOGICAL_ICON_SIZES.flatMap((logical) => SCALE_FACTORS.map((scale) => Math.round(logical * scale)))),
).sort((a, b) => a - b);

const ICONS = [
  ["Cursor", "ICON_CURSOR", "cursor", "cursor"],
  ["Move", "ICON_MOVE", "arrows-out-cardinal", "move"],
  ["Transform", "ICON_TRANSFORM", "bounding-box", "transform"],
  ["Rotate", "ICON_ROTATE", "arrow-clockwise", "rotate"],
  ["Cube", "ICON_CUBE", "cube", "cube"],
  ["Play", "ICON_PLAY", "play", "play"],
  ["Pause", "ICON_PAUSE", "pause", "pause"],
  ["Stop", "ICON_STOP", "stop", "stop"],
  ["Plus", "ICON_PLUS", "plus", "plus"],
  ["Search", "ICON_SEARCH", "magnifying-glass", "search"],
  ["Archive", "ICON_ARCHIVE", "archive", "archive"],
  ["File", "ICON_FILE", "file", "file"],
  ["Image", "ICON_IMAGE", "image", "image"],
  ["Gear", "ICON_GEAR", "gear", "gear"],
  ["Grid", "ICON_GRID", "grid-four", "grid"],
  ["Layers", "ICON_LAYERS", "stack", "layers"],
  ["Code", "ICON_CODE", "code", "code"],
  ["Box", "ICON_BOX", "package", "package"],
  ["Rocket", "ICON_ROCKET", "rocket", "rocket"],
  ["Download", "ICON_DOWNLOAD", "download", "download"],
  ["Dots", "ICON_DOTS", "dots-three", "more"],
  ["Chevron", "ICON_CHEVRON", "caret-right", "chevron-right"],
  ["Caret", "ICON_CARET", "caret-down", "caret-down"],
  ["Reset", "ICON_RESET", "arrow-counter-clockwise", "reset"],
  ["Component", "ICON_COMPONENT", "circles-four", "component"],
  ["Tokens", "ICON_TOKENS", "swatches", "tokens"],
  ["Eye", "ICON_EYE", "eye", "visibility"],
  ["Crosshair", "ICON_CROSSHAIR", "crosshair", "crosshair"],
];

function parseArgs(argv) {
  const args = new Map();
  for (let i = 2; i < argv.length; i += 2) {
    if (!argv[i]?.startsWith("--")) throw new Error(`Expected --flag, got ${argv[i]}`);
    args.set(argv[i].slice(2), argv[i + 1]);
  }
  return args;
}

function atlasId(physicalSize) {
  return ATLAS_ID_BASE + physicalSize;
}

function imageId(iconIndex, logicalSize, physicalSize) {
  return ICON_ID_BASE + iconIndex * 1000 + logicalSize * 10 + physicalSize;
}

function cellSize(physicalSize) {
  return physicalSize + PADDING * 2;
}

function rasterizeIcon({ sourceRoot, tmpRoot, icon, physicalSize }) {
  const [, symbol, sourceName] = icon;
  const svgPath = join(sourceRoot, "assets", "regular", `${sourceName}.svg`);
  let svg = readFileSync(svgPath, "utf8");
  svg = svg.replaceAll("currentColor", "white");
  const tempSvg = join(tmpRoot, `${symbol}-${physicalSize}.svg`);
  const tempRgba = join(tmpRoot, `${symbol}-${physicalSize}.rgba`);
  writeFileSync(tempSvg, svg);
  execFileSync(
    "magick",
    [
      "-background",
      "none",
      tempSvg,
      "-resize",
      `${physicalSize}x${physicalSize}`,
      "-depth",
      "8",
      tempRgba,
    ],
    { stdio: "inherit" },
  );
  const bytes = readFileSync(tempRgba);
  const expected = physicalSize * physicalSize * 4;
  if (bytes.length !== expected) {
    throw new Error(`${sourceName}@${physicalSize} produced ${bytes.length} bytes, expected ${expected}`);
  }
  return bytes;
}

function copyPixel(source, sourceWidth, sx, sy, dest, destWidth, dx, dy) {
  const sourceIndex = (sy * sourceWidth + sx) * 4;
  const destIndex = (dy * destWidth + dx) * 4;
  dest[destIndex] = source[sourceIndex];
  dest[destIndex + 1] = source[sourceIndex + 1];
  dest[destIndex + 2] = source[sourceIndex + 2];
  dest[destIndex + 3] = source[sourceIndex + 3];
}

function packAtlas(rasters, physicalSize) {
  const rows = Math.ceil(ICONS.length / COLUMNS);
  const cell = cellSize(physicalSize);
  const width = COLUMNS * cell;
  const height = rows * cell;
  const atlas = Buffer.alloc(width * height * 4);
  for (let index = 0; index < rasters.length; index += 1) {
    const raster = rasters[index];
    const column = index % COLUMNS;
    const row = Math.floor(index / COLUMNS);
    const x0 = column * cell;
    const y0 = row * cell;
    for (let y = 0; y < cell; y += 1) {
      const sy = Math.max(0, Math.min(physicalSize - 1, y - PADDING));
      for (let x = 0; x < cell; x += 1) {
        const sx = Math.max(0, Math.min(physicalSize - 1, x - PADDING));
        copyPixel(raster, physicalSize, sx, sy, atlas, width, x0 + x, y0 + y);
      }
    }
  }
  return { atlas, width, height, rows };
}

function atlasRecords(atlases) {
  return atlases.map(({ physicalSize, width, height, rows }) => ({
    idRaw: atlasId(physicalSize),
    physicalSize,
    padding: PADDING,
    cellSize: cellSize(physicalSize),
    columns: COLUMNS,
    rows,
    atlasWidth: width,
    atlasHeight: height,
    rgba: `atlas-${physicalSize}.rgba`,
    png: `atlas-${physicalSize}.png`,
  }));
}

function iconVariantRecords() {
  return ICONS.map(([variant, symbol, sourceName, alias], iconIndex) => ({
    variant,
    symbol,
    alias,
    sourceName,
    sourceSvg: `assets/regular/${sourceName}.svg`,
    variants: LOGICAL_ICON_SIZES.flatMap((logicalSize) =>
      SCALE_FACTORS.map((scaleFactor) => {
        const physicalSize = Math.round(logicalSize * scaleFactor);
        const cell = cellSize(physicalSize);
        const column = iconIndex % COLUMNS;
        const row = Math.floor(iconIndex / COLUMNS);
        return {
          idRaw: imageId(iconIndex, logicalSize, physicalSize),
          logicalSize,
          physicalSize,
          scaleFactor,
          atlas: atlasId(physicalSize),
          sourceRect: {
            x: column * cell + PADDING,
            y: row * cell + PADDING,
            width: physicalSize,
            height: physicalSize,
          },
        };
      }),
    ),
  }));
}

function writeManifest({ outputRoot, packageJson, atlases }) {
  const manifest = {
    schemaVersion: 2,
    source: {
      package: packageJson.name,
      version: packageJson.version,
      repository: packageJson.repository,
      license: packageJson.license,
      weight: "regular",
      pathLayout: "assets/regular/<icon>.svg",
    },
    raster: {
      format: "rgba8",
      colorModel: "white-alpha mask; runtime tint applies color",
      logicalIconSizes: LOGICAL_ICON_SIZES,
      scaleFactors: SCALE_FACTORS,
      physicalIconSizes: PHYSICAL_ICON_SIZES,
    },
    atlases: atlasRecords(atlases),
    icons: iconVariantRecords(),
  };
  writeFileSync(join(outputRoot, "manifest.json"), `${JSON.stringify(manifest, null, 2)}\n`);
}

function rustEnumVariants() {
  return ICONS.map(([variant]) => `    ${variant},`);
}

function writeRustMetadata({ outputRoot, atlases }) {
  const lines = [];
  lines.push("// @generated by tools/icon-atlas/generate-phosphor-icons.mjs");
  lines.push("");
  lines.push("use kinetik_ui::core::{ImageId, Rect};");
  lines.push("");
  lines.push("#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
  lines.push("pub(crate) enum PhosphorIcon {");
  lines.push(...rustEnumVariants());
  lines.push("}");
  lines.push("");
  lines.push("pub(crate) struct PhosphorAtlasEntry {");
  lines.push("    pub image: ImageId,");
  lines.push("    pub physical_size: u32,");
  lines.push("    pub width: u32,");
  lines.push("    pub height: u32,");
  lines.push("    pub bytes: &'static [u8],");
  lines.push("}");
  lines.push("");
  lines.push("pub(crate) struct PhosphorIconEntry {");
  lines.push("    pub icon: PhosphorIcon,");
  lines.push("    pub image: ImageId,");
  lines.push("    pub symbol: &'static str,");
  lines.push("    pub source_name: &'static str,");
  lines.push("    pub logical_size: u32,");
  lines.push("    pub physical_size: u32,");
  lines.push("    pub atlas: ImageId,");
  lines.push("    pub source: Rect,");
  lines.push("}");
  lines.push("");
  lines.push(`pub(crate) const DENSE_ICON_LOGICAL_SIZE: u32 = ${LOGICAL_ICON_SIZES[0]};`);
  lines.push(`pub(crate) const STANDARD_ICON_LOGICAL_SIZE: u32 = ${LOGICAL_ICON_SIZES[1]};`);
  lines.push(`pub(crate) const ICON_ATLAS_PADDING: u32 = ${PADDING};`);
  lines.push(`pub(crate) const ICON_ATLAS_COLUMNS: u32 = ${COLUMNS};`);
  lines.push(`pub(crate) const ICON_COUNT: usize = ${ICONS.length};`);
  lines.push("");
  for (const { physicalSize, width, height } of atlases) {
    lines.push(`pub(crate) const ICON_ATLAS_${physicalSize}: ImageId = ImageId::from_raw(${atlasId(physicalSize)});`);
    lines.push(`pub(crate) const ICON_ATLAS_${physicalSize}_WIDTH: u32 = ${width};`);
    lines.push(`pub(crate) const ICON_ATLAS_${physicalSize}_HEIGHT: u32 = ${height};`);
    lines.push(`pub(crate) const ICON_ATLAS_${physicalSize}_BYTES: &[u8] = include_bytes!(concat!(env!("CARGO_MANIFEST_DIR"), "/assets/icons/phosphor/atlas-${physicalSize}.rgba"));`);
  }
  lines.push("");
  lines.push("pub(crate) const ICON_ATLASES: &[PhosphorAtlasEntry] = &[");
  for (const { physicalSize, width, height } of atlases) {
    lines.push("    PhosphorAtlasEntry {");
    lines.push(`        image: ICON_ATLAS_${physicalSize},`);
    lines.push(`        physical_size: ${physicalSize},`);
    lines.push(`        width: ${width},`);
    lines.push(`        height: ${height},`);
    lines.push(`        bytes: ICON_ATLAS_${physicalSize}_BYTES,`);
    lines.push("    },");
  }
  lines.push("];");
  lines.push("");
  lines.push("pub(crate) const ICON_ENTRIES: &[PhosphorIconEntry] = &[");
  for (const [variant, symbol, sourceName] of ICONS) {
    const iconIndex = ICONS.findIndex((icon) => icon[0] === variant);
    for (const logicalSize of LOGICAL_ICON_SIZES) {
      for (const scaleFactor of SCALE_FACTORS) {
        const physicalSize = Math.round(logicalSize * scaleFactor);
        const cell = cellSize(physicalSize);
        const column = iconIndex % COLUMNS;
        const row = Math.floor(iconIndex / COLUMNS);
        const x = column * cell + PADDING;
        const y = row * cell + PADDING;
        lines.push("    PhosphorIconEntry {");
        lines.push(`        icon: PhosphorIcon::${variant},`);
        lines.push(`        image: ImageId::from_raw(${imageId(iconIndex, logicalSize, physicalSize)}),`);
        lines.push(`        symbol: "${symbol}",`);
        lines.push(`        source_name: "${sourceName}",`);
        lines.push(`        logical_size: ${logicalSize},`);
        lines.push(`        physical_size: ${physicalSize},`);
        lines.push(`        atlas: ICON_ATLAS_${physicalSize},`);
        lines.push(`        source: Rect::new(${x}.0, ${y}.0, ${physicalSize}.0, ${physicalSize}.0),`);
        lines.push("    },");
      }
    }
  }
  lines.push("];");
  lines.push("");
  lines.push("pub(crate) fn icon_image(icon: PhosphorIcon, logical_size: f32, scale_factor: f64) -> ImageId {");
  lines.push("    let logical_size = nearest_logical_icon_size(logical_size);");
  lines.push("    let physical_size = nearest_physical_icon_size(logical_size, scale_factor);");
  lines.push("    ICON_ENTRIES");
  lines.push("        .iter()");
  lines.push("        .find(|entry| entry.icon == icon && entry.logical_size == logical_size && entry.physical_size == physical_size)");
  lines.push("        .map_or_else(|| ICON_ENTRIES[0].image, |entry| entry.image)");
  lines.push("}");
  lines.push("");
  lines.push("fn nearest_logical_icon_size(size: f32) -> u32 {");
  lines.push("    if size <= 20.0 { DENSE_ICON_LOGICAL_SIZE } else { STANDARD_ICON_LOGICAL_SIZE }");
  lines.push("}");
  lines.push("");
  lines.push("fn nearest_physical_icon_size(logical_size: u32, scale_factor: f64) -> u32 {");
  lines.push("    let scale = if scale_factor.is_finite() && scale_factor > 0.0 { scale_factor } else { 1.0 };");
  lines.push("    let target = (f64::from(logical_size) * scale).round();");
  lines.push("    if let Some(entry) = ICON_ENTRIES");
  lines.push("        .iter()");
  lines.push("        .filter(|entry| entry.logical_size == logical_size)");
  lines.push("        .filter(|entry| f64::from(entry.physical_size) >= target)");
  lines.push("        .min_by_key(|entry| entry.physical_size)");
  lines.push("    {");
  lines.push("        return entry.physical_size;");
  lines.push("    }");
  lines.push("    ICON_ENTRIES");
  lines.push("        .iter()");
  lines.push("        .filter(|entry| entry.logical_size == logical_size)");
  lines.push("        .max_by_key(|entry| entry.physical_size)");
  lines.push("        .map_or(logical_size, |entry| entry.physical_size)");
  lines.push("}");
  writeFileSync(join(outputRoot, "phosphor_icons.rs"), `${lines.join("\n")}\n`);
}

const args = parseArgs(process.argv);
const sourceRoot = resolve(args.get("source") ?? join(TOOL_ROOT, "node_modules", "@phosphor-icons", "core"));
const outputRoot = resolve(args.get("output") ?? "apps/kinetik-ui-showcase/assets/icons/phosphor");
const packageJson = JSON.parse(readFileSync(join(sourceRoot, "package.json"), "utf8"));
mkdirSync(outputRoot, { recursive: true });
const tmpRoot = join(tmpdir(), `kinetik-phosphor-${Date.now()}`);
mkdirSync(tmpRoot, { recursive: true });
try {
  const atlases = [];
  for (const physicalSize of PHYSICAL_ICON_SIZES) {
    const rasters = ICONS.map((icon) => rasterizeIcon({ sourceRoot, tmpRoot, icon, physicalSize }));
    const { atlas, width, height, rows } = packAtlas(rasters, physicalSize);
    const atlasPath = join(outputRoot, `atlas-${physicalSize}.rgba`);
    const atlasPngPath = join(outputRoot, `atlas-${physicalSize}.png`);
    writeFileSync(atlasPath, atlas);
    execFileSync("magick", ["-size", `${width}x${height}`, "-depth", "8", `rgba:${atlasPath}`, atlasPngPath], {
      stdio: "inherit",
    });
    atlases.push({ physicalSize, width, height, rows });
  }
  writeManifest({ outputRoot, packageJson, atlases });
  writeRustMetadata({ outputRoot, atlases });
  console.log(`Generated ${ICONS.length} Phosphor icons across ${atlases.length} atlases into ${outputRoot}`);
} finally {
  rmSync(tmpRoot, { recursive: true, force: true });
}

