"""License client. All issuance/validation happens on the Thomsen backend; this only
sends the key and a device fingerprint and caches the server's verdict for a grace period."""
import hashlib
import json
import platform
import time
import urllib.error
import urllib.request
import uuid

from . import PRODUCT, __version__
from .config import atomic_write_json, read_json

OFFLINE_GRACE_SECONDS = 72 * 3600


class LicenseError(Exception):
    pass


def device_id():
    raw = f"{platform.node()}|{platform.machine()}|{uuid.getnode()}"
    return hashlib.sha256(raw.encode()).hexdigest()[:32]


class LicenseClient:
    def __init__(self, cfg, timeout=10, now=time.time):
        self.cfg, self.timeout, self.now = cfg, timeout, now
        self.cache_path = cfg.home / "license_cache.json"

    def _post(self, path, body):
        base = self.cfg.get("license_api_url").rstrip("/")
        if not base.startswith(("https://", "http://localhost", "http://127.0.0.1")):
            raise LicenseError("license_api_url must be https")
        req = urllib.request.Request(base + path, data=json.dumps(body).encode(), method="POST",
                                     headers={"Content-Type": "application/json",
                                              "User-Agent": f"{PRODUCT}/{__version__}"})
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as r:
                return json.load(r)
        except urllib.error.HTTPError as e:
            try:
                detail = json.load(e).get("error", f"HTTP {e.code}")
            except Exception:  # noqa: BLE001
                detail = f"HTTP {e.code}"
            raise LicenseError(detail)

    def validate(self, key=None):
        """Return license info dict {status, expires_at}. Raises LicenseError if not valid."""
        key = key or self.cfg.get("license_key")
        if not key:
            raise LicenseError("no license key configured")
        try:
            res = self._post("/v1/licenses/validate",
                             {"key": key, "product": PRODUCT, "device_id": device_id()})
        except (urllib.error.URLError, OSError):
            return self._offline(key)
        if res.get("valid") is not True:
            self.cache_path.unlink(missing_ok=True)
            raise LicenseError(res.get("reason") or "license is not valid")
        info = {"status": res.get("status", "active"), "expires_at": res.get("expires_at")}
        atomic_write_json(self.cache_path, {"key_hash": _h(key), "checked_at": self.now(), **info})
        return info

    def _offline(self, key):
        c = read_json(self.cache_path, {})
        if c.get("key_hash") == _h(key) and self.now() - c.get("checked_at", 0) < OFFLINE_GRACE_SECONDS:
            exp = c.get("expires_at")
            if exp is None or exp > self.now():
                return {"status": c.get("status"), "expires_at": exp, "offline": True}
        raise LicenseError("cannot reach license server and no valid cached validation")


def _h(key):
    return hashlib.sha256(key.encode()).hexdigest()
