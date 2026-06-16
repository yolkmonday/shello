# Changelog

All notable changes to Shello are documented here. This project follows [Conventional Commits](https://www.conventionalcommits.org/) and [Semantic Versioning](https://semver.org/).

## [0.1.11] — 2026-06-16

### Added

- **Linux aarch64 builds** — ARM64 `.deb`, `.rpm`, `.AppImage` now available for Raspberry Pi, Asahi Linux, and cloud ARM instances.
- `CLAUDE.md` — project-specific AI assistant context.
- `CONTRIBUTING.md` — contributing guidelines and release workflow.
- `CHANGELOG.md` — this file.

### Changed

- Release workflow: added `ubuntu-22.04-arm` to build matrix.
- Release workflow: Linux dependency install now matches all `ubuntu-*` runners.

## [0.1.10] — 2026-06-16

### Added

- Toggle credential encryption on/off in Settings.
- `vault_disable` configuration option.

### Removed

- Development bypass for vault encryption.

## [0.1.9] — 2026-06-16

### Added

- Remote (`-R`) port forwarding — phase 3 (handler routing, full port forwarding).

## [0.1.8] — 2026-06-16

### Added

- Remote (`-R`) forwarding — phase 2.

---

For the full history, see [GitHub Releases](https://github.com/yolkmonday/shello/releases).
