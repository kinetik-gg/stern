# stern-icons-phosphor

Complete generated definitions from the pinned `@phosphor-icons/core` 2.1.1
snapshot. The six flat modules (`thin`, `light`, `regular`, `bold`, `fill`, and
`duotone`) expose 1,512 canonical constants and 18 deprecated aliases each.

The generated tree uses deterministic shards capped at 640 source lines. Every
definition owns independent immutable path/layer/graphic statics; there is no
global lookup table, runtime registration, parsing, I/O, allocation, or
initialization. Run `cargo run -p stern-icon-atlas -- check` to verify generated
sources and `cargo run -p stern-icon-atlas -- linkage-check` to inspect release
linkage. The workspace release profile uses thin LTO so LLVM and the platform
linker can discard unreferenced private static definitions across Rust
codegen-unit boundaries. The proof builds in an isolated target directory and
otherwise uses the ordinary release profile.
