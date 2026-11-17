# Security policy

## Supported versions

Only the latest release gets security fixes.

## Reporting a vulnerability

Report it privately through GitHub's
[private vulnerability reporting](https://github.com/paxel/wipTracker/security/advisories/new)
(the repository's **Security** tab, **Report a vulnerability**). Do not open a public
issue, pull request or discussion for it.

Include the version, the operating system, how to reproduce it and what an attacker
gains. The report and its fix are discussed in that advisory, which stays private until
a release with the fix is out.

## What is already checked

The checks are the shared ones of [paxel/gates](https://github.com/paxel/gates). Every
pull request runs CodeQL, an OSV-Scanner check of the dependencies and their licences, and
SonarQube Cloud; OpenSSF Scorecard analyses the main branch. A release, and a weekly run,
fail on a known vulnerability in a dependency and on any open code-scanning alert, secret
alert or SonarQube Cloud finding that is not accepted with a reason and a review date in
`code-scanning-ignores.toml`. Every release file carries a build provenance attestation,
and the release a source SBOM.
The attestation is also attached to the release as `wiptracker-<tag>-provenance.sigstore.json`
for checks without the GitHub API:

```bash
gh attestation verify <file> -R paxel/wipTracker
gh attestation verify <file> -R paxel/wipTracker --bundle wiptracker-<tag>-provenance.sigstore.json
```

The decisions behind this are in
[docs/adr/0001-security-hardening.md](docs/adr/0001-security-hardening.md) and
[docs/adr/0002-move-to-gates.md](docs/adr/0002-move-to-gates.md).
