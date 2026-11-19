# Architecture

## Layers

```
React UI (src/)  --invoke / events-->  Tauri shell (src-tauri/)  -->  Engine (core/src/engine.rs)
                                                                          |
        analysis, journal, booster, cleaner, startup, games, fps ... <----+
                                                                          |
                                                              SystemOps trait (core/src/sys)
                                                                  |                   |
                                                          WindowsOps (winreg,     FakeOps (tests)
                                                          powercfg, Win32)
```

* **core** has no Tauri dependency. All Windows access is behind the `SystemOps` trait plus a few functions in `sys/win.rs` (window enumeration, recycle bin, version info, elevation), which have inert stubs on other platforms. This is what makes the safety-critical logic testable (79 unit tests run on Linux and Windows CI).
* **src-tauri** only defines commands (`commands.rs`), the tray, single-instance, autostart and updater. Blocking work runs on the blocking pool.
* **src** talks to Rust only through `services/api.ts` (typed wrappers). `services/mock.ts` is used *only* when `__TAURI_INTERNALS__` is absent (browser design preview) and the UI shows a banner saying so.

## Data (all local, `%APPDATA%\dev.thomssen.optimizer\`)

`settings.json`, `journal.json` (restore history), `games.json` (per-game settings, custom games), `sessions.json` (FPS sessions), `optimizations.json` (last 30 results), `app.log` (size-capped, user path/name redacted), `tools/` (optional PresentMon). Writes are atomic (temp + rename); a corrupt file is renamed `*.corrupt` instead of being overwritten.

## Adaptive monitoring (resource use)

The engine thread sleeps until the next due task:

| Situation | Sample interval |
| --- | --- |
| Dashboard / Performance / Game Booster visible | user setting (default 1 s, >= 2 s while gaming) |
| Other page visible | 5 s (10 s while gaming) |
| Window hidden / minimised / in tray | 15 s, no GPU probing, no UI events |

The process list is refreshed at most every 5 s for the sampler; game detection runs every 3 s (5 s while gaming) and can be switched off. GPU data comes from `nvidia-smi` when present, otherwise from Windows' GPU Engine counters (slower minimum cadence of 5 s), and is probed outside the monitor lock. The app measures and displays its own CPU/RAM use (Dashboard footer, Settings > About).

## Game Booster state machine (`booster.rs`)

`tick(running games)` -> events. First eligible game starts a session (scope `session:<uuid>`) and applies its profile through the journal. Later games join the session. A game must be absent for two consecutive ticks before it counts as closed; when the last one closes, the scope is reverted. On start-up, `Journal::recover_after_crash` reverts any `session:` scope that is not live. Manual activation creates a session with no games that ends only when the user restores it.

## Recommendation engine (`analysis.rs`)

`analyze(&Snapshot) -> Vec<Recommendation>` is a pure function. Each rule quotes the measurement that triggered it. Only rules that change a reversible Windows setting are `safe_auto` (power plan away from Power saver, Game Mode explicitly off, Game Bar capture on). Rules about memory, CPU, startup, storage, GPU and uptime are advice that links to the relevant page.

## Security notes

* Helper processes: `powercfg.exe`, `powershell.exe` (fixed read-only scripts, plus one fixed elevated restore-point script), `typeperf.exe`, `nvidia-smi`, and the user-selected PresentMon. Arguments are fixed or validated (GUID regex, `*.exe` name rules, `PresentMon*.exe` filename). No shell strings.
* Tauri capabilities grant the web view only window controls and events. File dialogs, links and elevation are Rust commands with fixed targets.
* CSP: `default-src 'self'`; no remote scripts or fonts (Inter is bundled).
* Updater: signed artifacts only; with no public key configured updating is disabled rather than unsigned.
