# Icon Atlas Plan

This note captures the preferred path for higher-quality icons without adding
live SVG/vector icon rendering to the Vello frame path.

## Source

Use `@phosphor-icons/core` as an offline source of truth for icon SVGs and
catalog metadata. The package exposes SVG assets grouped by weight under
`assets/<weight>/<icon>-<weight>.svg`, plus a catalog export with stable names,
categories, tags, versions, aliases, and codepoints. The package is MIT
licensed.

Phosphor is suitable as an asset source, not as a runtime renderer dependency.
The Kinetik runtime should continue to consume backend-independent image
resources and atlas regions.

## Recommended Pipeline

1. Select a limited first weight, likely `regular`, plus a small showcase/editor
   icon set.
2. Rasterize selected SVGs offline at fixed target sizes used by the theme,
   starting with 16 px dense controls and 24 px asset/sidebar icons.
3. Pack rasterized icons into one or more RGBA atlases with duplicated edge
   gutters, matching the existing showcase atlas strategy.
4. Generate stable Rust metadata that maps toolkit icon IDs or names to atlas
   source rectangles.
5. Register the atlas as one `ImageResource` and each icon as an atlas-backed
   `ImageResource` with `RenderImageSampling::UiIcon`.

## Why Atlas First

An atlas keeps icon memory compact by sharing one pixel payload across many
icons. It also reduces texture switches and upload churn compared with many
separate image resources. Vello may still encode one image patch per visible
icon, so this should be treated as texture/resource efficient rather than a
guaranteed single draw call.

## Non-Goals

- Do not render Phosphor SVG paths through Vello in normal UI frames.
- Do not put SVG parsing, rasterization, or package-manager access in
  `kinetik-ui-core`.
- Do not start with duotone icons; they require layered color handling and can
  be a later extension.
- Do not replace the renderer resource contract with Phosphor-specific types.

## Acceptance Criteria

- Icons remain backend-independent primitives referencing image handles.
- Generated atlas regions land on integer physical pixels at common scale
  factors.
- Atlas gutters duplicate edge pixels to prevent bleeding.
- The first icon set has deterministic tests for registration, region bounds,
  and destination sizing.
- The offline generator is separate from runtime crates and can be run manually
  or in build tooling without affecting ordinary UI frame cost.

## Implemented Slice

- The showcase editor consumes a 28-icon `regular` Phosphor subset from the
  official `@phosphor-icons/core` package.
- `tools/icon-atlas/generate-phosphor-icons.mjs` rasterizes SVGs to white RGBA8,
  packs guttered per-DPI atlases, and emits `manifest.json`,
  `atlas-<physical-size>.rgba`, `atlas-<physical-size>.png`, and Rust metadata.
- The first generated logical sizes are 16 px for dense/panel icons and 24 px
  for toolbar/asset icons, with exact physical buckets for 1.0x, 1.25x, 1.5x,
  1.75x, and 2.0x scale factors.
- Rebuild with `npm --prefix tools/icon-atlas ci --ignore-scripts` followed by
  `npm --prefix tools/icon-atlas run generate:phosphor`; the tool lockfile pins
  `@phosphor-icons/core` to 2.1.1.
- Runtime tinting happens through `ImagePrimitive::tint`; the renderer caches
  tinted atlas payloads by `(ImageId, tint)` so a color variant is uploaded once
  and reused across all atlas regions.
