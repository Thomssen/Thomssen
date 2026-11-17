# Limitations (read before relying on it)

Privacy Blur is a best-effort filter, not a guarantee. Anything it misses is visible.

## Detection

* **Names and addresses cannot be recognised perfectly from text.** Structured data (emails, IPs, card numbers with Luhn, IBANs with mod-97, tokens) is matched reliably. Real names and street addresses are matched reliably only when you add them under *Privacy Filters → My personal information* (several spellings are covered). Generic name/address detection is heuristic (labels such as “Name:”, “Dear …”, “Mr …”, street+number patterns) and will miss some and occasionally hit the wrong text.
* **OCR errors.** Windows OCR can misread small, low-contrast or stylised text. Misread text may not match. Lowering *OCR image scale* makes this worse.
* **Latency.** New text is covered after the next detection cycle (typically 100–500 ms). While scrolling quickly, text can be briefly visible before it is covered again. Lower the update interval for tighter following (costs CPU).
* **UI Automation availability.** Browsers and Electron apps expose text to UIA only when accessibility is active; many custom-drawn UIs do not at all. The OCR pass covers these. For label controls without per-character geometry the UIA rectangle is an estimate (padded); OCR supplies the precise one.
* **Text inside images, video, games and remote desktops** is only found by OCR.
* **Multi-line values** (an address split over two lines) are only hidden when each line matches on its own, or when you saved the address (each line containing a digit is matched separately).

## Windows restrictions

| Situation | Behaviour |
|---|---|
| **Elevated (administrator) windows** | UI Automation cannot read them from a normal-privilege process (UIPI). OCR still works, because screen capture is allowed. Overlays still draw above them. |
| **Exclusive-fullscreen games / DirectX exclusive mode** | Neither capture nor overlays can appear over exclusive fullscreen. Use borderless/windowed fullscreen. |
| **DRM / protected video, some HDR content** | The OS returns black pixels to GDI capture; nothing can be read there (and nothing readable is shown to other captures either). |
| **Secure desktop** (UAC prompt, lock screen, Ctrl+Alt+Del) | Not capturable by any normal app; protection pauses visually. |
| **Always-on-top apps** | Overlays are re-asserted topmost every ~0.8 s, so another topmost window may cover them briefly. |
| **Windows N / no language pack** | OCR is unavailable; UI Automation + patterns still work. The dashboard shows the reason. |

## Screen sharing and recording

Privacy Blur's overlay is a normal window. What a capture tool records depends on *how* it captures:

| Capture path | Blur recorded? | Basis |
|---|---|---|
| Whole-screen / monitor capture through GDI (`BitBlt` with `CAPTUREBLT`) | **Yes** (when “Show the blur in screen captures” is on) | **Measured** by the built-in capture test |
| Monitor capture through DXGI Desktop Duplication or Windows Graphics Capture (OBS *Display Capture*, Discord/Teams/Zoom *Screen*) | **Expected yes** | **Not measured** by the built-in test; these APIs capture the composed desktop. Verify with the test patch |
| **Single-window capture** (Discord/Teams/Zoom *Window*, OBS *Window Capture*, Game Capture) | **No, the original text is captured** | **Measured** for `PrintWindow`; window-based APIs render only the chosen window |
| Hardware capture cards / GPU-level capture of the final video signal | Yes, if the overlay is visible on your monitor | By construction |
| Another computer viewing via remote desktop of your session | Depends on the remote tool | Not tested |

* The Settings page contains a **Capture test**, which draws a magenta test patch and captures it through the paths above, reporting what it actually measured. Paths it cannot measure are labelled **Not verified**. Use *Keep test patch visible* and look at that patch in OBS/Discord/Teams' own preview to verify your exact setup. If you can see the magenta patch in the preview, the blur is recorded too.
* **When sharing, share the entire screen, not a window.** The dashboard warns when OBS/Discord/Teams/Zoom and similar are running.
* “Show the blur in screen captures” **off** excludes the overlay from every capture (`WDA_EXCLUDEFROMCAPTURE`): then viewers see the original text. Use it only if you want the blur for yourself alone.
* The **panic cover** is always part of captures.
* Nothing in this project claims that a particular app hides your data until you have verified that path yourself.

## Blur strength

* The Gaussian blur is a real 3-pass box-blur approximation, computed on the CPU from a snapshot, drawn opaque (not a translucent tint). A weak blur of large text can be partially legible or reconstructable. **Safe minimum** (default on) forces the radius to at least 0.6× text height and mosaic blocks to at least 0.55× text height. Opacity < 100 % shows the original through the overlay and is not protective.
* The overlay is rendered from the last snapshot. While content moves, the old position stays covered until the detector catches up.
