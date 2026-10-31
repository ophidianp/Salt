// Friends list + direct messages + world invites over the salt-social server.
import { api } from "../api.js";
import { icon } from "../icons.js";
import { store } from "../store.js";
import { h, btn, toast, promptDialog, headFor, confirmDialog, menu } from "../ui.js";
import { go } from "../app.js";

let selected = null;

export function render(root) {
  const s = store.social;
  const list = h("div.fl-scroll");
  const chat = h("div.chat.glass");
  const head = h("div.row", { style: { padding: "12px 12px 6px" } });

  const connectBox = () => {
    const box = h("div.empty");
    if (!store.activeAccount()) { box.append(h("h2", "Sign in first"), h("p", "Friends are tied to your Minecraft account.")); return box; }
    box.append(
      h("div", { html: icon("users") }),
      h("h2", s.state === "connecting" ? "Connecting…" : "Friends are offline"),
      h("p", s.error ? s.error : store.settings.social_enabled ? "Couldn't reach the social server." : "Turn on friends to chat and invite people to your worlds."),
      h("p.small.faint", "Server: " + store.settings.social_server),
      h("div.row", { style: { justifyContent: "center" } },
        btn(store.settings.social_enabled ? "Reconnect" : "Turn on friends", { cls: "primary", onclick: async () => {
          await store.saveSettings({ social_enabled: true });
          api.socialConnect().catch((e) => toast("Couldn't connect", String(e), "err"));
        } }),
        btn("Settings", { onclick: () => go("settings/social") }),
      ),
    );
    return box;
  };

  const friendRow = (f, extra) => {
    const row = h("div.friend" + (selected === f.uuid ? ".sel" : ""), h("canvas.head"), h("span.dot." + (f.status || "offline")), h("div.grow", h("div.li-title", f.name), h("div.li-sub", f.activity || f.status)), extra || null);
    const unread = s.unread.get(f.uuid);
    if (unread && !extra) row.appendChild(h("span.badge", unread));
    headFor(api, f.uuid, 34).then((c) => { c.className = "head"; row.firstChild.replaceWith(c); });
    return row;
  };

  const drawList = () => {
    list.innerHTML = "";
    if (s.incoming.length) {
      list.append(h("div.nav-section", "Requests"));
      for (const f of s.incoming) list.append(friendRow(f, h("div.row", { style: { gap: "4px" } },
        btn("", { cls: "icon sm primary", icon: "check", title: "Accept", onclick: (e) => { e.stopPropagation(); api.socialSend({ t: "friend_accept", uuid: f.uuid }); } }),
        btn("", { cls: "icon sm ghost", icon: "x", title: "Decline", onclick: (e) => { e.stopPropagation(); api.socialSend({ t: "friend_decline", uuid: f.uuid }); } }),
      )));
    }
    const order = { playing: 0, online: 1, away: 2, offline: 3 };
    const friends = [...s.friends].sort((a, b) => (order[a.status] ?? 3) - (order[b.status] ?? 3) || a.name.localeCompare(b.name));
    list.append(h("div.nav-section", `Friends — ${friends.filter((f) => f.status !== "offline").length}/${friends.length} online`));
    if (!friends.length) list.append(h("div.small.muted", { style: { padding: "8px 10px" } }, "No friends yet. Add someone by their Minecraft username."));
    for (const f of friends) {
      const row = friendRow(f);
      row.onclick = () => { selected = f.uuid; s.unread.delete(f.uuid); store.emit("social", { t: "_read" }); drawList(); drawChat(); };
      row.oncontextmenu = (e) => { e.preventDefault(); menu(row, [{ label: "Remove friend", icon: "trash", danger: true, onclick: async () => { if (await confirmDialog(`Remove ${f.name}?`, "You can add them again later.")) api.socialSend({ t: "friend_remove", uuid: f.uuid }); } }]); };
      list.appendChild(row);
    }
    if (s.outgoing.length) {
      list.append(h("div.nav-section", "Sent"));
      for (const f of s.outgoing) list.append(friendRow(f, h("span.small.faint", "pending")));
    }
  };

  const drawChat = () => {
    chat.innerHTML = "";
    const f = s.friends.find((x) => x.uuid === selected);
    if (!f) { chat.append(h("div.empty", { style: { margin: "auto" }, html: icon("message") }, h("div", "Pick a friend to chat."))); return; }
    const msgs = h("div.chat-msgs");
    for (const m of s.chats.get(f.uuid) || []) {
      if (m.kind === "invite") {
        msgs.append(h("div.msg.invite", h("b", `${m.from_name} invited you to play`), h("div.mono", m.address), h("div.small.muted", "Minecraft " + m.version), h("div.row", { style: { marginTop: "6px" } }, btn("Copy address", { cls: "sm primary", icon: "copy", onclick: () => navigator.clipboard.writeText(m.address).then(() => toast("Address copied", "Paste it in Multiplayer → Direct connect.", "ok")) }))));
        continue;
      }
      const me = m.from === s.me?.uuid;
      msgs.append(h("div.msg" + (me ? ".me" : ""), m.text, h("div.m-time", new Date(m.ts).toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" }))));
    }
    const input = h("input.input", { placeholder: `Message ${f.name}`, maxlength: 1000 });
    const send = () => {
      const text = input.value.trim();
      if (!text) return;
      api.socialSend({ t: "dm", to: f.uuid, text }).then(() => (input.value = "")).catch((e) => toast("Not sent", String(e), "err"));
    };
    input.addEventListener("keydown", (e) => e.key === "Enter" && send());
    const invite = async () => {
      const address = await promptDialog(`Invite ${f.name}`, "World address (from e4mc, or a server IP)", "", "something.e4mc.link");
      if (!address) return;
      const inst = store.instances.find((i) => store.running.has(i.id)) || store.instances[0];
      api.socialSend({ t: "invite", to: f.uuid, address, version: inst?.mc_version || "" }).then(() => toast("Invite sent", f.name, "ok"));
    };
    const headEl = h("canvas.head");
    chat.append(
      h("div.chat-head", headEl, h("div.grow", h("b", f.name), h("div.small.muted", f.activity || f.status)), btn("Invite to world", { cls: "sm", icon: "globe", onclick: invite })),
      msgs,
      h("div.chat-input", input, btn("", { cls: "icon primary", icon: "send", onclick: send })),
    );
    headFor(api, f.uuid, 34).then((c) => { c.style.borderRadius = "8px"; headEl.replaceWith(c); });
    msgs.scrollTop = msgs.scrollHeight;
    setTimeout(() => input.focus(), 50);
  };

  const draw = () => {
    root.innerHTML = "";
    root.append(h("div.page-head",
      h("div", h("h1", "Friends"), h("p", "Chat, see what friends are playing, and invite them to your worlds.")),
      s.state === "online" ? h("div.row", h("span.chip.ok", "● Online as " + (s.me?.name || "")), btn("Add friend", { cls: "primary", icon: "userPlus", onclick: async () => {
        const name = await promptDialog("Add a friend", "Their Minecraft username");
        if (name) api.socialSend({ t: "friend_request", name }).then(() => toast("Request sent", name, "ok")).catch((e) => toast("Couldn't send", String(e), "err"));
      } })) : null,
    ));
    if (s.state !== "online") { root.append(h("div.glass", connectBox())); return; }
    root.append(h("div.friends-layout", h("div.friends-list.glass", head, list), chat));
    drawList();
    drawChat();
  };
  draw();
  let lastState = s.state;
  const off = store.on("social", (msg) => {
    if (s.state !== lastState || msg.t === "welcome") { lastState = s.state; draw(); return; }
    if (msg.t === "error") toast("Friends", msg.message, "err");
    if (["friends", "presence"].includes(msg.t)) drawList();
    if (msg.t === "dm" || msg.t === "invite") { if (selected && (msg.from === selected || msg.to === selected)) { s.unread.delete(selected); drawChat(); } drawList(); }
  });
  return off;
}
