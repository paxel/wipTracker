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

Every push and pull request runs CodeQL and an OSV-Scanner dependency check, and
SonarQube Cloud and OpenSSF Scorecard analyse the main branch. A release fails on a
known vulnerability in a dependency or an open high or critical CodeQL alert. Every
release file carries a build provenance attestation, and the release a source SBOM:

```bash
gh attestation verify <file> -R paxel/wipTracker
```

The decisions behind this are in
[docs/adr/0001-security-hardening.md](docs/adr/0001-security-hardening.md).
