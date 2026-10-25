# Thomsen Shield — "Quiet on your system." (52.8 s)

## What the footage showed

**Website recording** (36 s): dark, near-black, heavy grotesque headlines, floating pill nav, dashboard mock-up
hero, the "Everything in one app." 3×3 feature grid, "Quiet on your system. Clear in your results."

**App recording** (53 s): login (skipped on purpose — it shows a username and typed password), Dashboard,
Live Connections (129 connections), Protection (Detection / Threat Intelligence / DNS Protection / Response),
Firewall, Blocklist, Allowlist, Events & Logs, Settings, Changelog, About.

Deliberately **not used**: the login screen, the Windows UAC prompt (Danish) during the DNS toggle, the desktop
wallpaper/taskbar, the "Stripe checkout unavailable — join Discord" notice on the product page, the browser's
"hold Esc to exit fullscreen" toast, empty-state pages (Firewall "No rules yet", Blocklist "Nothing blocked").

## Concept

*A quiet film about something you can't normally see.* Thomsen Shield's promise (from the site) is "See every
connection. Stop the ones you do not trust." So the film opens in darkness with a hairline of light, resolves
into a monochrome network of connections, isolates one of them, and then the beam opens on the product. After
that the real app does all the talking — slow pushes, no fake UI — and the film closes on the brand's own line:
"Quiet on your system. Clear in your results."

Palette: black, white, greys only. Type: Inter Display (closest installed match to the site's heavy grotesque).
No colour, no gradients other than the single overhead light beam, no templates.

## Storyboard (100 BPM, bar = 2.4 s, every cut on a bar/beat)

| Time | Scene | On screen | Sound |
|---|---|---|---|
| 0.0–2.4 | Black. Hairline of light grows from centre. | — | Room tone, sub drone A1 fades in |
| 2.4–4.8 | Hairline dissolves into a sparse node field. | "Every **app.**" → "Every **connection.**" | Soft ticks on each line |
| 4.8–9.6 | Field drifts in 3-D, links form. One node is "detected": three thin rings. | "Every **second.**" → "Most of it goes **unseen.**" | Plucked arpeggio enters, sub heartbeat, riser |
| 9.6–12.0 | Everything contracts to a point → light beam opens. | **Thomsen Shield** (tracks in, light sweep) + "See every connection. Stop the ones you do not trust." | Impact: sub boom + reverb tail, pad opens |
| 12.0–16.8 | Website footage: the page scrolls into "Everything in one app." The camera dollies into the **Dashboard** card… | (the website's own copy) | Rising air |
| 16.4–21.6 | …and arrives in the real app Dashboard. | DASHBOARD — "Everything, at a glance." | Kick enters |
| 21.6–24.0 | Live Connections | "See every connection." | |
| 24.0–28.8 | Protection (Detection) | "Spot what doesn't belong." | |
| 28.8–33.6 | DNS Protection: off → running | "Stop threats at the lookup." | Tick on state change |
| 33.6–36.0 | Allowlist | "Your network, trusted by default." | |
| 36.0–40.8 | Events & Logs, window sinks into the dark | "A clear record of everything." | Drums drop out |
| 40.8–45.6 | Black | "Quiet on your system." / "Clear in your results." | Riser |
| 45.6–52.8 | Final lock-up | **Thomsen Shield** · Network protection for Windows 10 and 11. · `thomsen-software.com` · THOMSEN SOFTWARE | Second impact, open-fifth chord, long tail, fade |

Every feature caption is paraphrased from copy on thomsen-software.com and every feature shown is visible in the
recording. Nothing was invented or mocked up.

## How AI/procedural visuals meet the real footage

No generative video model is available in this environment, so none was used — and the film is stronger for it:
the only non-recorded imagery is the hairline, the node field and the light beam, all procedural monochrome
(`render.py`). The product itself is only ever the real recording, cropped to the app window (rounded-corner
mask re-applied), lit with a rim light, floor reflection and soft shadow so it reads as a lit object.

## Sound

Original score synthesised from scratch (`audio.py`) — no samples, no third-party music, nothing to license.
A minor / open fifths; sub drone, detuned saw pads, plucked arpeggio, soft four-on-the-floor kick with sidechain
ducking from 16.8 s, two impacts (9.6 s, 45.6 s), risers, whooshes, UI ticks locked to text entrances, long
algorithmic reverb. No voice-over: the product is quiet, so the film is too.
If you want a human-composed track, drop it in with the ffmpeg line in the README — the cut points sit on a
100 BPM grid.
