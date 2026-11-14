# wipTracker

## Agent skills

### Issue tracker

Issues live as GitHub issues in `paxel/wipTracker`, managed with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` plus `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Quality gates

### Coverage ratchet

Line coverage (cargo-tarpaulin) may only go up. The floor lives in `.coverage-floor`;
`scripts/coverage.sh` fails below it and raises it when coverage rose — commit the raised
floor with the change that earned it. CI and the release gate run `scripts/coverage.sh
--check`. Every feature ships with tests that hold or raise coverage; a feature that
lowers it is not done.

### Before a release

Run the `verify-release` skill. For this repo that is: fmt, clippy (`-D warnings`, all
targets and features), the full test suite, osv-scanner with `tool/osv_gate.py`, the coverage ratchet, and the
packaging checks (`desktop-file-validate` plus shell syntax on the install scripts).

### Security rules

The decisions behind these are in `docs/adr/0001-security-hardening.md`.

- Shipped code may not panic: the crate roots (`src/lib.rs`, `src/main.rs`) deny unwrap,
  expect, panic, todo, unimplemented, unreachable, indexing, string slicing, overflowing
  arithmetic and `dbg!`. Unit tests are exempt through `cfg(test)`.
- `#[allow(...)]` is denied. A lint that must be silenced gets
  `#[expect(<lint>, reason = "...")]`; `unsafe` is allowed only that way, with a
  `// SAFETY:` comment at the block.
- Every action in `.github/workflows/` is pinned to a full commit SHA with its version in
  a trailing comment. Never reference an action by tag or branch alone.
- Rust is pinned in `rust-toolchain.toml` and mirrored in the workflows
  (`dtolnay/rust-toolchain@<sha> # <version> branch`). A toolchain bump is a commit of its
  own that changes the file and every workflow reference together.
- Every workflow starts with `permissions: contents: read`; a job that writes widens it
  itself.
- An advisory that cannot be fixed is ignored in `osv-scanner.toml` with the reason
  wipTracker is not exposed and an `ignoreUntil` date. Never ignore without both.
- The release fails on a known vulnerability, on an open high or critical CodeQL alert,
  and on a missing `CHANGELOG.md` section for the version.
