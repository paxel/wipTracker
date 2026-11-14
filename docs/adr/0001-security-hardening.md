# Security hardening: the sibling repos' standard

Decided on 2026-11-14, with the Sanshain service, oldbrainfix and multigit as the model.

wipTracker is a single-user desktop bar. It reads its own state file and the desktop's
window and idle state, and no file from anybody else. The threat is the supply chain:
the dependencies, the CI that builds the release, and the files Homebrew and Scoop hand
to other people.

**Scope.** The CI and the release supply chain, plus lints in the code. A threat model,
input limits and fuzzing are out: there is no untrusted input to fuzz.

**Lints.** The crate roots deny panicking operations in shipped code (unwrap, expect,
panic, todo, unimplemented, unreachable, indexing, string slicing, overflowing
arithmetic), `dbg!` and `#[allow]`. The two documented `unsafe` blocks carry
`#[expect(unsafe_code, reason)]`. No `cargo geiger`: the `unsafe_code` lint is the rule.

**Workflows.** Every action pinned by commit SHA; every workflow read-only unless a job
widens it; CI on every branch push and pull request with superseded runs cancelled.
Rust is pinned exactly in `rust-toolchain.toml` (1.99.1, as in the Sanshain service and
dedup-rs); a bump is a commit of its own.

**Dependencies.** osv-scanner on every push, in the release gate and weekly; the gate
(`tool/osv_gate.py`, from multigit) fails on a vulnerability and lists unmaintained
notices, which only the weekly run fails on. Ignores carry a reason and an
`ignoreUntil` date in `osv-scanner.toml`. Dependabot proposes Cargo and action updates
weekly, grouped, after a 14-day cooldown; the toolchain action is left to deliberate
bumps.

**Analysis.** CodeQL for Rust and the workflows on every push, pull request and week.
SonarQube Cloud analyses summer with the clippy report and coverage imported, report
only. OpenSSF Scorecard rates the repository weekly; its alerts count in the gates.

**Release gate.** A tag is refused on a known vulnerability, on any open code-scanning
alert on a branch that holds it (every tool, every severity) that
`code-scanning-ignores.toml` does not accept, and on a missing `CHANGELOG.md` section
for the version. The weekly Security run fails on the same alerts. An accepted finding
carries its tool, rule, reason and an `ignoreUntil` date, like the OSV ignores; nothing
is dismissed in GitHub. Amended later on 2026-11-14: counting only high and critical
CodeQL alerts let a release pass with Scorecard's findings open.

**Provenance.** Every release file gets a GitHub build provenance attestation, checked
with `gh attestation verify <file> -R paxel/wipTracker`, and the release carries a
CycloneDX SBOM of the source tree.

**Repository.** Private vulnerability reporting is on (`SECURITY.md`), Dependabot
security updates are on, and summer takes changes only through a pull request with
green CI, without force-push or deletion; no approval is required, as there is a single
maintainer.

## Consequences

- New workflows: CodeQL, SonarQube Cloud, Scorecard; Security runs osv-scanner instead
  of `cargo audit`; CI gains the OSV gate and its unit tests.
- Upgrading Rust changes `rust-toolchain.toml` and every workflow's toolchain pin in one
  commit.
- The rules agents follow are in `AGENTS.md` under "Security rules".
