# Releasing

## One-time setup

1. Create the update-signing key pair (keep the private key secret):
   ```bash
   npx tauri signer generate -w thomsen-optimizer.key
   ```
2. In the GitHub repository that publishes releases add:
   * secret `TAURI_SIGNING_PRIVATE_KEY` (contents of the private key file)
   * secret `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`
   * variable `TAURI_UPDATER_PUBKEY` (contents of the `.pub` file)
3. The updater endpoint in `src-tauri/tauri.conf.json` and the links in `commands.rs::open_external` point to `github.com/Thomssen/Thomsen-Optimizer`. If releases are published from another repository, change those two places.

## Publishing

1. Bump the version in `package.json`, `src-tauri/Cargo.toml`, `core/Cargo.toml`, `src-tauri/tauri.conf.json` and `src/config/app.ts`.
2. Tag and push: `git tag optimizer-v1.0.0 && git push origin optimizer-v1.0.0`.
3. `.github/workflows/thomsen-optimizer-release.yml` runs the tests, builds the x64 NSIS installer, signs the update bundle, and creates a **draft** release containing the installer, its signature and `latest.json`. Test the installer, then publish the draft.

Without the signing secrets the workflow still builds an installer, but that build has updates disabled (the app says so in Settings > About).

## Installer

NSIS, per-user install (no administrator rights needed), Start-menu shortcut in "Thomsen Software", desktop shortcut as offered by the Tauri NSIS template (verify on the first real build), uninstaller that removes the program files. User data in `%APPDATA%\dev.thomssen.optimizer` is kept on uninstall unless the user ticks the delete-data option. WebView2 is bootstrapped silently when missing. Windows restore points created by the app are system objects and are not removed.
