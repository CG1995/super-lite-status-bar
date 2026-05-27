# Security Policy

## Supported Versions

Security fixes target the latest released version of PulseRing.

## Reporting a Vulnerability

Please do not open a public issue for security-sensitive reports.

For now, report vulnerabilities by opening a private GitHub security advisory if available for this repository. If that is not available, contact the maintainer through GitHub and include only enough detail to establish impact until a private channel is available.

Useful details:

- Affected version or commit.
- Operating system and architecture.
- Steps to reproduce.
- Expected and actual behavior.
- Any logs that do not contain personal data or secrets.

## Scope

Security-sensitive areas include:

- Autostart behavior.
- Local config and log paths.
- Installer and release artifacts.
- Code signing and notarization.
- External command execution used for system metrics.

PulseRing should not require network access for normal metric collection.
