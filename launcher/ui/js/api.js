// Bridge to the Rust side. `withGlobalTauri` exposes window.__TAURI__, so no
// npm packages or bundler are needed for the UI.
const T = window.__TAURI__;

export const invoke = (cmd, args = {}) => T.core.invoke(cmd, args);
export const listen = (event, fn) => T.event.listen(event, (e) => fn(e.payload));
export const appWindow = () => T.window.getCurrentWindow();

export const api = {
  // app
  settings: () => invoke("get_settings"),
  saveSettings: (settings) => invoke("save_settings", { settings }),
  setCurseforgeKey: (key) => invoke("set_curseforge_key", { key }),
  hasCurseforgeKey: () => invoke("has_curseforge_key"),
  info: () => invoke("app_info"),
  openUrl: (url) => invoke("open_url", { url }),
  openDataFolder: () => invoke("open_data_folder"),
  launcherLogs: () => invoke("launcher_logs"),
  // accounts
  accounts: () => invoke("list_accounts"),
  loginStart: () => invoke("login_start"),
  verifyStart: () => invoke("verify_start"),
  verifyCancel: () => invoke("verify_cancel"),
  setActiveAccount: (uuid) => invoke("set_active_account", { uuid }),
  removeAccount: (uuid) => invoke("remove_account", { uuid }),
  // versions
  versions: () => invoke("list_versions"),
  fabricLoaders: (mc) => invoke("list_fabric_loaders", { mc }),
  // instances
  instances: () => invoke("list_instances"),
  instance: (id) => invoke("get_instance", { id }),
  createInstance: (req) => invoke("create_instance", { req }),
  updateInstance: (inst) => invoke("update_instance", { inst }),
  deleteInstance: (id) => invoke("delete_instance", { id }),
  duplicateInstance: (id) => invoke("duplicate_instance", { id }),
  openInstanceFolder: (id, sub) => invoke("open_instance_folder", { id, sub }),
  worlds: (id) => invoke("list_worlds", { id }),
  // mods
  mods: (id) => invoke("list_mods", { id }),
  toggleMod: (id, fileName, enabled) => invoke("toggle_mod", { id, fileName, enabled }),
  removeMod: (id, fileName) => invoke("remove_mod", { id, fileName }),
  addLocalMods: (id) => invoke("add_local_mods", { id }),
  checkUpdates: (id) => invoke("check_updates", { id }),
  bundle: () => invoke("bundle_listing"),
  // discover
  mrSearch: (query) => invoke("modrinth_search", { query }),
  mrProject: (id) => invoke("modrinth_project", { id }),
  mrVersions: (id, gameVersion, loader) => invoke("modrinth_versions", { id, gameVersion, loader }),
  mrInstall: (instanceId, projectId, versionId) => invoke("install_modrinth", { instanceId, projectId, versionId }),
  mrInstallPack: (versionId) => invoke("install_modrinth_pack", { versionId }),
  cfSearch: (query, kind, gameVersion, offset) => invoke("curseforge_search", { query, kind, gameVersion, offset }),
  cfInstall: (instanceId, modId) => invoke("install_curseforge", { instanceId, modId }),
  cfInstallPack: (modId) => invoke("install_curseforge_pack", { modId }),
  importPack: () => invoke("import_pack_file"),
  // hud
  hud: () => invoke("get_hud"),
  saveHud: (hud) => invoke("save_hud", { hud }),
  resetHud: () => invoke("reset_hud"),
  // skins
  skinProfile: () => invoke("skin_profile"),
  skinUpload: (slim) => invoke("skin_upload_file", { slim }),
  skinApplyLibrary: (id) => invoke("skin_apply_library", { id }),
  skinReset: () => invoke("skin_reset"),
  capeSet: (capeId) => invoke("cape_set", { capeId }),
  texture: (url) => invoke("texture", { url }),
  playerSkin: (uuid) => invoke("player_skin", { uuid }),
  skinLibrary: () => invoke("skin_library"),
  skinLibraryAdd: (slim) => invoke("skin_library_add", { slim }),
  skinLibraryRemove: (id) => invoke("skin_library_remove", { id }),
  // game
  launch: (id) => invoke("launch_instance", { id }),
  kill: (id) => invoke("kill_instance", { id }),
  running: () => invoke("running_instances"),
  // social
  socialConnect: () => invoke("social_connect"),
  socialDisconnect: () => invoke("social_disconnect"),
  socialSend: (msg) => invoke("social_send", { msg }),
};
