# Security Policy

## Reporting a Vulnerability

**Do not report security vulnerabilities through public GitHub issues.**

Instead, please report them via:

- **GitHub Security Advisory:** [Report a vulnerability](https://github.com/gregnazario/must/security/advisories/new)

Updates will be provided through the advisory thread as triage proceeds.

## What to Include

- Description of the vulnerability
- Steps to reproduce
- Affected versions
- Potential impact
- Suggested fix (if you have one)

## Scope

The following are in scope:

- Code execution vulnerabilities in the must CLI
- Path traversal or arbitrary file access
- Supply chain issues in dependency handling
- Plugin sandbox escapes

The following are out of scope:

- Denial of service via large inputs
- Issues in dependencies (report upstream)
- Social engineering attacks

## Supported Versions

| Version | Supported |
|---------|-----------|
| 0.2.x   | Yes       |
| < 0.2   | No        |

## Disclosure Policy

When a vulnerability is reported:

1. We will confirm the issue and determine affected versions
2. We will patch the issue and release a new version
3. We will publish a security advisory on GitHub
4. We will credit the reporter (unless they prefer to remain anonymous)
