// saltClient site behaviour: theme toggle, solid header after scroll, nav highlight,
// the pinned in-game stage, the FAQ slide, and the first-visit preloader.
(() => {
  const $ = (id) => document.getElementById(id);
  const root = document.documentElement;

  /* theme toggle, remembered per viewer (the head script applies it before paint) */
  const themeBtn = $("theme");
  if (themeBtn) themeBtn.addEventListener("click", () => {
    const dark = root.dataset.theme ? root.dataset.theme === "dark" : matchMedia("(prefers-color-scheme: dark)").matches;
    const next = dark ? "light" : "dark";
    const go = () => { root.dataset.theme = next; };
    document.startViewTransition ? document.startViewTransition(go) : go();
    try { localStorage.setItem("salt-theme", next); } catch (e) {}
  });

  /* pinned in-game stage: steps through the screenshots as you scroll */
  const sec = $("in-game");
  const steps = [...document.querySelectorAll("#steps li")];
  const layers = [...document.querySelectorAll("#tstage .layer")];
  const n = steps.length;
  let cur = -1;
  const setStep = (i) => {
    if (i === cur) return; cur = i;
    steps.forEach((s, k) => s.classList.toggle("on", k === i));
    layers.forEach((l, k) => l.classList.toggle("on", k === i));
    if ($("tlbl")) $("tlbl").textContent = steps[i].dataset.lbl;
    if ($("tcap")) $("tcap").textContent = steps[i].dataset.cap;
  };
  steps.forEach((s, i) => s.addEventListener("click", () => {
    if (innerWidth <= 900) { setStep(i); return; }
    const max = sec.offsetHeight - innerHeight;
    scrollTo({ top: sec.offsetTop + max * (i + 0.5) / n, behavior: "smooth" });
  }));
  const onTear = () => {
    if (!sec || !n) return;
    if (innerWidth <= 900) { if (cur < 0) setStep(0); $("bar").style.width = ((cur + 1) / n * 100) + "%"; return; }
    const r = sec.getBoundingClientRect(), max = sec.offsetHeight - innerHeight;
    const p = Math.min(1, Math.max(0, -r.top / max));
    $("bar").style.width = (p * 100) + "%";
    setStep(Math.min(n - 1, Math.floor(p * n)));
  };

  /* header: solid after scroll, always visible; nav marks the section in view */
  const hdr = $("hdr");
  const navs = [...document.querySelectorAll(".nav a[data-nav]")];
  const targets = navs.map((a) => $(a.dataset.nav)).filter(Boolean);
  const onScroll = () => {
    hdr.classList.toggle("solid", scrollY > 10);
    let on = "";
    for (const t of targets) if (t.getBoundingClientRect().top < innerHeight * 0.4) on = t.id;
    navs.forEach((a) => a.classList.toggle("on", a.dataset.nav === on));
    onTear();
  };
  addEventListener("scroll", onScroll, { passive: true });
  addEventListener("resize", onScroll);
  onScroll();

  /* FAQ: slide each answer open and closed instead of snapping */
  document.querySelectorAll(".faq details").forEach((d) => {
    const ans = d.querySelector(":scope > :not(summary)");
    let anim = null;
    d.querySelector("summary").addEventListener("click", (e) => {
      if (!ans || matchMedia("(prefers-reduced-motion: reduce)").matches) return;
      e.preventDefault();
      const opening = !d.open || d.classList.contains("closing");
      const from = d.open ? ans.offsetHeight : 0;
      if (anim) anim.cancel();
      d.open = true;
      d.classList.toggle("closing", !opening);
      const to = opening ? ans.scrollHeight : 0;
      anim = ans.animate([{ height: from + "px", opacity: opening ? 0 : 1 }, { height: to + "px", opacity: opening ? 1 : 0 }],
        { duration: opening ? 340 : 260, easing: "cubic-bezier(0.2, 0.7, 0.2, 1)" });
      anim.onfinish = () => { anim = null; if (!opening) { d.open = false; d.classList.remove("closing"); } };
    });
  });

  /* first-visit preloader: the logo fills in, then the page fades up */
  if (root.classList.contains("preload")) {
    let done = false;
    const finish = () => { if (done) return; done = true; root.classList.add("pre-done"); setTimeout(() => root.classList.remove("preload", "pre-done"), 650); };
    const go = () => setTimeout(finish, Math.max(0, 1350 - performance.now()));
    document.readyState === "complete" ? go() : addEventListener("load", go);
    setTimeout(finish, 4000);
  }
})();
