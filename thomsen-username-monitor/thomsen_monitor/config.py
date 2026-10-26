import json
import os
import tempfile
from pathlib import Path

DEFAULTS = {
    "license_api_url": "https://api.thomssen.dev",
    "license_key": "",
    "poll_interval_seconds": 300,
    "request_delay_seconds": 2.0,
    "provider": "manual",
    "provider_url": "",
    "provider_auth_header": "",
    "discord_webhook_url": "",
    "notify_console": True,
}
MIN_POLL_INTERVAL = 60
MIN_REQUEST_DELAY = 1.0


def data_dir():
    override = os.environ.get("THOMSEN_MONITOR_HOME")
    if override:
        return Path(override)
    base = os.environ.get("APPDATA") or os.environ.get("XDG_CONFIG_HOME") or str(Path.home() / ".config")
    return Path(base) / "ThomsenSoftware" / "UsernameMonitor"


def atomic_write_json(path, data):
    path.parent.mkdir(parents=True, exist_ok=True)
    fd, tmp = tempfile.mkstemp(dir=path.parent, suffix=".tmp")
    try:
        with os.fdopen(fd, "w", encoding="utf-8") as f:
            json.dump(data, f, indent=2)
        os.replace(tmp, path)
    except BaseException:
        if os.path.exists(tmp):
            os.unlink(tmp)
        raise


def read_json(path, default):
    try:
        with open(path, encoding="utf-8") as f:
            return json.load(f)
    except (FileNotFoundError, json.JSONDecodeError):
        return default


class Config:
    def __init__(self, home=None):
        self.home = Path(home) if home else data_dir()
        self.path = self.home / "config.json"
        self.values = {**DEFAULTS, **read_json(self.path, {})}

    def get(self, key):
        return self.values[key]

    def set(self, key, value):
        if key not in DEFAULTS:
            raise KeyError(key)
        if key == "poll_interval_seconds":
            value = max(MIN_POLL_INTERVAL, int(value))
        elif key == "request_delay_seconds":
            value = max(MIN_REQUEST_DELAY, float(value))
        self.values[key] = value
        self.save()

    def save(self):
        atomic_write_json(self.path, self.values)
        try:
            os.chmod(self.path, 0o600)  # holds license key and webhook URL
        except OSError:
            pass
