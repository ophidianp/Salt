// Accent picker: re-tints the page the way the launcher's accent carries into
// the game, including the in-game screenshots. The choice is remembered for
// this browser only.
(() => {
  const root = document.documentElement;
  const swatches = [...document.querySelectorAll(".swatch")];
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)").matches;

  // Dark text on light accents, light text on dark ones.
  const inkFor = (hex) => {
    const n = parseInt(hex.slice(1), 16);
    const lum = (0.2126 * (n >> 16) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255;
    return lum > 0.55 ? "#06101f" : "#ffffff";
  };

  // --- colour helpers (all channels 0..1) --------------------------------------
  const toHsl = (r, g, b) => {
    const max = Math.max(r, g, b), min = Math.min(r, g, b), l = (max + min) / 2;
    if (max === min) return [0, 0, l];
    const d = max - min;
    const s = l > 0.5 ? d / (2 - max - min) : d / (max + min);
    let h = max === r ? (g - b) / d + (g < b ? 6 : 0) : max === g ? (b - r) / d + 2 : (r - g) / d + 4;
    return [h / 6, s, l];
  };
  const hue2 = (p, q, t) => {
    if (t < 0) t += 1;
    if (t > 1) t -= 1;
    if (t < 1 / 6) return p + (q - p) * 6 * t;
    if (t < 1 / 2) return q;
    if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6;
    return p;
  };
  const fromHsl = (h, s, l) => {
    if (s === 0) return [l, l, l];
    const q = l < 0.5 ? l * (1 + s) : l + s - l * s, p = 2 * l - q;
    return [hue2(p, q, h + 1 / 3), hue2(p, q, h), hue2(p, q, h - 1 / 3)];
  };
  const hexHsl = (hex) => {
    const n = parseInt(hex.slice(1), 16);
    return toHsl((n >> 16) / 255, ((n >> 8) & 255) / 255, (n & 255) / 255);
  };

  // --- screenshots ---------------------------------------------------------------
  // The shots were taken with a pink accent (hue ~330°). Every pixel in that
  // hue band is the accent (the world's reds and oranges sit at 0-30° and
  // 350°+), so swapping its hue repaints the accent and leaves the world alone.
  const SOURCE = hexHsl("#d386ae");
  const HUE_MIN = 312 / 360, HUE_MAX = 346 / 360, SAT_MIN = 0.1;

  const shots = [...document.querySelectorAll("img[data-recolor]")].map((img) => ({ img, original: img.currentSrc || img.src, mask: null, url: null }));

  const prepare = (shot) => {
    const { img } = shot;
    const canvas = document.createElement("canvas");
    canvas.width = img.naturalWidth;
    canvas.height = img.naturalHeight;
    const ctx = canvas.getContext("2d", { willReadFrequently: true });
    ctx.drawImage(img, 0, 0);
    const data = ctx.getImageData(0, 0, canvas.width, canvas.height);
    const px = data.data;
    const idx = [], sat = [], lig = [];
    for (let i = 0; i < px.length; i += 4) {
      const [h, s, l] = toHsl(px[i] / 255, px[i + 1] / 255, px[i + 2] / 255);
      if (h >= HUE_MIN && h <= HUE_MAX && s > SAT_MIN) {
        idx.push(i);
        sat.push(s);
        lig.push(l);
      }
    }
    shot.mask = { canvas, ctx, data, idx: Int32Array.from(idx), sat: Float32Array.from(sat), lig: Float32Array.from(lig) };
  };

  const recolor = (shot, hex) => {
    if (!shot.mask) return;
    const { canvas, ctx, data, idx, sat, lig } = shot.mask;
    const [th, ts] = hexHsl(hex);
    const satScale = ts / SOURCE[1];
    const px = data.data;
    for (let k = 0; k < idx.length; k++) {
      const [r, g, b] = fromHsl(th, Math.min(1, sat[k] * satScale), lig[k]);
      const i = idx[k];
      px[i] = r * 255;
      px[i + 1] = g * 255;
      px[i + 2] = b * 255;
    }
    ctx.putImageData(data, 0, 0);
    canvas.toBlob((blob) => {
      if (!blob) return;
      if (shot.url) URL.revokeObjectURL(shot.url);
      shot.url = URL.createObjectURL(blob);
      shot.img.src = shot.url;
    }, "image/webp", 0.9);
  };

  let current = "#7cc8ff";
  const paintShots = () => {
    for (const shot of shots) {
      if (shot.mask) {
        recolor(shot, current);
        continue;
      }
      const go = () => {
        if (shot.mask) return;
        try { prepare(shot); } catch { return; } // a tainted canvas just keeps the original
        recolor(shot, current);
      };
      if (shot.img.complete && shot.img.naturalWidth) go();
      else shot.img.addEventListener("load", go, { once: true });
    }
  };

  // --- accent ---------------------------------------------------------------------
  const apply = (hex) => {
    current = hex;
    root.style.setProperty("--accent", hex);
    root.style.setProperty("--accent-ink", inkFor(hex));
    for (const s of swatches) s.setAttribute("aria-checked", String(s.dataset.accent === hex));
    paintShots();
  };

  let saved = null;
  try { saved = localStorage.getItem("salt-accent"); } catch {}
  apply(saved && swatches.some((s) => s.dataset.accent === saved) ? saved : current);

  for (const s of swatches) {
    s.addEventListener("click", () => {
      apply(s.dataset.accent);
      try { localStorage.setItem("salt-accent", s.dataset.accent); } catch {}
    });
  }

  // --- FAQ: animate answers open and closed -------------------------------------------
  for (const details of document.querySelectorAll(".faq details")) {
    const summary = details.querySelector("summary");
    const body = details.querySelector(".answer");
    let anim = null;

    summary.addEventListener("click", (e) => {
      if (reduceMotion || !body) return;
      e.preventDefault();
      const opening = !details.open || details.classList.contains("closing");
      if (anim) anim.cancel();
      details.classList.remove("closing");

      if (opening) {
        details.open = true;
        const h = body.scrollHeight;
        anim = body.animate(
          [{ height: "0px", opacity: 0, transform: "translateY(-6px)" }, { height: h + "px", opacity: 1, transform: "none" }],
          { duration: 380, easing: "cubic-bezier(0.16, 1, 0.3, 1)" }
        );
      } else {
        details.classList.add("closing");
        const h = body.offsetHeight;
        anim = body.animate(
          [{ height: h + "px", opacity: 1, transform: "none" }, { height: "0px", opacity: 0, transform: "translateY(-6px)" }],
          { duration: 260, easing: "cubic-bezier(0.65, 0, 0.35, 1)" }
        );
        anim.onfinish = () => {
          details.open = false;
          details.classList.remove("closing");
        };
      }
    });
  }
})();

// Liquid glass from the Argon app (liquid-glass.js): refraction with a red/blue fringe on
// the nav, ghost buttons, feature cards and FAQ (Chromium), a gliding lens on the nav links.
if (window.LiquidGlass) {
  LiquidGlass.init({ selectors: ".pill.ghost, .feature, .faq", strength: 50, lens: [".nav-links"], lensItems: "a", max: 16 });
  document.querySelector(".faq")?.classList.add("lg-strong");
} else {
  document.documentElement.classList.add("no-lens");
}
