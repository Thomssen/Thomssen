# Thomsen Software — Advertisement

**Format:** 1920×1080, 30 fps, 43.6 s, H.264 + AAC. 16:9 for YouTube and thomssen.dev.

## Creative direction
Black, white, dark grey, silver. One typeface family (Inter). Each product gets a
restrained, abstract visual that describes what it does, never a mock-up of an
interface. Slow drift, soft glow, subtle grain and vignette. Every scene dips
through black, so it plays like a series of reveals.

Nothing is claimed beyond what was supplied: Shield's four listed features, VM is
free, VPN and OSINT are labelled **Coming soon**, and VPN lists no features.

## Storyboard (43.6 s)
| Time | Scene | On screen | Sound |
|---|---|---|---|
| 0:00 | **Hook** | "Right now, your PC is talking to the internet. Do you know who to?" Rings pulse on each line. | Low drone, a sub hit on every line, then a riser. |
| 0:05 | **Reveal** | A T tile, a cursor glides in and clicks it, and "Thomsen. Software" opens out of the T. | Click, impact, and the beat drops in. |
| 0:09 | Trust | "Software you can actually trust." | Beat, bass, plucks. |
| 0:12 | **Thomsen Shield** | Network protection. Four features as pills. See every connection. Stop the ones you do not trust. | Impact and whoosh into each product. |
| 0:17 | **Thomsen VPN** | Privacy. Coming soon. Currently in development. | |
| 0:23 | **Thomsen OSINT** | Investigations. Coming soon. | |
| 0:28 | **Thomsen VM** | Virtualization. Free. | |
| 0:34 | Lineup | "Quiet on your system. Clear in your results." with four product cards. | |
| 0:38 | **Close** | "Software. Designed with purpose." then the logo and thomssen.dev. | Lifted chord, final hit, fade. |

Strictly black, white and silver. Light moves as soft silver glow only. A single
flag in `render.py` (`MONOCHROME`) controls this.

## Voice-over (optional, if you want to record narration)
Text matches the screen, so the film works silent or narrated.
1. "We build software that is genuinely useful."
2. "Thomsen Shield. Network protection for Windows. Monitoring, suspicious connection detection, threat intelligence and Windows Firewall protection."
3. "Thomsen VM. A free virtual machine application. Clean. Accessible. Practical."
4. "Thomsen VPN. Coming soon."
5. "Thomsen OSINT. Research publicly available information. Coming soon."
6. "Software. Designed with purpose."

## Sound
Synthesised in `audio.py`: a slow D-minor pad that changes chord per scene, low sub,
soft pitched ticks as text lands, sparse plucks from the first product onward,
filtered-air transitions, long reverb. No samples, so no licensing questions.

## Using your real assets
- **Logo:** put a light-on-transparent `advert/assets/logo.png` in place. It replaces
  the typographic wordmark everywhere. The current wordmark is a stand-in.
- **Real application footage:** none was supplied, so none is shown. Drop recordings
  in `advert/assets/` and the Shield and VM scenes are the right place to swap the
  abstract visual for real UI.
- **Rebuild:** `bash advert/build.sh`
