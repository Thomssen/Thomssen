import json
import tempfile
import threading
import unittest
from http.server import BaseHTTPRequestHandler, HTTPServer
from pathlib import Path

from thomsen_monitor.config import Config
from thomsen_monitor.license import LicenseClient, LicenseError
from thomsen_monitor.monitor import poll_once
from thomsen_monitor.providers import HttpProvider, ProviderError
from thomsen_monitor.store import Store, normalize_username

LICENSES = {"GOOD-KEY": {"valid": True, "status": "active", "expires_at": 4102444800},
            "REVOKED": {"valid": False, "reason": "license revoked"}}
AVAIL = {"alice": True, "bob": False}
seen = []


class H(BaseHTTPRequestHandler):
    def log_message(self, *a): pass

    def _send(self, code, obj):
        b = json.dumps(obj).encode()
        self.send_response(code); self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(b))); self.end_headers(); self.wfile.write(b)

    def do_POST(self):
        body = json.loads(self.rfile.read(int(self.headers["Content-Length"])))
        seen.append(body)
        lic = LICENSES.get(body["key"])
        if body["product"] != "thomsen-username-monitor" or lic is None:
            return self._send(404, {"error": "unknown license"})
        self._send(200, lic)

    def do_GET(self):
        from urllib.parse import urlparse, parse_qs
        u = parse_qs(urlparse(self.path).query).get("username", [""])[0]
        if u == "ratelimited": return self._send(429, {})
        self._send(200, {"available": AVAIL.get(u, False)})


class Base(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.srv = HTTPServer(("127.0.0.1", 0), H)
        cls.url = f"http://127.0.0.1:{cls.srv.server_port}"
        threading.Thread(target=cls.srv.serve_forever, daemon=True).start()

    @classmethod
    def tearDownClass(cls): cls.srv.shutdown()

    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory(); self.addCleanup(self.tmp.cleanup)
        self.cfg = Config(self.tmp.name)
        self.cfg.set("license_api_url", self.url)


class TestStore(Base):
    def test_validation(self):
        self.assertEqual(normalize_username(" @Alice_1 "), "alice_1")
        for bad in ("a", "x" * 33, "a..b", "bad name", "ünï"):
            with self.assertRaises(ValueError): normalize_username(bad)

    def test_add_remove_change_history_persist(self):
        s = Store(Path(self.tmp.name)); s.add("alice")
        with self.assertRaises(ValueError): s.add("alice")
        self.assertEqual(s.update_status("alice", "taken"), ("unknown", "taken"))
        self.assertIsNone(s.update_status("alice", "taken"))
        s2 = Store(Path(self.tmp.name))
        self.assertEqual(s2.targets["alice"]["status"], "taken"); self.assertEqual(len(s2.history), 1)
        s2.remove("alice"); self.assertEqual(s2.targets, {})


class TestMonitor(Base):
    def test_poll_detects_changes_and_spaces_requests(self):
        s = Store(Path(self.tmp.name)); s.add("alice"); s.add("bob")
        self.cfg.values["notify_console"] = False
        sleeps, p = [], HttpProvider(self.url)
        ch = poll_once(self.cfg, s, p, sleep=sleeps.append, out=lambda *_: None)
        self.assertEqual(sorted(ch), [("alice", "unknown", "available"), ("bob", "unknown", "taken")])
        self.assertEqual(sleeps, [self.cfg.get("request_delay_seconds")])
        AVAIL["bob"] = True
        try:
            self.assertEqual(poll_once(self.cfg, s, p, sleep=lambda x: None, out=lambda *_: None),
                             [("bob", "taken", "available")])
        finally:
            AVAIL["bob"] = False

    def test_429_backs_off(self):
        s = Store(Path(self.tmp.name)); s.add("ratelimited"); s.add("alice")
        out = []
        self.assertEqual(poll_once(self.cfg, s, HttpProvider(self.url), sleep=lambda x: None, out=out.append), [])
        self.assertEqual(s.targets["alice"]["status"], "unknown")  # stopped after 429

    def test_bad_provider_url(self):
        with self.assertRaises(ProviderError): HttpProvider("http://example.com/x")

    def test_min_intervals_enforced(self):
        self.cfg.set("poll_interval_seconds", 1); self.cfg.set("request_delay_seconds", 0)
        self.assertEqual(self.cfg.get("poll_interval_seconds"), 60)
        self.assertEqual(self.cfg.get("request_delay_seconds"), 1.0)


class TestLicense(Base):
    def test_valid_revoked_unknown(self):
        c = LicenseClient(self.cfg)
        self.assertEqual(c.validate("GOOD-KEY")["status"], "active")
        with self.assertRaisesRegex(LicenseError, "revoked"): c.validate("REVOKED")
        with self.assertRaisesRegex(LicenseError, "unknown"): c.validate("NOPE")
        with self.assertRaises(LicenseError): c.validate("")
        self.assertEqual(seen[-1]["product"], "thomsen-username-monitor")

    def test_offline_grace_and_expiry(self):
        c = LicenseClient(self.cfg); c.validate("GOOD-KEY")
        self.cfg.set("license_api_url", "http://127.0.0.1:1")  # unreachable
        self.assertTrue(LicenseClient(self.cfg).validate("GOOD-KEY")["offline"])
        with self.assertRaises(LicenseError): LicenseClient(self.cfg).validate("OTHER")
        late = LicenseClient(self.cfg, now=lambda: __import__("time").time() + 73 * 3600)
        with self.assertRaises(LicenseError): late.validate("GOOD-KEY")

    def test_revocation_clears_cache(self):
        c = LicenseClient(self.cfg); c.validate("GOOD-KEY")
        LICENSES["GOOD-KEY"] = {"valid": False, "reason": "license revoked"}
        try:
            with self.assertRaises(LicenseError): c.validate("GOOD-KEY")
            self.assertFalse(c.cache_path.exists())
        finally:
            LICENSES["GOOD-KEY"] = {"valid": True, "status": "active", "expires_at": 4102444800}


if __name__ == "__main__":
    unittest.main()
