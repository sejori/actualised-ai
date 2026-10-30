# Release Please Architecture & SemVer Release Guide

This document explains the versioning, changelog generation, and automated release lifecycle for the `actualised-ai` monorepo using [Google's Release Please](https://github.com/googleapis/release-please).

---

## 1. Monorepo Topology & Artifacts

The repository manages two primary publishable packages configured in `release-please-config.json` and `.release-please-manifest.json`:

| Package Path | Target Registry | Language / Package Name | Release Type |
|---|---|---|---|
| `crates/core` | **crates.io** | Rust (`actualised-core`) | `rust` |
| `packages/sdk` | **npm** | Node.js (`@actualised-ai/sdk`) | `node` |

### Configuration File (`release-please-config.json`)
```json
{
  "$schema": "https://raw.githubusercontent.com/googleapis/release-please/main/schemas/config.json",
  "packages": {
    "packages/sdk": {
      "release-type": "node",
      "package-name": "@actualised-ai/sdk",
      "changelog-path": "CHANGELOG.md",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "include-component-in-tag": true,
      "extra-files": ["Cargo.toml"]
    },
    "crates/core": {
      "release-type": "rust",
      "package-name": "actualised-core",
      "changelog-path": "CHANGELOG.md",
      "bump-minor-pre-major": true,
      "bump-patch-for-minor-pre-major": true,
      "include-component-in-tag": true
    }
  }
}
```

### Manifest File (`.release-please-manifest.json`)
Tracks the current released versions. Example:
```json
{
  "packages/sdk": "0.1.2",
  "crates/core": "0.1.2"
}
```

---

## 2. Conventional Commits to SemVer Mapping

Release Please parses commit messages on the target branch (`main`) since the last release tag (e.g. `actualised-core-v0.1.2`).

Because `bump-minor-pre-major: true` and `bump-patch-for-minor-pre-major: true` are enabled for pre-1.0 releases (`0.x.y`):

| Commit Prefix | Pre-1.0 Version Delta | Example (From `0.1.2`) | Changelog Section |
|---|---|---|---|
| `fix(...)` | Patch bump | `0.1.3` | **Bug Fixes** |
| `feat(...)` | Patch bump (due to `bump-patch-for-minor-pre-major: true`) | `0.1.3` | **Features** |
| `feat(...)!` / `BREAKING CHANGE:` | Minor bump | `0.2.0` | **BREAKING CHANGES** |
| `docs(...)`, `chore(...)`, `test(...)` | None (unless accompanied by feat/fix) | No release | Excluded |

> [!TIP]
> Always prefix pull request commits or squash-merge titles with valid conventional commit syntax, e.g.:
> - `feat(vcs): add file_exists to WorkspaceProvider trait`
> - `fix(core): resolve race condition in dag barrier wait`

---

## 3. The Release Staging Cycle (Step-by-Step)

The release cycle follows a two-phase staging mechanism:

```
[Feature Branch] 
       │ (gh pr merge --squash)
       ▼
 [main branch] ─── triggers ───► [.github/workflows/release.yml]
                                             │
                                             ▼
                             [release-please-action evaluates main]
                                             │
                  ┌──────────────────────────┴──────────────────────────┐
                  ▼                                                     ▼
      [Commit is a Regular Feature]                            [Commit is a Release PR Merge]
                  │                                                     │
                  ▼                                                     ▼
Creates / Updates Release PR                               Creates GitHub Release & Git Tag
Branch: release-please--branches--main                      (e.g. actualised-core-v0.1.3)
Updates:                                                    Triggers downstream publishing:
 - .release-please-manifest.json                             - publish-crate -> crates.io
 - crates/core/Cargo.toml                                    - publish-npm   -> npm
 - crates/core/CHANGELOG.md
```

### Phase 1: Feature Development & Merge
1. Author feature or fix branch: `feat/my-feature`.
2. Commit with conventional commit syntax (`feat(vcs): ...`).
3. Open a GitHub Pull Request targeting `main`.
4. Review, test, and merge the feature PR into `main`.

### Phase 2: Release Please Updates the Release PR
5. When the feature PR merges into `main`, GitHub Actions triggers `.github/workflows/release.yml`.
6. `release-please-action` analyzes the commit history since the last release tag.
7. It creates or updates an automated pull request:
   - Branch: `release-please--branches--main`
   - Title: `chore: release main`
   - Proposed Changes:
     - Updates version in `crates/core/Cargo.toml` and `.release-please-manifest.json`
     - Appends release notes to `crates/core/CHANGELOG.md`

### Phase 3: Merging the Release PR & Automated Publishing
8. Review the automated Release PR to ensure the version bump and changelog are correct.
9. Merge the Release PR into `main` (using squash or merge commit).
10. The merge triggers `.github/workflows/release.yml` again:
    - Release Please detects the merge of a release commit (`chore: release ...`).
    - Creates the GitHub Release and pushes the git tag (e.g. `actualised-core-v0.1.3`).
    - Outputs `core_released = true` and `sdk_released = true`.
    - Downstream job `publish-crate` runs `cargo publish` with `CARGO_REGISTRY_TOKEN`.
    - Downstream job `publish-npm` builds multi-platform native binaries and publishes to npm via OIDC.

---

## 4. Critical Invariants & Anti-Patterns

> [!CAUTION]
> **Never merge a stale Release Please PR while feature PRs are landing!**
> If a Release Please PR is open for `0.1.2`, and you merge a new feature intended for `0.1.3`, you **must wait** for GitHub Actions to run on `main` and update the Release PR to `0.1.3` before merging it.
> Merging the stale Release PR prematurely will tag the new feature under the old version number, causing version desync.

---

## 5. Troubleshooting & Recovery Runbook

### Case A: Release PR Did Not Open After a Feature Merge
- **Cause 1**: The commit message was not a conventional commit (`feat:`, `fix:`, or `perf:`). Non-releasing prefixes (`chore:`, `docs:`, `test:`) do not trigger a release unless accompanied by a releasing commit.
- **Cause 2**: The git tag already encompasses the commit.
- **Fix**: Check `gh run list --workflow=release.yml` to see the action log. Push a conventional commit (`feat: ...`) to trigger release-please.

### Case B: Release Please Output Not Triggering Crates.io Publish
- In `.github/workflows/release.yml`, publishing is guarded by:
  `if: ${{ needs.release-please.outputs.core_released == 'true' || inputs.force_publish_crate == true }}`
- If you need to re-run publishing for an already-tagged version, trigger the workflow manually with `workflow_dispatch`:
  ```bash
  gh workflow run release.yml -f force_publish_crate=true
  ```
