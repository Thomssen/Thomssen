import os
import sys

CYAN, GREEN, RED, YELLOW, DIM, RESET = "\033[96m", "\033[92m", "\033[91m", "\033[93m", "\033[90m", "\033[0m"
if os.environ.get("NO_COLOR") or not sys.stdout.isatty():
    CYAN = GREEN = RED = YELLOW = DIM = RESET = ""

BANNER = [
    "████████ ██  ██  ██████  ██    ██  ██████  ████████ ███    ██",
    "   ██    ██  ██ ██    ██ ███  ███ ██       ██       ████   ██",
    "   ██    ██████ ██    ██ ██ ██ ██  ██████  ██████   ██ ██  ██",
    "   ██    ██  ██ ██    ██ ██    ██       ██ ██       ██  ██ ██",
    "   ██    ██  ██  ██████  ██    ██  ██████  ████████ ██   ████",
]
WIDTH = 66


def box(lines, color=CYAN, width=WIDTH, center=False):
    out = [f"{color}┌{'─' * (width - 2)}┐{RESET}"]
    for ln in lines:
        pad = ln.center(width - 4) if center else ln.ljust(width - 4)
        out.append(f"{color}│{RESET} {pad} {color}│{RESET}")
    out.append(f"{color}└{'─' * (width - 2)}┘{RESET}")
    return "\n".join(out)


def clear():
    if sys.stdout.isatty():
        print("\033[2J\033[H", end="")


def header(version, subtitle="", stats=None):
    clear()
    print(CYAN + "\n".join(BANNER) + RESET)
    print(f"{DIM}Thomsen Software  v{version}  {subtitle}{RESET}")
    if stats:
        print(box([stats], center=True))


def menu(title, items):
    print()
    print(box([title.upper()], center=True))
    for key, label in items:
        print(f"  {CYAN}<{key}>{RESET}  {label}")
    print()


def status_color(status):
    return {"available": GREEN, "taken": RED}.get(status, YELLOW)
