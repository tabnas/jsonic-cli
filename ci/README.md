# ci/

Staging area for GitHub Actions workflow changes.

This directory exists because session credentials cannot write
`.github/workflows/*` — see admin `DECISIONS.md` ADR-8. To change CI:

1. Put the intended workflow file in `workflows/`.
2. A maintainer promotes it with the admin `rollout/apply-ci-folders.sh`
   script.

## Promoted, 2026-09-22

Both files that were staged here are now live, moved by the rollout
script rather than edited: `workflows/docs.yml` is
`.github/workflows/docs.yml` and `workflows/rust.yml` is
`.github/workflows/rust.yml`. Nothing is pending. Read the workflows
themselves rather than a description of them here.

`rust.yml` still opens with a header calling itself PROPOSED and telling
the reader to move it into `.github/workflows/`, which is where it
already is. The rollout moves files and does not rewrite their comments,
and session credentials cannot push `.github/workflows/*` to correct it,
so the fix is a staged copy here and another rollout run.

## What still lives here

- **`rust/run.sh`** is the Rust gate itself, over the `rs/` crate on the
  MSRV toolchain. `.github/workflows/rust.yml` runs it with the sibling
  checkouts the crate's path dependencies need, standalone rather than
  as an arm of `ci.yml`, because `ci.yml` calls the org-shared polyglot
  workflow and that takes no Rust input. `make test-rs` is the fast
  local loop and `bash ci/rust/run.sh` is the identical full gate.
