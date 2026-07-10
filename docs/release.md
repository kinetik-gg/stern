# Release Policy

Kinetik UI uses SemVer and Conventional Commits.

## Versioning

Versions follow:

```text
MAJOR.MINOR.PATCH[-PRERELEASE]
```

Examples:

```text
0.1.0-alpha.1
0.2.0
1.0.0
1.1.3
```

Tags use a leading `v`:

```text
v0.1.0-alpha.1
v1.0.0
```

The planned first package baseline is `0.1.0-alpha.1`. Alpha increments use
`0.1.0-alpha.N`. A version in a manifest is package metadata, not evidence that
the corresponding tag, registry release, or accepted alpha exists.

## Minimum Supported Rust Version

The workspace minimum supported Rust version (MSRV) is Rust 1.89, recorded by
`rust-version = "1.89"` in the package metadata. Release gates must compile and
test the workspace with that toolchain as well as the repository's current
toolchain.

An MSRV increase must be intentional: update the workspace metadata, CI
toolchain coverage, changelog, and any affected migration guidance in one
reviewed change. During prerelease development, the next alpha number must
carry the increase. An MSRV increase must never arrive as an incidental
dependency or lockfile change.

## SemVer Meaning

Patch releases:

- bug fixes
- documentation corrections
- test-only changes
- CI/build fixes that do not change API behavior

Minor releases:

- new components
- new behavior primitives
- new renderer/platform capabilities
- backwards-compatible API additions
- backwards-compatible performance improvements

Major releases:

- breaking API changes
- renamed or removed shared APIs
- changed behavior contracts
- changed crate boundaries that require application updates
- changed semantics for layout, input, actions, rendering, or state

During `0.x`, breaking changes may occur in minor releases, but they must still be clearly marked and documented.

Prerelease packages may change incompatibly between alpha numbers. Those
changes still require a breaking Conventional Commit marker and migration
notes when they affect shared APIs, crate boundaries, features, or MSRV.

## Conventional Commit Mapping

Commit types map to release notes:

```text
feat      -> Added / Changed
fix       -> Fixed
perf      -> Changed
docs      -> Documentation
test      -> Tests
refactor  -> Changed, if behavior/API is affected
build     -> Build
ci        -> CI
chore     -> Maintenance
style     -> Maintenance
revert    -> Reverted
```

Breaking changes must use either `!` after the type/scope:

```text
feat(layout)!: change measurement contract
```

Or a `BREAKING CHANGE:` footer:

```text
feat(actions): revise shortcut routing

BREAKING CHANGE: shortcuts now resolve through focused Frame context before global actions.
```

## Changelog

Maintain `CHANGELOG.md`.

Until a release is approved, keep only an `[Unreleased]` section and identify
the target version as planned. Do not invent a date, tag, publication, or
historical release entry. Convert that section to a dated version only as part
of an authorized release.

Release entries should be grouped as:

```text
Added
Changed
Fixed
Deprecated
Removed
Performance
Documentation
Internal
```

Each release entry should include:

- version
- release date
- notable changes
- breaking changes, if any
- migration notes, if needed

Migration notes are required when a release changes public crate boundaries,
renames crates, removes facade exports, or moves a subsystem behind a different
feature flag. They should include:

- old crate or import path
- new crate or import path
- the intended audience for each crate
- dependency snippets for common application use and lower-level integration use
- any feature flags required to restore prior behavior

The crate consolidation introduced in `ef7c2f9` is documented in
[`docs/crate-migration.md`](crate-migration.md). Keep that document updated if
the public crate graph changes again.

## Package Graph And Publish Order

The showcase application is never published. The seven library crates must be
packaged and, if separately authorized, published in this exact dependency
order:

1. `kinetik-ui-core`
2. `kinetik-ui-text`
3. `kinetik-ui-render`
4. `kinetik-ui-widgets`
5. `kinetik-ui-winit`
6. `kinetik-ui-vello`
7. `kinetik-ui`

Every internal dependency must contain both its local development `path` and
the exact registry requirement for the shared prerelease version. The path is
used in the workspace; Cargo removes it from the normalized manifest in the
generated archive and retains the exact version requirement.

## Package Verification

Generate archives in publish order from a clean release candidate. During a
local audit, `--allow-dirty` may be used so the candidate diff itself can be
validated; it is not appropriate evidence for the final clean release commit.
Because downstream Kinetik UI prereleases do not yet exist on crates.io,
archive generation uses `--no-verify` and is followed by a stronger explicit
archive build:

Before the dependency crates are published, Cargo also needs ephemeral
`patch.crates-io.<crate>.path` overrides for each unpublished internal
dependency of the package being assembled. Supply those overrides with
`cargo --config` arguments or a disposable Cargo configuration under
`target/`; never add them to a package archive. For example, packaging text
adds:

```text
--config patch.crates-io.kinetik-ui-core.path="crates/kinetik-ui-core"
```

Apply the same rule transitively to the remaining packages. With those local
verification overrides in place, run the logical package sequence:

```text
cargo package -p kinetik-ui-core --no-verify
cargo package -p kinetik-ui-text --no-verify
cargo package -p kinetik-ui-render --no-verify
cargo package -p kinetik-ui-widgets --no-verify
cargo package -p kinetik-ui-winit --no-verify
cargo package -p kinetik-ui-vello --no-verify
cargo package -p kinetik-ui --no-verify
```

For every generated `.crate` archive:

- extract it into a disposable directory under `target/`;
- confirm it includes `README.md`, its normalized `Cargo.toml`, and source;
- confirm the normalized package metadata includes the version, Rust 1.89,
  repository, and crate-specific description;
- confirm internal dependencies retain exact registry versions and no local
  paths;
- build every extracted crate with all targets and features from a disposable
  verification workspace whose `[patch.crates-io]` entries point to the
  extracted Kinetik UI packages.

The temporary patch table simulates dependency availability without claiming
that the crates are published. Remove or ignore all extracted files and target
artifacts after verification.

## Release-State Vocabulary

- **Packageable:** local archives generate, normalize, inspect, and build.
- **Published:** crates.io accepted all seven crates in dependency order.
- **Tagged:** the authorized release commit has a matching signed or annotated
  `vX.Y.Z[-PRERELEASE]` Git tag.
- **Alpha-accepted:** the separate product/readiness gate accepted the alpha.

These states are independent and must be reported separately. Packageability
does not authorize publishing, a tag, or alpha acceptance. Publishing and
tagging require explicit release authority beyond an implementation PR.

## Release Checklist

Before tagging a release:

```text
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace --all-features
cargo build --workspace --all-features
cargo check --workspace --examples --all-features
cargo doc --workspace --all-features --no-deps
```

Also verify:

- `CHANGELOG.md` is updated.
- Version numbers are updated consistently.
- Rust 1.89 MSRV checks pass.
- Breaking changes are documented.
- Migration notes exist when APIs changed.
- Showcase/examples compile.
- All seven normalized archives pass dependency-aware inspection and extracted
  builds in publish order.
- The release approver has explicitly authorized any publication or tag.

## Release Automation

Release automation may be added around this policy.

Automation must stop before registry publication or tag creation unless the
operator supplied explicit authority for that exact release.

Automation should preserve:

- SemVer versioning.
- Conventional Commit parsing.
- changelog generation or validation.
- `vX.Y.Z` tags.
- CI checks before release.
