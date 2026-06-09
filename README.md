# Shello

**Dive into any server.** Lightweight SSH client built with Tauri 2.0 + Vue 3 + Rust.

## Features

- **SSH terminal** — full PTY support, multiple concurrent sessions, OS detection, per-session logging.
- **Connection manager** — organize hosts into profiles and groups, with search and tags.
- **Encrypted vault** — passwords, passphrases, and keys are encrypted at rest behind a master password.
- **SSH key management** — generate and manage keys without leaving the app.
- **Snippets** — save and reuse common commands.
- **Recipes** — define multi-step command workflows and run them against any host.
- **Screenshots & recordings** — capture terminal output for sharing.

> **Local-first.** Everything is stored locally on your machine — no cloud account, no telemetry.

## Install (macOS)

### Via Homebrew

```sh
brew tap yolkmonday/shello
brew install --cask shello
```

### Manual download

Grab the latest `.dmg` from [Releases](https://github.com/yolkmonday/shello/releases), open it, and drag **Shello.app** into `/Applications`.

### First launch — Gatekeeper warning

The current builds are **not yet code-signed or notarized**, so on first launch macOS will say *"Shello.app is damaged and can't be opened"* or *"cannot be opened because the developer cannot be verified"*. This is expected. To bypass it once:

**Option A — right-click open:**

1. Open Finder → `/Applications`.
2. Right-click (or Control-click) **Shello.app** → **Open**.
3. Click **Open** again in the dialog. macOS remembers the choice; subsequent launches work normally.

**Option B — remove the quarantine attribute** (if Option A doesn't work):

```sh
xattr -dr com.apple.quarantine /Applications/Shello.app
```

Then double-click as usual.

> Code-signing + notarization are on the roadmap and will remove this step entirely.

## Development

### Prerequisites

- [Rust](https://rustup.rs/) 1.77.2+
- [Bun](https://bun.sh/) 1.0+
- Xcode Command Line Tools (macOS): `xcode-select --install`

### Setup

```sh
bun install
bun run tauri dev
```

### Scripts

| Command | Description |
|---------|-------------|
| `bun run tauri dev` | Start the full app (Rust + Vite dev server) |
| `bun run tauri build` | Production build (packaged app) |
| `bun run dev` | Frontend only (Vite, no Tauri window) |
| `bun run build` | Type-check + build frontend assets |

## Tech Stack

- **Frontend:** Vue 3, TypeScript, Tailwind CSS v3, Pinia
- **Backend:** Rust, Tauri 2.0
- **Package Manager:** Bun

## Project Structure

    src/            # Vue 3 frontend (components, stores)
    src-tauri/      # Rust backend (ssh, vault, db, registry, commands)
    docs/           # Release docs

## Releasing

Tag pushes (`v*`) trigger `.github/workflows/release.yml`, which builds a universal `.dmg` (arm64 + x64) and uploads it to a GitHub Release. See [`docs/homebrew-tap.md`](docs/homebrew-tap.md) for tap setup and the cask update flow.

## Contributing

Issues and pull requests are welcome. Please open an issue to discuss substantial changes before submitting a PR.

## License

[MIT](./LICENSE) © Ari Padrian
