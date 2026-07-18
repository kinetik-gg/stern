# Static Icon Pipeline

Stern uses generated immutable vector definitions rather than a runtime bitmap
atlas. The historical filename is retained so existing documentation links do
not break.

## Pinned source and provenance

`third-party/phosphor/phosphor-core-2.1.1.tgz` is the exact published
`@phosphor-icons/core` 2.1.1 archive. Its SHA-256 digest, npm SHA-512 integrity,
MIT license, upstream URL, schema notes, and catalog counts are recorded in
`third-party/phosphor/README.md` and `PROVENANCE.toml`. The archive is treated
only as data; Stern executes none of its JavaScript or TypeScript.

The pure-Rust development tool `stern-icon-atlas` validates and normalizes the
pinned archive. Despite its compatibility name, it does not build a raster
atlas. It generates the complete `stern-icons-phosphor` catalog: 1,512 icons
for each of six weights, 9,072 canonical definitions in total, plus the 18
upstream deprecated aliases in every weight namespace.

```text
cargo run -p stern-icon-atlas -- generate
cargo run -p stern-icon-atlas -- check
cargo run -p stern-icon-atlas -- linkage-check
```

Ordinary application builds read only generated Rust source. They perform no
archive inspection, SVG parsing, package-manager access, filesystem I/O,
registration, lookup, or runtime initialization.

## Runtime use

Applications depend on `stern-icons-phosphor` and pass a typed constant
directly to a widget or action boundary:

```rust
use stern_icons_phosphor as phosphor;

ui.icon_button(
    "save",
    save_rect,
    phosphor::regular::FLOPPY_DISK,
    "Save project",
    false,
);

let save = ActionDescriptor::new("file.save", "Save")
    .with_icon(phosphor::regular::FLOPPY_DISK);
```

`PhosphorIcon` converts to the library-neutral `stern_core::StaticIcon` handle.
Widgets emit one `Primitive::Icon` with the borrowed immutable graphic, theme
tint, and resolved destination rectangle. They do not clone or transform path
vectors. Renderers remain Phosphor-neutral and expand the generic primitive.

There is no icon declaration macro, feature-selected icon set, global table,
string lookup, icon registry, DPI bucket, bitmap atlas, or application pack
generation. Each generated definition owns independent statics so release
linking can discard unreferenced icons.

## Bitmap images remain separate

General `ImageId` resources and bitmap image widgets remain supported for
thumbnails, previews, artwork, and application-provided bitmap icons. They are
not a Phosphor presentation path and do not require the generated icon crate.
