# Contributing Agent Guide

Step-by-step workflow for AI agents making changes to sonos-cli. Follow this exactly.

## Branching

Always create a feature branch from `main`. Never commit directly to `main`.

```bash
git checkout main
git pull origin main
git checkout -b <type>/<short-description>
```

Branch naming convention: `<type>/<short-description>`

Examples:
- `feat/add-shuffle-command`
- `fix/volume-overflow`
- `refactor/split-run-module`
- `docs/update-readme`

## Commit messages

Must follow **conventional commit format**:

```
<type>: <description>
```

Rules:
- **Types:** `feat`, `fix`, `refactor`, `docs`, `test`, `chore`, `perf`, `build`, `ci`
- Subject starts **lowercase**
- No period at end
- Use imperative mood ("add", not "added" or "adds")
- Keep under 72 characters

Examples:
```
feat: add shuffle-no-repeat play mode
fix: use GroupRenderingControl for group volume
refactor: split run.rs into focused modules
docs: add install instructions to README
```

### Version impact

The commit type determines the automatic version bump:
- `feat:` → minor (0.1.0 → 0.2.0)
- `fix:` → patch (0.1.0 → 0.1.1)
- `feat!:` or `BREAKING CHANGE` footer → major (0.1.0 → 1.0.0)
- `docs:`, `chore:`, `ci:`, `test:`, `build:` → no release

## Opening PRs

```bash
git push -u origin <branch-name>
gh pr create --title "<type>: <description>" --body "$(cat <<'EOF'
## Summary
- What changed and why

## Test plan
- [ ] cargo test passes
- [ ] cargo clippy passes
- [ ] Manual verification steps
EOF
)"
```

The **PR title must follow conventional commit format** — this is enforced by CI. The PR title becomes the squash merge commit message, which drives changelog generation.

## CI validation

After pushing, verify CI passes before requesting merge:

```bash
# Check PR status
gh pr checks

# If a check fails, read the logs
gh run list
gh run view <run-id> --log-failed
```

Two checks run on every PR:
1. **CI / Check** — `cargo fmt --check`, `cargo clippy -- -D warnings`, `cargo test`
2. **PR Title / Conventional Commit** — validates PR title format

Fix any failures, push again. Do not merge with failing checks.

## Local validation before pushing

Run these locally to catch issues before CI:

```bash
cargo fmt --check
cargo clippy -- -D warnings
cargo test
```

## Versioning

**Never manually bump the version in Cargo.toml.** release-plz handles this automatically:

1. You merge a PR to `main`
2. release-plz opens a "release PR" with bumped version + updated CHANGELOG.md
3. The release PR is merged (by a human)
4. release-plz creates a git tag → cargo-dist builds binaries → GitHub Release published

Do not:
- Edit the `version` field in `Cargo.toml`
- Create git tags manually
- Create GitHub Releases manually
- Run `cargo publish` manually

## SDK changes

If your change requires modifications to `sonos-sdk`:

1. Make changes in `../sonos-sdk/sonos-sdk`
2. Test locally (the `Cargo.toml` path dependency uses the local SDK)
3. Ensure the SDK change is published to crates.io before the CLI PR is merged (CI resolves from crates.io, not the local path)

## Squash merge

All PRs are merged via **squash merge**. The PR title becomes the single commit message on `main`. Individual commit messages on the branch don't matter for the changelog — only the PR title does.

## Checklist before requesting merge

- [ ] Branch is up to date with `main`
- [ ] `cargo test` passes
- [ ] `cargo clippy -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] PR title follows conventional commit format
- [ ] CI checks are green (`gh pr checks`)
- [ ] Changes are tested (unit tests for new logic)
