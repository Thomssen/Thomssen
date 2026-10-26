# Thomsen Username Monitor

Terminal app (Python 3.10+, no dependencies) that watches a list of Discord usernames you choose
and notifies you when their reported availability changes. Styled after the terminal tool in the reference video.

    python -m thomsen_monitor            # interactive menu
    python -m thomsen_monitor add name_1 / remove / list / set-status / run [--once] / config KEY VALUE

## What it deliberately does not do
The reference tool brute-generated usernames and checked them through 10,000 proxies with 500 workers.
That is rate-limit evasion and is not implemented. There is also no automated claiming, user-token or
self-bot use. Discord has no public username-availability endpoint, so the sources are:
`manual` (you record what you see in Discord) or `http` (a JSON endpoint you are authorised to use,
returning `{"available": true|false}`). Polling is throttled (min 60s interval, 1s between requests) and
backs off on HTTP 429. Notifications: console and Discord webhook.

## Licensing
Requires a license validated by the Thomsen backend on every start (72h offline grace). See `docs/LICENSE_API.md`.
Config/data: `%APPDATA%\ThomsenSoftware\UsernameMonitor` (override with `THOMSEN_MONITOR_HOME`).

## Tests
    python -m unittest discover -s tests -t .
