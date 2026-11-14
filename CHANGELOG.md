# Changelog

All notable changes to WipTracker are documented in this file.

## [1.3.1]

### Security

- Release files carry a build provenance attestation, also attached to the release as a
  Sigstore bundle for offline checks, and the release a source SBOM; vulnerabilities are
  reported privately as `SECURITY.md` describes
  ([wip-1004](https://github.com/paxel/wipTracker/issues/21),
  [wip-1010](https://github.com/paxel/wipTracker/issues/29)).
- rustix is updated past RUSTSEC-2026-0486
  ([wip-1002](https://github.com/paxel/wipTracker/issues/19)).

---

Historical changes have been moved to [OLDER_CHANGES.md](OLDER_CHANGES.md).
