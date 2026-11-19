# Validation status

This document separates what was **actually run** from what **could not be run** in the build environment (a Linux container with no Windows machine, no GPU and no games). Nothing below is claimed to pass unless it ran.

## Ran and passed

| Check | Command | Result |
| --- | --- | --- |
| Core logic unit tests (79) | `cargo test -p thomsen-optimizer-core` (Linux) | 79 passed |
| Frontend type-check | `npm run typecheck` | clean (strict, `noUncheckedIndexedAccess`) |
| Frontend unit tests (4) | `npm test` | 4 passed |
| Frontend production build | `npm run build` | succeeds; initial JS 264 kB (charts lazy-loaded) |
| Windows-only code compiles | `cargo check -p thomsen-optimizer-core --tests --target x86_64-pc-windows-gnu` and `cargo check -p thomsen-optimizer --target x86_64-pc-windows-gnu` | clean (type-check of registry, powercfg, Win32, tray, updater, commands) |
| UI renders without console errors | Playwright/Chromium against `npm run dev` (design-preview mock), all 9 pages + Optimize Now dialog + settings tabs | no console/page errors; visually reviewed |

What the 79 core tests cover: restore journal (apply/revert, missing original value, automatic rollback of a failed batch, scope restore, crash recovery of dead sessions only, restart persistence, irreversible entries); Game Booster state machine (activation, restore after exit, one-tick tolerance, multiple simultaneous games, auto-boost off, failed activation not retried, manual activation, restore-on-exit off); recommendation rules; Optimize Now only applies safe settings and counts real work, idempotence, rollback on failure; cleaner (min-age, selected categories only, files changed after scan skipped, symlinks not followed, path guards, unknown categories rejected, shader cache never preselected); startup parsing/protection/toggle without deleting; Steam VDF + libraries, Epic manifests, Xbox scan, missing installs; process protection rules and grouping; FPS statistics, PresentMon v1/v2 CSV parsing, a stand-in capture process end-to-end, path validation; settings sanitising; engine cadence, settings validation, restore-all, stale scan/profile rejection; parsers for `nvidia-smi`, `typeperf`, `powercfg /list`, CIM JSON; log redaction.

## Not run - requires a real Windows PC (must be tested before public release)

* Building and running the NSIS installer; install, upgrade, **uninstall**, Start-menu / desktop shortcuts, WebView2 bootstrap; Windows 10 **and** 11 behaviour; x64 only.
* Real registry and `powercfg` behaviour (Game Mode / Game Bar values, power plans, creating Ultimate Performance), including on locked-down/OEM machines and in non-English Windows (parsing is locale-independent by design but untested).
* StartupApproved toggling: it is written under HKCU for both user and all-users entries; that Windows honours the per-user value for machine-wide Run entries has **not** been verified.
* PowerShell/CIM hardware query output, `typeperf` GPU counters, `nvidia-smi`; unsupported-sensor behaviour on real hardware; actual temperature availability.
* Detection on real Steam/Epic/Xbox/Battle.net installs. Xbox app and Battle.net detection are best-effort folder/uninstall-registry scans and may miss titles.
* PresentMon capture (both CLI flag styles), ETW permission failures, "Relaunch elevated", UAC flows, restore-point creation.
* Automatic booster activation/restoration with real games, multiple simultaneous games, anti-cheat-protected games (the design only reads the process list, but this is unverified in practice).
* Tray, minimise-to-tray, autostart with `--minimized`, single-instance, close-to-tray behaviour.
* Updater end to end (needs a signing key and a published release), including failure/rollback paths.
* **Idle CPU/RAM and frame-time impact of the app itself** - the app reports its own usage (Dashboard, Settings > About) but no benchmark numbers were measured.
* Permission-failure and crash-during-optimization paths on real Windows (simulated with fakes in unit tests only).

## Known limitations

* CPU temperature is only shown when Windows/sysinfo exposes it; most PCs need a vendor driver, which this app deliberately never installs.
* GPU temperature/VRAM: NVIDIA via `nvidia-smi`; AMD/Intel get utilisation (and used VRAM) from Windows counters but no temperature.
* "Startup impact" is shown as *Not measured* - Windows does not expose a reliable per-entry figure to normal apps.
* FPS needs PresentMon (separate download) and ETW permissions; one capture at a time.
* Design deviations from the brief: Tailwind CSS was not added - the existing Thomsen apps use plain CSS on shared design tokens and reusing them verbatim keeps the look identical; Lucide and Recharts are used as requested (charts are lazy-loaded). The palette comes from the existing apps' `tokens.css` (e.g. `#050505` background) as the brief asked ("use the existing application's design tokens where available"). The app icon is the shared Thomsen mark taken from Thomsen Shield; give Optimizer its own icon before release if desired.
* The Rust edition/crate versions were resolved at build time (`Cargo.lock` is committed); `sysinfo 0.33` is used because disk throughput is not available in 0.32 (Thomsen VM uses 0.32).
