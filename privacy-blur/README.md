# Privacy Blur

Real-time, fully local privacy protection for Windows 10/11. Privacy Blur finds sensitive text anywhere on your screen (email addresses, phone numbers, names, home addresses, IPs, card and bank numbers, tokens, usernames, dates of birth, and anything you add yourself) and draws a blur, mosaic or black box **over just that text**. Everything else stays visible.

* **Tauri 2 + React + TypeScript + Tailwind + Lucide** UI, **Rust** engine
* Detection: validated regular expressions, **Windows UI Automation** (exact text rectangles) and **local OCR** (`Windows.Media.Ocr`)
* Overlay: click-through layered windows, per-monitor, per-pixel alpha, DPI-aware
* No account, license key, telemetry or network access. No screenshot or text ever leaves memory.

> **Status:** the platform-independent core (detection, locating, tracking, blur renderers, settings) is unit-tested (`cargo test -p privacy-core`, 28 tests) and the whole Windows layer compiles against `x86_64-pc-windows-msvc`. The author's build environment was Linux, so the Windows runtime behaviour (overlay, UIA, OCR, tray, shortcuts) has **not yet been exercised on a real desktop**. Work through [docs/TESTING.md](docs/TESTING.md) on a Windows machine before relying on it, and read [docs/LIMITATIONS.md](docs/LIMITATIONS.md).

## Install

Download from the repository's *Releases* (or the Actions artifact):

* `Privacy Blur_x.y.z_x64-setup.exe`: per-user NSIS installer (no admin rights needed)
* `PrivacyBlur-portable.exe`: standalone executable (needs the WebView2 runtime, preinstalled on current Windows 10/11)
* `SHA256SUMS.txt`

Requirements: Windows 10 2004+ or Windows 11, and at least one Windows language pack (for OCR; the language you already use is enough).

## Using it

One small dark window: a **Start/Stop** button, toggles for what to hide, a **blur strength** slider with Blur / Pixels / Black box, and **My words** (type a name, address, email, phone number or any word; it is stored encrypted). *Options* holds Start with Windows, tray behaviour, skipping browsers and the panic button. Everything else (detection method, intervals, per-app rules) lives in `settings.json` with sensible defaults.

Default shortcuts (all configurable): `Ctrl+Shift+B` toggle protection · `Ctrl+Shift+P` open settings · `Ctrl+Shift+X` **panic** (black cover over every monitor, click-through, visible in captures).

## Develop

```bash
npm install
npm run tauri dev        # on Windows
cargo test -p privacy-core   # any OS
```

See [docs/BUILDING.md](docs/BUILDING.md), [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md), [docs/LIMITATIONS.md](docs/LIMITATIONS.md) and [docs/TESTING.md](docs/TESTING.md).

## License

MIT, © Thomsen Software
