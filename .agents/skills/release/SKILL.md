---
name: release
description: Release a new version — bumps version, updates changelog and README, commits, tags, and creates a GitHub release
disable-model-invocation: true
---

Perform a release for ticktick-cli.

Current version from Cargo.toml: `!cargo pkgid | sed 's/.*#//'`

Recent commits since last tag:
```
!git log "$(git describe --tags --abbrev=0 2>/dev/null || echo HEAD~20)"..HEAD --oneline
```

Current `[Unreleased]` section in CHANGELOG.md:
```
!sed -n '/^## \[Unreleased\]/,/^## \[/{ /^## \[Unreleased\]/d; /^## \[/d; p; }' CHANGELOG.md
```

## Steps

1. **Determine bump type** from the `[Unreleased]` changelog content above:
   - **major** — if any entry contains `**Breaking:**` or there is a `### Removed` section
   - **minor** — if there is a `### Added` or `### Changed` section (without breaking changes)
   - **patch** — if only `### Fixed`, `### Security`, or `### Deprecated` sections are present
   - If the `[Unreleased]` section is empty, stop and tell the user there is nothing to release.

2. **Compute new version**: Apply the determined bump to the current version. Print the current version, bump type, and new version for the user to confirm before proceeding.

3. **Update `Cargo.toml`**: Change the `version` field to the new version.

4. **Update `Cargo.lock`**: Run `cargo check` so the lockfile picks up the new version.

5. **Update `CHANGELOG.md`**:
   - Insert a new heading `## [X.Y.Z]` (with today's date is NOT included — this project does not use dates in headings) directly below the `## [Unreleased]` line.
   - Move all content currently under `[Unreleased]` into the new version section.
   - Leave the `## [Unreleased]` section empty (just the heading, followed by a blank line, then the new version heading).

6. **Update `README.md`**: Read `README.md` and the changelog entries being released. Update all sections that are affected by the new release:
   - **Features** bullet list — add, revise, or remove bullets to match the current feature set
   - **Commands** table — add new commands, update descriptions, remove deprecated ones
   - **Task flags** / **Project flags** tables — add new flags, update descriptions
   - **Examples** section — add examples for significant new features, update or remove stale examples
   - **Timezone handling**, **Output format**, or any other prose section that references behavior changed in this release
   - Do NOT bump version numbers in prose (there are none currently) or add a version badge

7. **Verify the build**:
   - `cargo check`
   - `cargo clippy -- -D warnings`
   - `cargo test`
   If any step fails, stop and fix before continuing.

8. **Commit**: Stage `Cargo.toml`, `Cargo.lock`, `CHANGELOG.md`, and `README.md` (if changed). Commit with the message: `release: v<NEW_VERSION>`

9. **Tag**: Create an annotated git tag: `git tag -a v<NEW_VERSION> -m "v<NEW_VERSION>"`

10. **Push and release**:
    - `git push && git push --tags`
    - Create a GitHub release using `gh release create v<NEW_VERSION> --title "v<NEW_VERSION>" --notes <changelog entries for this version>`

11. **Verify release pipeline**:
    - Poll `gh run list --repo karbassi/ticktick-cli --branch v<NEW_VERSION> --limit 1` until the Release workflow completes (check every 15 seconds, up to 10 minutes).
    - If the workflow fails, show the logs with `gh run view <run_id> --repo karbassi/ticktick-cli --log-failed` and stop.
    - Confirm release assets were uploaded: `gh release view v<NEW_VERSION> --repo karbassi/ticktick-cli` — expect 4 tar.gz files (macos-x86_64, macos-arm64, linux-x86_64, linux-arm64).

12. **Verify Homebrew tap**:
    - The Release workflow dispatches to `karbassi/homebrew-tap` to update the formula.
    - Poll `gh run list --repo karbassi/homebrew-tap --limit 1` until the tap update workflow completes (check every 10 seconds, up to 5 minutes).
    - If it fails, show the logs and stop.
    - Confirm the formula version: `gh api repos/karbassi/homebrew-tap/contents/Formula/ticktick-cli.rb -q '.content' | base64 -d | head -5` — the version should match `v<NEW_VERSION>`.
    - Print a final summary: version, release URL, and Homebrew tap status.
