# Building

## Prerequisites (Windows 10/11 x64)

* Rust stable with the MSVC toolchain (`rustup default stable-x86_64-pc-windows-msvc`) and **Visual Studio Build Tools** (“Desktop development with C++”)
* Node.js 20+ (22 recommended)
* WebView2 runtime (preinstalled on current Windows)
* NSIS is downloaded automatically by the Tauri bundler

## Development

```powershell
npm install
npm run tauri dev
```

Logs and settings live in `%APPDATA%\dev.thomsen.privacyblur`.

## Tests that run anywhere

```bash
cargo test -p privacy-core      # detection, locating, tracking, effects, settings
npm run build                   # TypeScript typecheck + production frontend build
```

Compile-check the Windows layer from Linux/macOS:

```bash
rustup target add x86_64-pc-windows-msvc
cargo check --target x86_64-pc-windows-msvc -p privacy-blur
```

## Release build

```powershell
npm ci
npx tauri build
```

Outputs:

* `target\release\bundle\nsis\Privacy Blur_<version>_x64-setup.exe`: installer (per-user, no admin)
* `target\release\privacy-blur.exe`: standalone executable

Release profile: LTO, single codegen unit, `panic=abort`, stripped.

## GitHub Actions

`.github/workflows/privacy-blur.yml` runs on every push/PR touching `privacy-blur/`:

1. `test`: `cargo test -p privacy-core`, frontend typecheck and build (Ubuntu)
2. `windows`: `tauri build`, uploads installer, portable exe and `SHA256SUMS.txt` as artifacts
3. `release`: on a tag named `privacy-blur-vX.Y.Z`, attaches the files to a GitHub release

To release: bump the version in `src-tauri/tauri.conf.json`, `src-tauri/Cargo.toml`, `package.json`, then `git tag privacy-blur-v0.1.0 && git push --tags`.

## Code signing

Builds are unsigned, so SmartScreen will warn on first run. To sign, add a certificate to the repository secrets and set `bundle.windows.certificateThumbprint` / use `signCommand` in `tauri.conf.json`.
