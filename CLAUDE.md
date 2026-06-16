# CLAUDE.md — Shello

## What This Is

Shello is a lightweight SSH client built with **Tauri 2.0 + Vue 3 + Rust**.
Local-first, no cloud, no telemetry. Everything stored in SQLite on the user's machine.

## Quick Reference

```sh
bun install          # install frontend deps
bun run tauri dev    # full app (Vite + Rust)
bun run tauri build  # production build
bun run dev          # frontend only (no Tauri window)
bun run build        # type-check + build frontend
```

## Architecture

```
src/                  # Vue 3 frontend
  components/         # Vue SFCs (PascalCase)
  stores/             # Pinia stores (camelCase)
  lib/                # shared utils (kebab-case)
  style.css           # Tailwind base + theme vars (otter theme)
  main.ts             # app entry

src-tauri/            # Rust backend
  src/
    lib.rs            # Tauri command registrations, module declarations
    main.rs           # entry point
    crypto.rs         # AES-GCM encryption primitives
    vault.rs          # encrypted credential vault
    registry.rs       # host metadata cache
    db/               # SQLite layer (sqlx)
      mod.rs          # DbPool, migrations
      profiles.rs     # SSH host profiles & groups
      snippets.rs     # saved commands
      custom_recipes.rs # multi-step command workflows
      tunnels.rs      # port forwarding rules
      registry.rs     # host cache persistence
    ssh/              # SSH subsystem (russh)
      client.rs       # low-level SSH client
      session.rs      # SessionManager (Tauri state)
      pty.rs          # PTY allocation & streaming
      keys.rs         # key generation & management
      types.rs        # ConnectionConfig, AuthMethod, SessionInfo
      config_import.rs # parse ~/.ssh/config
    sftp/             # SFTP file browser (russh-sftp)
    tunnel/           # local/remote port forwarding + SOCKS5
  Cargo.toml
  tauri.conf.json     # Tauri config (plugins, window, updater)
```

## Key Patterns

### Frontend (Vue 3 + TypeScript)
- **Composition API** + `<script setup>` everywhere.
- **Pinia stores** for state, calling Rust via `invoke()` from `@tauri-apps/api/core`.
- **Tailwind CSS v3** with custom `otter-*` color tokens (defined in `style.css` CSS vars, extended in `tailwind.config.js`).
- **xterm.js** (`@xterm/xterm`) with WebGL renderer, fit/search/web-links addons.
- **No ESLint/Prettier configured** — keep code clean manually.
- Components: `PascalCase.vue`. Stores: `camelCase.ts`. Utils: `kebab-case.ts`.

### Backend (Rust)
- **Tauri commands** are `#[tauri::command]` async functions registered in `lib.rs`.
- **Error handling:** return `Result<T, String>` from commands, `.map_err(|e| e.to_string())`.
- **State:** Tauri managed state (`tauri::State<>`) for `SessionManager`, `VaultState`, `DbPool`, `TunnelManager`.
- **Database:** SQLite via `sqlx` with embedded migrations in `db/migrations/`.
- **IDs:** ULID (`ulid` crate) — monotonic, sortable.
- **Encryption:** AES-256-GCM (`aes-gcm` + `pbkdf2`) for vault credentials.

### Cross-Process Bridge
- Vue calls Rust through `invoke("command_name", { args })`.
- Event streaming: Rust emits via `app.emit()`, frontend listens with `listen()` from `@tauri-apps/api/event`.
- Input schemas defined as TypeScript interfaces in stores, matching Rust struct field names (snake_case).

## Commit Convention

Use [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <description>
```

Types: `feat`, `fix`, `chore`, `docs`, `refactor`, `test`, `ci`, `perf`, `style`.
Scopes: `ssh`, `vault`, `db`, `tunnel`, `sftp`, `ui`, `release`, `workflow`.

Examples:
```
feat(tunnel): add SOCKS5 proxy support
fix(vault): handle corrupted master password gracefully
chore(release): bump version to 0.1.12
docs: add contributing guidelines
```

## Release Process

1. Update version in **3 files**: `package.json`, `src-tauri/Cargo.toml`, `src-tauri/tauri.conf.json`.
2. Commit: `chore(release): bump version to X.Y.Z`.
3. Push to `main`.
4. Tag: `git tag vX.Y.Z && git push origin vX.Y.Z`.
5. GitHub Actions builds artifacts for macOS (universal), Linux (x86_64 + aarch64), Windows and creates a Release.

## What NOT to Do

- Don't introduce Prisma, NextAuth, axios, or other heavy deps without discussion.
- Don't switch package manager (Bun is standard).
- Don't add ESLint/Prettier without updating this file.
- Don't store secrets in code — vault handles that at runtime.
- Don't change the theme token naming (`otter-*`) without migrating all components.
