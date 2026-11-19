# Thomsen Optimizer

A free, transparent and reversible gaming optimizer for Windows 10/11, built with Tauri 2 (Rust), React, TypeScript and Vite. It shares the design system of Thomsen Shield, Thomsen VM and Thomsen VPN.

**Principles:** analyse first, explain every change, apply only what the user chooses, record the original value of everything, and never show a number that was not measured. No accounts, ads, telemetry, kernel drivers, injection or "FPS boost" claims.

## What it does

| Page | Real functionality |
| --- | --- |
| Dashboard | Live CPU / GPU / RAM / disk / network readings, gaming status, Analyze System, Optimize Now (preview + confirm), Enable Game Booster, Restore Changes |
| Optimize | Analysis engine -> recommendations that state evidence, change, benefit, downside, restart need and how to undo; selectable background apps; history & restore |
| Game Booster | Three editable profiles (Balanced, Competitive, Maximum Performance) + custom profiles; automatic activation when a game starts, automatic restore when the last game closes |
| Games | Detects Steam, Epic, Xbox app, Battle.net installs; manual games; per-game profile, auto-boost, memory monitoring, Windows graphics preference |
| Performance | Live graphs; FPS via PresentMon (average, 1% low, frame-time graph, stability); saved sessions and before/after comparison |
| System Cleaner | Scan -> review -> confirm -> delete only ticked categories; shader caches are opt-in and discouraged |
| Startup Manager | Enable/disable via Windows' StartupApproved mechanism (nothing is deleted); protected entries locked |
| System Information | CPU, GPU, RAM, storage + physical-disk health, OS, from registry / CIM / system APIs |
| Settings | General, Gaming, Monitoring, Optimization (restore history), Appearance, Advanced (justified options + what is deliberately *not* done), About (signed updater) |

## Safety model

* Every Windows setting change goes through a **write-ahead journal** (`core/src/journal.rs`): the original value is saved before the change, batches are transactional (a failing step rolls the batch back), and each change can be restored individually, per gaming session, or all at once.
* Game-booster sessions are undone automatically after the game closes - and after a crash, at the next start.
* Closed apps and deleted files are logged but marked **not restorable**; the UI never claims otherwise.
* Windows components, security software, anti-cheat, GPU driver services, launchers of a running game and this app are protected from closing. Apps with windows get a polite close request (they can prompt to save).
* The cleaner only works on a fixed allow-list of cache/temp locations, never follows symlinks/junctions, skips files changed since the scan, and reports only what was really removed.
* Runs as a normal user. Elevation is requested (UAC) only for an optional restore point or on explicit "Relaunch elevated" for PresentMon.
* No command lines are ever built from UI input; only fixed arguments, validated GUIDs and validated executable names reach helper processes. The web view gets no file-system/shell/dialog permissions.

## Repository layout

```
core/         Platform-independent Rust logic (unit-tested on any OS)
  sys/        The Windows boundary: registry, powercfg, window enumeration, recycle bin
  journal.rs  Restore journal       analysis.rs  Recommendation engine (pure)
  booster.rs  Game session state machine       cleaner.rs  startup.rs  procs.rs
  games/      Steam VDF, Epic manifests, Xbox/Battle.net scans, matching
  fps.rs      PresentMon capture + frame statistics     history.rs  saved sessions
  monitor.rs  hwinfo.rs   engine.rs (ties it together)
src-tauri/    Thin Tauri shell: commands, tray, single instance, autostart, updater
src/          React UI (design tokens + UI kit reused from Thomsen Shield / VM / VPN)
docs/         ARCHITECTURE.md, VALIDATION.md (what was and was not tested), RELEASING.md
```

## Develop

```bash
npm install
npm run dev            # UI only, in a browser, with a clearly-labelled design-preview mock
npm run tauri dev      # full app (Windows; needs Rust + WebView2)
npm run typecheck && npm test
cargo test -p thomsen-optimizer-core   # all core logic tests, any OS
```

Build the installer (on Windows): `npm run tauri build` -> NSIS installer (x64, per-user install, Start-menu shortcut, optional desktop shortcut, clean uninstall).

## FPS measurement

FPS is measured with [Intel PresentMon](https://github.com/GameTechDev/PresentMon) (MIT). It is **not bundled**: download the console build, then choose it in Performance > Frame rate. Only a file named `PresentMon*.exe` is accepted. PresentMon needs permission to read ETW events (administrator, or the "Performance Log Users" group). Nothing is injected into games, so anti-cheat is not involved. If PresentMon is missing or cannot see a game's frames, FPS is shown as *Unavailable*.

See `docs/VALIDATION.md` for the honest status of testing and `docs/RELEASING.md` for signing and publishing.
