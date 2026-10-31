//! salt-core — the engine behind Salt Launcher.
//!
//! Everything that talks to the network or touches the disk lives here so the
//! Tauri app stays a thin layer of commands. Modules map 1:1 to launcher
//! features:
//!
//! | module       | what it does                                                     |
//! |--------------|------------------------------------------------------------------|
//! | `auth`       | Microsoft → Xbox → Minecraft sign-in, token storage in OS keychain |
//! | `meta`       | Mojang version manifest + Fabric / Legacy Fabric loader profiles |
//! | `download`   | parallel, hash-verified downloads                                 |
//! | `java`       | finds or installs the right Java runtime (Adoptium)              |
//! | `instance`   | multi-instance storage (`instances/<id>/instance.json`)         |
//! | `launch`     | builds the classpath + arguments and spawns the game             |
//! | `modrinth`   | Modrinth search / install / .mrpack modpacks                      |
//! | `curseforge` | CurseForge search / install / modpack zips                        |
//! | `catalog`    | the bundled Salt mod set (saltHUD, performance mods, Chlorine)    |
//! | `skins`      | skin + cape changes via the official Minecraft profile API        |
//! | `settings`   | launcher settings, theme and onboarding state                     |

pub mod auth;
pub mod catalog;
pub mod curseforge;
pub mod download;
pub mod http;
pub mod instance;
pub mod java;
pub mod launch;
pub mod meta;
pub mod modrinth;
pub mod paths;
pub mod progress;
pub mod secrets;
pub mod settings;
pub mod skins;
pub mod social;
pub mod verify;
pub mod version;

pub use paths::Paths;
pub use progress::{NoopReporter, Reporter};

pub const LAUNCHER_NAME: &str = "SaltLauncher";
pub const LAUNCHER_VERSION: &str = env!("CARGO_PKG_VERSION");
