# Security Policy

## Reporting a Vulnerability

**Do not open a public GitHub issue for security vulnerabilities.**
Public issues are indexed by search engines and cached before they can be
triaged, so even a short-lived issue can leak exploit details to the world.

### How to report

Use GitHub's private vulnerability reporting:

1. Go to the [Security tab][security-tab] on this repository.
2. Click **Report a vulnerability**.
3. Fill in the advisory form with repro steps, affected component, and any
   suggested remediation.

[security-tab]: https://github.com/BemusedVermin/evolution-simulation/security

If the Security tab is not available to you, email **liammcg44@gmail.com**
with `[SECURITY] beast-evolution-game` in the subject line. Encrypt the
message if you have access to my public key; otherwise plain email is
acceptable for an initial heads-up and we can move to an encrypted channel
once a report is acknowledged.

### What to include

- Affected commit / tag / branch.
- Affected component (crate name, module path, or manifest file).
- Reproduction steps that are **minimal and deterministic**. Because this
  project's correctness contract is deterministic replay, an issue that
  reproduces from a specific seed + tick count is much easier to triage
  than one that requires manual reproduction.
- Expected versus observed behaviour.
- Your assessment of severity (optional).

### What to expect

- Acknowledgement within **72 hours** of the initial report.
- A plain-language triage response within **7 days** describing whether
  the issue is in scope, the rough severity, and the expected fix
  timeline.
- A fix, mitigation, or explicit won't-fix decision within **30 days**
  for confirmed vulnerabilities. If circumstances require longer, you
  will be told why.
- Public disclosure (GitHub Security Advisory + release notes) coordinated
  with you, typically once a fix has been released.

## In scope

- All code under this repository (every `beast-*` crate, CI workflows,
  and any tooling scripts).
- The JSON manifest schemas under `documentation/schemas/` when a
  malformed input can trigger a panic, infinite loop, or resource
  exhaustion in the runtime loaders.
- Determinism invariants documented in `documentation/INVARIANTS.md`
  when a realistic input can cause bit-divergent replay (this is a
  correctness bug with save/load implications, treated like a security
  issue for disclosure purposes).

## Out of scope

- Vulnerabilities in third-party crates — please report those upstream.
  We track them via `cargo-deny` on every PR and respond to RUSTSEC
  advisories as they surface.
- Social engineering of the maintainers.
- Denial-of-service via unbounded input sizes you control (e.g. feeding
  an infinitely large JSON manifest). These are interesting but not
  treated as vulnerabilities at this stage of the project.

## Non-sensitive security concerns

For hardening suggestions, defense-in-depth ideas, or questions about
how a given subsystem handles malformed input, open a regular
**Security** issue from the template. Those are fine to discuss in
public.
