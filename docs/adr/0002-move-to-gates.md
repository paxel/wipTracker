# wipTracker moves to the gates standard

Decided on 2026-11-17.

wipTracker had hardened itself in its own way (`0001-security-hardening.md`,
[wip-1000](https://github.com/paxel/wipTracker/issues/17) to
[wip-1010](https://github.com/paxel/wipTracker/issues/29)): its own CI jobs, a coverage
script, copies of the OSV and code-scanning gates in `tool/`, its own release gate and
provenance steps. Every paxel repository had done so, each differently.
[paxel/gates](https://github.com/paxel/gates) is the one standard that replaces those, as
shared workflows a repository calls, and its audit found 18 checks of it missing here.

wipTracker meets the standard of gates v1.1.2. Where the standard decides, it is followed
without a note here: the shared OSV gate, Rust gate, SonarQube Cloud job, CodeQL, Scorecard,
fuzz, release gate and provenance workflows, one coverage ratchet, the lint set in
`Cargo.toml`, the files and the badge row. What follows is where wipTracker adds to it, or
had a choice.

**Desktop tests.** The X11 desktop tests need a session and fail without one; a test that
passes without a desktop has tested nothing. The shared Rust gate runs the tests under one
command, so `scripts/with-desktop.sh` starts Xvfb and openbox and runs what it is given. The
gate, the release and a workstation without a session use the same script.

**Coverage.** `scripts/coverage.sh` and `.coverage-floor` are gone; the floor is in
`coverage-floors.toml`. The desktop tests still sit the measurement out, so that the floor
means the same on CI and on a workstation: the measurement names every other test target.
The price is that a new file in `tests/` has to be added to that list, in `ci.yml` and in
`AGENTS.md`. The four adapters whose coverage depends on the machine stay out of the
measurement.

**The floor went from 92.12 to 91.09, with the same tests.** Rust 1.100.0 counts a closure
on a line of its own, such as `.map_err(|error| StoreError::Read(error.to_string()))?`, as
uncovered when it never ran; Rust 1.99.1 counted the line as covered with the statement
around it. Measured in one container with the same tests: 92.28 % on 1.99.1, 91.19 % on
1.100.0. The 33 lines that changed are error paths that were never run before either, 27 of
them in `redb_store.rs`. The new number is the truer one, and the floor is that measurement
minus 0.1. Staying on 1.99.1 to keep the number was rejected.

**Packaging check.** The desktop entry and the install scripts are checked in a job of
wipTracker's own; the standard has no such check.

**Windows and macOS.** The shared gate builds and runs clippy on the shipped code there.
wipTracker's own job also compiled the tests on both; that is lost.

**Release.** The shared release gate checks alerts, findings, vulnerabilities, licences and
the changelog section, but runs no tests. A tag can sit on any commit, and a release feeds
the Homebrew tap and the Scoop bucket, so the release keeps a job of its own that checks the
tag against the crate version and runs format, clippy, every test and the packaging checks
on the tagged commit.

**chrono.** RUSTSEC-2026-0543 declares chrono unmaintained, with no fixed release.
wipTracker computes every time with it and stores dates and times in its format. Replacing
it is a change of its own that has to keep reading the state files people have; it is not
part of the move. The notice is accepted until 2027-02-14.

**The fuzz crate's lockfile** stays uncommitted, as decided in
[wip-1008](https://github.com/paxel/wipTracker/issues/27): its dependencies are the main
crate's, which the root `Cargo.lock` covers, plus libfuzzer-sys, which is not scanned.

**Required checks.** The ruleset on `summer` requires the shared Rust gate with its Windows
and macOS builds, the OSV gate, SonarQube Cloud, the packaging check and both CodeQL
analyses. Dependabot's pull requests need `SQ_TOKEN` among the Dependabot secrets for the
SonarQube Cloud check.

## Consequences

- The unit tests inside `src/` are linted like the integration tests, in the second clippy
  run; before, `cfg(test)` exempted them from the whole set.
- SonarQube Cloud analyses pull requests as well as `summer`, and no longer imports a clippy
  report: clippy fails CI on any warning, so the report was always empty.
- Fuzzing, CodeQL, Scorecard and the weekly Security run come from gates; the weekly run
  also fails on SonarQube Cloud findings and secret alerts.
- The release and the weekly Security run need the secret `GATES_ALERTS_TOKEN`, a
  fine-grained token that can read the repository's alerts.
- The release's SBOM and Sigstore bundle keep their names,
  `wiptracker-<tag>-source.cdx.json` and `wiptracker-<tag>-provenance.sigstore.json`.
