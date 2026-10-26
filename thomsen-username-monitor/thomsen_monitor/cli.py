import argparse
import sys
import time
from datetime import datetime

from . import __version__
from .config import Config
from .license import LicenseClient, LicenseError
from .monitor import poll_once
from .providers import ProviderError, build_provider
from .store import STATUSES, Store
from . import ui


def fmt_ts(ts):
    return datetime.fromtimestamp(ts).strftime("%Y-%m-%d %H:%M:%S") if ts else "never"


def ensure_license(cfg, inp=input, out=print):
    client = LicenseClient(cfg)
    for attempt in range(3):
        try:
            info = client.validate()
            out(f"{ui.GREEN}AUTHENTICATED{ui.RESET}  status={info['status']} expires={info.get('expires_at') or 'never'}")
            return info
        except LicenseError as e:
            out(f"{ui.RED}License error:{ui.RESET} {e}")
            key = inp("License key (generate one with /license in the Thomsen Discord): ").strip()
            if not key:
                break
            cfg.set("license_key", key)
    return None


def print_targets(store, out=print):
    if not store.targets:
        out("  (no usernames monitored)")
    for name, t in sorted(store.targets.items()):
        out(f"  {name:<34}{ui.status_color(t['status'])}{t['status']:<10}{ui.RESET}{ui.DIM}checked {fmt_ts(t['checked_at'])}{ui.RESET}")


def run_loop(cfg, store, provider, once=False):
    provider_ok = provider.automatic
    if not provider_ok:
        print("Provider is 'manual': nothing to poll. Set status by hand or configure an http provider.")
        return
    try:
        while True:
            changes = poll_once(cfg, store, provider)
            print(f"{ui.DIM}[{fmt_ts(time.time())}] checked {len(store.targets)} username(s), {len(changes)} change(s){ui.RESET}")
            if once:
                return
            time.sleep(cfg.get("poll_interval_seconds"))
    except KeyboardInterrupt:
        print("\nStopped.")


def interactive(cfg, store):
    while True:
        ui.header(__version__, "MONITOR", f"Targets: {len(store.targets)}   Provider: {cfg.get('provider')}   Interval: {cfg.get('poll_interval_seconds')}s")
        print_targets(store)
        ui.menu("Main Menu", [("1", "Add username"), ("2", "Remove username"), ("3", "Set status manually"),
                              ("4", "Start monitoring"), ("5", "Change history"), ("6", "Settings"), ("0", "Exit")])
        choice = input("Option [0-6] > ").strip()
        try:
            if choice == "1":
                print("Added", store.add(input("Username: ")))
            elif choice == "2":
                store.remove(input("Username: ")); print("Removed.")
            elif choice == "3":
                name = input("Username: ").strip().lower()
                status = input(f"Status {STATUSES[1:]}: ").strip().lower()
                if name not in store.targets:
                    raise ValueError("not monitored")
                if status not in STATUSES[1:]:
                    raise ValueError("status must be available or taken")
                ch = store.update_status(name, status)
                if ch:
                    for e in __import__("thomsen_monitor.notify", fromlist=["notify"]).notify(cfg, name, *ch):
                        print(e)
            elif choice == "4":
                run_loop(cfg, store, build_provider(cfg))
            elif choice == "5":
                for h in store.history[-20:]:
                    print(f"  {fmt_ts(h['at'])}  {h['username']}: {h['from']} -> {h['to']}")
            elif choice == "6":
                settings(cfg)
            elif choice == "0":
                return
        except (ValueError, KeyError, ProviderError) as e:
            print(f"{ui.RED}Error:{ui.RESET} {e}")
        input(f"{ui.DIM}Press Enter...{ui.RESET}")


def settings(cfg):
    keys = [k for k in cfg.values if k != "license_key"]
    for i, k in enumerate(keys, 1):
        shown = "(set)" if "webhook" in k and cfg.get(k) else cfg.get(k)
        print(f"  <{i:02}> {k} = {shown}")
    sel = input("Setting number (blank to cancel): ").strip()
    if sel.isdigit() and 1 <= int(sel) <= len(keys):
        k = keys[int(sel) - 1]
        raw = input(f"New value for {k}: ").strip()
        cur = cfg.get(k)
        val = raw.lower() in ("1", "true", "yes", "y") if isinstance(cur, bool) else type(cur)(raw)
        cfg.set(k, val)


def main(argv=None):
    p = argparse.ArgumentParser(prog="thomsen-monitor", description="Thomsen Username Monitor")
    sub = p.add_subparsers(dest="cmd")
    a = sub.add_parser("add"); a.add_argument("username")
    r = sub.add_parser("remove"); r.add_argument("username")
    sub.add_parser("list")
    sub.add_parser("run").add_argument("--once", action="store_true")
    s = sub.add_parser("set-status"); s.add_argument("username"); s.add_argument("status", choices=STATUSES[1:])
    c = sub.add_parser("config"); c.add_argument("key"); c.add_argument("value")
    sub.add_parser("license")
    args = p.parse_args(argv)

    cfg = Config()
    if not ensure_license(cfg):
        print("A valid license is required."); return 2
    store = Store(cfg.home)
    try:
        if args.cmd == "add": print("Added", store.add(args.username))
        elif args.cmd == "remove": store.remove(args.username); print("Removed.")
        elif args.cmd == "list": print_targets(store)
        elif args.cmd == "run": run_loop(cfg, store, build_provider(cfg), once=args.once)
        elif args.cmd == "set-status":
            from .notify import notify
            ch = store.update_status(args.username.lower(), args.status)
            if ch: notify(cfg, args.username.lower(), *ch)
        elif args.cmd == "config":
            if args.key == "license_key": raise ValueError("use the license prompt")
            cur = cfg.get(args.key)
            cfg.set(args.key, args.value.lower() in ("1", "true", "yes") if isinstance(cur, bool) else type(cur)(args.value))
        elif args.cmd == "license": pass
        else: interactive(cfg, store)
    except (ValueError, KeyError, ProviderError) as e:
        print(f"Error: {e}"); return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
