# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.1.0] - 2025-05-15

### Added
- **TrackerClient trait** — pluggable HTTP transport layer; default `HttpTrackerClient` and mock support for testing
- **Keyring support** — optional system credential storage via macOS Keychain, Windows Credential Manager, or Linux Secret Service (`--features keyring`)
- **Property-based tests** — proptest suite covering BEncode roundtrips, URL encoding validity, speed simulation, and peer ID/key generation for all 41 profiles
- **Fuzz targets** — libFuzzer targets for `bencode_decode` and `torrent_parse` with CI integration (60s per target on push to main)
- **Criterion benchmarks** — micro-benchmarks for BEncode encode/decode/roundtrip, URL encoding, torrent parsing, speed simulation, and key generation
- **MSRV enforcement** — `rust-version = "1.88"` in all workspace crates, verified in CI
- **MSRV CI job** — dedicated workflow job installing Rust 1.88 and running `cargo check --workspace`
- **SECURITY.md** — vulnerability reporting, code signing verification, and build verification documentation

### Changed
- TCP handshake listener now binds to `127.0.0.1` by default instead of `0.0.0.0`
- HTTP response reads capped at 10 MB to prevent memory exhaustion
- Rate limiting on TCP handshake listener (max 10 concurrent connections)
- Hand-rolled Base64 encoder replaced with the audited `base64` crate
- All public error and config enums marked `#[non_exhaustive]`
- Credentials masked in `Debug` output for `ProxyConfig`, `socks5::Credentials`, and `http::Credentials`

### Fixed
- Potential memory exhaustion from unbounded HTTP response reads
- TCP listener accepting connections on all interfaces

## [1.0.0] - 2025-04-01

### Added
- Initial release of RatioMaster-Rust
- **Core library** (`ratiomaster-core`) with BEncode codec, torrent parser, tracker client, and client emulation engine
- **41 client profiles** across 16 families with accurate peer ID, key generation, and HTTP header emulation
- **Three frontends**: CLI (clap), TUI (ratatui + crossterm), and native GUI (egui/eframe)
- **Proxy support**: SOCKS4, SOCKS4a, SOCKS5, HTTP CONNECT with authentication
- **HTTP and HTTPS**: raw HTTP/1.0 and HTTP/1.1 with TLS (tokio-rustls)
- **Compression**: gzip decompression and chunked transfer encoding
- **Scrape support**: tracker scrape requests
- **Batch mode**: process multiple torrents simultaneously
- **TCP listener**: respond to incoming BitTorrent handshakes
- **Speed simulation**: configurable upload/download speeds with two-level randomization
- **Stop conditions**: upload/download amount, time, ratio, seeder/leecher count
- **Session persistence**: save and resume sessions
- **Configuration**: TOML-based config at `~/.config/ratiomaster/config.toml`
- **Cross-platform**: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- **macOS code signing**: Developer ID signing and Apple notarization
- **CI/CD**: GitHub Actions for build, test, clippy, format, audit, and 5-target release builds
- **365+ tests**: unit, integration, and doc tests

[1.1.0]: https://github.com/xbattlax/ratiomaster-rs/compare/v1.0.0...v1.1.0
[1.0.0]: https://github.com/xbattlax/ratiomaster-rs/releases/tag/v1.0.0
