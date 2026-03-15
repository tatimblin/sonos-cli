# Cross-Repo Release Pipeline Brainstorm

**Date:** 2026-03-14
**Status:** Draft
**Scope:** CI/CD coordination between `sonos-cli` and `sonos-sdk`

## What We're Building

A seamless deployment pipeline that lets you open PRs in both `sonos-sdk` and `sonos-cli` simultaneously when a CLI change depends on an SDK change. The pipeline automatically:

1. Detects which CLI PRs are waiting for an SDK release
2. Blocks CLI CI until the required SDK version is published to crates.io
3. Auto-updates the CLI PR's `Cargo.toml` with the new SDK version when it publishes
4. Re-runs CI and unblocks merge

The developer never manually tracks version numbers or sequences releases.

## Why This Approach

**Cross-repo dispatch** was chosen over polling (latency, waste) and Renovate (manual rebase, doesn't handle the "both PRs open" pattern). Dispatch gives immediate feedback after SDK publish with zero manual steps.

**The key insight:** CI should NOT clone `sonos-sdk`. By omitting the `git clone` step, Cargo naturally falls back from the `path` dependency to the `version` on crates.io. SDK-dependent PRs fail CI until the version is updated — which is the desired gating behavior.

## How It Works

### Developer Workflow

**CLI-only changes (no SDK dependency):**

1. Make changes in `sonos-cli`
2. Open CLI PR — no label needed
3. CI runs immediately (Cargo resolves existing SDK version from crates.io)
4. Green → merge

**Paired SDK + CLI changes:**

1. Make changes in both `../sonos-sdk` and `sonos-cli` locally (path deps work as normal)
2. Open SDK PR in `sonos-sdk` repo
3. Open CLI PR in `sonos-cli` repo, apply the `awaiting-sdk` label
4. CLI CI will fail (expected — SDK version not yet on crates.io)
5. SDK PR merges → release-plz bumps version → `cargo publish` → dispatch fires
6. CLI workflow receives dispatch, updates `Cargo.toml` on the PR branch, commits
7. CLI CI re-runs against the published SDK version → green → merge

The `awaiting-sdk` label is the opt-in signal. Without it, PRs follow the normal flow.

### Pipeline Architecture

```
sonos-sdk repo                          sonos-cli repo
─────────────                          ──────────────
PR merged to main                      CLI PR open (label: awaiting-sdk)
    │                                      │
    ▼                                      │ CI fails (SDK version
release-plz bumps version                 │ not on crates.io yet)
    │                                      │
    ▼                                      │
cargo publish to crates.io                │
    │                                      │
    ▼                                      ▼
repository_dispatch ──────────────► sdk-update workflow
  { sdk_version: "0.2.0" }                │
                                           ▼
                                    Update Cargo.toml on PR branch
                                    (version = "0.2", keep path key)
                                           │
                                           ▼
                                    CI re-runs → passes → merge ready
```

### SDK Repo Changes

Add a step to the SDK's release/publish workflow:

```yaml
- name: Notify sonos-cli
  uses: peter-evans/repository-dispatch@v3
  with:
    token: ${{ secrets.CLI_REPO_DISPATCH_TOKEN }}
    repository: tatimblin/sonos-cli
    event-type: sdk-published
    client-payload: '{"version": "${{ steps.version.outputs.version }}"}'
```

Requires a PAT (`CLI_REPO_DISPATCH_TOKEN`) with `repo` scope on `sonos-cli`, stored as a secret in the SDK repo.

### CLI Repo Changes

#### 1. New workflow: `sdk-update.yml`

Triggered by `repository_dispatch` with event type `sdk-published`.

Steps:
1. Receive the new SDK version from the dispatch payload
2. Find all open PRs with the `awaiting-sdk` label
3. For each PR:
   a. Check out the PR branch
   b. Update `Cargo.toml`: set `sonos-sdk` version to the new version (keep `path` key)
   c. Run `cargo update -p sonos-sdk` to update `Cargo.lock`
   d. Commit and push
4. The push triggers the existing CI workflow

#### 2. Modify `ci.yml`

- Remove the `git clone` step for sonos-sdk (if present)
- Cargo will fall back to crates.io when the path doesn't exist in CI

#### 3. Add merge protection

A required status check or branch protection rule that prevents merging PRs labeled `awaiting-sdk` until CI passes. Since CI naturally fails when the SDK version isn't on crates.io, this is enforced organically — no extra check needed.

### Cargo.toml Strategy

```toml
[dependencies]
# path for local dev, version for CI and crates.io
sonos-sdk = { version = "0.2", path = "../sonos-sdk/sonos-sdk" }
```

- **Locally:** Cargo uses the `path` (local SDK checkout)
- **In CI:** Path doesn't exist → Cargo uses `version` from crates.io
- **On dispatch:** Workflow bumps `version` field, keeps `path` key intact

## Key Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Coordination mechanism | Cross-repo dispatch | Immediate, no polling, fully automatic |
| PR linking | `awaiting-sdk` GitHub label | Simple, visible, manually applied |
| CI SDK resolution | crates.io only (no git clone) | Natural gating — CI fails until version published |
| Cargo.toml format | `{ version + path }` | Path for local dev, version for CI |
| Version detection | Dispatch payload carries version | No guessing, release-plz determines version |
| Path key in CI commits | Keep it | Harmless in CI (falls back to version), preserves local dev |

## Resolved Questions

1. **SDK publish timing:** No special handling needed. crates.io propagation is fast enough — dispatch immediately after publish.
2. **Multiple CLI PRs:** Update all PRs with the `awaiting-sdk` label. The version bump is identical across PRs, so conflicts are trivial or auto-resolvable.
3. **SDK PAT:** An existing PAT is available for cross-repo dispatch. Will be configured as a secret in the SDK repo.
4. **Rollback scenario:** Revert the SDK, publish a patch version, and the pipeline handles the rest automatically (new dispatch → new version bump on CLI PRs).
