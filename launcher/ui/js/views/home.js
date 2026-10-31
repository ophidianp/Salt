import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, fmtAgo, fmtDuration, headFor } from "../ui.js";
import { go, playButton } from "../app.js";
import { newInstanceDialog, instanceCard } from "./instances.js";

const TIPS = [
  ["layout", "Move your HUD", "In game, press Right Shift → saltHUD → Edit HUD layout to drag FPS, CPS, keystrokes and more."],
  ["globe", "Host a world", "Open a world, press Esc → Open to LAN. e4mc gives you a public address to send friends."],
  ["compass", "Grab modpacks", "Discover has every Modrinth and CurseForge pack. One click makes a new instance."],
  ["shirt", "Change your skin", "Upload a skin or switch capes in Skins & capes — it updates on every server."],
];

export function render(root) {
  const offs = [];
  const draw = () => {
    root.innerHTML = "";
    const a = store.activeAccount();
    const last = store.instances[0];
    const hour = new Date().getHours();
    const greet = hour < 5 ? "Up late" : hour < 12 ? "Good morning" : hour < 18 ? "Good afternoon" : "Good evening";

    const hero = h("section.hero.glass",
      h("img.crystal", { src: "img/crystal.png", alt: "" }),
      h("div.greet", `${greet}${a ? ", " + a.username : ""}`),
      h("h1", last ? last.name : "Let's play"),
      last
        ? h("div.hero-meta", h("span.chip.accent", last.mc_version), h("span.chip", last.loader === "fabric" ? "Fabric" : "Vanilla"), last.salt_bundle ? h("span.chip.salt", "Salt bundle") : null, h("span", "· " + fmtDuration(last.playtime_secs)), h("span", "· " + fmtAgo(last.last_played)))
        : h("div.hero-meta", "Create an instance to get started."),
      h("div.row", last ? playButton(last.id) : btn("New instance", { cls: "play", icon: "plus", onclick: () => newInstanceDialog() }), last ? btn("Open", { onclick: () => go("instance/" + last.id) }) : null),
    );
    root.appendChild(hero);

    const recent = store.instances.slice(0, 4);
    if (recent.length) {
      root.append(
        h("div.page-head", { style: { margin: "20px 4px 12px" } }, h("h2", { style: { margin: 0, fontSize: "17px" } }, "Jump back in"), btn("All instances", { cls: "ghost sm", onclick: () => go("instances") })),
        h("div.grid.cards.stagger", recent.map(instanceCard)),
      );
    }

    const friendsBox = h("div.section.glass", h("h2", "Friends online"));
    const online = store.social.friends.filter((f) => f.status !== "offline");
    if (store.social.state !== "online") {
      friendsBox.append(h("p.desc", "Connect to see who's playing."), btn("Open friends", { cls: "sm", icon: "users", onclick: () => go("friends") }));
    } else if (!online.length) {
      friendsBox.append(h("p.desc", "Nobody's online right now."));
    } else {
      const list = h("div.list");
      for (const f of online.slice(0, 6)) {
        const row = h("div.list-item", h("div.li-icon"), h("div.grow", h("div.li-title", f.name), h("div.li-sub", f.activity || f.status)));
        headFor(api, f.uuid, 38).then((c) => row.firstChild.replaceChildren(c));
        list.appendChild(row);
      }
      friendsBox.appendChild(list);
    }

    const tips = h("div.section.glass", h("h2", "Tips"), h("div.list", TIPS.map(([ico, t, d]) => h("div.list-item", h("div.li-icon", { html: icon(ico) }), h("div.grow", h("div.li-title", t), h("div.li-sub", { style: { whiteSpace: "normal" } }, d))))));
    root.appendChild(h("div.grid", { style: { gridTemplateColumns: "1fr 1fr", marginTop: "14px" } }, friendsBox, tips));
  };
  draw();
  offs.push(store.on("instances", draw), store.on("social", draw), store.on("accounts", draw));
  return () => offs.forEach((o) => o());
}
