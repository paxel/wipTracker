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
targets and features), the full test suite, `cargo audit`, the coverage ratchet, and the
packaging checks (`desktop-file-validate` plus shell syntax on the install scripts).
