# Contributing to Shello

Thanks for your interest in contributing! This document explains how to set up your environment, submit changes, and what to expect during review.

## Getting Started

### Prerequisites

- [Rust](https://rustup.rs/) 1.77.2+
- [Bun](https://bun.sh/) 1.0+
- OS-specific build dependencies:
  - **macOS:** `xcode-select --install`
  - **Ubuntu/Debian:** `sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libxdo-dev libssl-dev patchelf build-essential`
  - **Fedora:** `sudo dnf install webkit2gtk4.1-devel gtk3-devel libappindicator-gtk3-devel librsvg2-devel libxdo-devel openssl-devel patchelf`

### Setup

```sh
git clone https://github.com/yolkmonday/shello.git
cd shello
bun install
bun run tauri dev
```

## Development Workflow

1. **Fork** the repository and create a branch from `main`:
   ```sh
   git checkout -b feat/my-feature
   ```

2. Make your changes. Keep commits focused and atomic.

3. **Test locally** before pushing:
   ```sh
   bun run build        # type-check + build frontend
   bun run tauri build  # full production build (optional, slow)
   ```

4. Push and open a **Pull Request** against `main`.

## Commit Convention

This project uses [Conventional Commits](https://www.conventionalcommits.org/). Every commit message must follow this format:

```
<type>(<scope>): <description>
```

### Types

| Type       | When to use                                  |
| ---------- | -------------------------------------------- |
| `feat`     | New feature                                  |
| `fix`      | Bug fix                                      |
| `docs`     | Documentation only                           |
| `style`    | Formatting, missing semicolons, etc.         |
| `refactor` | Code change that neither fixes nor adds      |
| `perf`     | Performance improvement                      |
| `test`     | Adding or fixing tests                       |
| `chore`    | Build process, tooling, dependencies         |
| `ci`       | CI/CD pipeline changes                       |

### Scopes

Use the subsystem your change affects:

`ssh` · `vault` · `db` · `tunnel` · `sftp` · `ui` · `release` · `workflow`

If the change spans multiple subsystems, omit the scope:

```
feat: add dark/light theme toggle
```

### Examples

```
feat(tunnel): add SOCKS5 proxy support
fix(vault): handle corrupted master password gracefully
docs: add contributing guidelines
chore(release): bump version to 0.1.12
refactor(db): extract shared query helpers
ci: add linux aarch64 build target
```

## Pull Request Guidelines

### Before Opening a PR

- [ ] Code compiles without errors (`bun run build`)
- [ ] Commits follow the convention above
- [ ] Branch is up to date with `main` (rebase if needed)
- [ ] No unrelated changes bundled together

### PR Title

PR titles follow the same convention as commits:

```
feat(tunnel): add SOCKS5 proxy support
```

### PR Description

Include:

1. **What** changed and **why**.
2. **Screenshots/recordings** for UI changes.
3. **Testing steps** — how reviewers can verify.
4. **Breaking changes** — if any, describe migration steps.

### Review Process

- At least **1 approval** required before merge.
- Squash merge to `main` (keeps history clean).
- CI must pass (build + type-check).

## Release Process

Releases are automated via GitHub Actions. Only maintainers can trigger releases.

### Steps

1. Update version in all 3 files:
   - `package.json` → `"version"`
   - `src-tauri/Cargo.toml` → `version`
   - `src-tauri/tauri.conf.json` → `"version"`

2. Commit:
   ```sh
   git commit -m "chore(release): bump version to X.Y.Z"
   ```

3. Push to `main`:
   ```sh
   git push origin main
   ```

4. Tag and push:
   ```sh
   git tag vX.Y.Z
   git push origin vX.Y.Z
   ```

5. GitHub Actions automatically:
   - Builds for **macOS** (universal dmg), **Linux** (x86_64 + aarch64 `.deb`/`.rpm`/`.AppImage`), and **Windows** (`.msi`/`.exe`)
   - Creates a **GitHub Release** with all artifacts and release notes
   - Signs artifacts with `TAURI_SIGNING_PRIVATE_KEY`

### Versioning

This project follows [Semantic Versioning](https://semver.org/):

- **MAJOR** (`1.0.0`): Breaking changes
- **MINOR** (`0.2.0`): New features, backward compatible
- **PATCH** (`0.1.12`): Bug fixes, backward compatible

Pre-`1.0`, minor versions may include breaking changes.

## Reporting Issues

- Use [GitHub Issues](https://github.com/yolkmonday/shello/issues).
- Include OS, architecture, and Shello version.
- Steps to reproduce, expected vs actual behavior.
- Attach logs if available (check per-session logging in the app).

## Code of Conduct

Be respectful. Constructive feedback only. We're here to build something useful together.

## License

By contributing, you agree that your contributions will be licensed under the [MIT License](./LICENSE).
