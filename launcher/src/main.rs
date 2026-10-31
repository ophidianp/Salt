// Hide the console window in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod commands;
mod game;
mod reporter;
mod social;

use std::sync::Arc;

use parking_lot::RwLock;
use salt_core::{settings::Settings, Paths};
use tauri::Manager;

pub struct AppState {
    pub paths: Paths,
    pub settings: RwLock<Settings>,
    pub reporter: Arc<reporter::AppReporter>,
    pub games: game::Games,
    pub social: social::SocialHandle,
}

fn main() {
    tracing_subscriber::fmt().with_env_filter("info").init();

    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let paths = Paths::new(Paths::default_root());
            paths.ensure()?;
            let mut settings = Settings::load(&paths);
            let reporter = Arc::new(reporter::AppReporter::new(app.handle().clone(), paths.logs()));
            reporter.info(&format!("saltClient {} starting — data folder {}", salt_core::LAUNCHER_VERSION, paths.root.display()));
            let before = settings.seeded_versions.len();
            match salt_core::instance::seed_defaults(&paths, settings.default_memory_mb, &mut settings.seeded_versions) {
                Ok(made) => {
                    for i in &made {
                        reporter.info(&format!("Created default instance {}", i.name));
                    }
                    if settings.seeded_versions.len() != before {
                        let _ = settings.save(&paths);
                    }
                }
                Err(e) => reporter.info(&format!("Couldn't create default instances: {e}")),
            }
            app.manage(AppState {
                paths,
                settings: RwLock::new(settings),
                reporter,
                games: Default::default(),
                social: Default::default(),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // settings + app
            commands::get_settings,
            commands::save_settings,
            commands::set_curseforge_key,
            commands::has_curseforge_key,
            commands::app_info,
            commands::open_url,
            commands::open_data_folder,
            commands::launcher_logs,
            // accounts
            commands::list_accounts,
            commands::login_start,
            commands::verify_start,
            commands::verify_cancel,
            commands::set_active_account,
            commands::remove_account,
            // versions
            commands::list_versions,
            commands::list_fabric_loaders,
            // instances
            commands::list_instances,
            commands::get_instance,
            commands::create_instance,
            commands::update_instance,
            commands::delete_instance,
            commands::duplicate_instance,
            commands::open_instance_folder,
            commands::list_worlds,
            // mods
            commands::list_mods,
            commands::toggle_mod,
            commands::remove_mod,
            commands::add_local_mods,
            commands::check_updates,
            commands::bundle_listing,
            // discover
            commands::modrinth_search,
            commands::modrinth_project,
            commands::modrinth_versions,
            commands::install_modrinth,
            commands::install_modrinth_pack,
            commands::curseforge_search,
            commands::install_curseforge,
            commands::install_curseforge_pack,
            commands::import_pack_file,
            // hud
            commands::get_hud,
            commands::save_hud,
            commands::reset_hud,
            // skins
            commands::skin_profile,
            commands::skin_upload_file,
            commands::skin_apply_library,
            commands::skin_reset,
            commands::cape_set,
            commands::texture,
            commands::player_skin,
            commands::skin_library,
            commands::skin_library_add,
            commands::skin_library_remove,
            // game
            game::launch_instance,
            game::kill_instance,
            game::running_instances,
            // social
            social::social_connect,
            social::social_disconnect,
            social::social_send,
        ])
        .run(tauri::generate_context!())
        .expect("error while running saltClient");
}
