# Crates.io Publish Workflow

The `Publish crates.io package` workflow prepares the SDK for crates.io release without exposing registry credentials to untrusted pull requests.

## Triggers

- Pull requests that touch the workflow, Cargo files, source, or tests run package validation and a `katyo/publish-crates` dry run without `CARGO_REGISTRY_TOKEN`.
- `workflow_dispatch` with `publish: false` runs the same dry-run path.
- `push` of a `v*.*.*` tag runs the real publish job after package validation.
- `workflow_dispatch` with `publish: true` only publishes when the selected ref is a `v*.*.*` tag.
- Every path verifies that at least one workspace package is publishable to
  crates.io before invoking the publish action.

## Required Repository Setup

- Add a repository or environment secret named `CARGO_REGISTRY_TOKEN` containing a crates.io API token.
- Configure the `crates-io` GitHub environment with required reviewers before enabling release publishing.
- Keep the token scoped to crates.io publishing only. Do not store the token in repository files, logs, comments, or issue metadata.

## Third-Party Action Review

The workflow uses `katyo/publish-crates` pinned to commit `02cc2f1ad653fb25c7d1ff9eb590a8a50d06186b`, the commit referenced by the `v2` tag during implementation review.

Source review notes:

- The action is a Node 20 action with `dist/index.js` as its runtime entrypoint.
- It reads the optional `registry-token` input and passes it to cargo as `CARGO_REGISTRY_TOKEN`.
- With `dry-run: true`, it skips `cargo publish` execution and skips waiting for crates.io propagation.
- In publish mode it runs `cargo publish`, then checks crates.io for the published version and runs `cargo update --dry-run`.
- It uses crates.io API reads and GitHub API reads for package consistency checks.

The action is third-party and not GitHub-certified. Pinning the full commit SHA reduces tag-move risk, but maintainers should re-review the action source before changing that SHA.

## Current Package Readiness Note

BOG-241 / PR #5 added crates.io package metadata and package-readiness documentation to the integration branch. BOG-245 removed the stale `publish = false` guard so the package is publishable by Cargo metadata, while actual upload remains controlled by the `v*.*.*` tag trigger, the `crates-io` environment, and `CARGO_REGISTRY_TOKEN`.

This workflow is useful for validating the release gate, secret boundary, and trigger behavior before a real publish. Before any real publish attempt, rerun package validation and review the generated package contents.

## Publishability Guard

The workflow runs `python3 scripts/verify-crates-publishable.py` before `katyo/publish-crates`.
The guard reads `cargo metadata --locked --format-version 1 --no-deps` and fails when no workspace package is publishable to crates.io.

This specifically catches the false-positive mode where `Cargo.toml` has `publish = false`: Cargo metadata reports `publish: []`, `katyo/publish-crates` can print an empty `Found packages:` list, and the action can exit successfully without uploading anything. With the guard in place, the workflow fails before secret use or publish action execution.

Expected successful guard output includes the package name and version, for example:

```text
Publishable crates.io package(s):
- kis-sdk 0.2.1
```

If a release workflow succeeds but crates.io does not show the crate, check the publish job log for the guard output, the `Publish crate` step, and the action's crates.io propagation check. A missing guard output or an empty action package list indicates the publish path did not prove that a workspace package was publishable.
