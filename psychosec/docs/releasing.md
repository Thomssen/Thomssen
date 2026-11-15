# Building and releasing

## One-time: release signing key
```bash
node scripts/release-keygen.mjs      # prints PSYCHOSEC_UPDATE_PUBKEY, writes release-signing/release-private.pem
```
Store the PEM in a password manager and as the GitHub secret `RELEASE_SIGNING_KEY`. Put the public key in the GitHub
secret `PSYCHOSEC_UPDATE_PUBKEY`; it is compiled into the executable. A build without it refuses every update.

## Build locally (Windows 10/11)
Install Rust (MSVC toolchain), Inno Setup 6, Node 20+.
```powershell
pwsh scripts/build-windows.ps1 -Version 1.0.0 -UpdatePubKey <base64>
```
Produces `target\release\Psychosec.exe` and `installer\Output\Psychosec-Setup-v1.0.0.exe` (+ `Psychosec-Setup.exe`).
Release profile: fat LTO, `panic=abort`, symbols stripped, build paths remapped, static CRT.

## Publish
1. Tag `v1.0.0` on the private repository → `.github/workflows/release.yml` builds and uploads the installer and
   `SHA256SUMS.txt` (never source archives).
2. Host the installer at a public HTTPS URL (assets of a private GitHub repository are not public). Options: Vercel Blob,
   `apps/website/public/dl/`, object storage.
3. Sign and register the release so the app and website can offer it:
   ```bash
   DATABASE_URL=... node scripts/release-sign.mjs installer/Output/Psychosec-Setup-v1.0.0.exe 1.0.0 \
     https://psychosec.org/dl/Psychosec-Setup-v1.0.0.exe --notes "What changed" --publish
   ```
   The manifest signs `version`, `download_url` and the installer's SHA-256 with Ed25519. The CLI verifies the
   signature **and** re-hashes the download before it will run anything.

## Authenticode (optional, recommended)
Add `WINDOWS_CERT_PFX_BASE64` and `WINDOWS_CERT_PASSWORD` secrets; the release workflow then signs the installer with
`signtool`. Without a certificate Windows SmartScreen will warn on first runs; this is expected and is not bypassed.

## Upgrades / uninstall
The installer uses a fixed `AppId`, so a newer installer upgrades in place and keeps user data. The uninstaller offers to
delete `%LOCALAPPDATA%\PSYCHOSEC` (cases, settings); the default is to keep it. Installed per-user to
`%LOCALAPPDATA%\Programs\PSYCHOSEC`; registered in Installed Apps.
