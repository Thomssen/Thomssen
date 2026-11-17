# Architecture

```
privacy-blur/
├─ crates/privacy-core/   platform-independent, unit-tested (runs on Linux/macOS/Windows)
│   detect.rs   text → (byte span, category) with validated patterns + user entries
│   locate.rs   spans → pixel rectangles (word rects, proportional cut for partial words)
│   track.rs    temporal stabiliser: hold time, jitter filter, scroll following
│   effects.rs  Gaussian / pixelate / solid renderers, safe-minimum rules
│   settings.rs / entries.rs   serde models, validation, clamping
├─ src-tauri/             Windows-only
│   win/capture.rs   GDI BitBlt capture + 64×64 tile hashing (dirty-region detection)
│   win/ocr.rs       Windows.Media.Ocr wrapper, crop/scale helper
│   win/uia.rs       UI Automation (TextPattern visible ranges, element names)
│   win/overlay.rs   pooled click-through layered windows, panic cover
│   win/winlist.rs   z-ordered windows, owning executables, process list
│   win/dpapi.rs     CryptProtectData/CryptUnprotectData
│   engine.rs        render / detect / uia threads
│   store.rs         settings.json + DPAPI-encrypted entries.dat
│   commands.rs, app.rs, lib.rs   Tauri commands, tray, global shortcuts, autostart
└─ src/                   React UI (talks to Rust only through typed `invoke` calls)
```

## Pipeline

```
 render thread (every interval_ms)             detect thread                    uia thread
 ───────────────────────────────────          ─────────────────────────       ──────────────────────
 BitBlt each enabled monitor                   receive newest frame            walk visible windows' UIA tree
 hash 64×64 tiles → changed bbox               OCR only the changed rows       TextPattern → exact rects
 changed? → send Arc<frame> (drop if busy)     (+ full rescan every ≥3 s)      names of labels → estimated rects
 draw overlays from latest regions  ◄───────   merge OCR + UIA, drop UIA hits
 using the freshest pixels                      hidden under other windows
                                                apply app rules (owner window)
                                                Tracker: hold, jitter, scroll
```

* **Where do coordinates come from?** OCR gives word rectangles; UIA gives exact character-range rectangles for TextPattern controls. Hits inside a line are mapped to rectangles per word, cutting partially matched words proportionally by character count, widened by half a character.
* **Overlay content.** Each region window shows the *censored copy of the real pixels* (Gaussian/mosaic/solid) with `UpdateLayeredWindow`. Memory scales with the censored area, not with the screen. `WS_EX_LAYERED | WS_EX_TRANSPARENT` makes input fall through even across processes; `WS_EX_NOACTIVATE` leaves keyboard focus alone; `WM_NCHITTEST → HTTRANSPARENT` and `WM_MOUSEACTIVATE → MA_NOACTIVATE` as belt and braces.
* **Why the detector does not see its own overlay.** `BitBlt(SRCCOPY)` from the screen DC leaves layered windows out, so the detector gets an overlay-free picture while the overlay stays on screen (no flicker, no hide/show cycle). This is an OS behaviour we depend on, so the *Capture test* measures it on your machine. If it ever fails, you get flicker (regions drop out and return after the hold time), never a leak.
* **Flicker control.** Regions are tracked across passes; a region is only removed after `hold_ms` without being seen. Sub-pixel jitter is ignored (IoU ≥ 0.9). Redraws happen only when regions or underlying pixels changed.
* **CPU/RAM.** Idle screen = one BitBlt + tile hash per interval, no OCR. When something changes only the affected rows are OCR'd. Capture buffers are released when protection is stopped. UIA runs on its own thread at ≥ 600 ms intervals with element and time budgets.
* **DPI.** The process is Per-Monitor-V2 DPI aware, so every rectangle (GDI, UIA, windows, overlays) is in physical pixels; mixed 100–300 % scaling works without conversion.
* **Why GDI instead of Windows.Graphics.Capture?** WGC includes layered windows in its frame, so the detector would have to hide or exclude the overlay to avoid reading its own blur, which means either flicker or giving up capture-visible overlays. GDI gives the clean frame for free. WGC could replace GDI later for HDR/protected-content cases.

## Data & security

* `%APPDATA%\dev.thomsen.privacyblur\settings.json`: preferences only (toggles, app lists, appearance).
* `…\entries.dat`: your personal entries, DPAPI-encrypted (user scope, app-specific entropy). If it cannot be decrypted it is renamed `.unreadable`, never deleted.
* `…\privacy-blur.log`: events such as “protection started”; the pipeline keeps no text, so none can be logged. Capped at 256 KiB.
* The Tauri capability file grants the UI only `core:default` and window dragging: no filesystem, shell or HTTP access, and no network plugins are compiled in. The CSP forbids remote connections.
* Matched text exists only as transient strings on the detection thread; `Hit`, `Region` and the status DTO carry categories and rectangles only.
