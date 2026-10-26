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
    ("logo", 0.0, 7.0),
    ("hero", 7.0, 5.5),
    ("statement", 12.5, 6.5),
    ("shield", 19.0, 11.5),
    ("vpn", 30.5, 11.5),
    ("osint", 42.0, 11.5),
    ("vm", 53.5, 11.5),
    ("unified", 65.0, 8.5),
    ("end", 73.5, 9.5),
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
        F["semi"] = lambda s: font("Inter-SemiBold", s)
        F["heavy"] = lambda s: font("InterDisplay-ExtraBold", s)
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
    text(d, cx, base, "Thomsen.", f["heavy"](150 * k), a, "c", -6 * k)
    text(d, cx, base + 62 * k, "SOFTWARE", f["semi"](24 * k), a * 0.55, "c", 22 * k)


# ---------------------------------------------------------------- scenes
# Each returns (img, spot) where spot = (cx, cy, strength) for the spotlight.
def new_layer():
    img = Image.new("L", (W * S, H * S), 0)
    return img, ImageDraw.Draw(img)


def cursor(d, x, y, s, a):
    """Standard arrow pointer with its tip at (x, y)."""
    pts = [(0, 0), (0, 22), (5.5, 17.2), (9.2, 25.6), (12.8, 24.0), (9.2, 15.8), (16.2, 15.8)]
    k = s * 2.0
    out = [(x + (u - 1.5) * k, y + (v - 2.0) * k) for u, v in pts]
    inn = [(x + u * k, y + v * k) for u, v in pts]
    d.polygon([(px(u), px(v)) for u, v in out], fill=0)
    d.line([(px(u), px(v)) for u, v in out + [out[0]]], fill=0, width=px(5))
    d.polygon([(px(u), px(v)) for u, v in inn], fill=gv(0.98 * a))


def bez(p0, p1, p2, u):
    return ((1 - u) ** 2 * p0[0] + 2 * (1 - u) * u * p1[0] + u * u * p2[0],
            (1 - u) ** 2 * p0[1] + 2 * (1 - u) * u * p1[1] + u * u * p2[1])


def scene_logo(t, dur):
    """A T, a click, and the Thomsen Software wordmark opens out from it."""
    img, d = new_layer()
    f = fonts()
    cx, cy = W / 2, 540
    size, tr, base = 150, -6, 590
    big = f["heavy"](size)
    adv_T = big.getlength("T") / S + tr
    full_w = text_width("Thomsen.", big, tr)
    x_left = cx - full_w / 2
    click = 2.95
    fade = 1 - smooth(prog(t, dur - 1.1, dur))

    ta = smooth(prog(t, 0.7, 1.8))
    sc = 0.92 + 0.08 * out_cubic(prog(t, 0.7, 2.0))
    tile_a = ta * (1 - smooth(prog(t, 3.1, 3.9)))
    hs = 150 * sc
    flash = math.exp(-(t - click) * 5) if t > click else 0
    rrect(d, (cx - hs, cy - hs, cx + hs, cy + hs), 64 * sc, 0.5 * tile_a, 1.6, fill=0.07 * tile_a)
    slide = in_out(prog(t, 3.15, 4.4))
    Tw = big.getlength("T") / S
    tx = lerp(cx - Tw / 2, x_left, slide)
    text(d, tx, base, "T", big, min(1.0, ta * (0.92 + 0.2 * flash)), "l", 0)

    rest, rd = new_layer()
    rx = tx + adv_T
    text(rd, rx, base, "homsen.", big, 0.98, "l", tr)
    open_p = in_out(prog(t, 3.3, 4.7))
    edge = rx + text_width("homsen.", big, tr) * open_p
    xs = np.arange(W, dtype=np.float32)[None, :]
    m = np.clip((edge - xs) / 90, 0, 1)
    m = m * m * (3 - 2 * m)
    layer = layer_of(img) + layer_of(rest) * m

    sw, swd = new_layer()
    sa = smooth(prog(t, 4.4, 5.6))
    text(swd, cx, base + 66, "SOFTWARE", f["semi"](25), 0.58 * sa, "c", 22,
         (1 - out_cubic(prog(t, 4.4, 5.8))) * 14)
    layer = layer + layer_of(sw)

    rip, rpd = new_layer()
    tipx, tipy = cx + 6, cy + 20
    if t > click:
        q = prog(t, click, click + 1.0)
        circle(rpd, (tipx, tipy), 8 + 120 * out_cubic(q), 0.55 * (1 - q) ** 2, 1.6)
    layer = layer + layer_of(rip)

    cur, cd = new_layer()
    u = in_out(prog(t, 1.7, 2.85))
    p = bez((cx + 560, cy + 400), (cx + 330, cy + 360), (tipx, tipy), u)
    press = smooth(prog(t, click - 0.05, click + 0.04)) * (1 - smooth(prog(t, click + 0.1, click + 0.25)))
    ca = smooth(prog(t, 1.5, 2.1)) * (1 - smooth(prog(t, 3.6, 4.3)))
    drift = smooth(prog(t, 3.4, 4.3)) * 40
    if ca > 0.01:
        # opaque dark outline first so the pointer reads cleanly over the T
        cursor(cd, p[0] + drift, p[1] + drift * 0.6, 1.0 - 0.14 * press, ca)
        carr = layer_of(cur)
        solid = np.asarray(cur.resize((W, H), Image.BILINEAR), np.float32) / 255
        outline = np.asarray(cur.filter(ImageFilter.MaxFilter(9)).resize((W, H), Image.BILINEAR), np.float32) > 0
        layer = np.where(outline, carr, layer)
    spot = (cx, cy, 0.7 * smooth(prog(t, 0.6, 2.4)))
    return layer * fade, spot, fade


def fade_io(t, dur, a=0.8, b=0.8):
    return smooth(prog(t, 0, a)) * (1 - smooth(prog(t, dur - b, dur)))


def layer_of(img):
    return np.asarray(img.resize((W, H), Image.LANCZOS), np.float32) / 255


def pill_outline(d, x, y, label, a, h=40, size=17, tracking=0.3, border=0.4, fillv=0.03):
    f = fonts()
    fnt = f["med"](size)
    w = text_width(label, fnt, tracking) + 38
    rrect(d, (x, y, x + w, y + h), h / 2, border * a, 1.2, fill=fillv * a)
    text(d, x + 19, y + h / 2 + size * 0.36, label, fnt, 0.88 * a, "l", tracking)
    return x + w


def pill_white(d, x, y, label, a, h=46, size=19):
    f = fonts()
    fnt = f["semi"](size)
    w = text_width(label, fnt) + 48
    rrect(d, (x, y, x + w, y + h), h / 2, 0, 1, fill=0.97 * a)
    d.text((px(x + 24), px(y + h / 2 + size * 0.36)), label, font=fnt, fill=0, anchor="ls")
    return x + w


def heavy(d, x, y, s, size, a, anchor="l", rise=0.0):
    text(d, x, y, s, fonts()["heavy"](size), a, anchor, -0.04 * size, rise)


def eyebrow(d, x, y, s, a, size=15, anchor="l", rise=0.0):
    text(d, x, y, s.upper(), fonts()["semi"](size), a, anchor, size * 0.22, rise)


def scene_hero(t, dur):
    img, d = new_layer()
    f = fonts()
    a, r = reveal(t, 0.4, 1.0)
    label = "THOMSEN SOFTWARE  ·  INDEPENDENT DEVELOPER"
    fnt = f["semi"](15)
    w = text_width(label, fnt, 3.2) + 56
    rrect(d, (W / 2 - w / 2, 318, W / 2 + w / 2, 358), 20, 0.35 * a, 1.2, fill=0.04 * a)
    circle(d, (W / 2 - w / 2 + 26, 338), 3.5, 0.8 * a, fill=True)
    text(d, W / 2 + 10, 343, label, fnt, 0.75 * a, "c", 3.2)
    a1, r1 = reveal(t, 0.8, 1.3)
    heavy(d, W / 2, 500, "Software you can", 116, a1, "c", r1)
    a2, r2 = reveal(t, 1.5, 1.3)
    big = f["heavy"](116)
    tr = -0.04 * 116
    w1 = text_width("actually ", big, tr)
    w2 = text_width("trust.", big, tr)
    x0 = W / 2 - (w1 + w2) / 2
    text(d, x0, 626, "actually ", big, a2, "l", tr, r2)
    text(d, x0 + w1, 626, "trust.", big, a2 * (0.42 + 0.45 * smooth(prog(t, 2.8, 4.4))), "l", tr, r2)
    a3, r3 = reveal(t, 2.4, 1.2)
    text(d, W / 2, 728, "Independent applications for cybersecurity, digital", f["reg"](28), 0.55 * a3, "c", 0, r3)
    text(d, W / 2, 768, "investigations and system utilities.", f["reg"](28), 0.55 * a3, "c", 0, r3)
    return layer_of(img), (W / 2, 520, 0.7), fade_io(t, dur, 0.9, 0.9)


def scene_statement(t, dur):
    img, d = new_layer()
    f = fonts()
    lines = [["Functionality", "first."], ["Design", "with", "purpose."],
             ["Build", "for", "reliability."]]
    big = f["heavy"](112)
    tr = -0.04 * 112
    k = 0
    for li, words in enumerate(lines):
        x = 150
        for wd in words:
            st = 0.6 + 0.34 * k
            lit = smooth(prog(t, st, st + 0.6))
            a = 0.2 + 0.8 * lit
            text(d, x, 380 + li * 132, wd, big, a, "l", tr)
            x += text_width(wd + " ", big, tr)
            k += 1
    for i, lab in enumerate(["Planned", "Tested", "Released"]):
        a, r = reveal(t, 3.9 + 0.45 * i, 0.9)
        cx = 150 + i * 330
        circle(d, (cx + 17, 808 + r * 0.3), 17, 0.7 * a, 1.6)
        line(d, [(cx + 9, 808 + r * 0.3), (cx + 15, 815 + r * 0.3), (cx + 26, 800 + r * 0.3)], 0.95 * a, 2.2)
        text(d, cx + 48, 817, lab, f["med"](30), 0.9 * a, "l", 0.2, r)
    return layer_of(img), (700, 520, 0.6), fade_io(t, dur, 0.9, 0.9)


PRODUCTS = [
    ("shield", "Network protection", "Thomsen Shield",
     "See every connection. Stop the ones you do not trust.",
     ["Network monitoring", "Suspicious connection detection", "Threat intelligence",
      "Windows Firewall protection"], None, "Windows 10  ·  Windows 11"),
    ("vpn", "Privacy", "Thomsen VPN",
     "Private connectivity, currently in development.", [], "Coming soon", "Windows 10  ·  Windows 11"),
    ("osint", "Investigations", "Thomsen OSINT",
     "Public information, organized into one clear picture.", [], "Coming soon", "Windows 10  ·  Windows 11"),
    ("vm", "Virtualization", "Thomsen VM",
     "A full machine, running inside your desktop.", [], "Free", "Windows 10  ·  Windows 11  ·  Free"),
]


def lineup(d, t, idx):
    """The site's 'Four tools. One developer.' rail, with one product active."""
    f = fonts()
    RX, X = 118, 150
    eyebrow(d, RX, 84, "The lineup", 0.5)
    heavy(d, RX, 170, "Four tools.", 78, 1.0)
    heavy(d, RX, 246, "One developer.", 78, 1.0)
    y = 326
    items = []
    for i, (_, cat, name, desc, feats, status, meta) in enumerate(PRODUCTS):
        h = (340 if feats else 294) if i == idx else 84
        items.append(y)
        y += h
    top_end = y - 6
    line(d, [(RX, 326), (RX, top_end)], 0.13, 1.2)
    for i, (key, cat, name, desc, feats, status, meta) in enumerate(PRODUCTS):
        y0 = items[i]
        if i != idx:
            eyebrow(d, X, y0 + 18, cat, 0.26, 13)
            heavy(d, X, y0 + 64, name, 44, 0.3)
            continue
        h = 340 if feats else 294
        bar = out_cubic(prog(t, 0.2, 1.0))
        line(d, [(RX, y0 + 4), (RX, y0 + 4 + (h - 20) * bar)], 1.0, 2.6)
        a, r = reveal(t, 0.35, 0.9)
        eyebrow(d, X, y0 + 18, cat, 0.85 * a, 13, "l", r)
        a, r = reveal(t, 0.55, 1.1)
        heavy(d, X, y0 + 90, name, 74, a, "l", r)
        a, r = reveal(t, 1.1, 1.0)
        text(d, X, y0 + 140, desc, f["reg"](26), 0.66 * a, "l", 0, r)
        if feats:
            rows = [feats[:2], feats[2:]]
            for ri, row in enumerate(rows):
                xx = X
                for ci, ft in enumerate(row):
                    a, r = reveal(t, 1.7 + 0.3 * (ri * 2 + ci), 0.8)
                    xx = pill_outline(d, xx, y0 + 172 + ri * 50 + r * 0.3, ft, a) + 12
            my = y0 + 300
        else:
            a, r = reveal(t, 1.7, 0.8)
            if status == "Coming soon":
                pill_white(d, X, y0 + 172 + r * 0.3, status, a)
            else:
                pill_white(d, X, y0 + 172 + r * 0.3, status, a)
            my = y0 + 258
        a, r = reveal(t, 2.5, 0.9)
        text(d, X, my, meta, f["reg"](17), 0.4 * a, "l", 0.3, r)


def panel(d, t):
    p = smooth(prog(t, 0.2, 1.3))
    rrect(d, (1010, 170, 1830, 910), 36, 0.17 * p, 1.4, fill=0.032 * p)
    for j in range(3):
        circle(d, (1782 + 16 * j - 16, 202), 4.2, 0.4 * p, fill=True)
    return p


def finish_product(img, t, dur, cx=1420):
    return layer_of(img), (cx, 540, 0.7), fade_io(t, dur, 0.5, 0.5)


def scene_shield(t, dur):
    img, d = new_layer()
    lineup(d, t, 0)
    panel(d, t)
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
        r = lerp(440, 322, travel)
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
    return finish_product(img, t, dur)


def scene_vm(t, dur):
    img, d = new_layer()
    lineup(d, t, 3)
    panel(d, t)
    cx, cy = 1430, 548
    hp = out_cubic(prog(t, 0.4, 1.9))
    hw, hh = 330 * (0.9 + 0.1 * hp), 215 * (0.9 + 0.1 * hp)
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
    return finish_product(img, t, dur)


def scene_vpn(t, dur):
    img, d = new_layer()
    lineup(d, t, 1)
    panel(d, t)
    x0, x1, cy = 1075, 1775, 540
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
    return finish_product(img, t, dur)


def scene_osint(t, dur):
    img, d = new_layer()
    lineup(d, t, 2)
    panel(d, t)
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
        rr = lerp(330, 255, p)
        tx, ty = c[0] + rr * math.cos(ang) * 0.98, c[1] + rr * math.sin(ang) * 0.9
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
    return finish_product(img, t, dur)


def scene_unified(t, dur):
    img, d = new_layer()
    f = fonts()
    RX = 150
    a, r = reveal(t, 0.3, 0.9)
    eyebrow(d, RX, 150, "Why Thomsen Software", 0.5 * a, 15, "l", r)
    a, r = reveal(t, 0.5, 1.2)
    heavy(d, RX, 270, "Quiet on your system.", 100, a, "l", r)
    a, r = reveal(t, 1.1, 1.2)
    heavy(d, RX, 380, "Clear in your results.", 100, a * (0.45 + 0.55 * smooth(prog(t, 3.2, 5.0))), "l", r)
    cards = [("Network protection", "Thomsen Shield", "Windows 10  ·  Windows 11"),
             ("Privacy", "Thomsen VPN", "Coming soon"),
             ("Investigations", "Thomsen OSINT", "Coming soon"),
             ("Virtualization", "Thomsen VM", "Free")]
    cw, gap, y0, ch = 390, 20, 520, 280
    for i, (cat, name, st) in enumerate(cards):
        a, r = reveal(t, 1.9 + 0.4 * i, 1.1)
        x = RX + i * (cw + gap)
        yy = y0 + r * 1.2
        rrect(d, (x, yy, x + cw, yy + ch), 30, 0.16 * a, 1.3, fill=0.05 * a)
        eyebrow(d, x + 34, yy + 62, cat, 0.5 * a, 13)
        heavy(d, x + 34, yy + 134, name, 38, a)
        line(d, [(x + 34, yy + 196), (x + cw - 34, yy + 196)], 0.12 * a, 1)
        text(d, x + 34, yy + 238, st, f["reg"](20), 0.55 * a, "l", 0.3)
    layer = layer_of(img)
    bx = lerp(RX - 250, RX + 4 * (cw + gap) + 250, in_out(prog(t, 4.6, 7.0)))
    xs = np.arange(W, dtype=np.float32)[None, :]
    ys = np.arange(H, dtype=np.float32)[:, None]
    band = np.exp(-((xs - bx) / 160) ** 2) * np.exp(-((ys - 660) / 200) ** 2)
    on = smooth(prog(t, 4.6, 5.2)) * (1 - smooth(prog(t, 6.4, 7.2)))
    layer = layer * (1 + 0.5 * band * on)
    return layer, (W / 2, 560, 0.65), fade_io(t, dur)


def scene_end(t, dur):
    img, d = new_layer()
    f = fonts()
    out1 = 1 - smooth(prog(t, 4.0, 4.9))
    a1, r1 = reveal(t, 0.6, 1.4)
    heavy(d, W / 2, 530, "Software.", 140, a1 * out1, "c", r1)
    a2, r2 = reveal(t, 1.9, 1.4)
    heavy(d, W / 2, 680, "Designed with purpose.", 140, a2 * out1 * 0.5, "c", r2)
    la = smooth(prog(t, 5.0, 6.8))
    draw_logo(img, d, W / 2, 500, la, 600)
    sa, sr = reveal(t, 6.4, 1.1)
    pill_white(d, W / 2 - text_width(SITE, f["semi"](24)) / 2 - 28, 660 + sr * 0.4, SITE, sa, 60, 24)
    layer = layer_of(img)
    end_fade = 1 - smooth(prog(t, dur - 1.4, dur))
    return layer * end_fade, (W / 2, 560, 0.65 * smooth(prog(t, 0, 1))), 1.0


SCENE_FN = {"logo": scene_logo, "hero": scene_hero, "statement": scene_statement,
            "shield": scene_shield, "vpn": scene_vpn, "osint": scene_osint, "vm": scene_vm,
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
    lift = 0.022 * alpha * (smooth(prog(lt, 3.0, 5.0)) if name == 'logo' else 1.0)
    base = (spotv + val + lift) * VIG
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
