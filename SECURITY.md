# Security Policy

`mdx-rust` edits and executes Rust projects during optimization, so security
reports are taken seriously.

## Supported Versions

Only the latest published version is currently supported.

| Version | Supported |
| ------- | --------- |
| 0.1.x   | Yes       |

## Reporting A Vulnerability

Please report security issues privately through GitHub Security Advisories for
this repository.

If advisories are not available, open a minimal public issue that says you have
a security report to share, without including exploit details.

Useful report details:

- affected command or module
- reproduction steps
- expected and actual behavior
- whether the issue can modify user source, leak secrets, bypass validation, or
  execute unexpected commands

## Security Model

The optimizer should never count a change as accepted unless it passes isolated
validation, net-positive scoring, final validation, and rollback-safe landing.
See [SAFETY_INVARIANTS.md](./SAFETY_INVARIANTS.md).
