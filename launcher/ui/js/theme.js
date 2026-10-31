// Applies Settings → Appearance to CSS variables, live.

export const ACCENTS = ["#63a7ff", "#7cc8ff", "#a78bfa", "#5ee6a0", "#ffb86b", "#ff7ab6", "#ff6b6b", "#ffd84d", "#4de1ff", "#e8edf5"];

export const BACKGROUNDS = {
  black: { label: "Black", css: "#000000" },
  charcoal: { label: "Charcoal", css: "#111214" },
  midnight: { label: "Midnight", css: "linear-gradient(180deg,#05070d,#0c1222)" },
  plum: { label: "Plum", css: "linear-gradient(180deg,#07050a,#1a1020)" },
};

function hexToRgb(hex) {
  const m = /^#?([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})/i.exec(hex || "");
  return m ? [parseInt(m[1], 16), parseInt(m[2], 16), parseInt(m[3], 16)] : [124, 200, 255];
}

export function applyTheme(t) {
  const root = document.documentElement;
  const mode = t.mode === "system" ? (matchMedia("(prefers-color-scheme: light)").matches ? "light" : "dark") : t.mode;
  root.dataset.theme = mode;
  const [r, g, b] = hexToRgb(t.accent);
  root.style.setProperty("--accent", t.accent);
  root.style.setProperty("--accent-rgb", `${r}, ${g}, ${b}`);
  // Dark text on light accents, light text on dark ones.
  root.style.setProperty("--accent-ink", 0.299 * r + 0.587 * g + 0.114 * b > 150 ? "#06121f" : "#ffffff");
  root.style.setProperty("--blur", `${t.blur}px`);
  root.style.setProperty("--glass-a", String(t.glass_opacity));
  root.style.setProperty("--radius", `${t.radius}px`);
  root.style.setProperty("--speed", String(t.animation_speed || 1));
  root.style.setProperty("--bg-dim", String(t.background_dim));
  root.classList.toggle("no-anim", !t.animations);
  root.classList.toggle("compact", !!t.compact);

  const img = document.getElementById("backdrop-image");
  // Unknown/old preset names fall back to black.
  const preset = BACKGROUNDS[t.background] || (t.background?.startsWith("data:image/") ? null : BACKGROUNDS.black);
  if (preset) {
    // Light mode always uses a plain light grey.
    document.body.style.background = mode === "light" ? "#e4e5e8" : preset.css;
    img.classList.remove("on");
  } else if (t.background?.startsWith("data:image/")) {
    img.style.backgroundImage = `url("${t.background}")`;
    img.classList.add("on");
  }
}
