# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.1.x   | :white_check_mark: |

## Reporting a Vulnerability

We take security vulnerabilities seriously. If you discover a security issue, please report it responsibly.

### How to Report

**Please do NOT report security vulnerabilities through public GitHub issues.**

Instead, please report them via one of these methods:

1. **GitHub Security Advisories** (Preferred): Use [GitHub's private vulnerability reporting](https://github.com/YOUR_USERNAME/mindia/security/advisories/new) to report the issue directly.

2. **Email**: Send details to `security@example.com` (replace with your actual security contact).

### What to Include

When reporting a vulnerability, please include:

- A description of the vulnerability
- Steps to reproduce the issue
- Potential impact of the vulnerability
- Any possible mitigations you've identified

### What to Expect

- **Acknowledgment**: We will acknowledge receipt of your report within 48 hours.
- **Initial Assessment**: Within 7 days, we will provide an initial assessment of the report.
- **Resolution Timeline**: We aim to resolve critical vulnerabilities within 30 days.
- **Disclosure**: We will coordinate with you on the disclosure timeline and credit.

### Safe Harbor

We support safe harbor for security researchers who:

- Make a good faith effort to avoid privacy violations, destruction of data, and interruption of services
- Only interact with accounts you own or with explicit permission
- Do not exploit a vulnerability beyond what is necessary to demonstrate it
- Report vulnerabilities promptly and do not publicly disclose before we've had a chance to address them

## Security Best Practices for Users

When deploying Mindia, ensure you:

1. **Use strong API keys**: Generate API keys with sufficient entropy (minimum 32 characters)
2. **Enable HTTPS**: Always run behind a TLS-terminating proxy in production
3. **Restrict network access**: Limit database and storage access to trusted networks
4. **Rotate secrets regularly**: Periodically rotate API keys and JWT secrets
5. **Keep dependencies updated**: Regularly update to the latest version
6. **Review access logs**: Monitor for unusual API access patterns
