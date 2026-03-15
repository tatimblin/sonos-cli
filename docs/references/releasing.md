# Releasing

## CLI-only changes (no SDK dependency)

1. Open a PR in `sonos-cli`
2. CI passes (resolves `sonos-sdk` from crates.io)
3. Merge

## Paired SDK + CLI changes

When your CLI change depends on unreleased SDK work:

1. **Develop locally.** `.cargo/config.toml` (gitignored) patches `sonos-sdk`
   to your local checkout at `../sonos-sdk/sonos-sdk`. Build and test as normal.

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
2. release-plz creates a git tag and GitHub Release
3. The tag triggers the next step in the pipeline

For `sonos-sdk`: the tag triggers `cargo publish` to crates.io.

For `sonos-cli`: the tag triggers cargo-dist to build binaries and
create a GitHub Release with installers + Homebrew formula.

**Never manually bump versions, create tags, or run cargo publish.**

## Local development setup

To develop against a local `sonos-sdk` checkout:

1. Clone `sonos-sdk` adjacent to `sonos-cli`:
   ```
   ~/repos/
     sonos-cli/
     sonos-sdk/
   ```

2. The `.cargo/config.toml` (gitignored) patches `sonos-sdk` to the
   local path automatically. No changes to `Cargo.toml` needed.

3. If you don't have a local SDK checkout, builds resolve from crates.io.
