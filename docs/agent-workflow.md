# Agent Execution Workflow

This document codifies the workflow for AI agents executing work on Stern via GitHub issues and pull requests. All milestone work is executed by agents supervised through issues and PRs.

## Workflow

### 1. Understand the Spec
Read the GitHub issue fully — the issue body is the complete specification. Do not expand scope beyond what is explicitly stated in the issue.

### 2. Prepare the Workspace
Work in a git worktree if available; otherwise ensure `git status` is clean first. This prevents conflicts with other branches.

### 3. Create a Feature Branch
```bash
git fetch origin
git checkout -b issue/<NNN>-<slug> origin/main
```

Where `<NNN>` is the issue number and `<slug>` is a kebab-case summary (e.g., `issue/899-agent-workflow`).

### 4. Make Incremental Commits
- Use Conventional Commits for all commits (see [Commit Message Format](AGENTS.md#commit-message-format))
- Each commit should be scoped and independently compile
- The final commit body must end with `Refs #<NNN>` where `<NNN>` is the issue number
- Example:
  ```
  docs(workflow): codify agent execution workflow
  
  Document the step-by-step process for agents to execute GitHub issues,
  including branch creation, commits, checks, and PR submission.
  
  Refs #899
  ```

### 5. Run Checks Before Creating a PR
All of the following must pass before submitting a PR. If any check fails, do not create the PR:

```bash
cargo fmt --all
cargo clippy -p <touched> --all-targets --all-features -- -D warnings
cargo test -p <touched> --all-features
cargo check --workspace --all-targets
```

If the issue touches multiple crates, run clippy and test for each touched crate.

### 5a. Visual Work Requires Rendered Images (AUDIT #941 §C)

**No PR that changes anything visual merges without a rendered image (CPU
raster dump or screenshot) attached to the PR, reviewed by a human.** Green
tests are necessary, never sufficient, for visual work.

Concretely, before opening a PR that touches widget painting, theme recipes,
composition, or anything else a user can see:

1. Render the relevant stories headlessly with the story harness:
   ```bash
   cargo run -p stern-stories -- render --filter <family-or-story-id>
   ```
   Output lands in `target/stern-stories/render/` (PNGs, a contact sheet,
   and a deterministic manifest).
2. **Look at the PNGs yourself.** Describe what you actually see in the PR
   body — including defects you did not fix. Do not describe the images from
   the code; open them.
3. Attach the rendered images (at minimum the contact sheet) to the PR.
4. If the change alters intended pixels, run
   `cargo run -p stern-stories -- diff` against the goldens and include the
   result. Only a human decides to run `bless`; agents never bless goldens,
   and the harness never blesses automatically.
5. New visual behavior needs a story. Add or extend one in
   `apps/stern-stories/src/stories/` so the change stays reviewable.

### 6. Push and Create a PR
```bash
git push -u origin <branch>
```

Then create the PR:

```bash
gh pr create --base main \
  --title "<issue title>" \
  --body "<summary of changes and results of checks>

Closes #<NNN>"
```

The PR body should include:
- A brief summary of what was implemented
- The results of running the checks above
- "Closes #<NNN>" to link the PR to the issue

### 7. Agents Never Merge
Agents submit PRs but never merge them. A supervisor merges with:

```bash
gh pr checks <PR> --watch --interval 60 > /dev/null 2>&1 && \
  gh pr merge <PR> --squash
```

**Important:** Never pipe the `gh pr checks` command into anything — a pipe swallows the failing exit code.

### 8. Handle Spec Mismatches
If the specification in the issue mismatches reality (the codebase):
- Proceed if intent is unambiguous
- Stop and comment on the issue with the blocker if intent is unclear
- Never delete anything not explicitly listed in the issue
- Report any discovered gaps to KNOWN-GAPS.md or an issue comment — never silently drop them

## Model Routing

The model assigned to an issue influences the approach:

- **Opus**: Design-sensitive work (new APIs, RFCs, architecture)
- **Sonnet**: Deterministic, well-specified implementation and refactors
- **Haiku**: Mechanical find-replace and doc-sync tasks

## House Rules

These rules preserve the architecture and project stability:

- Keep `stern-core` dependency-free: no winit, wgpu, vello, OS APIs, or renderer-specific types
- Pre-alpha: breaking changes are allowed when the issue explicitly says so; do not add deprecation shims unless asked
- Never edit `crates/stern-core/src/theme/generated_tokens.rs` by hand — this file is vendored and drift-tested
- The design-system repo (`../stern-design-system`) is read-only unless the issue explicitly permits editing
- Report any discovered gaps to KNOWN-GAPS.md or as an issue comment — never silently drop gaps out of scope

## Verification

Before marking work complete:

1. Ensure all markdown renders correctly (lint by eye)
2. Verify AGENTS.md links are updated if this issue touches guidance
3. Confirm `cargo check --workspace` passes with no changes to untouched files

## Final Report Format

When work is complete, provide a report ≤15 lines containing:
- Files changed (paths relative to repo root)
- Check results (fmt, clippy, test, check)
- Commits created (list with conventional commit types)
- PR URL
- Any deviations from the workflow
