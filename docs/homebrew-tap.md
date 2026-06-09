# Homebrew Tap Setup

How to publish Shello via your own Homebrew tap.

## 1. Create the tap repo

Create a new public repo on GitHub named **`homebrew-shello`** (the `homebrew-` prefix is required by Homebrew).

```
github.com/yolkmonday/homebrew-shello
```

Inside it, create a `Casks/` folder.

## 2. Cask file

Save as `Casks/shello.rb` in the tap repo:

```ruby
cask "shello" do
  version "0.1.0"
  sha256 "REPLACE_WITH_SHA256"

  url "https://github.com/yolkmonday/shello/releases/download/v#{version}/Shello_v#{version}_universal.dmg"
  name "Shello"
  desc "Lightweight SSH terminal client"
  homepage "https://github.com/yolkmonday/shello"

  app "Shello.app"

  zap trash: [
    "~/Library/Application Support/dev.shello.client",
    "~/Library/Preferences/dev.shello.client.plist",
    "~/Library/Caches/dev.shello.client",
  ]
end
```

## 3. Cutting a release

1. Bump `version` in `src-tauri/tauri.conf.json` and `package.json`.
2. Commit, then tag and push:
   ```
   git tag v0.1.0
   git push origin v0.1.0
   ```
3. The `.github/workflows/release.yml` workflow builds a universal `.dmg` (arm64 + x64) and creates a GitHub Release with a `.sha256` file.
4. Grab the SHA from the release assets.
5. Update `Casks/shello.rb` in the tap repo: bump `version` and replace `sha256`. Commit + push.

## 4. Install

Users then install with:

```
brew tap yolkmonday/shello
brew install --cask shello
```

To upgrade later: `brew upgrade --cask shello`.

## Future: notarization

The current build is **unsigned**. macOS Gatekeeper will block the app on first launch — see the README for the right-click → Open workaround. To remove that friction, you'll need an Apple Developer ID ($99/year), then wire up signing via `tauri.conf.json` (`bundle.macOS.signingIdentity`) and notarization env vars (`APPLE_ID`, `APPLE_PASSWORD`, `APPLE_TEAM_ID`) as GitHub Actions secrets.
