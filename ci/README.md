# ci/

Staging area for GitHub Actions workflow changes.

This directory exists because session credentials cannot write
`.github/workflows/*` — see admin `DECISIONS.md` ADR-8. To change CI:

1. Put the intended workflow file in `workflows/`.
2. A maintainer promotes it with the admin `rollout/apply-ci-folders.sh`
   script.

## Pending

- **`workflows/docs.yml`** — the prose gate: Vale over the reader-facing
  pages at the levels set in `.vale.ini`, on the file list
  `ts/scripts/gated-docs.cjs` produces. See `docs/STYLE-GUIDE.md`.

  It needs no sibling checkouts and no secrets, and pins its own Vale
  version. Errors fail the job; warnings go to the run summary as a
  report. `make prose` runs the identical check locally, and the test
  suite already runs the other half of the gate
  (`ts/test/docs.test.js`), so promoting this adds the spelling and
  Google-convention arm rather than the whole gate.

- **`workflows/rust.yml`** — the Rust gate: `ci/rust/run.sh` over the
  `rs/` crate, on the MSRV toolchain, with the sibling checkouts the
  crate's path dependencies need. Standalone rather than an arm of
  `ci.yml`, because `ci.yml` calls the org-shared polyglot workflow and
  that takes no Rust input, so promoting this needs no change in
  `tabnas/.github`. `make test-rs` is the fast local loop and
  `bash ci/rust/run.sh` is the identical full gate.
