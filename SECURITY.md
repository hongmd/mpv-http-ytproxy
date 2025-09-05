# Security Policy

## Supported Versions

| Version | Supported          |
| ------- | ------------------ |
| 0.2.x   | :white_check_mark: |
| 0.1.x   | :x:                |

## Security Considerations

**⚠️ Important:** This software implements a Man-in-the-Middle (MITM) proxy that intercepts HTTPS traffic. Please understand the security implications:

### By Design Security Risks

1. **TLS Verification Bypass**: The proxy disables TLS certificate verification for mpv
2. **Traffic Interception**: All YouTube requests pass through the local proxy
3. **Self-Signed Certificates**: Uses locally generated certificates with known passphrase

### Safe Usage Guidelines

- ✅ **Only use on trusted, personal devices**
- ✅ **Only for YouTube content optimization**
- ✅ **Ensure proxy binds only to localhost (127.0.0.1)**
- ✅ **Review code before installation**
- ❌ **Do not use on shared or public networks**
- ❌ **Do not use for sensitive browsing**
- ❌ **Do not expose proxy to external networks**

## Reporting Security Issues

If you discover a security vulnerability, please report it privately:

1. **Do not open a public issue**
2. Email security concerns to: [maintainer email]
3. Include detailed steps to reproduce
4. Allow 90 days for response and patching

## Security Best Practices

### For Users
- Regularly review the proxy logs for unexpected activity
- Disable the proxy when not actively streaming YouTube
- Consider using VPN as an alternative for sensitive environments
- Keep the software updated

### For Developers
- All user input must be validated and sanitized
- Error handling should never expose sensitive information
- Dependencies should be regularly updated for security patches
- Code reviews must include security considerations

## Threat Model

### In Scope
- Local privilege escalation via the proxy
- Network traffic manipulation beyond intended YouTube optimization
- Certificate/key management vulnerabilities
- Process management security issues

### Out of Scope
- Network-level attacks (proxy only binds to localhost)
- YouTube's servers or services
- mpv player vulnerabilities (unless caused by this proxy)
- Operating system level vulnerabilities

## Compliance

This software is intended for personal use only. Users are responsible for ensuring compliance with:
- Local laws and regulations
- YouTube Terms of Service
- Network/ISP acceptable use policies
- Employer IT policies (if applicable)

---

**Remember**: This tool prioritizes functionality over security. Use with full understanding of the risks involved.
