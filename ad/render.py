#!/usr/bin/env python3
"""Thomsen Shield — 52.8 s commercial.  Procedural compositor built on the two
real screen recordings (website + application).  Deterministic: every frame is a
pure function of its index, so the render can be chunked across cores.

    python3 render.py --scale 1 --out out/shield-1080p.mp4        # 1920x1080
    python3 render.py --scale 2 --out out/shield-4k.mp4           # 3840x2160
    python3 render.py --stills 3,10.5,20 --scale 1                # QA PNGs
"""
import argparse, functools, math, os, subprocess, sys
from multiprocessing import Pool

import numpy as np
from PIL import Image, ImageDraw, ImageFilter, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
SRC_APP = os.path.join(HERE, "source", "app-recording.mp4")
SRC_WEB = os.path.join(HERE, "source", "website-recording.mp4")
FONT_DIR = "/usr/share/fonts/opentype/inter/"

FPS = 30
DUR = 52.8                      # 22 bars @ 100 BPM (bar = 2.4 s)
NF = int(round(DUR * FPS))

# app window inside the 1920x1080 desktop capture (excludes wallpaper/taskbar)
APP_CROP = (1280, 840, 328, 97)
# website: the "Everything in one app." plate (below the floating nav pill)
WEB_CROP = (1060, 750, 430, 55)

S = 1
W = H = 0
G = {}                          # per-process precomputed arrays / caches


# ----------------------------------------------------------------- helpers
def clamp(x, a=0.0, b=1.0):
    return a if x < a else b if x > b else x

def sstep(x):
    x = clamp(x); return x * x * (3 - 2 * x)

def eo_expo(x):
    x = clamp(x); return 1.0 if x >= 1 else 1 - 2 ** (-10 * x)

def eo_cubic(x):
    x = clamp(x); return 1 - (1 - x) ** 3

def ei_cubic(x):
    x = clamp(x); return x ** 3

def eio_cubic(x):
    x = clamp(x); return 4 * x ** 3 if x < .5 else 1 - (-2 * x + 2) ** 3 / 2

def eio_sine(x):
    x = clamp(x); return .5 - .5 * math.cos(math.pi * x)

def lerp(a, b, t):
    return a + (b - a) * t


def init(scale):
    global S, W, H
    S = scale; W = 1920 * S; H = 1080 * S
    yy = (np.arange(H, dtype=np.float32) / H)[:, None]
    xx = ((np.arange(W, dtype=np.float32) - W / 2) / H)[None, :]
    wide = np.exp(-(xx / (0.05 + 0.60 * yy)) ** 2 * 1.3) * (1 - yy) ** 1.6
    core = np.exp(-(xx / (0.015 + 0.16 * yy)) ** 2 * 2.0) * (1 - yy) ** 2.2
    nx = (np.arange(W, dtype=np.float32) - W / 2) / (W / 2)
    ny = (np.arange(H, dtype=np.float32) - H / 2) / (H / 2)
    vig = np.clip(1 - 0.30 * (nx[None, :] ** 2 + ny[:, None] ** 2), 0, 1)
    G["beam"] = ((0.62 * wide + 0.55 * core) * vig).astype(np.float32)
    G["seq"] = {}
    # rounded-rect masks / rim light for the two plates
    G["plate"] = {
        "app": make_plate(APP_CROP[0], APP_CROP[1], 10),
        "web": make_plate(WEB_CROP[0], WEB_CROP[1], 14),
    }
    # nodes (network field)
    rng = np.random.default_rng(11)
    n = 120
    P = np.stack([rng.uniform(-1.8, 1.8, n), rng.uniform(-1.0, 1.0, n),
                  rng.uniform(0.5, 3.4, n)], 1)
    P[0] = [0.42, -0.07, 1.35]                      # the one that matters
    d = np.linalg.norm(P[:, None] - P[None], axis=2)
    G["nodes"] = P
    G["pairs"] = [(i, j) for i in range(n) for j in range(i + 1, n) if d[i, j] < 0.60]


def make_plate(w, h, radius):
    ss = 4
    m = Image.new("L", (w * ss, h * ss), 0)
    ImageDraw.Draw(m).rounded_rectangle([ss * 0.5, ss * 0.5, w * ss - ss * .5, h * ss - ss * .5],
                                        radius=radius * ss, fill=255)
    mask = m.resize((w, h), Image.LANCZOS)
    inner = Image.new("L", (w * ss, h * ss), 0)
    ImageDraw.Draw(inner).rounded_rectangle([ss * 2.0, ss * 2.0, w * ss - ss * 2.0, h * ss - ss * 2.0],
                                            radius=(radius - 1.5) * ss, fill=255)
    inner = inner.resize((w, h), Image.LANCZOS)
    ring = np.clip(np.asarray(mask, np.float32) - np.asarray(inner, np.float32), 0, 255) / 255.0
    grad = np.linspace(0.34, 0.07, h, dtype=np.float32)[:, None]
    return mask, (ring * grad).astype(np.float32)


# ----------------------------------------------------------------- footage
class Seq:
    def __init__(self, path, t0, t1, crop):
        w, h, x, y = crop
        cmd = ["ffmpeg", "-v", "error", "-ss", f"{t0:.3f}", "-t", f"{t1 - t0 + 0.1:.3f}", "-i", path,
               "-vf", f"crop={w}:{h}:{x}:{y}", "-f", "rawvideo", "-pix_fmt", "rgb24", "-"]
        raw = subprocess.run(cmd, capture_output=True, check=True).stdout
        n = len(raw) // (w * h * 3)
        self.fr = np.frombuffer(raw, np.uint8)[: n * w * h * 3].reshape(n, h, w, 3)
        self.t0 = t0; self.n = n

    def get(self, t, nearest=False):
        f = (t - self.t0) * FPS
        f = clamp(f, 0, self.n - 1)
        i = int(math.floor(f + (0.5 if nearest else 0))); a = 0 if nearest else f - i; j = min(i + 1, self.n - 1)
        if a < 0.03 or i == j:
            return self.fr[i].astype(np.float32)
        return self.fr[i] * (1 - a) + self.fr[j] * a


def seq(name, path, t0, t1, crop):
    cache = G["seq"]
    if name not in cache:
        while len(cache) >= 3:
            cache.pop(next(iter(cache)))
        cache[name] = Seq(path, t0, t1, crop)
    return cache[name]


# ----------------------------------------------------------------- text
@functools.lru_cache(maxsize=None)
def font(name, px):
    return ImageFont.truetype(FONT_DIR + name + ".otf", int(round(px)))

@functools.lru_cache(maxsize=2048)
def text_mask(text, fname, px, track):
    """Anti-aliased coverage mask (float32) with per-pair kerning + tracking."""
    ss = 2
    f = font(fname, px * ss)
    xs, x = [], 0.0
    for i, ch in enumerate(text):
        xs.append(x)
        adv = f.getlength(ch)
        if i + 1 < len(text):
            adv = f.getlength(text[i:i + 2]) - f.getlength(text[i + 1])
        x += adv + track * px * ss
    width = x - track * px * ss
    asc, desc = f.getmetrics()
    pad = int(px * ss * 0.35)
    im = Image.new("L", (int(width) + 2 * pad, asc + desc + 2 * pad), 0)
    d = ImageDraw.Draw(im)
    for ch, xx in zip(text, xs):
        d.text((pad + xx, pad), ch, font=f, fill=255)
    im = im.resize((im.width // ss, im.height // ss), Image.LANCZOS)
    return np.asarray(im, np.float32) / 255.0


def blit(cv, m, x, y, col, alpha=1.0, extra=None):
    h, w = m.shape
    x0, y0 = max(0, x), max(0, y); x1, y1 = min(W, x + w), min(H, y + h)
    if x1 <= x0 or y1 <= y0 or alpha <= 0:
        return
    sub = m[y0 - y:y1 - y, x0 - x:x1 - x] * alpha
    reg = cv[y0:y1, x0:x1]
    c = np.asarray(col, np.float32)
    reg += (c - reg) * sub[..., None]
    if extra is not None:
        reg += (extra[y0 - y:y1 - y, x0 - x:x1 - x] * sub)[..., None]


def blur_mask(m, r):
    if r < 0.4:
        return m
    im = Image.fromarray((np.clip(m, 0, 1) * 255).astype(np.uint8))
    return np.asarray(im.filter(ImageFilter.GaussianBlur(r)), np.float32) / 255.0


def text_line(cv, parts, fname, size, cx, cy, a=1.0, dy=0.0, bl=0.0, track=0.0,
              align="center", sweep=None):
    """parts = [(text, gray)] drawn on one baseline. size in 1080p units."""
    if a <= 0.002:
        return
    px = size * S
    ms = []
    for t, g in parts:
        ms.append((text_mask(t, fname, px, round(track, 3)), g))
    space = font(fname, px).getlength(" ") + track * px
    pad = int(px * 0.35)
    # widths without padding
    wid = [m.shape[1] - 2 * pad for m, _ in ms]
    total = sum(wid) + space * (len(ms) - 1)
    x = cx * S - total / 2 if align == "center" else cx * S
    for (m, g), w in zip(ms, wid):
        mm = blur_mask(m, bl * S) if bl > 0 else m
        extra = None
        if sweep is not None:                     # sweep = (pos 0..1, strength)
            pos, strength = sweep
            colv = (np.arange(mm.shape[1], dtype=np.float32) - pad) / max(1, w)
            prof = np.exp(-((colv - pos) / 0.11) ** 2) * strength
            extra = np.broadcast_to(prof[None, :], mm.shape)
        blit(cv, mm, int(round(x - pad)), int(round(cy * S + dy - m.shape[0] / 2)), g, a, extra)
        x += w + space


def anim(t, t_in, t_out=None, din=0.9, dout=0.5, rise=14.0, blur=9.0):
    e = eo_expo((t - t_in) / din)
    a = eo_cubic((t - t_in) / (din * 0.9)); dy = rise * (1 - e); b = blur * (1 - e)
    if t_out is not None and t > t_out:
        e2 = eio_sine((t - t_out) / dout)
        a *= 1 - e2; dy -= 8 * e2; b += blur * 0.7 * e2
    return a, dy * S, b


# ----------------------------------------------------------------- plates
def plate_rgba(kind, frame_rgb):
    mask, rim = G["plate"][kind]
    arr = frame_rgb.astype(np.float32)
    arr += (255 - arr) * rim[..., None]
    out = np.empty(arr.shape[:2] + (4,), np.uint8)
    out[..., :3] = np.clip(arr, 0, 255).astype(np.uint8)
    out[..., 3] = np.asarray(mask)
    return Image.fromarray(out, "RGBA")


def place_plate(cv, kind, frame_rgb, k, fx, fy, px, py, alpha=1.0, blur=0.0, sharpen=True):
    """Draw plate so that source point (fx,fy) lands on screen (px,py) at scale k."""
    if alpha <= 0.002:
        return
    k = k * S; px = px * S; py = py * S
    src = plate_rgba(kind, frame_rgb)
    sh = src.height
    data = (1 / k, 0, fx - px / k, 0, 1 / k, fy - py / k)
    L = src.transform((W, H), Image.AFFINE, data, resample=Image.BICUBIC)
    if sharpen and k > 1.25 * S:
        rgb = L.convert("RGB").filter(ImageFilter.UnsharpMask(radius=0.9 * S, percent=45, threshold=0))
        L = Image.merge("RGBA", (*rgb.split(), L.getchannel("A")))
    if blur > 0.3:
        L = L.filter(ImageFilter.GaussianBlur(blur * S))
    arr = np.asarray(L, np.float32) / 255.0
    a = arr[..., 3] * alpha
    # soft shadow (computed at 1/8 res)
    sm = Image.fromarray((a * 255).astype(np.uint8)).resize((W // 8, H // 8), Image.BILINEAR)
    sm = sm.filter(ImageFilter.GaussianBlur(max(2.0, 26 * S / 8)))
    sa = np.asarray(sm, np.float32) / 255.0
    sa = np.roll(sa, int(34 * S / 8), axis=0)
    sa = np.asarray(Image.fromarray((sa * 255).astype(np.uint8)).resize((W, H), Image.BILINEAR), np.float32) / 255.0
    cv *= (1 - 0.75 * sa)[..., None]
    # reflection on the "floor"
    yb = py + (sh - fy) * k
    if 0 < yb < H * 0.93 and alpha > 0.05:
        ry0 = int(yb); rows = min(H - ry0, int(0.30 * H))
        d2 = (1 / k, 0, fx - px / k, 0, -1 / k, sh + (yb - ry0) / k)
        R = src.transform((W, rows), Image.AFFINE, d2, resample=Image.BICUBIC)
        R = R.filter(ImageFilter.GaussianBlur(2.2 * S))
        ra = np.asarray(R, np.float32) / 255.0
        fade = (np.clip(1 - np.arange(rows, dtype=np.float32) / rows, 0, 1) ** 2.2)[:, None]
        ral = ra[..., 3] * fade * 0.13 * alpha
        reg = cv[ry0:ry0 + rows]
        reg += (ra[..., :3] - reg) * ral[..., None]
    cv += (arr[..., :3] - cv) * a[..., None]


# ----------------------------------------------------------------- light
def beam_level(t):
    """Intensity of the overhead light beam over time."""
    v = 0.0
    v += 0.10 * sstep((t - 2.4) / 3.0) + 0.14 * sstep((t - 5.5) / 4.0)           # slow build
    imp = lambda t0, amp, tau: amp * math.exp(-(t - t0) / tau) if t >= t0 else 0.0
    if t < 9.6:
        pass
    else:
        v = 0.0
        v += 0.55 * sstep((t - 9.6) / 0.5) + imp(9.6, 0.9, 0.55)
    if t >= 12.0:
        v = 0.50 + 0.1 * sstep((t - 12.0) / 2.0)
    if t >= 36.0:
        v = lerp(0.60, 0.18, sstep((t - 39.8) / 1.2))
    if t >= 40.8:
        v = 0.18 + 0.10 * sstep((t - 42.5) / 2.0)
    if t >= 45.6:
        v = 0.40 + imp(45.6, 0.85, 0.7)
    return v


def draw_hairline(cv, t):
    if not (0.5 < t < 4.0):
        return
    L = eo_expo((t - 0.8) / 1.7) * 0.19 * W
    inten = sstep((t - 0.7) / 0.6) * (1 - sstep((t - 2.6) / 1.2))
    if inten <= 0 or L < 2:
        return
    cy = H // 2; band = int(70 * S)
    ys = np.arange(cy - band, cy + band, dtype=np.float32)[:, None] - cy + 0.5
    xs = np.arange(W, dtype=np.float32)[None, :] - W / 2
    along = 1 - np.clip((np.abs(xs) - (L * 0.55)) / (L * 0.45 + 1), 0, 1)
    along = along ** 1.5 * (np.abs(xs) <= L)
    core = np.exp(-(ys / (0.9 * S)) ** 2) * along
    glow = np.exp(-(ys / (16 * S)) ** 2) * along * 0.10
    cv[cy - band:cy + band] += ((core * 0.95 + glow) * inten)[..., None]


def draw_nodes(cv, t):
    if not (2.4 < t < 9.7):
        return
    P = G["nodes"]; pairs = G["pairs"]
    vis = sstep((t - 2.5) / 2.0) * 0.8
    contract = eio_cubic((t - 8.5) / 1.1)
    vis *= 1 - 0.65 * contract
    if vis <= 0.01:
        return
    th = 0.11 * t - 0.45
    c, s = math.cos(th), math.sin(th)
    x = P[:, 0] * c + (P[:, 2] - 1.9) * s
    z = -P[:, 0] * s + (P[:, 2] - 1.9) * c + 1.9 - 0.035 * t
    y = P[:, 1]
    f = 0.95 * H
    z = np.maximum(z, 0.12)
    sx = W / 2 + f * x / z; sy = H / 2 + f * y / z
    # converge on the centre before the reveal
    sx = W / 2 + (sx - W / 2) * (1 - 0.9 * contract)
    sy = H / 2 + (sy - H / 2) * (1 - 0.9 * contract)
    depth = np.clip(1.25 - z / 3.6, 0.08, 1.0) * (lambda q: q * q * (3 - 2 * q))(np.clip((z - 0.30) / 0.35, 0, 1))
    ss = 2
    cd = Image.new("L", (W * ss, H * ss), 0); dd = ImageDraw.Draw(cd)
    cl = Image.new("L", (W * ss, H * ss), 0); dl = ImageDraw.Draw(cl)
    for i, j in pairs:
        if depth[i] < 0.02 or depth[j] < 0.02:
            continue
        v = int(255 * 0.30 * (depth[i] + depth[j]) / 2)
        if v > 3:
            dl.line([sx[i] * ss, sy[i] * ss, sx[j] * ss, sy[j] * ss], fill=v, width=max(1, int(S * ss * 0.9)))
    for i in range(len(P)):
        if depth[i] < 0.02 or not (-20 < sx[i] < W + 20 and -20 < sy[i] < H + 20):
            continue
        r = max(1.1, 2.3 * S * ss / (z[i] ** 0.85)) * (1.9 if i == 0 else 1.0)
        v = int(255 * min(1.0, depth[i] * (1.5 if i == 0 else 1.0)))
        dd.ellipse([sx[i] * ss - r, sy[i] * ss - r, sx[i] * ss + r, sy[i] * ss + r], fill=v)
    # detection rings around node 0
    for t0 in (6.5, 7.35, 8.2):
        if t0 < t < t0 + 1.5:
            p = (t - t0) / 1.5
            r = (0.012 + 0.16 * eo_cubic(p)) * H * ss
            val = int(255 * 0.8 * (1 - p) ** 1.6)
            dl.ellipse([sx[0] * ss - r, sy[0] * ss - r, sx[0] * ss + r, sy[0] * ss + r],
                       outline=val, width=max(1, int(S * ss)))
    comb = np.maximum(np.asarray(cd.reduce(ss), np.float32), np.asarray(cl.reduce(ss), np.float32)) / 255.0
    glow_src = cd.reduce(8)
    glow = np.asarray(glow_src.filter(ImageFilter.GaussianBlur(2.2)).resize((W, H), Image.BILINEAR), np.float32) / 255.0
    g = comb * vis + glow * 0.55 * vis
    # pulse the hero node's glow while it is "detected"
    cv += g[..., None]


# ----------------------------------------------------------------- captions
CAPS = {
    "dash":  ("DASHBOARD", "Everything, at a glance.",
              "Protection status, live counters and a running graph of your traffic."),
    "live":  ("LIVE CONNECTIONS", "See every connection.",
              "Every active connection on your PC — with the app that owns it."),
    "prot":  ("PROTECTION", "Spot what doesn’t belong.",
              "Connection spikes, port scans and SYN floods — detected as they happen."),
    "dns":   ("DNS PROTECTION", "Stop threats at the lookup.",
              "Known malware and command-and-control domains, blocked locally."),
    "allow": ("ALLOWLIST", "Your network, trusted by default.",
              "Add your own trusted addresses whenever you like."),
    "logs":  ("EVENTS & LOGS", "A clear record of everything.",
              "Every connection Thomsen Shield has acted on — and why."),
}


def caption(cv, key, t, g0, g1):
    eb, hl, sub = CAPS[key]
    out = g1 - 0.55
    # scrim
    a_s = sstep((t - g0 - 0.2) / 0.9) * (1 - sstep((t - out) / 0.5))
    if a_s > 0:
        ramp = np.clip((np.arange(H, dtype=np.float32) / H - 0.64) / 0.17, 0, 1)
        ramp = ramp * ramp * (3 - 2 * ramp)
        cv *= (1 - 0.965 * a_s * ramp)[:, None, None]
    X = 150
    a, dy, b = anim(t, g0 + 0.45, out, 0.9, 0.45, rise=10, blur=6)
    text_line(cv, [(eb, 0.60)], "Inter-SemiBold", 16, X, 819, a, dy, b, track=0.22, align="left")
    a, dy, b = anim(t, g0 + 0.60, out, 1.0, 0.45)
    text_line(cv, [(hl, 0.97)], "InterDisplay-SemiBold", 58, X, 884, a, dy, b, track=-0.018, align="left")
    a, dy, b = anim(t, g0 + 0.95, out, 1.0, 0.45, rise=10, blur=6)
    text_line(cv, [(sub, 0.68)], "Inter-Regular", 23, X, 950, a, dy, b, track=0.0, align="left")


# ----------------------------------------------------------------- scenes
APP_SHOTS = {
    # key: (global start, global end, src start, src end, camera function)
    "dash":  (16.4, 21.6, 16.0, 19.6),
    "live":  (21.6, 24.0, 19.85, 20.5),
    "prot":  (24.0, 28.8, 21.0, 23.7),
    "dnsA":  (28.8, 30.9, 24.3, 25.8),
    "dnsB":  (30.6, 33.6, 31.0, 33.0),
    "allow": (33.6, 36.0, 37.5, 38.2),
    "logs":  (36.0, 40.8, 39.0, 41.8),
}
XF = 0.45   # cross-dissolve length between app shots


def cam_for(key, t):
    """(k, fx, fy) for each shot — slow, deliberate pushes."""
    g0, g1 = APP_SHOTS[key][:2]
    u = clamp((t - g0) / (g1 - g0))
    e = eio_sine(u)
    if key == "dash":
        arrive = eo_expo((t - 16.45) / 1.5)
        k = lerp(1.14, 1.27, e) * (1 + 0.62 * (1 - arrive))
        return k, 640, 400
    if key == "live":
        return lerp(1.24, 1.34, u), 640, lerp(300, 320, u)
    if key == "prot":
        return lerp(1.24, 1.34, u), 640, lerp(300, 335, u)
    if key in ("dnsA", "dnsB"):
        uu = clamp((t - 28.8) / (33.6 - 28.8))
        return lerp(1.30, 1.40, eio_sine(uu)), 640, lerp(440, 470, uu)
    if key == "allow":
        return lerp(1.24, 1.34, u), 640, lerp(300, 315, u)
    if key == "logs":
        return lerp(1.24, 1.36, e), 640, lerp(300, 335, u)


def app_alpha(key, t):
    g0, g1 = APP_SHOTS[key][:2]
    a = 1.0
    if key != "dash":
        a *= sstep((t - (g0 - XF * 0.0)) / XF) if key not in ("dnsB",) else sstep((t - g0) / 0.35)
    else:
        a *= sstep((t - 16.35) / 0.8)
    if key == "logs":
        a *= 1 - sstep((t - 40.0) / 0.8)
    return a


def draw_app(cv, t):
    if not (16.3 <= t <= 40.9):
        return
    order = ["dash", "live", "prot", "dnsA", "dnsB", "allow", "logs"]
    for key in order:
        g0, g1, s0, s1 = APP_SHOTS[key]
        last = key in ("logs",)
        end = g1 + XF if not last else g1
        if key == "dnsA":
            end = 30.95
        if not (g0 - 0.01 <= t <= end):
            continue
        sq = seq("app_" + key, SRC_APP, s0, s1 + 0.1, APP_CROP)
        u = (t - g0) / (g1 - g0)
        st = lerp(s0, s1, clamp(u))
        fr = sq.get(st)
        k, fx, fy = cam_for(key, t)
        a = app_alpha(key, t)
        if key in ("dash",) and t > 16.3:
            pass
        # a previous shot stays solid underneath while the next one dissolves in
        a_draw = a
        if key == "dnsA" and t > 30.6:
            a_draw = 1.0
        if key not in ("dash",) and key != "dnsB" and t > g1:
            a_draw = 1.0
        blur = 0.0
        if key == "dash":
            blur = 7.0 * (1 - eo_expo((t - 16.45) / 1.2))
        # camera moves a touch through the exit
        if key == "logs" and t > 40.0:
            k *= 1 + 0.03 * sstep((t - 40.0) / 0.8)
        place_plate(cv, "app", fr, k, fx, fy, W / 2 / S, (0.47 if key == "dash" else 0.40) * H / S, a_draw, blur)


def draw_web(cv, t):
    if not (11.9 <= t <= 16.9):
        return
    # real scroll-in (website recording) then a held frame
    sq = seq("web", SRC_WEB, 13.2, 13.9, WEB_CROP)
    st = clamp(13.2 + (t - 12.45), 13.2, 13.77)
    fr = sq.get(st, nearest=True)
    rise = (1 - eo_expo((t - 12.0) / 1.3)) * 70
    appear = sstep((t - 12.0) / 0.9)
    zoom_in = eio_cubic((t - 15.15) / 1.3)
    k_base = lerp(1.22, 1.36, sstep((t - 12.0) / 3.4))
    k = k_base * math.exp(math.log(5.0) * zoom_in)
    # focus slides from plate centre onto the Dashboard card
    fx = lerp(530, 214, zoom_in); fy = lerp(360, 330, zoom_in)
    alpha = appear * (1 - sstep((t - 15.95) / 0.8))
    blur = 8.0 * zoom_in ** 2
    place_plate(cv, "web", fr, k, fx, fy, W / 2 / S, (H / 2) / S + rise, alpha, blur)


def draw_texts(cv, t):
    # ---- Act I / II (kinetic type over the network field)
    lines = [("Every", "app.", 2.4, 3.55), ("Every", "connection.", 3.6, 4.75),
             ("Every", "second.", 4.8, 6.5)]
    for dim, noun, t_in, t_out in lines:
        if t_in - 0.1 < t < t_out + 1:
            a, dy, b = anim(t, t_in, t_out, 1.0, 0.5)
            text_line(cv, [(dim, 0.42), (noun, 0.97)], "InterDisplay-SemiBold", 84, 960, 540, a, dy, b, track=-0.02)
    if 6.9 < t < 9.4:
        a, dy, b = anim(t, 7.0, 8.7, 1.1, 0.6)
        text_line(cv, [("Most of it goes", 0.55), ("unseen.", 0.97)], "InterDisplay-SemiBold", 84, 960, 540, a, dy, b, track=-0.02)

    # ---- reveal
    if 9.6 <= t < 13.2:
        e = eo_expo((t - 9.75) / 1.9)
        a = eo_cubic((t - 9.75) / 1.1)
        out = sstep((t - 12.0) / 0.9)
        a *= 1 - out
        bl = 14 * (1 - e) + 8 * out
        trk = lerp(0.16, -0.035, e)
        sw = ((t - 10.3) / 1.9, 0.20 * (1 - sstep((t - 12.0) / 0.8))) if 10.3 < t < 12.3 else None
        text_line(cv, [("Thomsen Shield", 0.88)], "InterDisplay-ExtraBold", 150, 960, 478 - out * 14, a, 0, bl, track=trk, sweep=sw)
        a2, dy2, b2 = anim(t, 10.9, 11.95, 1.0, 0.7, rise=10, blur=6)
        text_line(cv, [("See every connection. Stop the ones you do not trust.", 0.66)], "Inter-Regular", 27,
                  960, 590, a2, dy2, b2, track=0.0)

    # ---- tagline + finale
    if 40.7 <= t < 45.9:
        a, dy, b = anim(t, 41.0, 44.9, 1.2, 0.6)
        text_line(cv, [("Quiet on your system.", 0.97)], "InterDisplay-SemiBold", 92, 960, 500, a, dy, b, track=-0.02)
        a, dy, b = anim(t, 42.3, 44.9, 1.2, 0.6)
        text_line(cv, [("Clear in your results.", 0.55)], "InterDisplay-SemiBold", 92, 960, 612, a, dy, b, track=-0.02)
    if t >= 45.6:
        e = eo_expo((t - 45.7) / 2.0)
        a = eo_cubic((t - 45.7) / 1.0)
        trk = lerp(0.10, -0.035, e)
        bl = 12 * (1 - e)
        sw = ((t - 46.2) / 2.0, 0.20 * (1 - sstep((t - 48.0) / 1.0))) if 46.2 < t < 48.6 else None
        text_line(cv, [("Thomsen Shield", 0.92)], "InterDisplay-ExtraBold", 166, 960, 448, a, 0, bl, track=trk, sweep=sw)
        a2, dy2, b2 = anim(t, 47.0, None, 1.1, rise=10, blur=6)
        text_line(cv, [("Network protection for Windows 10 and 11.", 0.62)], "Inter-Regular", 29, 960, 556, a2, dy2, b2)
        # CTA
        a3, dy3, b3 = anim(t, 48.2, None, 1.1, rise=8, blur=5)
        ln = eo_expo((t - 48.0) / 1.2) * 90 * S
        if ln > 1:
            x0 = int(W / 2 - ln); x1 = int(W / 2 + ln); yy = int(700 * S)
            cv[yy:yy + max(1, S), x0:x1] += 0.38
        text_line(cv, [("thomsen-software.com", 0.97)], "Inter-Medium", 38, 960, 752, a3, dy3, b3, track=0.01)
        a4, dy4, b4 = anim(t, 49.0, None, 1.0, rise=6, blur=4)
        text_line(cv, [("THOMSEN SOFTWARE", 0.46)], "Inter-SemiBold", 15, 960, 985, a4, 0, 0, track=0.34)


def draw_shot_captions(cv, t):
    spans = [("dash", 16.4, 21.6), ("live", 21.6, 24.0), ("prot", 24.0, 28.8),
             ("dns", 28.8, 33.6), ("allow", 33.6, 36.0), ("logs", 36.0, 40.8)]
    for key, g0, g1 in spans:
        if g0 - 0.01 <= t <= g1 + 0.02:
            caption(cv, key, t, g0 + (0.5 if key == "dash" else 0), g1)


# ----------------------------------------------------------------- frame
def bloom(cv):
    im = Image.fromarray((np.clip(cv, 0, 1) * 255).astype(np.uint8))
    small = im.reduce(4).filter(ImageFilter.GaussianBlur(5))
    b = np.asarray(small.resize((W, H), Image.BILINEAR), np.float32) / 255.0
    cv += np.clip(b - 0.30, 0, 1) * 0.16


def render_frame(n):
    t = n / FPS
    cv = np.zeros((H, W, 3), np.float32)
    lvl = beam_level(t)
    if lvl > 0:
        cv += (G["beam"] * (0.30 * lvl))[..., None]
    draw_hairline(cv, t)
    draw_nodes(cv, t)
    draw_web(cv, t)
    draw_app(cv, t)
    draw_shot_captions(cv, t)
    draw_texts(cv, t)
    bloom(cv)
    # grade: gentle toe, neutral
    cv = np.clip(cv, 0, 1)
    cv = cv * cv * (3 - 2 * cv) * 0.18 + cv * 0.82
    rng = np.random.default_rng(n)
    noise = rng.standard_normal((H // 2, W // 2), dtype=np.float32)
    noise = np.asarray(Image.fromarray(noise).resize((W, H), Image.BILINEAR))
    lum = cv.mean(axis=2)
    cv += (noise * (0.004 + 0.010 * lum))[..., None]
    cv += (rng.random((H, W, 1), dtype=np.float32) - 0.5) * (1.6 / 255)
    fade = 1 - sstep((t - (DUR - 1.4)) / 1.3)
    cv *= fade
    return (np.clip(cv, 0, 1) * 255 + 0.5).astype(np.uint8)


# ----------------------------------------------------------------- driver
def encode_chunk(args):
    scale, i0, i1, path, crf = args
    if not W:
        init(scale)
    cmd = ["ffmpeg", "-v", "error", "-y", "-f", "rawvideo", "-pix_fmt", "rgb24", "-s", f"{W}x{H}",
           "-r", str(FPS), "-i", "-",
           "-vf", "scale=out_color_matrix=bt709:out_range=tv,format=yuv420p",
           "-c:v", "libx264", "-preset", "slow", "-crf", str(crf), "-g", "60", "-bf", "3",
           "-colorspace", "bt709", "-color_primaries", "bt709", "-color_trc", "bt709", "-an", path]
    p = subprocess.Popen(cmd, stdin=subprocess.PIPE)
    for n in range(i0, i1):
        p.stdin.write(render_frame(n).tobytes())
    p.stdin.close(); p.wait()
    return path


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--scale", type=int, default=1)
    ap.add_argument("--out", default=os.path.join(HERE, "out", "silent.mp4"))
    ap.add_argument("--workers", type=int, default=4)
    ap.add_argument("--crf", type=int, default=14)
    ap.add_argument("--stills", default="")
    ap.add_argument("--range", default="")
    a = ap.parse_args()
    init(a.scale)
    if a.stills:
        os.makedirs(os.path.join(HERE, "out", "stills"), exist_ok=True)
        for ts in a.stills.split(","):
            t = float(ts)
            Image.fromarray(render_frame(int(round(t * FPS)))).save(
                os.path.join(HERE, "out", "stills", f"t{t:06.2f}.png"))
            print("still", t)
        return
    lo, hi = (0, NF)
    if a.range:
        lo, hi = map(int, a.range.split(":"))
    tmp = a.out + ".parts"; os.makedirs(tmp, exist_ok=True)
    step = 60
    jobs = [(a.scale, i, min(i + step, hi), os.path.join(tmp, f"p{i:05d}.mp4"), a.crf)
            for i in range(lo, hi, step)]
    with Pool(a.workers) as pool:
        for k, path in enumerate(pool.imap(encode_chunk, jobs)):
            print(f"chunk {k + 1}/{len(jobs)} {os.path.basename(path)}", flush=True)
    lst = os.path.join(tmp, "list.txt")
    with open(lst, "w") as f:
        for j in jobs:
            f.write(f"file '{os.path.abspath(j[3])}'\n")
    subprocess.run(["ffmpeg", "-v", "error", "-y", "-f", "concat", "-safe", "0", "-i", lst, "-c", "copy", a.out], check=True)
    print("wrote", a.out)


if __name__ == "__main__":
    main()
