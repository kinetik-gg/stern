# Renderer Snapshot Strategy

Kinetik UI renderer tests should prefer deterministic command and resource
snapshots over pixel images. Pixel tests are useful for showcase-level smoke
coverage, but they are brittle across platforms, fonts, GPU drivers, and backend
encoders.

## Snapshot Layers

Use the narrowest stable layer that proves the behavior:

1. Backend-neutral resource snapshots from `RenderResources::snapshot()`.
2. Backend command snapshots from `kinetik-ui-vello::render_translation_snapshot`.
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

## Command Snapshots

Vello command snapshots serialize translated commands after sanitization but
before Vello scene encoding. They include:

- command order
- layer, clip, and transform state
- geometry, brushes, strokes, text, image, and texture references
- recoverable diagnostics

Add or update a command snapshot when a primitive translation contract changes.
Keep backend-neutral tests in `kinetik-ui-render` and Vello-specific snapshots in
`kinetik-ui-vello`.

## Pixel Tests

Pixel tests should not be the default renderer regression strategy. Use them only
when the test can remain stable without a GPU and when the assertion is about
visible end-to-end output rather than primitive translation details.
