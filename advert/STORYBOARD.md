# Thomsen Software — Advertisement

**Format:** 1920×1080, 30 fps, 48.6 s, H.264 + AAC. 16:9 for YouTube and thomsen-software.com.

## Creative direction
Black, white, dark grey, silver. One typeface family (Inter). Each product gets a
restrained, abstract visual that describes what it does, never a mock-up of an
interface. Slow drift, soft glow, subtle grain and vignette. Every scene dips
through black, so it plays like a series of reveals.

Nothing is claimed beyond what was supplied: Shield's four listed features, VM is
free, VPN and OSINT are labelled **Coming soon**, and VPN lists no features.

## Storyboard (48.6 s)
| Time | Scene | On screen | Voice-over |
|---|---|---|---|
| 0:00 | **Hook** | "Right now, your PC is talking to the internet. Do you know who to?" | "Right now, your PC is talking to the internet." / "Do you know who to?" |
| 0:05 | **Reveal** | A T tile, a cursor glides in and clicks it, and "Thomsen. Software" opens out of the T. | "Meet Thomsen Software." |
| 0:09 | Trust | "Software you can actually trust." | Same. |
| 0:12 | **Thomsen Shield** | The website's "Four tools. One developer." showcase: the rail with Shield active, its description and buttons, and the device mock-up with its dashboard building in. | "Thomsen Shield. See every connection. Stop the ones you do not trust." |
| 0:18 | **Thomsen VPN** | Same showcase. The mock-up dims behind a "Coming soon" badge. | "Thomsen VPN. Your traffic, encrypted. One click to connect. Coming soon." |
| 0:25 | **Thomsen OSINT** | Same showcase. | "Thomsen OSINT. Public information, organized into one clear picture." |
| 0:31 | **Thomsen VM** | Same showcase. Free. | "Thomsen VM. A full machine, running inside your desktop. Free." |
| 0:38 | Lineup | "Quiet on your system. Clear in your results." with four product cards. | Same. |
| 0:43 | **Close** | "Software. Designed with purpose." then the logo and thomsen-software.com. | "Software. Designed with purpose." / "Thomsen Software dot com." |

Strictly black, white and silver. Light moves as soft silver glow only. A single
flag in `render.py` (`MONOCHROME`) controls this.

## Voice-over
Synthesised with Kokoro (open neural text-to-speech, voice `af_heart`) by `voiceover.py`,
polished with a rumble filter, light compression and a limiter, with a touch of room.
The music ducks under it. To change the voice, edit `VOICE` in `voiceover.py`
(for example `am_michael` or `bf_emma`) or rewrite the `LINES` list.

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
