// CSS-3D skin viewer. Every body part is a box of six divs whose
// backgrounds are crops of the skin texture (image-rendering: pixelated), so
// it's crisp at any size without WebGL or external libraries.
import { h } from "./ui.js";

// [u, v, w, h, d] for each part (64×64 skin layout).
const PARTS = (slim) => {
  const aw = slim ? 3 : 4;
  return {
    head: { box: [0, 0, 8, 8, 8], over: [32, 0, 8, 8, 8], pos: [4, 0], origin: "center bottom" },
    body: { box: [16, 16, 8, 12, 4], over: [16, 32, 8, 12, 4], pos: [4, 8] },
    "arm-r": { box: [40, 16, aw, 12, 4], over: [40, 32, aw, 12, 4], pos: [4 - aw, 8], origin: "center 2px" },
    "arm-l": { box: [32, 48, aw, 12, 4], over: [48, 48, aw, 12, 4], legacy: [40, 16, aw, 12, 4], pos: [12, 8], origin: "center 2px" },
    "leg-r": { box: [0, 16, 4, 12, 4], over: [0, 32, 4, 12, 4], pos: [4, 20], origin: "center top" },
    "leg-l": { box: [16, 48, 4, 12, 4], over: [0, 48, 4, 12, 4], legacy: [0, 16, 4, 12, 4], pos: [8, 20], origin: "center top" },
  };
};

function box(url, texH, [u, v, w, hh, d], S, inflate = 0) {
  const W = w * S, H = hh * S, D = d * S;
  const k = 1 + inflate;
  const el = h("div.part");
  el.style.width = W + "px";
  el.style.height = H + "px";
  if (inflate) el.style.transform = `scale3d(${k},${k},${k})`;
  const face = (fw, fh, fu, fv, transform, left = 0, top = 0) => {
    const f = h("div.face");
    Object.assign(f.style, {
      width: fw * S + "px",
      height: fh * S + "px",
      left: left + "px",
      top: top + "px",
      backgroundImage: `url("${url}")`,
      backgroundSize: `${64 * S}px ${texH * S}px`,
      backgroundPosition: `${-fu * S}px ${-fv * S}px`,
      transform,
    });
    el.appendChild(f);
  };
  face(w, hh, u + d, v + d, `translateZ(${D / 2}px)`); // front
  face(w, hh, u + d + w + d, v + d, `rotateY(180deg) translateZ(${D / 2}px)`); // back
  face(d, hh, u, v + d, `rotateY(-90deg) translateZ(${W / 2}px)`, (W - D) / 2); // right
  face(d, hh, u + d + w, v + d, `rotateY(90deg) translateZ(${W / 2}px)`, (W - D) / 2); // left
  face(w, d, u + d, v, `rotateX(90deg) translateZ(${H / 2}px)`, 0, (H - D) / 2); // top
  face(w, d, u + d + w, v, `rotateX(-90deg) translateZ(${H / 2}px) scaleY(-1)`, 0, (H - D) / 2); // bottom
  return el;
}

/**
 * Builds a viewer into `stage`. Returns { set(skinUrl, {slim, cape}), destroy() }.
 * Drag to rotate; idles with a slow sway + walk cycle.
 */
export function skinViewer(stage, { scale = 13, walk = true } = {}) {
  let rotY = -28, rotX = -8, dragging = false, lastX = 0, lastY = 0, raf = 0, idle = 0;
  const model = h("div.model" + (walk ? ".walk" : ""));
  stage.appendChild(model);

  const apply = () => { model.style.transform = `rotateX(${rotX}deg) rotateY(${rotY}deg)`; };
  const tick = () => {
    if (!dragging) { idle += 0.008; rotY += Math.sin(idle) * 0.12; apply(); }
    raf = requestAnimationFrame(tick);
  };
  const down = (e) => { dragging = true; lastX = e.clientX; lastY = e.clientY; };
  const move = (e) => {
    if (!dragging) return;
    rotY += (e.clientX - lastX) * 0.6;
    rotX = Math.max(-40, Math.min(40, rotX - (e.clientY - lastY) * 0.3));
    lastX = e.clientX; lastY = e.clientY; apply();
  };
  const up = () => (dragging = false);
  stage.addEventListener("pointerdown", down);
  addEventListener("pointermove", move);
  addEventListener("pointerup", up);
  raf = requestAnimationFrame(tick);

  async function set(url, { slim = false, cape = null } = {}) {
    model.innerHTML = "";
    if (!url) return;
    const texH = await new Promise((res) => { const i = new Image(); i.onload = () => res(i.height === 32 ? 32 : 64); i.onerror = () => res(64); i.src = url; });
    const S = scale;
    model.style.width = 16 * S + "px";
    model.style.height = 32 * S + "px";
    for (const [name, p] of Object.entries(PARTS(slim))) {
      const uv = texH === 32 && p.legacy ? p.legacy : p.box;
      const group = h("div.part." + name);
      const [x, y] = p.pos;
      const base = `translate3d(${x * S}px, ${y * S}px, 0)`;
      group.style.setProperty("--base", base);
      group.style.transform = base;
      group.style.transformOrigin = p.origin || "center";
      group.appendChild(box(url, texH, uv, S));
      if (texH === 64 || name === "head") {
        const o = box(url, texH, p.over, S, name === "head" ? 0.12 : 0.06);
        o.classList.add("overlay");
        group.appendChild(o);
      }
      model.appendChild(group);
    }
    if (cape) {
      // Cape texture: 64×32, outer face at (1,1) 10×16.
      const c = h("div.cape-plane");
      Object.assign(c.style, {
        width: 10 * S + "px", height: 16 * S + "px",
        transform: `translate3d(${3 * S}px, ${8 * S}px, ${-2.6 * S}px) rotateX(-8deg) rotateY(180deg)`,
        backgroundImage: `url("${cape}")`, backgroundSize: `${64 * S}px ${32 * S}px`,
        backgroundPosition: `${-1 * S}px ${-1 * S}px`, imageRendering: "pixelated",
      });
      model.appendChild(c);
    }
    apply();
  }

  return {
    set,
    destroy() {
      cancelAnimationFrame(raf);
      removeEventListener("pointermove", move);
      removeEventListener("pointerup", up);
    },
  };
}

/** Flat front view of a skin, for library tiles. */
export function skinFront(dataUrl, { slim = false } = {}) {
  const c = h("canvas", { width: 16, height: 32 });
  const img = new Image();
  img.onload = () => {
    const x = c.getContext("2d");
    x.imageSmoothingEnabled = false;
    const legacy = img.height === 32;
    const aw = slim ? 3 : 4;
    const d = (sx, sy, w, hh, dx, dy) => x.drawImage(img, sx, sy, w, hh, dx, dy, w, hh);
    d(8, 8, 8, 8, 4, 0); d(40, 8, 8, 8, 4, 0); // head + hat
    d(20, 20, 8, 12, 4, 8); // body
    d(44, 20, aw, 12, 4 - aw, 8); // right arm
    legacy ? d(44, 20, aw, 12, 12, 8) : d(36, 52, aw, 12, 12, 8); // left arm
    d(4, 20, 4, 12, 4, 20); // right leg
    legacy ? d(4, 20, 4, 12, 8, 20) : d(20, 52, 4, 12, 8, 20); // left leg
    if (!legacy) { d(20, 36, 8, 12, 4, 8); d(44, 36, aw, 12, 4 - aw, 8); d(52, 52, aw, 12, 12, 8); d(4, 36, 4, 12, 4, 20); d(4, 52, 4, 12, 8, 20); }
  };
  img.src = dataUrl;
  return c;
}
