"""saltClient logo: "salt" written as one connected piece with the same flat, horizontal pen
nib as the Mouse 33 wordmark (logo_gen.py): thick strokes on the climbs, hairlines on the
turns, swept along Bezier centrelines, then sheared forward 44 degrees."""
import os, sys
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
from shapely.ops import unary_union
from shapely import affinity
from logo_gen import stroke, to_path, H

W, T = 5.6, 2.2
TOP, BOT = T / 2, H - T / 2
X = 9.6  # x-height line


def line(a, b):
    """a straight segment as a Bezier"""
    return (a, (a[0] + (b[0] - a[0]) / 3, a[1] + (b[1] - a[1]) / 3),
            (a[0] + 2 * (b[0] - a[0]) / 3, a[1] + 2 * (b[1] - a[1]) / 3), b)


def build(skew=44.0, mark_only=False):
    parts = []
    # --- s: top bar from the right, down the left, spine across, bowl round to a baseline flick
    sx, sw = 0.0, 10.5
    s = [
        ((sx + sw + 1.0, X), (sx + sw * 0.45, X), (sx + 0.6, X), (sx + 0.6, X + 3.4)),
        ((sx + 0.6, X + 3.4), (sx + 0.6, X + 7.6), (sx + sw, X + 6.6), (sx + sw, BOT - 4.4)),
        ((sx + sw, BOT - 4.4), (sx + sw, BOT), (sx + sw * 0.45, BOT), (sx - 1.6, BOT)),
    ]
    parts.append(stroke(s, W, T))
    if mark_only:
        g = unary_union(parts)
        return finish(g, skew)
    # --- a: bowl hangs off the s's top bar; the stem sweeps down to the baseline and on into the l
    ax, aw = 19.2, 13.4
    bowl = [
        ((sx + sw * 0.7, X), (ax + 2.6, X), (ax, X + 2.4), (ax, X + 7.4)),
        ((ax, X + 7.4), (ax, BOT - 1.2), (ax + aw * 0.45, BOT), (ax + aw - 0.6, BOT - 5.8)),
    ]
    stem_a = [((ax + 2.6, X), (ax + aw * 0.7, X), (ax + aw - 1.2, X + 1.0), (ax + aw - 0.9, X + 4.0)),
              ((ax + aw - 0.9, X + 4.0), (ax + aw - 0.4, BOT - 6.0), (ax + aw + 0.6, BOT), (ax + aw + 4.6, BOT)),
              ((ax + aw + 4.6, BOT), (ax + aw + 6.0, BOT), (ax + aw + 6.6, BOT), (ax + aw + 7.6, BOT - 0.4))]
    parts += [stroke(bowl, W, T), stroke(stem_a, W, T)]
    # --- l: hairline lead-in at the top, a long sweep down, then out along the baseline into the t
    lx = ax + aw + 6.6
    l = [((lx - 4.0, TOP), (lx - 1.0, TOP), (lx - 0.4, TOP + 1.0), (lx - 0.2, TOP + 4.0)),
         ((lx - 0.2, TOP + 4.0), (lx + 0.4, BOT - 7.0), (lx + 1.2, BOT), (lx + 5.4, BOT)),
         ((lx + 5.4, BOT), (lx + 6.8, BOT), (lx + 7.4, BOT), (lx + 8.4, BOT - 0.4))]
    parts.append(stroke(l, W, T))
    # --- t: shorter stem, crossbar on the x-height reaching back to the l, tail out along the baseline
    tx = lx + 8.0
    t = [((tx - 0.6, 3.8), (tx, BOT - 7.0), (tx + 0.8, BOT), (tx + 5.4, BOT)),
         ((tx + 5.4, BOT), (tx + 7.4, BOT), (tx + 8.6, BOT - 0.6), (tx + 9.6, BOT - 1.6))]
    bar = [line((lx + 1.6, X), (tx + 6.4, X))]
    parts += [stroke(t, W, T), stroke(bar, W, T)]
    return finish(unary_union(parts), skew)


def finish(g, skew):
    g = affinity.skew(g, xs=-skew, origin=(0, H)).simplify(0.035)
    minx, miny, maxx, maxy = g.bounds
    g = affinity.translate(g, -minx, -miny)
    return g, (maxx - minx, maxy - miny)


def preview(path, g, size, scale=12, pad=20):
    from PIL import Image, ImageDraw
    from shapely.geometry import MultiPolygon
    w, h = size
    img = Image.new("RGB", (int(w * scale) + 2 * pad, int(h * scale) + 2 * pad), "white")
    d = ImageDraw.Draw(img)
    for p in (g.geoms if isinstance(g, MultiPolygon) else [g]):
        d.polygon([(pad + x * scale, pad + y * scale) for x, y in p.exterior.coords], fill="#0b0b0d")
        for r in p.interiors:
            d.polygon([(pad + x * scale, pad + y * scale) for x, y in r.coords], fill="white")
    img.save(path)


if __name__ == "__main__":
    out = sys.argv[1] if len(sys.argv) > 1 else "salt_preview.png"
    g, size = build()
    n = len(getattr(g, "geoms", [g]))
    print("parts", n, "size", [round(v, 2) for v in size], "holes", sum(len(p.interiors) for p in getattr(g, "geoms", [g])))
    preview(out, g, size)
