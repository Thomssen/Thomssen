"""Thomsen Software advertisement: frame renderer.

Every frame is a pure function of time, drawn as a monochrome light layer
(supersampled 2x), then given a soft glow, a faint spotlight, vignette and
film grain. Output is piped straight into ffmpeg.

Brand assets: if advert/assets/logo.png exists (white or light artwork on a
transparent background) it replaces the typographic wordmark everywhere.
"""
import colorsys
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
SITE = "thomsen-software.com"

# (name, start, length)
SPEED = {"hook": 1.0, "logo": 1.8, "hero": 1.9, "shield": 1.8, "vpn": 1.8,
         "osint": 1.8, "vm": 1.8, "unified": 1.7, "end": 1.58}
_LEN = [("hook", 5.2), ("logo", 3.9), ("hero", 2.9), ("shield", 6.4), ("vpn", 6.4),
        ("osint", 6.4), ("vm", 6.4), ("unified", 5.0), ("end", 6.0)]
SCENES = []
_t = 0.0
for _n, _l in _LEN:
    SCENES.append((_n, _t, _l))
    _t += _l
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
        F["heavy"] = lambda s: font("InterDisplay-Bold", s)
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
    tile_a = ta * (1 - smooth(prog(t, 3.05, 3.5)))
    hs = 150 * sc
    flash = math.exp(-(t - click) * 5) if t > click else 0
    slide = in_out(prog(t, 3.15, 4.4))
    Tw = big.getlength("T") / S
    tx = lerp(cx - Tw / 2, x_left, slide)
    text(d, tx, base, "T", big, smooth(prog(t, 2.9, 3.1)), "l", 0)

    rest, rd = new_layer()
    rx = tx + adv_T
    text(rd, rx, base, "homsen.", big, 0.98, "l", tr)
    open_p = in_out(prog(t, 3.3, 4.7))
    edge = rx + text_width("homsen.", big, tr) * open_p
    xs = np.arange(W, dtype=np.float32)[None, :]
    m = np.clip((edge - xs) / 90, 0, 1)
    m = m * m * (3 - 2 * m)
    layer = layer_of(img) + layer_of(rest) * m
    tarr = np.zeros((H, W), np.float32)
    pr = smooth(prog(t, click - 0.05, click + 0.04)) * (1 - smooth(prog(t, click + 0.1, click + 0.25)))
    lum_, al_ = tile_lum("T", lerp(-1.5, 1.4, in_out(prog(t, 0.7, 3.0))))
    blit_tile(tarr, lum_, al_, cx, 535 + 4 * math.sin(t * 1.2), 300 * sc * (1 - 0.04 * pr), a=tile_a,
              tilt=0.6 * (1 - out_cubic(prog(t, 0.7, 2.4))), reflect=0.3)
    layer = layer + tarr

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
    text(d, x, y, s, fonts()["heavy"](size), a, anchor, -0.032 * size, rise)


def eyebrow(d, x, y, s, a, size=15, anchor="l", rise=0.0):
    text(d, x, y, s.upper(), fonts()["semi"](size), a, anchor, size * 0.22, rise)


def scene_hook(t, dur):
    img, d = new_layer()
    lines = [(0.25, "Right now,"), (1.0, "your PC is talking"), (1.8, "to the internet.")]
    gone = 1 - smooth(prog(t, 2.55, 2.9))
    for i, (tb, s) in enumerate(lines):
        a, r = reveal(t, tb, 0.35)
        nxt = prog(t, lines[i + 1][0], lines[i + 1][0] + 0.3) if i + 1 < len(lines) else 0
        heavy(d, W / 2, 400 + i * 150, s, 118, a * gone * (1 - 0.5 * nxt), "c", r)
    a, r = reveal(t, 3.1, 0.4)
    heavy(d, W / 2, 500, "Do you know", 150, a, "c", r)
    a, r = reveal(t, 3.7, 0.4)
    heavy(d, W / 2, 672, "who to?", 150, a, "c", r)
    for tb in (0.25, 1.0, 1.8, 3.1, 3.7):
        q = prog(t, tb, tb + 1.1)
        circle(d, (W / 2, 540), 120 + 760 * out_cubic(q), 0.2 * (1 - q) ** 2, 1.6)
    fade = smooth(prog(t, 0, 0.12)) * (1 - smooth(prog(t, dur - 0.25, dur)))
    return layer_of(img), (W / 2, 540, 0.5), fade


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


# ---------------------------------------------------------------- glass objects
# Each product is a polished squircle slab with an embossed mark. Geometry is built
# once as a height field; lighting (a key light, a rim and a sweeping softbox) is
# evaluated per frame, so the slab catches light as it turns.
_GLYPH = {}
_GEO = {}
TN = 600


def _chaikin(pts, n=3):
    for _ in range(n):
        out = []
        for i in range(len(pts)):
            p, q = pts[i], pts[(i + 1) % len(pts)]
            out.append((0.75 * p[0] + 0.25 * q[0], 0.75 * p[1] + 0.25 * q[1]))
            out.append((0.25 * p[0] + 0.75 * q[0], 0.25 * p[1] + 0.75 * q[1]))
        pts = out
    return pts


def glyph_mask(kind, N=TN):
    key = (kind, N)
    if key in _GLYPH:
        return _GLYPH[key]
    S2 = 2 * N
    im = Image.new("L", (S2, S2), 0)
    g = ImageDraw.Draw(im)
    u = lambda v: v * S2
    wd = lambda v: max(1, int(v * S2))

    def cap(x, y, w):
        g.ellipse([u(x) - wd(w) / 2, u(y) - wd(w) / 2, u(x) + wd(w) / 2, u(y) + wd(w) / 2], fill=255)

    if kind == "T":
        fnt = ImageFont.truetype(FONT_DIR + "InterDisplay-Bold.otf", N)
        bb = g.textbbox((0, 0), "T", font=fnt)
        g.text((S2 / 2 - (bb[2] - bb[0]) / 2 - bb[0], S2 / 2 - (bb[3] - bb[1]) / 2 - bb[1]), "T", font=fnt, fill=255)
    elif kind == "shield":
        pts = _chaikin([(0.5, 0.20), (0.75, 0.28), (0.75, 0.50), (0.70, 0.63), (0.5, 0.80),
                        (0.30, 0.63), (0.25, 0.50), (0.25, 0.28)], 3)
        poly = [(u(x), u(y)) for x, y in pts]
        g.line(poly + [poly[0]], fill=255, width=wd(0.05), joint="curve")
        g.line([(u(.41), u(.50)), (u(.485), u(.575)), (u(.60), u(.42))], fill=255, width=wd(0.05), joint="curve")
        cap(.41, .50, .05)
        cap(.60, .42, .05)
    elif kind == "vm":
        g.rounded_rectangle([u(.25), u(.28), u(.65), u(.62)], radius=u(.045), outline=255, width=wd(0.04))
        g.rounded_rectangle([u(.35), u(.38), u(.75), u(.72)], radius=u(.045), fill=0)
        g.rounded_rectangle([u(.35), u(.38), u(.75), u(.72)], radius=u(.045), outline=255, width=wd(0.04))
        g.line([(u(.35), u(.46)), (u(.75), u(.46))], fill=255, width=wd(0.03))
    elif kind == "vpn":
        g.rounded_rectangle([u(.29), u(.47), u(.71), u(.76)], radius=u(.055), outline=255, width=wd(0.05))
        g.arc([u(.355), u(.22), u(.645), u(.56)], 180, 360, fill=255, width=wd(0.05))
        g.line([(u(.355), u(.39)), (u(.355), u(.48))], fill=255, width=wd(0.05))
        g.line([(u(.645), u(.39)), (u(.645), u(.48))], fill=255, width=wd(0.05))
        g.ellipse([u(.465), u(.565), u(.535), u(.635)], fill=255)
        g.line([(u(.5), u(.60)), (u(.5), u(.69))], fill=255, width=wd(0.035))
    elif kind == "osint":
        g.ellipse([u(.25), u(.25), u(.63), u(.63)], outline=255, width=wd(0.055))
        g.line([(u(.58), u(.58)), (u(.76), u(.76))], fill=255, width=wd(0.08))
        cap(.58, .58, .08)
        cap(.76, .76, .08)
    m = np.asarray(im.resize((N, N), Image.LANCZOS), np.float32) / 255
    _GLYPH[key] = m
    return m


def _geo(kind, N=TN):
    key = (kind, N)
    if key in _GEO:
        return _GEO[key]
    yy, xx = np.mgrid[0:N, 0:N].astype(np.float32)
    x = (xx - (N - 1) / 2) / (N / 2)
    y = (yy - (N - 1) / 2) / (N / 2)
    r = (np.abs(x) ** 5 + np.abs(y) ** 5) ** 0.2
    inside = 1 - r
    alpha = np.clip(inside * (N / 2) / 1.6, 0, 1)
    b = np.clip(inside / 0.15, 0, 1)
    h = (b * b * (3 - 2 * b)) ** 0.75
    gm = glyph_mask(kind, N)
    gb = np.asarray(Image.fromarray((gm * 255).astype(np.uint8)).filter(ImageFilter.GaussianBlur(3.2)), np.float32) / 255
    ht = h + 0.14 * gb
    gy, gx = np.gradient(ht)
    k = 30.0
    nx, ny = -gx * k, -gy * k
    nz = 1 / np.sqrt(1 + nx * nx + ny * ny)
    nx, ny = nx * nz, ny * nz
    _GEO[key] = (nx, ny, nz, alpha, gm, x, y, inside)
    return _GEO[key]


def tile_lum(kind, sweep):
    nx, ny, nz, alpha, gm, x, y, inside = _geo(kind)
    L = np.array([-0.45, -0.70, 0.75], np.float32)
    L = L / np.linalg.norm(L)
    Hh = L + np.array([0, 0, 1], np.float32)
    Hh = Hh / np.linalg.norm(Hh)
    ndl = np.clip(nx * L[0] + ny * L[1] + nz * L[2], 0, 1)
    ndh = np.clip(nx * Hh[0] + ny * Hh[1] + nz * Hh[2], 0, 1)
    top = np.clip(0.5 - y * 0.5, 0, 1)
    body = 0.060 + 0.085 * top + 0.11 * ndl
    spec = ndh ** 60 * 0.85 + ndh ** 9 * 0.10
    rim = (1 - nz) ** 2.0 * 0.95 * (0.5 + 0.5 * top)
    edge = np.exp(-((inside - 0.013) / 0.009) ** 2) * 0.5 * (0.30 + 0.70 * top)
    ang = math.radians(-35)
    dd = x * math.cos(ang) + y * math.sin(ang)
    strip = np.exp(-((dd - sweep) / 0.17) ** 2) * 0.30 * (0.4 + 0.6 * nz)
    glyph = 0.46 + 0.42 * ndl + spec * 0.9 + strip * 1.5
    lum = (body + spec + rim + strip) * (1 - gm) + glyph * gm + edge
    return np.clip(lum, 0, 1.15).astype(np.float32), alpha


def blit_tile(canvas, lum, alpha, cx, cy, size, a=1.0, tilt=0.0, reflect=0.35):
    """Composite a rendered slab onto canvas (in place) with an optional floor reflection."""
    if a <= 0.003:
        return
    N = lum.shape[0]
    s = size / N
    cs = max(0.25, math.cos(tilt))
    co = (1 / (s * cs), 0, N / 2 - cx / (s * cs), 0, 1 / s, N / 2 - cy / s)
    prem = np.asarray(Image.fromarray(lum * alpha, "F").transform((W, H), Image.AFFINE, co, resample=Image.BILINEAR), np.float32)
    am = np.asarray(Image.fromarray(alpha, "F").transform((W, H), Image.AFFINE, co, resample=Image.BILINEAR), np.float32)
    if reflect > 0:
        yb = cy + size / 2 + 8
        cr = (1 / (s * cs), 0, N / 2 - cx / (s * cs), 0, -1 / s, N / 2 + (2 * (yb - 8) - cy) / s)
        refl = np.asarray(Image.fromarray(lum * alpha, "F").transform((W, H), Image.AFFINE, cr, resample=Image.BILINEAR), np.float32)
        fade = np.clip(1 - (YY - yb) / (0.40 * size), 0, 1) ** 1.8 * (YY > yb)
        canvas += refl * fade * reflect * a
    canvas *= 1 - am * a
    canvas += prem * a


def silver(layer, y0, y1, bottom=0.70):
    """Top-to-bottom silver falloff on a text layer, in the manner of a keynote title."""
    g = 1 - (1 - bottom) * np.clip((YY - y0) / (y1 - y0), 0, 1)
    return layer * g


PRODUCTS = [
    ("shield", "Network protection", "Thomsen Shield",
     "See every connection. Stop the ones you do not trust.", "Buy on Discord",
     "Windows 10  ·  Windows 11  ·  €10 / month"),
    ("vpn", "Privacy", "Thomsen VPN",
     "Your traffic, encrypted. One click to connect.", "Coming soon",
     "Windows 10  ·  Windows 11  ·  €10 / month"),
    ("osint", "Investigations", "Thomsen OSINT",
     "Public information, organized into one clear picture.", "Buy on Discord",
     "Windows 10  ·  Windows 11  ·  €10 / month"),
    ("vm", "Virtualization", "Thomsen VM",
     "A full machine, running inside your desktop.", "Download",
     "Windows 10  ·  Windows 11  ·  Free"),
]
def lineup(d, t, idx):
    """The website's 'Four tools. One developer.' rail, one product active."""
    f = fonts()
    RX, X = 118, 150
    eyebrow(d, RX, 84, "The lineup", 0.5)
    heavy(d, RX, 170, "Four tools.", 78, 1.0)
    heavy(d, RX, 246, "One developer.", 78, 1.0)
    y = 326
    tops = []
    for i in range(len(PRODUCTS)):
        tops.append(y)
        y += 294 if i == idx else 84
    line(d, [(RX, 326), (RX, y - 6)], 0.13, 1.2)
    for i, (key, cat, name, desc, btn, meta) in enumerate(PRODUCTS):
        y0 = tops[i]
        if i != idx:
            eyebrow(d, X, y0 + 18, cat, 0.26, 13)
            heavy(d, X, y0 + 64, name, 44, 0.3)
            continue
        bar = out_cubic(prog(t, 0.2, 1.0))
        line(d, [(RX, y0 + 4), (RX, y0 + 4 + 274 * bar)], 1.0, 2.6)
        a, r = reveal(t, 0.35, 0.9)
        eyebrow(d, X, y0 + 18, cat, 0.85 * a, 13, "l", r)
        a, r = reveal(t, 0.55, 1.1)
        heavy(d, X, y0 + 90, name, 74, a, "l", r)
        a, r = reveal(t, 1.1, 1.0)
        text(d, X, y0 + 140, desc, f["reg"](26), 0.66 * a, "l", 0, r)
        a, r = reveal(t, 1.6, 0.8)
        xe = pill_white(d, X, y0 + 172 + r * 0.3, btn, a)
        text(d, xe + 26, y0 + 202 + r * 0.3, "Learn more", f["semi"](20), 0.9 * a, "l", 0)
        a, r = reveal(t, 2.2, 0.9)
        text(d, X, y0 + 258, meta, f["reg"](17), 0.4 * a, "l", 0.3, r)


def _blk(d, x, y, w, h, a, fill=None, r=6, line_a=0.0):
    if a <= 0.003:
        return
    rrect(d, (x, y, x + w, y + h), r, line_a * a, 1.2, fill=None if fill is None else fill * a)


def device(d, t, key, name):
    """The site's device mock-up: a dark tablet with a skeleton dashboard."""
    f = fonts()
    x0, y0, x1, y1 = 930, 300, 1830, 780
    p = smooth(prog(t, 0.2, 1.3))
    rrect(d, (x0, y0, x1, y1), 44, 0.28 * p, 1.6, fill=0.035 * p)
    rrect(d, (x0 + 14, y0 + 14, x1 - 14, y1 - 14), 34, 0.10 * p, 1.2, fill=0.02 * p)
    # title bar
    text(d, x0 + 36, y0 + 46, name, f["semi"](13), 0.9 * p, "l", 0)
    text(d, x0 + 36 + text_width(name, f["semi"](13)) + 14, y0 + 46, "/ Dashboard", f["reg"](12), 0.4 * p, "l", 0)
    for j in range(3):
        circle(d, (x1 - 58 + 16 * j, y0 + 42), 4.2, 0.55 * p, fill=True)
    line(d, [(x0 + 14, y0 + 62), (x1 - 14, y0 + 62)], 0.10 * p, 1)
    line(d, [(x0 + 190, y0 + 62), (x0 + 190, y1 - 14)], 0.10 * p, 1)
    n = [0]

    def A():
        n[0] += 1
        return reveal(t, 0.9 + 0.09 * n[0], 0.6)[0] * p

    dim = 0.4 if key == "vpn" else 1.0
    # sidebar
    sx = x0 + 34
    _blk(d, sx - 8, y0 + 84, 150, 28, A() * dim, 0.11, 8)
    _blk(d, sx + 6, y0 + 94, 80, 8, A() * dim, 0.85, 4)
    for k in range(4):
        _blk(d, sx + 6, y0 + 136 + 26 * k, 60 + 14 * ((k * 5) % 3), 7, A() * dim, 0.22, 3)
    _blk(d, sx + 6, y1 - 54, 50, 7, A() * dim, 0.2, 3)
    _blk(d, sx + 90, y1 - 60, 46, 18, A() * dim, 0.12, 9, 0.3)
    mx0, mx1 = x0 + 216, x1 - 38
    mw = mx1 - mx0
    if key in ("shield", "vpn"):
        _blk(d, mx0, y0 + 84, 190, 24, A() * dim, 0.95, 6)
        _blk(d, mx0, y0 + 118, 130, 8, A() * dim, 0.25, 4)
        cw = (mw - 14) / 2
        for c in range(2):
            cx = mx0 + c * (cw + 14)
            _blk(d, cx, y0 + 144, cw, 46, A() * dim, 0.04, 12, 0.16)
            _blk(d, cx + 16, y0 + 162, 110, 10, A() * dim, 0.7, 5)
            _blk(d, cx + cw - 72, y0 + 160, 54, 16, A() * dim, 0.14, 8, 0.3)
        sw = (mw - 28) / 3
        for c in range(3):
            cx = mx0 + c * (sw + 14)
            _blk(d, cx, y0 + 204, sw, 70, A() * dim, 0.04, 12, 0.16)
            _blk(d, cx + 16, y0 + 220, 70, 7, A() * dim, 0.3, 3)
            _blk(d, cx + 16, y0 + 242, 42, 18, A() * dim, 0.95 if c < 2 else 0.5, 5)
        _blk(d, mx0, y0 + 290, mw, 160, A() * dim, 0.04, 14, 0.16)
        cp = out_cubic(prog(t, 1.6, 3.4)) * dim * p
        pts = [(mx0 + 20 + (mw - 40) * u / 24, y0 + 410 - 14 * math.sin(u * 0.55) - (10 if u > 12 else 0) + 2 * math.sin(u * 2.1)) for u in range(25)]
        k_ = max(2, int(len(pts) * cp))
        if cp > 0.02:
            line(d, pts[:k_], 0.9, 1.8)
            circle(d, pts[k_ - 1], 3.6, 0.95, fill=True)
        _blk(d, mx0 + 20, y0 + 310, 110, 8, A() * dim, 0.3, 3)
    elif key == "osint":
        _blk(d, mx0, y0 + 84, 170, 24, A(), 0.95, 6)
        _blk(d, mx0, y0 + 120, mw * 0.78, 7, A(), 0.22, 3)
        _blk(d, mx0, y0 + 134, mw * 0.55, 7, A(), 0.22, 3)
        _blk(d, mx0, y0 + 164, mw, 92, A(), 0.04, 14, 0.16)
        _blk(d, mx0 + 20, y0 + 182, 70, 8, A(), 0.3, 3)
        a_ = A()
        _blk(d, mx0 + 20, y0 + 206, mw - 40, 34, a_, 0.07, 10)
        blink = 1.0 if int(t * 1.6) % 2 == 0 else 0.0
        _blk(d, mx0 + 34, y0 + 214, 100, 7, a_, 0.5, 3)
        line(d, [(mx0 + 142, y0 + 213), (mx0 + 142, y0 + 233)], 0.95 * a_ * blink, 2)
        _blk(d, mx1 - 100, y0 + 214, 70, 18, a_, 0.45, 9)
        _blk(d, mx0, y0 + 280, 100, 8, A(), 0.3, 3)
        a_ = A()
        _blk(d, mx0, y0 + 304, mw, 52, a_, 0.04, 14, 0.16)
        circle(d, (mx0 + 28, y0 + 330), 8, 0.55 * a_, 1.4)
        _blk(d, mx0 + 52, y0 + 322, 120, 8, a_, 0.8, 4)
        _blk(d, mx0 + 52, y0 + 336, 160, 6, a_, 0.25, 3)
        _blk(d, mx1 - 94, y0 + 322, 66, 16, a_, 0.3, 8)
    else:
        _blk(d, mx0, y0 + 84, 230, 26, A(), 0.95, 6)
        _blk(d, mx0, y0 + 122, 140, 8, A(), 0.25, 4)
        a_ = A()
        _blk(d, mx1 - 170, y0 + 82, 170, 40, a_, 0.97, 20)
        _blk(d, mx1 - 150, y0 + 98, 12, 3, a_ * 0, 0.0)
        d.text((px(mx1 - 85), px(y0 + 108)), "+  New machine", font=f["semi"](16), fill=0, anchor="ms")
        sw = (mw - 28) / 3
        for rr in range(2):
            for c in range(3):
                cx = mx0 + c * (sw + 14)
                cy = y0 + 148 + rr * 82
                _blk(d, cx, cy, sw, 70, A(), 0.04, 12, 0.16)
                _blk(d, cx + 16, cy + 16, 70, 7, A(), 0.3, 3)
                _blk(d, cx + 16, cy + 38, 40 + 12 * c, 18, A(), 0.95 if (rr + c) % 2 == 0 else 0.5, 5)
        _blk(d, mx0, y0 + 322, mw, 128, A(), 0.04, 14, 0.16)
        _blk(d, mx0 + 20, y0 + 340, 110, 8, A(), 0.5, 3)
        for k in range(2):
            a_ = A()
            yy = y0 + 372 + 36 * k
            circle(d, (mx0 + 28, yy + 6), 3.4, 0.8 * a_, fill=True)
            _blk(d, mx0 + 46, yy, 90, 8, a_, 0.8, 4)
            _blk(d, mx0 + mw * 0.55, yy + 1, mw * 0.3, 5, a_, 0.2, 3)
            _blk(d, mx1 - 80, yy - 4, 60, 16, a_, 0.18, 8, 0.3)
    if key == "vpn":
        a_ = smooth(prog(t, 1.6, 2.4)) * p
        cx, cy = (x0 + x1) / 2 + 90, (y0 + y1) / 2
        rrect(d, (cx - 112, cy - 28, cx + 112, cy + 28), 28, 0, 1, fill=0.97 * a_)
        d.text((px(cx), px(cy + 8)), "Coming soon", font=f["semi"](22), fill=0, anchor="ms")


def site_scene(t, dur, idx):
    img, d = new_layer()
    key, cat, name = PRODUCTS[idx][0], PRODUCTS[idx][1], PRODUCTS[idx][2]
    lineup(d, t, idx)
    device(d, t, key, name)
    lay = np.asarray(Image.fromarray(layer_of(img), "F").transform(
        (W, H), Image.AFFINE, (1, 0, 0, 0, 1, -70), resample=Image.BILINEAR), np.float32)
    return lay, (1380, 620, 0.7), fade_io(t, dur, 0.5, 0.5)


def scene_shield(t, dur):
    return site_scene(t, dur, 0)


def scene_vpn(t, dur):
    return site_scene(t, dur, 1)


def scene_osint(t, dur):
    return site_scene(t, dur, 2)


def scene_vm(t, dur):
    return site_scene(t, dur, 3)


def scene_unified(t, dur):
    img, d = new_layer()
    f = fonts()
    a, r = reveal(t, 0.3, 0.9)
    eyebrow(d, W / 2, 150, "Why Thomsen Software", 0.5 * a, 15, "c", r)
    a, r = reveal(t, 0.5, 1.2)
    heavy(d, W / 2, 250, "Quiet on your system.", 88, a, "c", r)
    a, r = reveal(t, 1.1, 1.2)
    heavy(d, W / 2, 346, "Clear in your results.", 88, a * (0.5 + 0.5 * smooth(prog(t, 3.2, 5.0))), "c", r)
    items = [("shield", "Thomsen Shield", "Windows 10  ·  Windows 11"),
             ("vpn", "Thomsen VPN", "Coming soon"),
             ("osint", "Thomsen OSINT", "Windows 10  ·  Windows 11"),
             ("vm", "Thomsen VM", "Free")]
    base_y = 600
    arr = np.zeros((H, W), np.float32)
    sweep = -1.5 + 3.0 * ((t * 0.12) % 1)
    for i, (key, name, st) in enumerate(items):
        s0 = 1.6 + 0.4 * i
        p = out_cubic(prog(t, s0, s0 + 1.2))
        cx = W / 2 + (i - 1.5) * 400
        lum, al = tile_lum(key, sweep - 0.2 * i)
        blit_tile(arr, lum, al, cx, base_y + 14 * (1 - p), 250 * (0.9 + 0.1 * p), a=p, tilt=0.18 * math.sin(t * 0.8 + i), reflect=0.28)
        a, r = reveal(t, s0 + 0.5, 0.9)
        heavy(d, cx, 808, name, 34, a, "c", r)
        text(d, cx, 846, st, f["reg"](20), 0.5 * a, "c", 0.3, r)
    layer = layer_of(img)
    layer = silver(layer, 100, 400, 0.78)
    return layer + arr, (W / 2, 620, 0.7), fade_io(t, dur)


def scene_end(t, dur):
    img, d = new_layer()
    f = fonts()
    out1 = 1 - smooth(prog(t, 4.0, 4.9))
    a1, r1 = reveal(t, 0.6, 1.4)
    heavy(d, W / 2, 530, "Software.", 140, a1 * out1, "c", r1)
    a2, r2 = reveal(t, 1.9, 1.4)
    heavy(d, W / 2, 680, "Designed with purpose.", 140, a2 * out1 * 0.5, "c", r2)
    la = smooth(prog(t, 5.0, 6.8))
    draw_logo(img, d, W / 2, 640, la, 560)
    sa, sr = reveal(t, 6.4, 1.1)
    pill_white(d, W / 2 - text_width(SITE, f["semi"](24)) / 2 - 28, 760 + sr * 0.4, SITE, sa, 60, 24)
    base = layer_of(img)
    # the T slab returns above the wordmark, catching one last sweep of light
    tile_a = smooth(prog(t, 4.8, 6.4))
    lum, al = tile_lum("T", lerp(-1.5, 1.5, in_out(prog(t, 5.0, 8.5))))
    arr = np.zeros((H, W), np.float32)
    blit_tile(arr, lum, al, W / 2, 330 + 5 * math.sin(t * 1.1), 280, a=tile_a, tilt=0.35 * math.sin(t * 0.7), reflect=0.0)
    layer = silver(base, 100, 700, 0.9) + arr
    end_fade = 1 - smooth(prog(t, dur - 1.4, dur))
    return layer * end_fade, (W / 2, 560, 0.65 * smooth(prog(t, 0, 1))), 1.0


SCENE_FN = {"hook": scene_hook, "logo": scene_logo, "hero": scene_hero, "statement": scene_statement,
            "shield": scene_shield, "vpn": scene_vpn, "osint": scene_osint, "vm": scene_vm,
            "unified": scene_unified, "end": scene_end}

XX = np.arange(W, dtype=np.float32)[None, :]
YY = np.arange(H, dtype=np.float32)[:, None]
VIG = (1 - 0.35 * (((XX - W / 2) / (W / 2)) ** 2 + ((YY - H / 2) / (H / 2)) ** 2) * 0.5).astype(np.float32)
MONOCHROME = True
HUES = {"shield": 0.58, "vpn": 0.40, "osint": 0.075, "vm": 0.77}
ORDER = ["shield", "vpn", "osint", "vm"]
_X1 = np.arange(W, dtype=np.float32)
_Y1 = np.arange(H, dtype=np.float32)


def hsv(h, s=0.75, v=1.0):
    return np.array(colorsys.hsv_to_rgb(h % 1.0, s, v), np.float32)


def grad(h0, span, drift=0.0, s=0.7):
    """Horizontal hue sweep as a (1, W, 3) colour row."""
    cols = np.array([colorsys.hsv_to_rgb((h0 + span * x / W + drift) % 1.0, s, 1.0)
                     for x in range(0, W, 8)], np.float32)
    row = np.stack([np.interp(_X1, np.arange(0, W, 8), cols[:, k]) for k in range(3)], 1)
    return row[None, :, :]


def band(a, b, soft=14):
    return np.clip(np.minimum((_Y1 - a) / soft, (b - _Y1) / soft), 0, 1)[:, None]


def vband(a, b, soft=14):
    return np.clip(np.minimum((_X1 - a) / soft, (b - _X1) / soft), 0, 1)[None, :]


def orb(cx, cy, rx, ry, col, k):
    g = np.exp(-(((XX - cx) / rx) ** 2 + ((YY - cy) / ry) ** 2)).astype(np.float32)
    return g[..., None] * (col * k)


def plan(name, lt, dur):
    """-> (accent rgb, colour field or None, mix mask or None, ambient light)."""
    amb = 0
    if name == "hook":
        acc = hsv(0.02 + 0.04 * math.sin(lt * 2), 0.85)
        flash = max([math.exp(-(lt - b) * 5) for b in (0.25, 1.0, 1.8, 3.1, 3.7) if lt > b] + [0])
        amb = orb(W / 2, 540, 900, 520, acc, 0.06 + 0.2 * flash)
        col = grad(0.0, 0.16, lt * 0.1, 0.8)
        mix = band(540, 700) * 0.95 * smooth(prog(lt, 3.6, 4.0))
        return acc, col, mix, amb
    if name == "logo":
        acc = hsv(0.66 + 0.08 * math.sin(lt), 0.6)
        k = smooth(prog(lt, 2.9, 4.0)) * 0.22
        amb = orb(W / 2, 540, 900, 380, acc, k) + orb(W / 2 + 300, 600, 600, 300, hsv(0.78), k * 0.5)
        return acc, None, None, amb
    if name == "hero":
        acc = hsv(0.62, 0.6)
        col = grad(0.55 + 0.08 * lt, 0.4, 0, 0.65)
        mix = band(520, 670) * 0.92
        amb = orb(W / 2, 560, 1000, 420, acc, 0.12)
        return acc, col, mix, amb
    if name in HUES:
        h = HUES[name]
        acc = hsv(h, 0.75)
        sway = math.sin(lt * 0.8)
        amb = (orb(960 + 30 * sway, 612, 700, 460, acc, 0.15) +
               orb(960, 1000, 800, 160, acc, 0.06))
        return acc, None, None, amb
    if name == "unified":
        acc = hsv(0.6, 0.6)
        for i in range(4):
            r = smooth(prog(lt, 1.6 + 0.4 * i, 2.8 + 0.4 * i))
            amb = amb + orb(W / 2 + (i - 1.5) * 400, 620, 260, 240, acc, 0.11 * r)
        return acc, None, None, amb
    if name == "end":
        acc = hsv(0.66 + 0.06 * math.sin(lt), 0.6)
        col = grad(0.5 + 0.1 * lt, 0.45, 0, 0.7)
        mix = band(575, 700) * 0.92 * (1 - smooth(prog(lt, 4.0, 4.9)))
        amb = orb(W / 2, 540, 900, 440, acc, 0.12 + 0.08 * smooth(prog(lt, 5.0, 7.0)))
        return acc, col, mix, amb
    return TINT, None, None, 0


def render_frame(n):
    fonts()
    t = n / FPS
    name, start, dur = next(s for s in reversed(SCENES) if t >= s[1] - 1e-9)
    k = SPEED[name]
    lt = (t - start) * k
    dur = dur * k
    layer, spot, alpha = SCENE_FN[name](lt, dur)
    layer = layer.astype(np.float32)
    z = 1.0 + 0.05 * clamp(lt / dur)
    layer = np.asarray(Image.fromarray(layer, "F").transform(
        (W, H), Image.AFFINE, (1 / z, 0, W / 2 * (1 - 1 / z), 0, 1 / z, H / 2 * (1 - 1 / z)),
        resample=Image.BILINEAR), np.float32)
    small = Image.fromarray((np.clip(layer, 0, 1) * 255).astype(np.uint8)).resize((W // 4, H // 4), Image.BILINEAR)
    glow = np.asarray(small.filter(ImageFilter.GaussianBlur(5)).resize((W, H), Image.BILINEAR), np.float32) / 255
    wide = np.asarray(small.filter(ImageFilter.GaussianBlur(16)).resize((W, H), Image.BILINEAR), np.float32) / 255
    hi = Image.fromarray((np.clip(layer - 0.55, 0, 1) * 600).clip(0, 255).astype(np.uint8)).resize((W // 4, H // 4), Image.BILINEAR)
    streak = np.asarray(hi.resize((W // 40, H // 4), Image.BILINEAR).resize((W, H), Image.BILINEAR), np.float32) / 255
    acc, col, mix, amb = plan(name, lt, dur)
    if MONOCHROME:
        # brand rule: black, white and silver only. Keep the animated light as silver.
        acc, col, mix = TINT, None, None
        if not isinstance(amb, int):
            amb = amb.mean(axis=2, keepdims=True) * TINT * 0.8
    cx, cy, st = spot
    spotv = st * alpha * 0.045 * np.exp(-(((XX - cx) / 760) ** 2 + ((YY - cy) / 470) ** 2))
    lift = 0.02 * alpha * (smooth(prog(lt, 3.0, 5.0)) if name == "logo" else 1.0)
    if col is not None:
        cmix = TINT * (1 - mix[..., None]) + col * mix[..., None]
    else:
        cmix = TINT
    halo_col = TINT * 0.35 + acc * 0.65
    halo = (glow * 0.04 + wide * 0.07 + streak * 0.10) * alpha
    rgb = (layer * alpha)[..., None] * cmix
    rgb = rgb + halo[..., None] * halo_col
    rgb = rgb + spotv[..., None] * (TINT * 0.5 + acc * 0.5) + lift * TINT
    if not isinstance(amb, int):
        rgb = rgb + amb * alpha
    rgb = rgb * VIG[..., None]
    rng = np.random.default_rng(n)
    grain = rng.standard_normal((H, W)).astype(np.float32) * (1.1 / 255)
    gate = np.clip(rgb.max(axis=2) * 60, 0, 1)
    rgb = np.clip(rgb + (grain * gate)[..., None], 0, 1)
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
