"""Writes the saltClient logo files from salt_logo.py: logo.svg (wordmark), favicon.svg
(the s on an ice-blue tile), apple-touch-icon.png and og.png, plus brand/logo.json with the
path data the pages inline. Run from anywhere: python -I brand/make_assets.py
(Adapted from the Mouse 33 brand script, which logo_gen.py also comes from.)"""
import os, sys, json
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from PIL import Image, ImageDraw
from shapely.geometry import MultiPolygon
from logo_gen import to_path
from salt_logo import build

HERE = os.path.dirname(os.path.abspath(__file__)); SITE = os.path.join(HERE, "..")
ACCENT, ACCENT_DEEP, INK, NIGHT = "#7cc8ff", "#1f6fb0", "#06101f", "#0b0d12"
word, (ww, wh) = build()
mark, (mw, mh) = build(mark_only=True)
D_WORD, D_MARK = to_path(word), to_path(mark)
pad = 0.6


def svg_word(fill=ACCENT_DEEP):
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="{-pad} {-pad} {ww+2*pad:.2f} {wh+2*pad:.2f}">'
            f'<title>saltClient</title><path fill="{fill}" d="{D_WORD}"/></svg>\n')


def svg_icon():
    s = 24 / max(mw, mh); ox, oy = (32 - mw*s)/2, (32 - mh*s)/2
    return (f'<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 32 32"><rect width="32" height="32" rx="8" fill="{ACCENT}"/>'
            f'<path fill="{INK}" transform="translate({ox:.2f} {oy:.2f}) scale({s:.4f})" d="{D_MARK}"/></svg>\n')


def raster(g, scale, offset, img, color, hole):
    d = ImageDraw.Draw(img)
    for p in (g.geoms if isinstance(g, MultiPolygon) else [g]):
        d.polygon([(offset[0] + x*scale, offset[1] + y*scale) for x, y in p.exterior.coords], fill=color)
        for ring in p.interiors:
            d.polygon([(offset[0] + x*scale, offset[1] + y*scale) for x, y in ring.coords], fill=hole)


def touch_icon(n=180, ss=4):
    N = n*ss; img = Image.new("RGB", (N, N), ACCENT)
    s = N*0.70 / max(mw, mh)
    raster(mark, s, ((N - mw*s)/2, (N - mh*s)/2), img, INK, ACCENT)
    return img.resize((n, n), Image.LANCZOS)


def recolor(img, target=ACCENT, source="#d386ae"):
    """the same hue swap accent.js does: the shots were taken with a pink accent, so every
    pixel in the 312-346 degree band is the accent; move it to the target hue."""
    import numpy as np, colorsys
    a = np.asarray(img).astype(np.float32) / 255
    r, g, b = a[..., 0], a[..., 1], a[..., 2]
    mx, mn = a.max(-1), a.min(-1); l = (mx + mn) / 2; d = mx - mn
    s = np.where(d == 0, 0, d / np.maximum(np.where(l > 0.5, 2 - mx - mn, mx + mn), 1e-9))
    h = np.where(mx == r, ((g - b) / (d + 1e-9)) % 6, np.where(mx == g, (b - r) / (d + 1e-9) + 2, (r - g) / (d + 1e-9) + 4)) / 6
    m = (h >= 312/360) & (h <= 346/360) & (s > 0.1) & (d > 0)
    th, tl, ts = colorsys.rgb_to_hls(*[int(target[i:i+2], 16)/255 for i in (1, 3, 5)])
    sh, sl, ss_ = colorsys.rgb_to_hls(*[int(source[i:i+2], 16)/255 for i in (1, 3, 5)])
    s2 = np.minimum(1, s * ts / ss_)
    q = np.where(l < 0.5, l * (1 + s2), l + s2 - l * s2); pp = 2 * l - q
    def hue2(t):
        t = t % 1
        return np.where(t < 1/6, pp + (q - pp) * 6 * t, np.where(t < 1/2, q, np.where(t < 2/3, pp + (q - pp) * (2/3 - t) * 6, pp)))
    out = a.copy()
    for i, off in enumerate((1/3, 0, -1/3)):
        out[..., i] = np.where(m, hue2(th + off), a[..., i])
    return Image.fromarray((out * 255).clip(0, 255).astype("uint8"))


def og(ss=2):
    W, H = 1200*ss, 630*ss; img = Image.new("RGB", (W, H), NIGHT)
    shot = Image.open(os.path.join(SITE, "assets", "img", "title-screen.webp")).convert("RGB")
    shot = recolor(shot)
    k = (H*0.74) / shot.height; shot = shot.resize((int(shot.width*k), int(shot.height*k)), Image.LANCZOS)
    mask = Image.new("L", shot.size, 0); ImageDraw.Draw(mask).rounded_rectangle((0, 0, *shot.size), radius=28*ss, fill=255)
    img.paste(shot, (W - int(shot.width*0.72), (H - shot.height)//2), mask)
    # fade the screenshot into the background on its left edge
    fade = Image.new("L", (W, H), 0); fd = ImageDraw.Draw(fade)
    x0 = W - int(shot.width*0.72)
    for i in range(int(260*ss)):
        fd.line([(x0 + i, 0), (x0 + i, H)], fill=int(255 * (1 - i/(260*ss))))
    fd.rectangle((0, 0, x0, H), fill=255)
    img = Image.composite(Image.new("RGB", (W, H), NIGHT), img, fade)
    s = 520*ss / ww
    raster(word, s, (72*ss, 215*ss), img, ACCENT, NIGHT)
    return img.resize((1200, 630), Image.LANCZOS)


if __name__ == "__main__":
    open(os.path.join(SITE, "logo.svg"), "w").write(svg_word())
    open(os.path.join(SITE, "favicon.svg"), "w").write(svg_icon())
    touch_icon().save(os.path.join(SITE, "apple-touch-icon.png"))
    og().save(os.path.join(SITE, "og.png"), optimize=True)
    parts = len(getattr(word, "geoms", [word]))
    json.dump({"word": D_WORD, "w": round(ww+2*pad, 2), "h": round(wh+2*pad, 2), "pad": pad, "mark": D_MARK, "parts": parts},
              open(os.path.join(HERE, "logo.json"), "w"))
    print("word", round(ww, 2), round(wh, 2), len(D_WORD), "chars, parts", parts, "; mark", round(mw, 2), round(mh, 2))
