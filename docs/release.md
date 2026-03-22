# Release Guide

This guide defines the minimum safe procedure for cutting a new `taskspace` release.

## Scope

- Distribution: GitHub Releases (tag-driven workflow)
- Trigger: push a tag matching `v*`
- Version source of truth: `Cargo.toml` (`[workspace.package].version`)
- Out of scope: crates.io publish (`publish = false`)

## 1) Prepare

1. Confirm the target version in `Cargo.toml`.
2. Ensure the repository is on the commit you want to release.
3. Ensure the release tag will match the Cargo version exactly:
   - Cargo version: `X.Y.Z`
   - Git tag: `vX.Y.Z`

## 2) Validate Locally

Run the project quality gates before tagging:

```bash
mise run check
mise run coverage
```

If `mise` is unavailable, run the equivalent Cargo commands defined in `AGENTS.md`.

## 3) Cut the Release

Create and push a single tag:

```bash
git tag vX.Y.Z
git push origin vX.Y.Z
```

Notes:

- Push one release tag at a time.
- The GitHub `Release` workflow runs automatically on this tag.

## 4) Verify Release Output

After the workflow completes, verify in GitHub Releases that:

1. A release `vX.Y.Z` exists.
2. Assets were uploaded for all configured targets.
3. Each archive has a matching `.sha256` file.

Optional local checksum verification:

```bash
sha256sum -c taskspace-vX.Y.Z-<target>.tar.gz.sha256
```

## 5) Post-Release Checks

1. Confirm install paths resolve the new version:
   - Homebrew: `brew install viasnake/tap/taskspace` (or upgrade path)
   - mise: `mise use -g github:viasnake/taskspace@vX.Y.Z`
2. Confirm binary version output:

```bash
taskspace --version
```

## Failure Handling

- Do not move or overwrite an existing release tag.
- If a release is wrong, fix forward with a new patch version (for example, `vX.Y.(Z+1)`).
