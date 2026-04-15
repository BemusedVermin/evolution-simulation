<!--
Security PR template. Use this for fixes to security issues, hardening
changes, and dependency advisories.

> If this PR fixes an UNDISCLOSED vulnerability, DO NOT open it as a
> normal PR against master. Use a GitHub Security Advisory
> (Security tab → Report a vulnerability → proposed fix), which creates
> a private fork + temporary private branch for coordinated disclosure.
> Public PRs for unfixed CVEs turn the diff into an exploit manual.

Activate via: ?template=security.md on the PR creation URL.
-->

## Summary

<!-- One-paragraph description of the security change. -->

## Vulnerability class

<!--
Pick all that apply:
- Panic / DoS on attacker-controlled input
- Memory safety (only possible via `unsafe`, which is forbidden — flag if you're relaxing the forbid)
- Input validation gap
- Cryptographic misuse
- Secret / credential leakage
- Dependency advisory (RUSTSEC-…)
- Hardening / defense-in-depth (no known exploit, reducing attack surface)
- Other
-->

## Linked issue / advisory

<!--
- Fixes GHSA-… (private advisory)
- Fixes #<security-issue-number> (non-sensitive)
- Addresses RUSTSEC-…
-->

## Affected versions

<!-- Commit range / tag range / "all published versions". -->

## Fix summary

<!--
What changed. For non-sensitive PRs, include detail. For PRs coordinated
with a GitHub Security Advisory, keep this high-level until disclosure.
-->

## Regression coverage

<!--
For non-sensitive security PRs: include the failing-before / passing-after
test inline.

For PRs coordinated with a private advisory: describe the test that
will land with disclosure, or link it in the advisory draft.
-->

- [ ] Regression test covers the vulnerable path.
- [ ] Test is deterministic.
- [ ] `cargo deny check` clean (advisory fixed or acknowledged in `deny.toml`).

## Disclosure timeline

<!--
- Reported: <date>
- Triaged: <date>
- Fix ready: <date>
- Public disclosure target: <date or "with release">
-->

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`
- [ ] `cargo test --workspace --all-targets --locked`
- [ ] `cargo deny check`
- [ ] Manual reproduction of the vulnerability pre-fix confirms failure; post-fix confirms success.
