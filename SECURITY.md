# Security Policy

## Reporting a Vulnerability

If you discover a security vulnerability, please report it responsibly:

1. **Do not** open a public issue
2. Email security concerns to the repository owner
3. Allow reasonable time for a fix before public disclosure

## Code Signing

macOS binaries in releases are signed with a Developer ID Application certificate and submitted for Apple notarization. Verify signatures with:

```sh
codesign --verify --verbose=2 RatioMaster.app
spctl --assess --verbose=2 RatioMaster.app
```

## Build Verification

All release binaries are built in GitHub Actions CI with checksums (SHA-256) provided alongside each release asset. Verify downloads with:

```sh
shasum -a 256 -c ratiomaster-*.sha256
```

## Dependencies

This project uses only well-known Rust crates. Run `cargo audit` to check for known vulnerabilities in dependencies.
