---
title: "ci: cross-repo release pipeline for sonos-sdk and sonos-cli"
type: ci
status: completed
date: 2026-03-14
origin: docs/brainstorms/2026-03-14-cross-repo-release-pipeline-brainstorm.md
---

# ci: Cross-Repo Release Pipeline

## Overview

Make it dead simple to develop against a local `sonos-sdk` checkout and release paired changes across both repos. No clever automation — just a clean local dev setup, Dependabot for version bumps, and a clear checklist.

## Problem Statement

When a CLI change depends on an SDK change, the process today is unclear:
- `Cargo.toml` has a `path` dep that doesn't work in CI without a `git clone` hack
- The `git clone` in CI masks version mismatches (CI builds against HEAD, not a published version)
- There's no documented step-by-step for paired releases
- Decision fatigue: "do I need to publish the SDK first? update the version? which version?"

## Proposed Solution

Three things:

1. **Clean local dev setup** — `.cargo/config.toml` (gitignored) patches `sonos-sdk` to a local path. `Cargo.toml` stays version-only. CI works without hacks.
2. **Dependabot** — Auto-opens a PR when a new `sonos-sdk` version appears on crates.io. Zero config beyond a YAML file.
3. **Releasing guide** — A markdown checklist for paired changes. Follow the steps, don't think.

## Implementation Plan

### Step 1: Fix Cargo.toml — version only, no path

Revert `Cargo.toml` to use crates.io version without `path`:

```toml
[dependencies]
sonos-sdk = "0.1"

[dev-dependencies]
sonos-sdk = { version = "0.1", features = ["test-support"] }
```

The `path` key causes CI to fail unless we clone the SDK repo. Removing it means CI resolves from crates.io naturally — no `git clone` step needed.

### Step 2: Remove `git clone` from CI workflows

Remove the `git clone` steps from `ci.yml` and `release-plz.yml`. They exist only to work around the `path` dep. Without the path, they're unnecessary.

**`.github/workflows/ci.yml`** — delete:
```yaml
- name: Checkout sonos-sdk (path dependency)
  run: git clone --depth 1 https://github.com/tatimblin/sonos-sdk.git ../sonos-sdk
```

**`.github/workflows/release-plz.yml`** — delete:
```yaml
- name: Checkout sonos-sdk (path dependency)
  run: git clone --depth 1 https://github.com/tatimblin/sonos-sdk.git ../sonos-sdk
```

### Step 3: Add `.cargo/config.toml` for local dev

Create `.cargo/config.toml` and gitignore it. This lets local builds use the adjacent SDK checkout without polluting `Cargo.toml`:

**`.cargo/config.toml`:**
```toml
[patch.crates-io]
sonos-sdk = { path = "../sonos-sdk/sonos-sdk" }
```

**`.gitignore`** — add:
```
.cargo/config.toml
```

When the local SDK path exists, Cargo uses it. When it doesn't (CI, other machines), Cargo uses crates.io. No conditional logic, no hacks.

### Step 4: Add Dependabot for crates.io

Create `.github/dependabot.yml`:

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: daily
    commit-message:
      prefix: "chore(deps)"
    labels:
      - "dependencies"
```

When `sonos-sdk` publishes a new version on crates.io, Dependabot opens a PR bumping `Cargo.toml` and `Cargo.lock`. Merge it and you're done.

### Step 5: Write the releasing guide

Create `docs/references/releasing.md` — a step-by-step checklist for both workflows:

```markdown
# Releasing

## CLI-only changes (no SDK dependency)

1. Open a PR in `sonos-cli`
2. CI passes (resolves `sonos-sdk` from crates.io)
3. Merge

## Paired SDK + CLI changes

When your CLI change depends on unreleased SDK work:

1. **Develop locally.** `.cargo/config.toml` patches `sonos-sdk` to your
   local checkout at `../sonos-sdk/sonos-sdk`. Build and test as normal.

2. **Ship the SDK first.**
   - Open and merge the SDK PR
   - release-plz opens a release PR in `sonos-sdk` — merge it
   - release-plz tags and publishes to crates.io

3. **Update the CLI.**
   - Dependabot will open a PR bumping `sonos-sdk` in `Cargo.toml`
     (or run `cargo update -p sonos-sdk` on your CLI branch manually)
   - If your CLI branch already exists, rebase onto `main` after
     merging the Dependabot PR, or cherry-pick the version bump
   - Open / update your CLI PR
   - CI passes against the published SDK version
   - Merge

## How releases work

Both repos use **release-plz** for automated versioning:

1. PR merged to `main` with conventional commit title
2. release-plz opens a release PR (bumped version + changelog)
3. You merge the release PR
4. release-plz creates a git tag

For `sonos-sdk`: the tag triggers `cargo publish` to crates.io.
For `sonos-cli`: the tag triggers cargo-dist to build binaries and
create a GitHub Release + Homebrew formula.

**Never manually bump versions, create tags, or run cargo publish.**
```

### Step 6: Update the contributing guide

Replace the "SDK changes" section in `docs/references/contributing-agent-guide.md`:

```markdown
## SDK changes

If your change requires modifications to `sonos-sdk`:

1. Develop locally — `.cargo/config.toml` (gitignored) patches `sonos-sdk`
   to your local checkout at `../sonos-sdk/sonos-sdk`
2. Ship the SDK change first (merge PR → merge release PR → published to crates.io)
3. Update the CLI's SDK version (Dependabot PR or `cargo update -p sonos-sdk`)
4. Then open / update your CLI PR

See `docs/references/releasing.md` for the full step-by-step.
```

## Acceptance Criteria

- [x] `Cargo.toml` uses `sonos-sdk = "0.1"` (no `path` key)
- [x] `.cargo/config.toml` patches sonos-sdk to local path, gitignored
- [x] `ci.yml` has no `git clone` step for sonos-sdk
- [x] `release-plz.yml` has no `git clone` step for sonos-sdk
- [x] `.github/dependabot.yml` configured for daily cargo checks
- [x] `docs/references/releasing.md` exists with both workflows
- [x] Contributing guide updated to reference the releasing guide
- [ ] CI passes on a test PR (confirming crates.io resolution works)
