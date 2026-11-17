# Verification checklist (run on Windows)

Automated: `cargo test -p privacy-core`, CI typecheck and build. Everything below needs a real desktop. Record the results in your release notes.

## Smoke test

1. Start the app → Dashboard → **Start protection**. Dashboard shows *Local OCR: Ready*.
2. Open Notepad and type `jane.doe@example.com  +1 (555) 012-3456  192.168.1.20`. Within ~0.5 s all three are covered; the rest of the screen is untouched. Click and type *through* the overlay: input must work normally and focus must stay in Notepad.
3. Delete the text → the overlays disappear after the hold time (default 0.7 s).
4. Scroll a long web page containing an email address → the cover follows the address.
5. Add your name under *My personal information* and type it in Notepad → covered.
6. Drag the window to a second monitor with a different scale factor → still aligned.
7. Appearance: switch all three styles; check the previews match what appears on screen. Set Opacity < 100 % and confirm the warning appears.

## Shortcuts, tray, startup

* `Ctrl+Shift+B` toggles protection; tray label and notification update. `Ctrl+Shift+P` opens Settings. `Ctrl+Shift+X` covers all monitors in black; press again to remove.
* Rebind a shortcut; the new combination works, and an invalid or taken one produces a warning.
* Close the window with *Keep running in the system tray* on → process stays; tray *Open* restores it. Exit via tray.
* *Start with Windows* → sign out/in; app starts minimised in the tray.

## Capture safety (do not skip if you stream)

1. Settings → Screen sharing & recording → **Run capture test**. Expected with default settings: *GDI display capture* = overlay recorded; *detector capture* = **not** recorded (if it says recorded, the detector sees its own blur and you will see flicker: report it); *window capture* = not recorded.
2. Start OBS (Display Capture and Window Capture sources), Discord (Screen and Window), Teams/Zoom. Run the test with **Keep test patch visible = 60 s** and note in each app's preview whether the magenta patch appears. That is the only evidence that path hides your data.

## Rules and robustness

* Applications → *Selected only* with just Notepad → other apps are not censored. Exclude an app → never censored. Turn *Browser protection* off → Chrome/Edge untouched.
* Disable a monitor → nothing is drawn or scanned there.
* Elevated window (run Notepad as administrator): OCR still covers it.
* Unplug/replug a monitor while running: overlays recover within ~1.5 s.
* Performance: Task Manager shows low CPU while the screen is static; CPU rises while scrolling.
