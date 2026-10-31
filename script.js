// Accent picker: re-tints the page the way the launcher's accent carries into
// the game. The choice is remembered for this browser only.
(() => {
  const root = document.documentElement;
  const swatches = [...document.querySelectorAll(".swatch")];

  // Dark text on light accents, light text on dark ones.
  const inkFor = (hex) => {
    const n = parseInt(hex.slice(1), 16);
    const lum = (0.2126 * (n >> 16) + 0.7152 * ((n >> 8) & 255) + 0.0722 * (n & 255)) / 255;
    return lum > 0.55 ? "#06101f" : "#ffffff";
  };

  const apply = (hex) => {
    root.style.setProperty("--accent", hex);
    root.style.setProperty("--accent-ink", inkFor(hex));
    for (const s of swatches) s.setAttribute("aria-checked", String(s.dataset.accent === hex));
  };

  let saved = null;
  try { saved = localStorage.getItem("salt-accent"); } catch {}
  if (saved && swatches.some((s) => s.dataset.accent === saved)) apply(saved);

  for (const s of swatches) {
    s.addEventListener("click", () => {
      apply(s.dataset.accent);
      try { localStorage.setItem("salt-accent", s.dataset.accent); } catch {}
    });
  }
})();
