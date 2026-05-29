# Crates.io Publish Workflow

The `Publish crates.io package` workflow prepares the SDK for crates.io release without exposing registry credentials to untrusted pull requests.

## Triggers

- Pull requests that touch the workflow, Cargo files, source, or tests run package validation and a `katyo/publish-crates` dry run without `CARGO_REGISTRY_TOKEN`.
- `workflow_dispatch` with `publish: false` runs the same dry-run path.
- `push` of a `v*.*.*` tag runs the real publish job after package validation.
- `workflow_dispatch` with `publish: true` only publishes when the selected ref is a `v*.*.*` tag.

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

`Cargo.toml` currently has `publish = false`, so the publish action will not find a publishable package until package-readiness work removes that guard and completes crates.io metadata. This workflow is still useful for validating the release gate, secret boundary, and trigger behavior before the package is enabled for publishing.
