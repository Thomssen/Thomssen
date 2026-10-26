import json
import urllib.request

COLORS = {"available": 0x2ECC71, "taken": 0xE74C3C, "unknown": 0x95A5A6}


def format_change(username, old, new):
    return f"{username}: {old} -> {new}"


def send_webhook(url, username, old, new, timeout=10):
    """Post to a Discord webhook (a supported, documented way to notify)."""
    if not url.startswith("https://"):
        raise ValueError("webhook url must be https")
    payload = {
        "username": "Thomsen Username Monitor",
        "embeds": [{
            "title": f"Status changed: {username}",
            "description": f"`{old}` -> **{new}**",
            "color": COLORS.get(new, 0x95A5A6),
        }],
        "allowed_mentions": {"parse": []},
    }
    req = urllib.request.Request(url, data=json.dumps(payload).encode(),
                                 headers={"Content-Type": "application/json"}, method="POST")
    with urllib.request.urlopen(req, timeout=timeout):
        pass


def notify(cfg, username, old, new, out=print):
    errors = []
    if cfg.get("notify_console"):
        out("\a[CHANGE] " + format_change(username, old, new))
    if cfg.get("discord_webhook_url"):
        try:
            send_webhook(cfg.get("discord_webhook_url"), username, old, new)
        except Exception as e:  # noqa: BLE001 - notification failure must not stop monitoring
            errors.append(f"webhook failed: {e}")
    return errors
