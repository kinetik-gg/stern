# Renderer Snapshot Strategy

Stern renderer tests should prefer deterministic command and resource
snapshots over pixel images. Pixel tests are useful for showcase-level smoke
coverage, but they are brittle across platforms, fonts, GPU drivers, and backend
encoders.

## Snapshot Layers

Use the narrowest stable layer that proves the behavior:

1. Backend-neutral resource snapshots from `RenderResources::snapshot()`.
2. Backend command snapshots from `stern-vello::render_translation_snapshot`.
3. Structured assertions for backend scene side effects when command snapshots do
   not prove encoding happened.
4. Pixel/raster tests only for deliberately stable showcase smoke coverage.

## Resource Snapshots

Resource snapshots are sorted by raw handle and include only deterministic
metadata:

- image handle, size, and whether CPU pixels exist
- texture handle, size, and whether a CPU snapshot exists
- shaped text layout handle, size, line count, and glyph count

Do not snapshot raw pixel bytes, font bytes, or backend-native resource objects.
Snapshot numbers are formatted to three decimals. Non-finite values and negative
zero are normalized to `0.000` before text formatting.

The resource snapshot grammar intentionally remains a payload-presence
inventory: image/texture format and alpha metadata are not added to this public
surface. Direct `RenderImage` assertions and private backend upload tests prove
that metadata without exposing payload details or widening the stable snapshot
API.

Resource conformance tests keep expected text inline in the test source. The
inline literal is the source of truth; tests do not bless, update, or rewrite
source files. Matching comparisons do not write artifacts.

When a backend-neutral resource snapshot mismatch occurs, the test helper writes
review artifacts under `target/stern-artifacts/stern-render/resource-snapshots/`:

- `expected.txt`
- `actual.txt`
- `diff.txt`

The panic message includes these paths so a human can inspect the generated
files and then intentionally edit the inline expected literal when the behavior
change is correct.

## Command Snapshots

Vello command snapshots serialize translated commands after sanitization but
before Vello scene encoding. They include:

- command order
- layer, clip, and transform state
- geometry, brushes, strokes, text, image, and texture references
- recoverable diagnostics

Commands stay in translated primitive order, including when nested layer, clip,
and transform scopes are active. Diagnostics are serialized as stable strings.
Colors in command snapshots are the sanitized values that bind to the backend,
not unchecked constructor input. Invalid gradient offsets are diagnosed before
the corresponding stop color, and every invalid color-bearing occurrence emits
one contextual diagnostic.

Add or update a command snapshot when a primitive translation contract changes.
Keep backend-neutral tests in `stern-render` and Vello-specific snapshots in
`stern-vello`.

Command conformance tests keep expected text inline in the test source. The
inline literal or inline constant is the source of truth; tests do not bless,
update, or rewrite source files. Matching comparisons do not write artifacts.

When a Vello command snapshot mismatch occurs, the test helper writes review
artifacts under `target/stern-artifacts/stern-vello/command-snapshots/`:

- `expected.txt`
- `actual.txt`
- `diff.txt`

The panic message includes these paths. To accept an intentional command stream
change, inspect the generated `target/` artifacts and then manually update the
inline expected literal or inline expected constant in the test source. There is
no automatic bless/update workflow for command snapshots.

## Pixel Tests

Pixel tests should not be the default renderer regression strategy. Use them only
when the test can remain stable without a GPU and when the assertion is about
visible end-to-end output rather than primitive translation details.

Color/alpha conformance uses exact CPU byte goldens for straight and
premultiplied RGBA/BGRA tint, explicit Peniko gradient metadata and interpolation
tests, raw encoded-stop assertions, and deterministic solid-color draw words.
GPU pixels and CPU-raster dumps are not authoritative evidence for these
contracts. Vello 0.9's resolved 512-sample gradient ramp is private; its current
sRGB/premultiplied implementation is source-verified residual evidence, while
the executable sentinels stop at public encoded stops.

There is currently no CLI-driven CPU raster review-dump workflow; the demo
binary (`apps/stern-demo/src/main.rs`) only supports
`--dump-identity-evidence`. A showcase-driven raster review workflow remains
future work.
