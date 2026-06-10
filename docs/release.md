# Release Policy

Kinetik UI uses SemVer and Conventional Commits.

## Versioning

Versions follow:

```text
MAJOR.MINOR.PATCH
```

Examples:

```text
0.1.0
0.2.0
1.0.0
1.1.3
```

Tags use a leading `v`:

```text
v0.1.0
v1.0.0
```

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
- Breaking changes are documented.
- Migration notes exist when APIs changed.
- Showcase/examples compile.

## Release Automation

Release automation may be added around this policy.

Automation should preserve:

- SemVer versioning.
- Conventional Commit parsing.
- changelog generation or validation.
- `vX.Y.Z` tags.
- CI checks before release.

