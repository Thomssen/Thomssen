# Thomsen Software — Advertisement

**Format:** 1920×1080, 30 fps, 75.5 s, H.264 + AAC. 16:9 for YouTube and thomssen.dev.

## Creative direction
Black, white, dark grey, silver. One typeface family (Inter). Each product gets a
restrained, abstract visual that describes what it does, never a mock-up of an
interface. Slow drift, soft glow, subtle grain and vignette. Every scene dips
through black, so it plays like a series of reveals.

Nothing is claimed beyond what was supplied: Shield's four listed features, VM is
free, VPN and OSINT are labelled **Coming soon**, and VPN lists no features.

## Storyboard
| Time | Scene | On-screen text | Motion / sound |
|---|---|---|---|
| 0:00–0:06.5 | Opening | THOMSEN SOFTWARE | Black. A thin line of light grows, then a band sweeps across and reveals the wordmark. Rising air, then a soft low hit. |
| 0:06.5–0:12 | Brand statement | "We build software that is genuinely useful." | Two-beat type reveal. |
| 0:12–0:23.5 | **Thomsen Shield** | Network protection · Network monitoring · Suspicious connection detection · Threat intelligence · Windows Firewall protection | Concentric rings and nodes. Traffic flows to the core. Unknown connections approach and are stopped at the outer ring. |
| 0:23.5–0:35 | **Thomsen VM** | Virtual machines · Free · Clean, focused interface · Accessible virtual machines · Practical virtual environments | Host window; three virtual machines cascade in and come alive. |
| 0:35–0:46.5 | **Thomsen VPN** | Privacy and connectivity · Coming soon · "Private connectivity, currently in development." | A tunnel of rings between two endpoints; data passes through. |
| 0:46.5–0:58 | **Thomsen OSINT** | Open-source intelligence · Coming soon · Usernames · IP addresses · Domains and email addresses | Four query types converge on a scanning core. Uses placeholder examples (203.0.113.42 is a reserved documentation address). |
| 0:58–1:06 | Unified | "One company. Four tools." Shield · VM · VPN · OSINT | Four columns reveal in sequence; one pass of light. |
| 1:06–1:15.5 | Ending | "Software. Designed with purpose." → logo → thomssen.dev | Closing line, then wordmark and website. Fade to black. |

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
