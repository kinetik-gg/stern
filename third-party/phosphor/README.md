# Phosphor Icons source snapshot

`phosphor-core-2.1.1.tgz` is the exact published npm archive for
`@phosphor-icons/core` 2.1.1. Stern treats it as data: no JavaScript or
TypeScript in the archive is executed.

- Source: <https://registry.npmjs.org/@phosphor-icons/core/-/core-2.1.1.tgz>
- SHA-256: `313332be6190b724da24107addd781799b48bf76b13963f24501112ffe1baadd`
- npm integrity: `sha512-v4ARvrip4qBCImOE5rmPUylOEK4iiED9ZyKjcvzuezqMaiRASCHKcRIuvvxL/twvLpkfnEODCOJp5dM4eZilxQ==`
- Package: `@phosphor-icons/core`
- Version: `2.1.1`
- License: MIT (see `LICENSE`)
- Upstream repository: <https://github.com/phosphor-icons/phosphor-core>

The canonical catalog tuple comes from `package/dist/icons.d.ts`; its
`IconEntry` schema is declared in `package/dist/types.d.ts`. The schema contains
name, Pascal name, optional deprecated alias, categories, Figma category, tags,
codepoint, published version, and updated version. It has no RTL or mirroring
field; Stern records RTL metadata as absent, without guessing from icon names
or shapes.

The six source weights are `thin`, `light`, `regular`, `bold`, `fill`, and
`duotone`. The archive contains 1,512 canonical icons in each weight, for
9,072 SVG assets total, and 18 deprecated aliases in the catalog.

The pure-Rust `stern-icon-atlas` development tool validates this snapshot and
generates `stern-icons-phosphor`; its historical crate name does not imply a
raster atlas. Run `cargo run -p stern-icon-atlas -- check` to verify the
generated tree. Applications consume typed constants directly as borrowed
`StaticIcon` handles and do not inspect this archive at build or runtime.
