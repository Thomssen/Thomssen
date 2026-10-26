import time

from .notify import notify
from .providers import ProviderError


def poll_once(cfg, store, provider, sleep=time.sleep, out=print):
    """Check every target once, spaced by request_delay_seconds. Returns list of changes."""
    changes = []
    if not provider.automatic:
        return changes
    for i, name in enumerate(list(store.targets)):
        if i:
            sleep(cfg.get("request_delay_seconds"))
        try:
            status = provider.check(name)
        except ProviderError as e:
            out(f"[WARN] {name}: {e}")
            if "429" in str(e):
                break  # back off instead of hammering
            continue
        change = store.update_status(name, status)
        if change:
            changes.append((name, *change))
            for err in notify(cfg, name, *change, out=out):
                out(f"[WARN] {err}")
    return changes
