//! Tauri commands: thin wrappers that call into salt-core and turn errors
//! into readable strings for the UI.

use std::path::PathBuf;

use salt_core::auth::{self, Account, AccountKind, AccountStore, DeviceCode, Session};
use salt_core::instance::{self, Instance, ModView, NewInstance, WorldView};
use salt_core::settings::{self, Settings};
use salt_core::{catalog, curseforge, meta, modrinth, skins};
use serde::Serialize;
use serde_json::{json, Value};
use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_opener::OpenerExt;

use crate::AppState;

pub type CmdResult<T> = Result<T, String>;

/// Developer options (offline test launches) exist only in debug builds and in
/// builds made with SALT_DEV set; the installer you ship has them compiled out.
pub const DEV_OPTIONS: bool = cfg!(debug_assertions) || option_env!("SALT_DEV").is_some();

pub fn err(e: anyhow::Error) -> String {
    format!("{e:#}")
}

/// The signed-in session for the active account (refreshing if needed).
/// Session for launching the game. Same as `active_session`, except that with
/// no account and the developer "offline test launch" setting on, it returns
/// an offline test profile (no Microsoft token; singleplayer only).
pub async fn launch_session(state: &AppState) -> anyhow::Result<Session> {
    let has_account = AccountStore::load(&state.paths).active_account().is_some();
    let (offline, name) = {
        let s = state.settings.read();
        (s.offline_test, s.offline_name.clone())
    };
    if !has_account && offline && DEV_OPTIONS {
        let name: String = name.chars().filter(|c| c.is_ascii_alphanumeric() || *c == '_').take(16).collect();
        let name = if name.len() < 3 { "Player".to_string() } else { name };
        // Stable per-name UUID, like vanilla's offline "OfflinePlayer:<name>" ids.
        use sha1_smol_free::digest;
        let uuid = digest(&format!("OfflinePlayer:{name}"));
        state.reporter.warn(&format!("Offline test launch as {name} (developer setting; no Microsoft account)"));
        return Ok(Session { account: Account { uuid, username: name, skin_url: None, xuid: None, kind: AccountKind::Verified }, access_token: "0".into() });
    }
    active_session(state).await
}

/// Tiny dependency-free SHA-1 → 32 hex chars, only used for offline test UUIDs.
mod sha1_smol_free {
    pub fn digest(s: &str) -> String {
        let mut h: [u32; 5] = [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0];
        let mut msg = s.as_bytes().to_vec();
        let bits = (msg.len() as u64) * 8;
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bits.to_be_bytes());
        for chunk in msg.chunks(64) {
            let mut w = [0u32; 80];
            for i in 0..16 {
                w[i] = u32::from_be_bytes([chunk[i * 4], chunk[i * 4 + 1], chunk[i * 4 + 2], chunk[i * 4 + 3]]);
            }
            for i in 16..80 {
                w[i] = (w[i - 3] ^ w[i - 8] ^ w[i - 14] ^ w[i - 16]).rotate_left(1);
            }
            let (mut a, mut b, mut c, mut d, mut e) = (h[0], h[1], h[2], h[3], h[4]);
            for (i, wi) in w.iter().enumerate() {
                let (f, k) = match i {
                    0..=19 => ((b & c) | (!b & d), 0x5A827999),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDC),
                    _ => (b ^ c ^ d, 0xCA62C1D6u32),
                };
                let t = a.rotate_left(5).wrapping_add(f).wrapping_add(e).wrapping_add(k).wrapping_add(*wi);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = t;
            }
            for (x, y) in h.iter_mut().zip([a, b, c, d, e]) {
                *x = x.wrapping_add(y);
            }
        }
        h.iter().map(|x| format!("{x:08x}")).collect::<String>()[..32].to_string()
    }
}

pub async fn active_session(state: &AppState) -> anyhow::Result<Session> {
    let store = AccountStore::load(&state.paths);
    let account = store.active_account().cloned().ok_or_else(|| anyhow::anyhow!("Sign in first."))?;
    if account.kind == AccountKind::Verified {
        // Ownership was proven by joining Salt's sign-in server; there's no
        // token, so the game runs in offline mode under the real name and UUID.
        return Ok(Session { account, access_token: "0".into() });
    }
    let client_id = state.settings.read().microsoft_client_id.clone();
    auth::refresh(&state.paths, &client_id, &account).await
}

// ------------------------------------------------------------------ settings

#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> Settings {
    state.settings.read().clone()
}

#[tauri::command]
pub fn save_settings(state: State<'_, AppState>, settings: Settings) -> CmdResult<()> {
    settings.save(&state.paths).map_err(err)?;
    // Push the accent to every instance, including ones that are running.
    for inst in instance::list(&state.paths) {
        let _ = settings::sync_theme_into(&settings.theme, &inst.game_dir(&state.paths));
    }
    *state.settings.write() = settings;
    Ok(())
}

#[tauri::command]
pub fn set_curseforge_key(key: String) -> CmdResult<()> {
    settings::set_curseforge_key(&key).map_err(err)
}

#[tauri::command]
pub fn has_curseforge_key() -> bool {
    !settings::curseforge_key().is_empty()
}

#[tauri::command]
pub fn app_info(state: State<'_, AppState>) -> Value {
    json!({
        "version": salt_core::LAUNCHER_VERSION,
        "dataDir": state.paths.root.display().to_string(),
        "os": std::env::consts::OS,
        "arch": std::env::consts::ARCH,
        "totalMemoryMb": total_memory_mb(),
        "devOptions": DEV_OPTIONS,
    })
}

#[cfg(windows)]
fn total_memory_mb() -> u64 {
    #[repr(C)]
    struct MemoryStatusEx {
        length: u32,
        memory_load: u32,
        total_phys: u64,
        avail_phys: u64,
        total_page_file: u64,
        avail_page_file: u64,
        total_virtual: u64,
        avail_virtual: u64,
        avail_extended_virtual: u64,
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn GlobalMemoryStatusEx(buf: *mut MemoryStatusEx) -> i32;
    }
    let mut s = MemoryStatusEx { length: std::mem::size_of::<MemoryStatusEx>() as u32, memory_load: 0, total_phys: 0, avail_phys: 0, total_page_file: 0, avail_page_file: 0, total_virtual: 0, avail_virtual: 0, avail_extended_virtual: 0 };
    // SAFETY: plain Win32 call filling a correctly sized, initialised struct.
    if unsafe { GlobalMemoryStatusEx(&mut s) } != 0 { s.total_phys / 1024 / 1024 } else { 8192 }
}

#[cfg(not(windows))]
fn total_memory_mb() -> u64 {
    8192
}

#[tauri::command]
pub fn open_url(app: AppHandle, url: String) -> CmdResult<()> {
    if !(url.starts_with("https://") || url.starts_with("http://")) {
        return Err("Only web links can be opened".into());
    }
    app.opener().open_url(url, None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_data_folder(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    app.opener().open_path(state.paths.root.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn launcher_logs(state: State<'_, AppState>) -> Vec<crate::reporter::LauncherLog> {
    state.reporter.recent()
}

// ------------------------------------------------------------------ accounts

#[derive(Serialize)]
pub struct AccountsView {
    accounts: Vec<Account>,
    active: Option<String>,
}

#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> AccountsView {
    let s = AccountStore::load(&state.paths);
    AccountsView { accounts: s.accounts, active: s.active }
}

/// Starts Microsoft's device-code sign-in. Returns the code to show; the
/// result arrives later as a `salt://login` event.
#[tauri::command]
pub async fn login_start(app: AppHandle, state: State<'_, AppState>) -> CmdResult<DeviceCode> {
    let client_id = state.settings.read().microsoft_client_id.clone();
    let code = auth::start_device_code(&client_id).await.map_err(err)?;
    let code2 = code.clone();
    let paths = state.paths.clone();
    let reporter = state.reporter.clone();
    tauri::async_runtime::spawn(async move {
        match auth::poll_device_code(&paths, &client_id, &code2).await {
            Ok(session) => {
                reporter.info(&format!("Signed in as {}", session.account.username));
                let _ = app.emit("salt://login", json!({ "ok": true, "account": session.account }));
            }
            Err(e) => {
                reporter.warn(&format!("Sign-in failed: {e:#}"));
                let _ = app.emit("salt://login", json!({ "ok": false, "error": format!("{e:#}") }));
            }
        }
    });
    Ok(code)
}

/// The running "Sign in with Minecraft" wait, so starting again (or
/// cancelling) stops the previous local sign-in server.
static VERIFY: parking_lot::Mutex<Option<tauri::async_runtime::JoinHandle<()>>> = parking_lot::Mutex::new(None);

#[derive(Serialize)]
pub struct VerifyStart {
    address: String,
}

/// Starts "Sign in with Minecraft": a local server the player joins from their
/// normal Minecraft (see salt_core::verify). Returns the address to join; the
/// result arrives later as a `salt://login` event, like Microsoft sign-in.
#[tauri::command]
pub async fn verify_start(app: AppHandle, state: State<'_, AppState>) -> CmdResult<VerifyStart> {
    if let Some(old) = VERIFY.lock().take() {
        old.abort();
    }
    let listener = salt_core::verify::listen().await.map_err(err)?;
    let address = listener.address();
    let paths = state.paths.clone();
    let reporter = state.reporter.clone();
    reporter.info(&format!("Sign in with Minecraft: waiting for a join on {address}"));
    let task = tauri::async_runtime::spawn(async move {
        let result = listener.wait(std::time::Duration::from_secs(15 * 60)).await.and_then(|profile| {
            let account = Account::verified(profile);
            let mut store = AccountStore::load(&paths);
            store.upsert(account.clone());
            store.save(&paths)?;
            Ok(account)
        });
        match result {
            Ok(account) => {
                reporter.info(&format!("Signed in as {} (verified with Minecraft)", account.username));
                let _ = app.emit("salt://login", json!({ "ok": true, "account": account }));
            }
            Err(e) => {
                reporter.warn(&format!("Sign-in failed: {e:#}"));
                let _ = app.emit("salt://login", json!({ "ok": false, "error": format!("{e:#}") }));
            }
        }
    });
    *VERIFY.lock() = Some(task);
    Ok(VerifyStart { address })
}

#[tauri::command]
pub fn verify_cancel() {
    if let Some(task) = VERIFY.lock().take() {
        task.abort();
    }
}

#[tauri::command]
pub fn set_active_account(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    let mut s = AccountStore::load(&state.paths);
    if s.accounts.iter().any(|a| a.uuid == uuid) {
        s.active = Some(uuid);
    }
    s.save(&state.paths).map_err(err)
}

#[tauri::command]
pub fn remove_account(state: State<'_, AppState>, uuid: String) -> CmdResult<()> {
    let mut s = AccountStore::load(&state.paths);
    s.remove(&uuid);
    s.save(&state.paths).map_err(err)
}

// ------------------------------------------------------------------ versions

#[tauri::command]
pub async fn list_versions() -> CmdResult<Vec<meta::ManifestEntry>> {
    meta::supported_versions().await.map_err(err)
}

#[tauri::command]
pub async fn list_fabric_loaders(mc: String) -> CmdResult<Vec<meta::LoaderVersion>> {
    meta::fabric_loaders(&mc).await.map_err(err)
}

// ----------------------------------------------------------------- instances

#[tauri::command]
pub fn list_instances(state: State<'_, AppState>) -> Vec<Instance> {
    instance::list(&state.paths)
}

#[tauri::command]
pub fn get_instance(state: State<'_, AppState>, id: String) -> CmdResult<Instance> {
    instance::get(&state.paths, &id).map_err(err)
}

#[tauri::command]
pub fn create_instance(state: State<'_, AppState>, req: NewInstance) -> CmdResult<Instance> {
    let inst = instance::create(&state.paths, req).map_err(err)?;
    state.reporter.info(&format!("Created instance {} ({} {:?})", inst.name, inst.mc_version, inst.loader));
    Ok(inst)
}

#[tauri::command]
pub fn update_instance(state: State<'_, AppState>, inst: Instance) -> CmdResult<Instance> {
    // Only settings change here; the id and folder stay the same.
    let current = instance::get(&state.paths, &inst.id).map_err(err)?;
    let merged = Instance { created: current.created, playtime_secs: current.playtime_secs, last_played: current.last_played, ..inst };
    instance::save(&state.paths, &merged).map_err(err)?;
    Ok(merged)
}

#[tauri::command]
pub fn delete_instance(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if state.games.is_running(&id) {
        return Err("Close the game before deleting this instance.".into());
    }
    instance::delete(&state.paths, &id).map_err(err)?;
    state.reporter.info(&format!("Deleted instance {id}"));
    Ok(())
}

#[tauri::command]
pub fn duplicate_instance(state: State<'_, AppState>, id: String) -> CmdResult<Instance> {
    instance::duplicate(&state.paths, &id).map_err(err)
}

#[tauri::command]
pub fn open_instance_folder(app: AppHandle, state: State<'_, AppState>, id: String, sub: Option<String>) -> CmdResult<()> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    let mut dir = inst.game_dir(&state.paths);
    if let Some(s) = sub.filter(|s| matches!(s.as_str(), "mods" | "saves" | "resourcepacks" | "shaderpacks" | "screenshots" | "logs" | "config")) {
        dir = dir.join(s);
    }
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    app.opener().open_path(dir.display().to_string(), None::<&str>).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_worlds(state: State<'_, AppState>, id: String) -> CmdResult<Vec<WorldView>> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    Ok(instance::list_worlds(&state.paths, &inst))
}

// ---------------------------------------------------------------------- mods

#[tauri::command]
pub fn list_mods(state: State<'_, AppState>, id: String) -> CmdResult<Vec<ModView>> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    Ok(instance::list_mods(&state.paths, &inst))
}

#[tauri::command]
pub fn toggle_mod(state: State<'_, AppState>, id: String, file_name: String, enabled: bool) -> CmdResult<()> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    instance::set_mod_enabled(&state.paths, &inst, &file_name, enabled).map_err(err)
}

#[tauri::command]
pub fn remove_mod(state: State<'_, AppState>, id: String, file_name: String) -> CmdResult<()> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    instance::remove_mod(&state.paths, &inst, &file_name).map_err(err)
}

/// Opens a file picker and copies the chosen .jar files into the instance.
#[tauri::command]
pub async fn add_local_mods(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<Vec<String>> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    let picked = pick_files(&app, "Mods", &["jar"]).await;
    let mut added = Vec::new();
    for p in picked {
        added.push(instance::import_local_mod(&state.paths, &inst, &p).map_err(err)?);
    }
    Ok(added)
}

async fn pick_files(app: &AppHandle, label: &str, ext: &[&str]) -> Vec<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter(label, ext).pick_files(move |files| {
        let _ = tx.send(files.unwrap_or_default().into_iter().filter_map(|f| f.into_path().ok()).collect::<Vec<_>>());
    });
    rx.await.unwrap_or_default()
}

async fn pick_file(app: &AppHandle, label: &str, ext: &[&str]) -> Option<PathBuf> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    app.dialog().file().add_filter(label, ext).pick_file(move |f| {
        let _ = tx.send(f.and_then(|f| f.into_path().ok()));
    });
    rx.await.ok().flatten()
}

#[tauri::command]
pub async fn check_updates(state: State<'_, AppState>, id: String) -> CmdResult<Vec<modrinth::UpdateInfo>> {
    let inst = instance::get(&state.paths, &id).map_err(err)?;
    modrinth::check_updates(&state.paths, &inst).await.map_err(err)
}

#[tauri::command]
pub fn bundle_listing() -> Value {
    catalog::bundle_listing()
}

// ------------------------------------------------------------------ discover

#[tauri::command]
pub async fn modrinth_search(query: modrinth::SearchQuery) -> CmdResult<modrinth::SearchResult> {
    modrinth::search(&query).await.map_err(err)
}

#[tauri::command]
pub async fn modrinth_project(id: String) -> CmdResult<modrinth::Project> {
    modrinth::project(&id).await.map_err(err)
}

#[tauri::command]
pub async fn modrinth_versions(id: String, game_version: Option<String>, loader: Option<String>) -> CmdResult<Vec<modrinth::Version>> {
    modrinth::versions(&id, game_version.as_deref(), loader.as_deref()).await.map_err(err)
}

#[tauri::command]
pub async fn install_modrinth(state: State<'_, AppState>, instance_id: String, project_id: String, version_id: Option<String>) -> CmdResult<Vec<String>> {
    let inst = instance::get(&state.paths, &instance_id).map_err(err)?;
    modrinth::install_project(&state.paths, &inst, &project_id, version_id.as_deref(), state.reporter.as_ref()).await.map_err(err)
}

#[tauri::command]
pub async fn install_modrinth_pack(state: State<'_, AppState>, version_id: String) -> CmdResult<Instance> {
    modrinth::install_modpack_version(&state.paths, &version_id, None, state.reporter.as_ref()).await.map_err(err)
}

#[tauri::command]
pub async fn curseforge_search(query: String, kind: String, game_version: Option<String>, offset: u32) -> CmdResult<Vec<curseforge::Hit>> {
    curseforge::search(&settings::curseforge_key(), &query, &kind, game_version.as_deref(), offset, 20).await.map_err(err)
}

#[tauri::command]
pub async fn install_curseforge(state: State<'_, AppState>, instance_id: String, mod_id: u64) -> CmdResult<Vec<String>> {
    let inst = instance::get(&state.paths, &instance_id).map_err(err)?;
    curseforge::install_mod(&state.paths, &settings::curseforge_key(), &inst, mod_id, None, state.reporter.as_ref()).await.map_err(err)
}

#[tauri::command]
pub async fn install_curseforge_pack(state: State<'_, AppState>, mod_id: u64) -> CmdResult<Instance> {
    curseforge::install_modpack(&state.paths, &settings::curseforge_key(), mod_id, None, state.reporter.as_ref()).await.map_err(err)
}

/// Import a .mrpack (Modrinth) or .zip (CurseForge) modpack from disk.
#[tauri::command]
pub async fn import_pack_file(app: AppHandle, state: State<'_, AppState>) -> CmdResult<Option<Instance>> {
    let Some(path) = pick_file(&app, "Modpacks", &["mrpack", "zip"]).await else { return Ok(None) };
    let bytes = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let r = state.reporter.as_ref();
    let inst = if path.extension().and_then(|e| e.to_str()) == Some("mrpack") {
        modrinth::install_mrpack_bytes(&state.paths, &bytes, None, r).await
    } else {
        curseforge::install_zip(&state.paths, &settings::curseforge_key(), &bytes, None, r).await
    };
    inst.map(Some).map_err(err)
}

// ----------------------------------------------------------------------- hud

#[tauri::command]
pub fn get_hud(state: State<'_, AppState>) -> Value {
    settings::load_hud(&state.paths)
}

#[tauri::command]
pub fn save_hud(state: State<'_, AppState>, hud: Value) -> CmdResult<()> {
    settings::save_hud(&state.paths, &hud).map_err(err)
}

#[tauri::command]
pub fn reset_hud(state: State<'_, AppState>) -> CmdResult<Value> {
    let hud = settings::default_hud();
    settings::save_hud(&state.paths, &hud).map_err(err)?;
    Ok(hud)
}

// --------------------------------------------------------------------- skins

#[tauri::command]
pub async fn skin_profile(state: State<'_, AppState>) -> CmdResult<auth::McProfile> {
    let s = active_session(&state).await.map_err(err)?;
    skins::profile(&s.access_token).await.map_err(err)
}

#[tauri::command]
pub async fn skin_upload_file(app: AppHandle, state: State<'_, AppState>, slim: bool) -> CmdResult<Option<auth::McProfile>> {
    let Some(path) = pick_file(&app, "Skin", &["png"]).await else { return Ok(None) };
    let png = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Skin".into());
    let _ = skins::library_add(&state.paths, &name, slim, &png);
    let s = active_session(&state).await.map_err(err)?;
    let p = skins::upload_skin(&s.access_token, png, slim).await.map_err(err)?;
    state.reporter.info("Skin updated");
    Ok(Some(p))
}

#[tauri::command]
pub async fn skin_apply_library(state: State<'_, AppState>, id: String) -> CmdResult<auth::McProfile> {
    let (png, slim) = skins::library_png(&state.paths, &id).map_err(err)?;
    let s = active_session(&state).await.map_err(err)?;
    skins::upload_skin(&s.access_token, png, slim).await.map_err(err)
}

#[tauri::command]
pub async fn skin_reset(state: State<'_, AppState>) -> CmdResult<auth::McProfile> {
    let s = active_session(&state).await.map_err(err)?;
    skins::reset_skin(&s.access_token).await.map_err(err)
}

#[tauri::command]
pub async fn cape_set(state: State<'_, AppState>, cape_id: Option<String>) -> CmdResult<auth::McProfile> {
    let s = active_session(&state).await.map_err(err)?;
    skins::set_cape(&s.access_token, cape_id.as_deref()).await.map_err(err)
}

#[tauri::command]
pub async fn texture(url: String) -> CmdResult<String> {
    skins::texture_data_url(&url).await.map_err(err)
}

#[tauri::command]
pub async fn player_skin(uuid: String) -> CmdResult<String> {
    skins::skin_data_url_for(&uuid).await.map_err(err)
}

#[tauri::command]
pub fn skin_library(state: State<'_, AppState>) -> Vec<skins::LibrarySkinView> {
    skins::library_list(&state.paths)
}

#[tauri::command]
pub async fn skin_library_add(app: AppHandle, state: State<'_, AppState>, slim: bool) -> CmdResult<Option<skins::LibrarySkin>> {
    let Some(path) = pick_file(&app, "Skin", &["png"]).await else { return Ok(None) };
    let png = tokio::fs::read(&path).await.map_err(|e| e.to_string())?;
    let name = path.file_stem().map(|s| s.to_string_lossy().to_string()).unwrap_or_else(|| "Skin".into());
    skins::library_add(&state.paths, &name, slim, &png).map(Some).map_err(err)
}

#[tauri::command]
pub fn skin_library_remove(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    skins::library_remove(&state.paths, &id).map_err(err)
}

/// Used by game.rs to minimise/close the launcher after launching.
pub fn apply_launch_behaviour(app: &AppHandle, behaviour: &str) {
    if let Some(w) = app.get_webview_window("main") {
        match behaviour {
            "minimize" => {
                let _ = w.minimize();
            }
            "close" => {
                let _ = w.hide();
            }
            _ => {}
        }
    }
}
