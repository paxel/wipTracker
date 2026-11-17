# wipTracker

## Agent skills

### Issue tracker

Issues live as GitHub issues in `paxel/wipTracker`, managed with the `gh` CLI. See `docs/agents/issue-tracker.md`.

### Triage labels

Default canonical vocabulary: `needs-triage`, `needs-info`, `ready-for-agent`, `ready-for-human`, `wontfix`. See `docs/agents/triage-labels.md`.

### Domain docs

Single-context: `CONTEXT.md` plus `docs/adr/` at the repo root. See `docs/agents/domain.md`.

## Quality gates

The standard wipTracker is held to is [paxel/gates](https://github.com/paxel/gates);
`docs/adr/0002-move-to-gates.md` says where wipTracker adds to it and why. The workflows
call gates' shared workflows, pinned by commit; Dependabot moves the pin.

### Before a commit

`cargo fmt`, clippy and the tests, on the toolchain `rust-toolchain.toml` pins. Clippy is
two runs, as in the shared Rust gate: the shipped code under the whole lint set, and the
tests with the lints allowed that are how a test fails:

```sh
cargo clippy --locked --all-features -- -D warnings
cargo clippy --locked --tests --benches --examples --all-features -- -D warnings \
  -A clippy::unwrap_used -A clippy::expect_used -A clippy::panic -A clippy::indexing_slicing \
  -A clippy::string_slice -A clippy::arithmetic_side_effects -A clippy::unreachable
```

The X11 desktop tests need a session: `scripts/with-desktop.sh cargo test --locked
--all-features` gives them Xvfb and openbox, as CI does.

### Coverage ratchet

Line coverage may only go up. The Rust gate in `ci.yml` fails under the floor in
`coverage-floors.toml`. Every feature ships with tests that hold or raise coverage; a
feature that lowers it is not done. When coverage rises, raise the floor and the badge in
the same pull request, with gates checked out beside the repository:

```sh
cargo tarpaulin --engine llvm --locked --all-features --workspace --lib --bins \
  --test bar_interaction --test bar_render --test cli --test hidden_bar --test journal \
  --test palette_render --test persistence --test windows_smoke \
  --skip-clean --out Lcov --output-dir coverage && mv coverage/lcov.info coverage/rust.lcov
python3 ../gates/scripts/coverage_ratchet.py --floors coverage-floors.toml
```

The test targets are named so that the X11 desktop tests sit the measurement out; a new file
in `tests/` is added here and in `ci.yml`. Never lower the floor.

### Before a release

Run the `verify-release` skill. For this repo that is: fmt, both clippy runs, the full test
suite under `scripts/with-desktop.sh`, osv-scanner with gates' `scripts/osv_gate.py`, the
coverage ratchet, and the packaging checks (`desktop-file-validate` plus shell syntax on the
install scripts). The release workflow runs the shared release gate and the tests on the
tagged commit before it builds.

### Security rules

The decisions behind these are in `docs/adr/0001-security-hardening.md` and
`docs/adr/0002-move-to-gates.md`.

- Shipped code may not panic: `Cargo.toml` denies unwrap, expect, panic, todo,
  unimplemented, unreachable, indexing, string slicing, overflowing arithmetic and `dbg!`
  under `[lints]`. Tests are exempt through the second clippy run.
- `#[allow(...)]` is denied. A lint that must be silenced gets
  `#[expect(<lint>, reason = "...")]`; `unsafe` is allowed only that way, with a
  `// SAFETY:` comment at the block.
- Every action in `.github/workflows/` is pinned to a full commit SHA with its version in
  a trailing comment. Never reference an action by tag or branch alone.
- Rust is pinned in `rust-toolchain.toml`; the shared workflows read it from there. The
  release workflow's own jobs mirror it (`dtolnay/rust-toolchain@<sha> # <version> branch`).
  A toolchain bump is a commit of its own that changes the file and those references together.
- Every workflow starts with `permissions: contents: read`; a job that writes widens it
  itself.
- A dependency advisory is fixed by an update. When no fixed release exists, it is accepted
  in `osv-scanner.toml` with the reason wipTracker is not exposed and an `ignoreUntil` date.
  Never ignore without both.
- A code-scanning alert or a SonarQube Cloud finding that stays open is accepted in
  `code-scanning-ignores.toml` with its tool, rule, file, reason and an `ignoreUntil` date.
  Nothing is dismissed in GitHub or marked in SonarQube Cloud.
- The release fails on a known vulnerability, on every open code-scanning alert, secret
  alert and SonarQube Cloud finding that is not accepted, and on a missing `CHANGELOG.md`
  section for the version. The weekly Security run fails on the same, and on unmaintained
  dependencies and passed dates.
- `summer` takes pull requests only; the required checks are the shared Rust gate with its
  Windows and macOS builds, the OSV gate, SonarQube Cloud, the packaging check and CodeQL.
