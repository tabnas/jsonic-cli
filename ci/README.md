# ci/

The scripts the CI workflows run, kept here so that you can run them
too. See "What still lives here" below.

To change CI, edit `.github/workflows/` in a reviewed pull request.
Session credentials push workflow files (admin `DECISIONS.md` ADR-8, as
amended 2026-09-24), so staging a workflow here first for a maintainer
to promote is optional. Sessions still cannot push tags, so a maintainer
pushes any tag that a tag-triggered workflow needs.

The amendment also asks for the same change in `tabnas/admin` wherever
admin keeps a copy of the workflow. If admin's `rollout/workflows/`
holds a `jsonic-cli__<file>` template for the workflow you changed, make
the same edit there: admin `scripts/verify.sh` compares each template
with its deployed copy, and a maintainer's
`rollout/apply-workflows.sh --apply` would push the older text back over
yours.

## Promoted, 2026-09-22

Both files that were staged here are now live, moved by the rollout
script rather than edited: `workflows/docs.yml` is
`.github/workflows/docs.yml` and `workflows/rust.yml` is
`.github/workflows/rust.yml`. Nothing is pending. Read the workflows
themselves rather than a description of them here.

`rust.yml` opened with a header calling itself PROPOSED and telling the
reader to move it into `.github/workflows/`, which is where it already
was: the rollout moves files and does not rewrite their comments. The
header was corrected in place once ADR-8's amendment let a session edit
the live file.

## What still lives here

- **`rust/run.sh`** is the Rust gate itself, over the `rs/` crate on the
  MSRV toolchain. `.github/workflows/rust.yml` runs it with the sibling
  checkouts the crate's path dependencies need, standalone rather than
  as an arm of `ci.yml`, because `ci.yml` calls the org-shared polyglot
  workflow and that takes no Rust input. `make test-rs` is the fast
  local loop and `bash ci/rust/run.sh` is the identical full gate.
