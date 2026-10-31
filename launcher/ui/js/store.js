// Global app state + change notifications. Views subscribe with
// store.on(key, fn) and get an unsubscribe function back.
import { api, listen } from "./api.js";

const subs = new Map();

export const store = {
  settings: null,
  accounts: { accounts: [], active: null },
  instances: [],
  running: new Set(),
  preparing: new Set(),
  versions: null,
  gameLogs: new Map(), // instance id -> [{level, thread, time, message}]
  launcherLogs: [],
  social: { state: "offline", me: null, friends: [], incoming: [], outgoing: [], chats: new Map(), unread: new Map(), invites: [] },
  skins: new Map(), // uuid -> data URL

  on(key, fn) {
    if (!subs.has(key)) subs.set(key, new Set());
    subs.get(key).add(fn);
    return () => subs.get(key).delete(fn);
  },
  emit(key, data) {
    subs.get(key)?.forEach((fn) => { try { fn(data); } catch (e) { console.error(e); } });
  },

  activeAccount() {
    return this.accounts.accounts.find((a) => a.uuid === this.accounts.active) || null;
  },

  async refreshInstances() {
    this.instances = await api.instances();
    this.emit("instances", this.instances);
  },
  async refreshAccounts() {
    this.accounts = await api.accounts();
    this.emit("accounts", this.accounts);
  },
  async saveSettings(patch) {
    this.settings = { ...this.settings, ...patch };
    await api.saveSettings(this.settings);
    this.emit("settings", this.settings);
  },
  async loadVersions() {
    if (!this.versions) this.versions = await api.versions();
    return this.versions;
  },
};

const MAX_GAME_LINES = 8000;
const MAX_LAUNCHER_LINES = 3000;

export async function initStore() {
  const [settings, accounts, instances, running, logs, info] = await Promise.all([api.settings(), api.accounts(), api.instances(), api.running(), api.launcherLogs(), api.info()]);
  Object.assign(store, { settings, accounts, instances, launcherLogs: logs, info });
  store.running = new Set(running);

  listen("salt://log", (line) => {
    store.launcherLogs.push(line);
    if (store.launcherLogs.length > MAX_LAUNCHER_LINES) store.launcherLogs.splice(0, store.launcherLogs.length - MAX_LAUNCHER_LINES);
    store.emit("launcher-log", line);
  });

  listen("salt://game-log", ({ id, lines }) => {
    const buf = store.gameLogs.get(id) || [];
    buf.push(...lines);
    if (buf.length > MAX_GAME_LINES) buf.splice(0, buf.length - MAX_GAME_LINES);
    store.gameLogs.set(id, buf);
    store.emit("game-log", { id, lines });
  });

  listen("salt://game", (ev) => {
    const { id, state } = ev;
    if (state === "preparing") { store.preparing.add(id); store.gameLogs.set(id, []); }
    if (state === "running") { store.preparing.delete(id); store.running.add(id); }
    if (state === "exited" || state === "error") { store.preparing.delete(id); store.running.delete(id); store.refreshInstances(); }
    store.emit("game", ev);
  });

  listen("salt://progress", (p) => store.emit("progress", p));
  listen("salt://login", (r) => { store.refreshAccounts(); store.emit("login", r); });
  listen("salt://social", (msg) => handleSocial(msg));
}

function handleSocial(msg) {
  const s = store.social;
  switch (msg.t) {
    case "_connection":
      s.state = msg.state;
      s.error = msg.message || null;
      break;
    case "welcome":
      s.me = { uuid: msg.uuid, name: msg.name };
      break;
    case "friends":
      s.friends = msg.friends; s.incoming = msg.incoming; s.outgoing = msg.outgoing;
      break;
    case "presence": {
      const i = s.friends.findIndex((f) => f.uuid === msg.friend.uuid);
      if (i >= 0) s.friends[i] = msg.friend;
      break;
    }
    case "dm": {
      const other = msg.from === s.me?.uuid ? msg.to : msg.from;
      const list = s.chats.get(other) || [];
      list.push(msg);
      s.chats.set(other, list.slice(-300));
      if (msg.from !== s.me?.uuid) s.unread.set(other, (s.unread.get(other) || 0) + 1);
      break;
    }
    case "invite": {
      const list = s.chats.get(msg.from) || [];
      list.push({ ...msg, kind: "invite", ts: Date.now() });
      s.chats.set(msg.from, list);
      s.unread.set(msg.from, (s.unread.get(msg.from) || 0) + 1);
      break;
    }
    case "error":
      s.error = msg.message;
      break;
  }
  store.emit("social", msg);
}
