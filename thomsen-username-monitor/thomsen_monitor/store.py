import re
import time

from .config import atomic_write_json, read_json

# Discord's documented username rules: 2-32 chars, a-z 0-9 _ . ; no consecutive periods.
_USERNAME_RE = re.compile(r"^[a-z0-9_.]{2,32}$")
STATUSES = ("unknown", "available", "taken")
MAX_TARGETS = 200
MAX_HISTORY = 500


def normalize_username(name):
    name = name.strip().lower().lstrip("@")
    if not _USERNAME_RE.match(name) or ".." in name:
        raise ValueError("invalid username: 2-32 chars of a-z, 0-9, '_' and '.' (no '..')")
    return name


class Store:
    def __init__(self, home):
        self.path = home / "targets.json"
        data = read_json(self.path, {})
        self.targets = data.get("targets", {})
        self.history = data.get("history", [])

    def save(self):
        atomic_write_json(self.path, {"targets": self.targets, "history": self.history[-MAX_HISTORY:]})

    def add(self, name):
        name = normalize_username(name)
        if name in self.targets:
            raise ValueError(f"{name} is already monitored")
        if len(self.targets) >= MAX_TARGETS:
            raise ValueError(f"target limit reached ({MAX_TARGETS})")
        self.targets[name] = {"status": "unknown", "checked_at": None, "changed_at": None}
        self.save()
        return name

    def remove(self, name):
        name = normalize_username(name)
        if name not in self.targets:
            raise KeyError(name)
        del self.targets[name]
        self.save()

    def update_status(self, name, status, now=None):
        """Record a status report. Returns (old, new) if it changed, else None."""
        if status not in STATUSES:
            raise ValueError(status)
        now = now or time.time()
        t = self.targets[name]
        old = t["status"]
        t["checked_at"] = now
        changed = old != status
        if changed:
            t["status"] = status
            t["changed_at"] = now
            self.history.append({"username": name, "from": old, "to": status, "at": now})
        self.save()
        return (old, status) if changed else None
