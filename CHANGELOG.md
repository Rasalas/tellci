# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.0] - 2026-08-22

### Added

- `--details` flag for `tellci skip` to override the skipped testcase message
- Installer verifies downloads against a published `SHA256SUMS` file; the release workflow now publishes checksums for every artifact
- Declared MSRV: Rust 1.85 via `rust-version`
- `CHANGELOG.md`

### Changed

- Report files are written atomically through a temporary file and rename, so interrupted runs cannot leave a truncated report
- `tellci run` caps captured stdout/stderr in failure details at 8 KiB per stream with an explicit truncation marker

### Fixed

- The dogfood CI check verifies its GitHub job summary within the same workflow step that writes it

## [0.1.1] - 2026

### Added

- `install.sh` release installer with `TELLCI_VERSION`, `TELLCI_INSTALL_DIR`, and `TELLCI_REPO` overrides
- GitHub Actions job summaries and error annotations via `tellci finish --platform`
- CI platform auto-detection for `tellci finish`
- `skip`, `run`, `reset`, and `status` commands
- `--fatal` flag, `--details` flag, and `TELLCI_FILE` environment variable

## [0.1.0] - 2026

### Added

- Initial release: `pass`, `fail`, `error`, `finish` commands
- JUnit XML report output compatible with GitLab merge request test reports
