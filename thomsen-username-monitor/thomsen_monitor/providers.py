"""Availability sources.

Discord publishes no public endpoint for checking whether a username is free, and
checking through a user token (self-botting) or proxy rotation is against its terms.
So this app does not scrape Discord. Supported sources:

  manual - you record what you see in Discord's own username settings.
  http   - a JSON endpoint you are authorised to use, returning {"available": true|false}.
"""
import json
import urllib.error
import urllib.parse
import urllib.request


class ProviderError(Exception):
    pass


class ManualProvider:
    name = "manual"
    automatic = False

    def check(self, username):
        raise ProviderError("manual provider cannot poll; set status by hand")


class HttpProvider:
    name = "http"
    automatic = True

    def __init__(self, url, auth_header="", timeout=10):
        if not url.startswith(("https://", "http://localhost", "http://127.0.0.1")):
            raise ProviderError("provider_url must be https")
        self.url, self.auth_header, self.timeout = url, auth_header, timeout

    def check(self, username):
        sep = "&" if "?" in self.url else "?"
        req = urllib.request.Request(f"{self.url}{sep}username={urllib.parse.quote(username)}")
        if self.auth_header:
            k, _, v = self.auth_header.partition(":")
            req.add_header(k.strip(), v.strip())
        try:
            with urllib.request.urlopen(req, timeout=self.timeout) as r:
                body = json.load(r)
        except urllib.error.HTTPError as e:
            if e.code == 429:
                raise ProviderError("rate limited (429); increase the poll interval")
            raise ProviderError(f"HTTP {e.code}")
        except (urllib.error.URLError, json.JSONDecodeError, OSError) as e:
            raise ProviderError(str(e))
        if not isinstance(body, dict) or not isinstance(body.get("available"), bool):
            raise ProviderError('response must be {"available": true|false}')
        return "available" if body["available"] else "taken"


def build_provider(cfg):
    if cfg.get("provider") == "http":
        return HttpProvider(cfg.get("provider_url"), cfg.get("provider_auth_header"))
    return ManualProvider()
