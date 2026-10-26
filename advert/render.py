"""Thomsen Software advertisement: frame renderer.

Every frame is a pure function of time, drawn as a monochrome light layer
(supersampled 2x), then given a soft glow, a faint spotlight, vignette and
film grain. Output is piped straight into ffmpeg.

Brand assets: if advert/assets/logo.png exists (white or light artwork on a
transparent background) it replaces the typographic wordmark everywhere.
"""
import math
import os
import subprocess
import sys
from multiprocessing import Pool

import numpy as np
from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageFont

W, H, FPS, S = 1920, 1080, 30, 2
HERE = os.path.dirname(os.path.abspath(__file__))
FONT_DIR = "/usr/share/fonts/opentype/inter/"
MONO = "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf"
TINT = np.array([236, 238, 243], np.float32) / 255.0
SITE = "thomssen.dev"

# (name, start, length)
SCENES = [
    ("logo", 0.0, 6.5),
    ("tag", 6.5, 5.5),
    ("shield", 12.0, 11.5),
    ("vm", 23.5, 11.5),
    ("vpn", 35.0, 11.5),
    ("osint", 46.5, 11.5),
    ("unified", 58.0, 8.0),
    ("end", 66.0, 9.5),
]
DURATION = SCENES[-1][1] + SCENES[-1][2]


# ---------------------------------------------------------------- helpers
def font(name, size):
    return ImageFont.truetype(FONT_DIR + name + ".otf", int(size * S))


F = {}


def fonts():
    if not F:
        F["light"] = lambda s: font("InterDisplay-Light", s)
        F["reg"] = lambda s: font("Inter-Regular", s)
        F["med"] = lambda s: font("Inter-Medium", s)
        F["semi"] = lambda s: font("InterDisplay-SemiBold", s)
        F["mono"] = lambda s: ImageFont.truetype(MONO, int(s * S))
    return F


def clamp(x, a=0.0, b=1.0):
    return max(a, min(b, x))


def prog(t, a, b):
    return clamp((t - a) / (b - a)) if b > a else float(t >= a)


def smooth(x):
    x = clamp(x)
    return x * x * (3 - 2 * x)


def out_cubic(x):
    x = clamp(x)
    return 1 - (1 - x) ** 3


def in_out(x):
    x = clamp(x)
    return 0.5 * (1 - math.cos(math.pi * x))


def lerp(a, b, x):
    return a + (b - a) * x


def px(v):
    return v * S


def gv(a):
    return int(clamp(a) * 255)


def line(d, pts, a, w=1.0):
    if a <= 0.003:
        return
    d.line([(px(x), px(y)) for x, y in pts], fill=gv(a), width=max(1, round(w * S)))


def circle(d, c, r, a, w=1.0, fill=False):
    if a <= 0.003 or r <= 0:
        return
    box = [px(c[0] - r), px(c[1] - r), px(c[0] + r), px(c[1] + r)]
    if fill:
        d.ellipse(box, fill=gv(a))
    else:
        d.ellipse(box, outline=gv(a), width=max(1, round(w * S)))


def ellipse(d, c, rx, ry, a, w=1.0):
    if a <= 0.003:
        return
    d.ellipse([px(c[0] - rx), px(c[1] - ry), px(c[0] + rx), px(c[1] + ry)],
              outline=gv(a), width=max(1, round(w * S)))


def arc(d, c, r, a0, a1, a, w=1.0):
    if a <= 0.003:
        return
    d.arc([px(c[0] - r), px(c[1] - r), px(c[0] + r), px(c[1] + r)],
          a0, a1, fill=gv(a), width=max(1, round(w * S)))


def rrect(d, box, r, a, w=1.0, fill=None):
    b = [px(box[0]), px(box[1]), px(box[2]), px(box[3])]
    d.rounded_rectangle(b, radius=px(r), fill=None if fill is None else gv(fill),
                        outline=gv(a) if a > 0.003 else None, width=max(1, round(w * S)))


def text_width(s, fnt, tracking=0.0):
    if tracking == 0:
        return fnt.getlength(s) / S
    return sum(fnt.getlength(ch) for ch in s) / S + tracking * (len(s) - 1)


def text(d, x, y, s, fnt, a, anchor="l", tracking=0.0, rise=0.0):
    """Baseline-anchored text. rise = vertical offset in px (animates in)."""
    if a <= 0.003:
        return
    w = text_width(s, fnt, tracking)
    x0 = x - (w / 2 if anchor == "c" else w if anchor == "r" else 0)
    y = y + rise
    if tracking == 0:
        d.text((px(x0), px(y)), s, font=fnt, fill=gv(a), anchor="ls")
        return
    cx = x0
    for ch in s:
        d.text((px(cx), px(y)), ch, font=fnt, fill=gv(a), anchor="ls")
        cx += fnt.getlength(ch) / S + tracking


def reveal(t, start, dur=1.0):
    """(alpha, rise) for a fade-up text entrance."""
    p = prog(t, start, start + dur)
    return smooth(p), (1 - out_cubic(p)) * 18


_LOGO = {}


def logo_image():
    if "img" not in _LOGO:
        p = os.path.join(HERE, "assets", "logo.png")
        img = None
        if os.path.exists(p):
            im = Image.open(p).convert("RGBA")
            lum = im.convert("L")
            img = ImageChops.multiply(lum, im.getchannel("A"))
        _LOGO["img"] = img
    return _LOGO["img"]


def draw_logo(img, d, cx, base, a, width=620, big=True):
    """Wordmark (or the supplied logo) centred on cx; base is the baseline."""
    f = fonts()
    lg = logo_image()
    if lg is not None:
        ratio = px(width) / lg.width
        sc = lg.resize((px(width), max(1, int(lg.height * ratio))), Image.LANCZOS)
        sc = sc.point(lambda v: int(v * clamp(a)))
        x0, y0 = px(cx) - sc.width // 2, px(base) - sc.height
        region = img.crop((x0, y0, x0 + sc.width, y0 + sc.height))
        img.paste(ImageChops.lighter(region, sc), (x0, y0))
        return
    k = width / 620.0
    text(d, cx, base, "THOMSEN", f["light"](118 * k), a, "c", 30 * k)
    text(d, cx, base + 66 * k, "SOFTWARE", f["med"](27 * k), a * 0.62, "c", 26 * k)


# ---------------------------------------------------------------- scenes
# Each returns (img, spot) where spot = (cx, cy, strength) for the spotlight.
def new_layer():
    img = Image.new("L", (W * S, H * S), 0)
    return img, ImageDraw.Draw(img)


def scene_logo(t, dur):
    img, d = new_layer()
    f = fonts()
    cx, cy = W / 2, 540
    # anamorphic line grows, then a light band sweeps across revealing the mark
    gl = out_cubic(prog(t, 0.6, 1.7))
    fade = 1 - smooth(prog(t, 5.5, 6.4))
    line_a = (0.55 * gl) * (1 - smooth(prog(t, 4.2, 5.4)))
    half = 80 + 520 * gl
    line(d, [(cx - half, cy), (cx + half, cy)], line_a, 1.2)
    # the mark itself, drawn then masked by the sweep
    mark, md = new_layer()
    draw_logo(mark, md, cx, 560, 0.97)
    mark_arr = np.asarray(mark.resize((W, H), Image.LANCZOS), np.float32) / 255
    bx = lerp(-300, W + 300, in_out(prog(t, 1.7, 4.4)))
    xs = np.arange(W, dtype=np.float32)[None, :]
    mask = np.clip((bx - xs) / 260 + 0.0, 0, 1)
    mask = mask * mask * (3 - 2 * mask)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    layer = layer + mark_arr * mask
    ys = np.arange(H, dtype=np.float32)[:, None]
    band_on = smooth(prog(t, 1.6, 2.2)) * (1 - smooth(prog(t, 4.2, 4.8)))
    band = np.exp(-((xs - bx) / 38) ** 2) * np.exp(-((ys - cy) / 210) ** 2)
    layer = layer + band * 0.55 * band_on
    spot = (cx, cy, 0.7 * smooth(prog(t, 1.4, 3.2)))
    return layer * fade, spot, fade


def scene_tag(t, dur):
    img, d = new_layer()
    f = fonts()
    a1, r1 = reveal(t, 0.7, 1.3)
    a2, r2 = reveal(t, 2.4, 1.3)
    text(d, W / 2, 500, "We build software", f["semi"](96), 0.97 * a1, "c", rise=r1)
    text(d, W / 2, 618, "that is genuinely useful.", f["semi"](96), 0.97 * a2 * 0.55 + 0.4 * a2 * smooth(prog(t, 3.4, 5.0)), "c", rise=r2)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    return layer, (W / 2, 560, 0.6), fade_io(t, dur, 0.9, 0.9)


def fade_io(t, dur, a=0.8, b=0.8):
    return smooth(prog(t, 0, a)) * (1 - smooth(prog(t, dur - b, dur)))


# ---- shared product text block
def product_text(d, t, eyebrow, title, desc, bullets, status=None):
    f = fonts()
    x = 160
    a, r = reveal(t, 0.7, 1.0)
    text(d, x, 372, eyebrow.upper(), f["med"](21), 0.55 * a, "l", 6, r)
    if status:
        ew = text_width(eyebrow.upper(), f["med"](21), 6)
        sa, _ = reveal(t, 1.0, 1.0)
        wpill = text_width(status.upper(), f["med"](17), 4) + 36
        rrect(d, (x + ew + 28, 349, x + ew + 28 + wpill, 381), 16, 0.5 * sa, 1)
        text(d, x + ew + 46, 371, status.upper(), f["med"](17), 0.85 * sa, "l", 4)
    a, r = reveal(t, 1.0, 1.2)
    text(d, x - 4, 500, title, f["semi"](108), 0.98 * a, "l", -1.5, r)
    for i, ln in enumerate(desc):
        a, r = reveal(t, 1.7 + 0.25 * i, 1.1)
        text(d, x, 568 + 46 * i, ln, f["reg"](33), 0.62 * a, "l", 0, r)
    y0 = 568 + 46 * len(desc) + 52
    for i, b in enumerate(bullets):
        a, r = reveal(t, 2.7 + 0.4 * i, 1.0)
        yy = y0 + 54 * i
        line(d, [(x, yy - 36), (x + lerp(0, 470, a), yy - 36)], 0.22 * a, 1)
        text(d, x, yy, b, f["med"](24), 0.82 * a, "l", 0.3, r)


def finish(img, zoom, focus):
    return img, zoom, focus


def scene_shield(t, dur):
    img, d = new_layer()
    product_text(d, t, "Network protection", "Thomsen Shield",
                 ["Windows network protection,", "built to keep watch."],
                 ["Network monitoring", "Suspicious connection detection",
                  "Threat intelligence", "Windows Firewall protection"])
    c = (1420, 540)
    radii = [(110, 0.26), (210, 0.18), (310, 0.14)]
    # rings build outward
    for i, (r, al) in enumerate(radii):
        p = out_cubic(prog(t, 0.5 + 0.35 * i, 2.0 + 0.35 * i))
        circle(d, c, r * (0.82 + 0.18 * p), al * p, 1.2)
    # core
    cp = out_cubic(prog(t, 0.3, 1.6))
    pulse = 0.5 + 0.5 * math.sin(t * 1.6)
    circle(d, c, 9 * cp, 0.95 * cp, fill=True)
    circle(d, c, (22 + 5 * pulse) * cp, 0.35 * cp, 1.2)
    # nodes
    nodes = []
    for i in range(14):
        r = 210 if i % 2 == 0 else 310
        ang = i * (2 * math.pi / 14) + t * (0.05 if r == 210 else -0.035) + 0.3
        nodes.append((c[0] + r * math.cos(ang), c[1] + r * math.sin(ang), ang, r))
    for i, (nx, ny, ang, r) in enumerate(nodes):
        p = out_cubic(prog(t, 1.4 + 0.1 * i, 2.6 + 0.1 * i))
        line(d, [c, (lerp(c[0], nx, p), lerp(c[1], ny, p))], 0.09 * p, 1)
        circle(d, (nx, ny), 4.5 * p, 0.8 * p, fill=True)
        # healthy traffic flowing to the core
        ph = (t * 0.22 + i * 0.137) % 1
        if p > 0.9:
            tx, ty = lerp(nx, c[0], ph), lerp(ny, c[1], ph)
            circle(d, (tx, ty), 2.4, 0.55 * math.sin(math.pi * ph), fill=True)
    # intruders that are stopped at the outer ring
    for k, (t0, deg) in enumerate([(4.2, -38), (7.0, 152), (9.4, 28)]):
        lt = t - t0
        if lt < 0 or lt > 3.0:
            continue
        ang = math.radians(deg)
        ux, uy = math.cos(ang), math.sin(ang)
        travel = out_cubic(prog(lt, 0, 1.2))
        r = lerp(560, 322, travel)
        fadeI = 1 - smooth(prog(lt, 1.35, 2.0))
        tail = [(c[0] + (r + s) * ux, c[1] + (r + s) * uy) for s in (0, 30, 60)]
        line(d, tail[:2], 0.5 * fadeI * (1 - travel * 0.0), 1.4)
        line(d, tail[1:], 0.22 * fadeI, 1.2)
        circle(d, tail[0], 6, 0.95 * fadeI, fill=True)
        if lt > 1.2:
            hit = prog(lt, 1.2, 3.0)
            decay = (1 - hit) ** 2
            arc(d, c, 312, deg - 22 - 10 * hit, deg + 22 + 10 * hit, 0.95 * decay, 2.4)
            circle(d, (c[0] + 310 * ux, c[1] + 310 * uy), 10 + 70 * out_cubic(hit),
                   0.6 * decay, 1.4)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    return layer, (1420, 540, 0.75), fade_io(t, dur)


def scene_vm(t, dur):
    img, d = new_layer()
    product_text(d, t, "Virtual machines", "Thomsen VM",
                 ["A free virtual machine application,", "clean and accessible."],
                 ["Clean, focused interface", "Accessible virtual machines",
                  "Practical virtual environments"], status="Free")
    cx, cy = 1430, 548
    hp = out_cubic(prog(t, 0.4, 1.9))
    hw, hh = 330 * (0.9 + 0.1 * hp), 215 * (0.9 + 0.1 * hp)
    rrect(d, (cx - hw, cy - hh, cx + hw, cy + hh), 26, 0.4 * hp, 1.4, fill=0.035 * hp)
    # three windows cascade in
    for i, (dx, dy, st) in enumerate([(-110, -58, 1.6), (0, -13, 2.6), (110, 32, 3.6)]):
        p = out_cubic(prog(t, st, st + 1.3))
        if p <= 0:
            continue
        bob = math.sin(t * 0.9 + i * 1.7) * 3
        wx, wy = cx + dx, cy + dy + bob + (1 - p) * 26
        w2, h2 = 150, 98
        a = p
        rrect(d, (wx - w2, wy - h2, wx + w2, wy + h2), 14, 0.62 * a, 1.3, fill=0.075 * a)
        line(d, [(wx - w2, wy - h2 + 32), (wx + w2, wy - h2 + 32)], 0.2 * a, 1)
        for j in range(3):
            circle(d, (wx - w2 + 20 + 16 * j, wy - h2 + 16), 3.4, 0.55 * a, fill=True)
        # power indicator: a small ring that breathes once the window is up
        live = prog(t, st + 1.0, st + 2.0)
        circle(d, (wx, wy + 14), 17, 0.28 * live, 1.4)
        circle(d, (wx, wy + 14), 3.2 + 1.2 * math.sin(t * 2 + i), 0.9 * live, fill=True)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    return layer, (1430, 548, 0.7), fade_io(t, dur)


def scene_vpn(t, dur):
    img, d = new_layer()
    product_text(d, t, "Privacy and connectivity", "Thomsen VPN",
                 ["Private connectivity,", "currently in development."], [],
                 status="Coming soon")
    x0, x1, cy = 1030, 1800, 548
    n = 24
    pulse = (t * 0.2) % 1.25 - 0.1
    ap = out_cubic(prog(t, 0.6, 3.0))
    line(d, [(x0, cy), (lerp(x0, x1, ap), cy)], 0.16 * ap, 1)
    for i in range(n):
        u = i / (n - 1)
        if u > ap:
            continue
        ry = 36 + 150 * math.sin(math.pi * u) ** 0.85
        a = 0.14 + 0.7 * math.exp(-((u - pulse) * 7) ** 2)
        ellipse(d, (lerp(x0, x1, u), cy), ry * 0.26, ry, a * min(1, (ap - u) * 8), 1.2)
    # endpoints
    for ex, off in ((x0, 0.0), (x1, 1.0)):
        ea = out_cubic(prog(t, 0.5 + off * 2.2, 1.8 + off * 2.2))
        circle(d, (ex, cy), 10 * ea, 0.95 * ea, fill=True)
        circle(d, (ex, cy), 26 * ea + 3 * math.sin(t * 1.6 + off), 0.3 * ea, 1.2)
    # data moving through the tunnel
    for k in range(5):
        ph = (t * 0.17 + k * 0.2) % 1
        if ph < ap:
            px_ = lerp(x0, x1, ph)
            tail = lerp(x0, x1, max(0, ph - 0.05))
            line(d, [(tail, cy), (px_, cy)], 0.6 * math.sin(math.pi * ph), 1.8)
            circle(d, (px_, cy), 3.2, 0.9 * math.sin(math.pi * ph), fill=True)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    return layer, (1415, 548, 0.7), fade_io(t, dur)


def scene_osint(t, dur):
    img, d = new_layer()
    product_text(d, t, "Open-source intelligence", "Thomsen OSINT",
                 ["Research publicly available", "information, in one place."],
                 ["Usernames", "IP addresses", "Domains and email addresses"],
                 status="Coming soon")
    f = fonts()
    c = (1430, 548)
    cp = out_cubic(prog(t, 0.4, 1.8))
    circle(d, c, 92 * cp, 0.5 * cp, 1.6)
    circle(d, c, 58 * cp, 0.2 * cp, 1.2)
    circle(d, c, 6 * cp, 0.95 * cp, fill=True)
    # scanning sweep (fake gradient from stacked arcs)
    sw = (t * 70) % 360
    sa = smooth(prog(t, 1.2, 2.4))
    for k in range(14):
        arc(d, c, 92, sw - k * 4.5 - 4, sw - k * 4.5, 0.65 * (1 - k / 14) * sa, 2.2)
    tokens = [("@username", -135), ("203.0.113.42", -45),
              ("example.com", 135), ("name@example.com", 45)]
    fm = f["mono"](23)
    for i, (s, deg) in enumerate(tokens):
        st = 1.4 + 0.65 * i
        p = out_cubic(prog(t, st, st + 1.2))
        ang = math.radians(deg)
        rr = lerp(380, 255, p) if abs(math.cos(ang)) > 0 else 255
        tx, ty = c[0] + rr * math.cos(ang) * 1.15, c[1] + rr * math.sin(ang) * 0.82
        tw = text_width(s, fm) + 44
        # connector
        ex, ey = lerp(tx, c[0], 0.0), lerp(ty, c[1], 0.0)
        ux, uy = c[0] - tx, c[1] - ty
        L = math.hypot(ux, uy)
        ux, uy = ux / L, uy / L
        a_pt = (tx + ux * (tw / 2 if abs(ux) > 0.5 else 26) * 0.9, ty + uy * 28)
        b_pt = (c[0] - ux * 100, c[1] - uy * 100)
        line(d, [a_pt, (lerp(a_pt[0], b_pt[0], p), lerp(a_pt[1], b_pt[1], p))], 0.16 * p, 1)
        rrect(d, (tx - tw / 2, ty - 28, tx + tw / 2, ty + 28), 28, 0.5 * p, 1.2, fill=0.05 * p)
        text(d, tx, ty + 8, s, fm, 0.9 * p, "c")
        # a query dot travels into the core, then the core ripples
        cyc = (t - st - 1.4) / 4.0
        if cyc > 0:
            ph = cyc % 1
            if ph < 0.35:
                q = ph / 0.35
                circle(d, (lerp(a_pt[0], b_pt[0], q), lerp(a_pt[1], b_pt[1], q)),
                       3, 0.9 * math.sin(math.pi * q), fill=True)
            elif ph < 0.8:
                q = (ph - 0.35) / 0.45
                circle(d, c, 92 + 60 * out_cubic(q), 0.4 * (1 - q) ** 2, 1.4)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    return layer, (1430, 548, 0.7), fade_io(t, dur)


def scene_unified(t, dur):
    img, d = new_layer()
    f = fonts()
    a, r = reveal(t, 0.4, 1.2)
    text(d, W / 2, 290, "One company. Four tools.", f["semi"](64), 0.96 * a, "c", -0.5, r)
    items = [("Shield", "For Windows"), ("VM", "Free"),
             ("VPN", "Coming soon"), ("OSINT", "Coming soon")]
    colw = 360
    x_start = W / 2 - colw * 2
    for i, (name, status) in enumerate(items):
        st = 1.5 + 0.55 * i
        a, r = reveal(t, st, 1.2)
        cx = x_start + colw * (i + 0.5)
        if i > 0:
            line(d, [(x_start + colw * i, 470), (x_start + colw * i, 770 * 1.0)], 0.2 * a, 1)
        text(d, cx, 560, "THOMSEN", f["med"](20), 0.5 * a, "c", 8, r)
        text(d, cx, 650, name, f["semi"](72), 0.98 * a, "c", -1, r)
        text(d, cx, 710, status, f["reg"](26), (0.55 if i < 2 else 0.42) * a, "c", 0.3, r)
    # a single pass of light across the row
    bx = lerp(x_start - 200, x_start + colw * 4 + 200, in_out(prog(t, 4.2, 6.6)))
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    xs = np.arange(W, dtype=np.float32)[None, :]
    ys = np.arange(H, dtype=np.float32)[:, None]
    band = np.exp(-((xs - bx) / 140) ** 2) * np.exp(-((ys - 640) / 160) ** 2)
    on = smooth(prog(t, 4.2, 4.8)) * (1 - smooth(prog(t, 6.0, 6.8)))
    layer = layer * (1 + 0.55 * band * on) + 0.0
    return layer, (W / 2, 600, 0.65), fade_io(t, dur)


def scene_end(t, dur):
    img, d = new_layer()
    f = fonts()
    a1 = smooth(prog(t, 0.6, 2.0)) * (1 - smooth(prog(t, 3.9, 4.8)))
    _, r1 = reveal(t, 0.6, 1.4)
    text(d, W / 2, 566, "Software.", f["semi"](104), 0.98 * a1, "c", -1.5, r1)
    a2 = smooth(prog(t, 1.9, 3.1)) * (1 - smooth(prog(t, 3.9, 4.8)))
    text(d, W / 2, 566 + 0, "", f["semi"](10), 0)
    # second half of the line sits below, grey
    text(d, W / 2, 684, "Designed with purpose.", f["semi"](104), 0.6 * a2, "c", -1.5, (1 - out_cubic(prog(t, 1.9, 3.3))) * 18)
    # logo + site
    la = smooth(prog(t, 5.0, 6.8))
    draw_logo(img, d, W / 2, 520, la, 600)
    sa, sr = reveal(t, 6.6, 1.2)
    text(d, W / 2, 740, SITE, f["med"](34), 0.85 * sa, "c", 1.5, sr)
    layer = np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255
    end_fade = 1 - smooth(prog(t, dur - 1.4, dur))
    return layer * end_fade, (W / 2, 580, 0.65 * smooth(prog(t, 0, 1))), 1.0


SCENE_FN = {"logo": scene_logo, "tag": scene_tag, "shield": scene_shield,
            "vm": scene_vm, "vpn": scene_vpn, "osint": scene_osint,
            "unified": scene_unified, "end": scene_end}

XX = np.arange(W, dtype=np.float32)[None, :]
YY = np.arange(H, dtype=np.float32)[:, None]
VIG = (1 - 0.35 * (((XX - W / 2) / (W / 2)) ** 2 + ((YY - H / 2) / (H / 2)) ** 2) * 0.5).astype(np.float32)


def render_frame(n):
    fonts()
    t = n / FPS
    name, start, dur = next(s for s in reversed(SCENES) if t >= s[1] - 1e-9)
    lt = t - start
    out = SCENE_FN[name](lt, dur)
    if len(out) == 3 and isinstance(out[0], np.ndarray):
        layer, spot, alpha = out
    else:
        layer, spot, alpha = out
    layer = layer.astype(np.float32)
    # glow
    small = Image.fromarray((np.clip(layer, 0, 1) * 255).astype(np.uint8)).resize((W // 4, H // 4), Image.BILINEAR)
    glow = np.asarray(small.filter(ImageFilter.GaussianBlur(5)).resize((W, H), Image.BILINEAR), np.float32) / 255
    wide = np.asarray(small.filter(ImageFilter.GaussianBlur(16)).resize((W, H), Image.BILINEAR), np.float32) / 255
    val = layer * alpha + glow * 0.32 * alpha + wide * 0.28 * alpha
    cx, cy, st = spot
    spotv = st * alpha * 0.050 * np.exp(-(((XX - cx) / 760) ** 2 + ((YY - cy) / 470) ** 2))
    base = (spotv + val) * VIG
    rng = np.random.default_rng(n)
    grain = rng.standard_normal((H, W)).astype(np.float32) * (1.1 / 255)
    gate = np.clip(base * 60, 0, 1)
    rgb = np.clip((base + grain * gate)[..., None] * TINT, 0, 1)
    return (rgb * 255 + 0.5).astype(np.uint8).tobytes()


def main():
    out = sys.argv[1]
    limit = int(sys.argv[2]) if len(sys.argv) > 2 else int(DURATION * FPS)
    cmd = ["ffmpeg", "-y", "-loglevel", "error", "-f", "rawvideo", "-pix_fmt", "rgb24",
           "-s", f"{W}x{H}", "-r", str(FPS), "-i", "-", "-c:v", "libx264",
           "-preset", "slow", "-crf", "15", "-pix_fmt", "yuv420p", "-an", out]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    with Pool(4) as pool:
        for i, fr in enumerate(pool.imap(render_frame, range(limit), chunksize=2)):
            p.stdin.write(fr)
            if i % 60 == 0:
                print(f"{i}/{limit}", flush=True)
    p.stdin.close()
    p.wait()


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "still":
        Image.frombytes("RGB", (W, H), render_frame(int(float(sys.argv[2]) * FPS))).save(sys.argv[3])
    else:
        main()
