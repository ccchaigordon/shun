# Security Policy

## Supported Versions

Shun is currently a pre-1.0 project. Security fixes are applied to the `main` branch and included in the next release. Older commits and downstream forks are not maintained by this project.

| Version | Supported |
| ------- | --------- |
| `main`  | Yes       |

## Reporting a Vulnerability

Do not open a public issue for a suspected vulnerability.

Use GitHub's private vulnerability reporting from the repository's **Security** tab. If that option is unavailable, email `chaigordon75@gmail.com` with the subject `Security report for Shun`. Include:

- The affected version or commit.
- The operating system and relevant environment details.
- Reproduction steps or a minimal proof of concept.
- The expected security impact.
- Any known workaround or mitigation.

The maintainer aims to acknowledge a report within seven days. After validation, the reporter and maintainer will coordinate remediation and disclosure. Please allow a reasonable period for a fix before publishing details.

Shun is maintained on a best-effort basis, so remediation timelines depend on severity, reproducibility, maintainer availability, and the complexity of the required fix.

Use the public issue tracker for ordinary defects that do not create a confidentiality, integrity, availability, or code-execution risk.

## Scope

Security reports may include unsafe repository traversal, unintended file disclosure or modification, malicious snapshot handling, command execution, or dependency vulnerabilities that are reachable through Shun. Search relevance problems and documentation-audit false positives are normal bug reports unless they cross a security boundary.
