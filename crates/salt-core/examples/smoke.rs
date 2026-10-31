//! End-to-end smoke test of the launch pipeline, without a Microsoft account.
//!
//!   cargo run -p salt-core --example smoke -- 26.3 [seconds]
//!
//! Uses a throwaway data folder (%USERPROFILE%\SaltLauncherBuild\smoke-data),
//! creates a Fabric instance with the Salt bundle, downloads everything,
//! launches with an offline test profile, watches the log for mod/mixin
//! problems, then closes the game. Salt mod jars are copied from the real
//! launcher's salt-mods folder.

use std::time::{Duration, Instant};

use salt_core::auth::{Account, Session};
use salt_core::instance::{self, NewInstance};
use salt_core::launch::{self, LogParser, PrepareOptions};
use salt_core::meta::Loader;
use salt_core::progress::{Progress, Reporter};
use salt_core::{catalog, settings, Paths};
use tokio::io::{AsyncBufReadExt, BufReader};

struct Print;

impl Reporter for Print {
    fn progress(&self, p: Progress) {
        if p.finished || p.done == 0 || p.done % 400 == 0 {
            println!("  [progress] {} {}/{}", p.label, p.done, p.total);
        }
    }
    fn log(&self, level: &str, message: &str) {
        println!("  [{level}] {message}");
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mc = args.get(1).cloned().unwrap_or_else(|| "26.3".into());
    let secs: u64 = args.get(2).and_then(|s| s.parse().ok()).unwrap_or(90);

    let root = std::path::PathBuf::from(std::env::var("USERPROFILE")?).join("SaltLauncherBuild").join("smoke-data");
    let paths = Paths::new(&root);
    paths.ensure()?;
    // Bring in locally built Salt mods + Chlorine.
    let real = Paths::new(Paths::default_root()).salt_mods().join(&mc);
    let dest = paths.salt_mods().join(&mc);
    std::fs::create_dir_all(&dest)?;
    if let Ok(rd) = std::fs::read_dir(&real) {
        for e in rd.flatten() {
            std::fs::copy(e.path(), dest.join(e.file_name()))?;
        }
    }

    let id = format!("smoke-{}", mc.replace('.', "-"));
    let inst = match instance::get(&paths, &id) {
        Ok(i) => i,
        Err(_) => {
            let mut i = instance::create(&paths, NewInstance { name: format!("smoke {mc}"), mc_version: mc.clone(), loader: Loader::Fabric, loader_version: None, icon: None, group: None, memory_mb: Some(4096), salt_bundle: true })?;
            // create() derives the id from the name; keep it predictable.
            if i.id != id {
                std::fs::rename(paths.instance(&i.id), paths.instance(&id))?;
                i.id = id.clone();
                instance::save(&paths, &i)?;
            }
            i
        }
    };
    println!("== instance {} ({} fabric)", inst.id, inst.mc_version);

    let t = Instant::now();
    println!("== syncing Salt bundle");
    catalog::sync(&paths, &inst, &[], &Print).await?;
    settings::sync_hud_into(&paths, &inst.game_dir(&paths))?;
    for m in instance::list_mods(&paths, &inst) {
        println!("  mod: {:<28} {:<12} {:?}", m.title, m.version.unwrap_or_default(), m.source);
    }

    let session = Session {
        account: Account { uuid: "5a17c0de5a17c0de5a17c0de5a17c0de".into(), username: "SaltSmoke".into(), skin_url: None, xuid: None, kind: Default::default() },
        access_token: "offline-smoke-token".into(),
    };
    println!("== preparing launch");
    let prepared = launch::prepare(&paths, &inst, &session, PrepareOptions { concurrency: 32, global_java_path: None, extra_jvm_args: &[] }, &Print).await?;
    println!("== prepared in {:.0}s; java = {}", t.elapsed().as_secs_f32(), prepared.java.display());
    assert!(!prepared.display_command().contains("offline-smoke-token"), "token leaked into the displayed command");

    // Extra game arguments for manual checks, e.g. SMOKE_ARGS="--quickPlayMultiplayer localhost:25599".
    let mut prepared = prepared;
    if let Ok(extra) = std::env::var("SMOKE_ARGS") {
        prepared.args.extend(extra.split_whitespace().map(str::to_string));
    }
    let mut child = launch::spawn(&prepared)?;
    let stdout = child.stdout.take().unwrap();
    let stderr = child.stderr.take().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<String>();
    for s in [Box::new(stdout) as Box<dyn tokio::io::AsyncRead + Unpin + Send>, Box::new(stderr)] {
        let tx = tx.clone();
        tokio::spawn(async move {
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(l)) = lines.next_line().await {
                let _ = tx.send(l);
            }
        });
    }
    drop(tx);

    let mut parser = LogParser::new(&prepared.secret);
    let deadline = Instant::now() + Duration::from_secs(secs);
    let (mut errors, mut mixin_fail, mut salt_ready, mut title, mut menu) = (0, 0, false, false, false);
    loop {
        let line = tokio::select! {
            l = rx.recv() => l,
            _ = tokio::time::sleep_until(deadline.into()) => None,
            s = child.wait() => { println!("== game exited: {:?}", s?); break; }
        };
        let Some(raw) = line else { break };
        let Some(l) = parser.feed(&raw) else { continue };
        let m = &l.message;
        if m.contains("saltHUD ready") { salt_ready = true; }
        if m.contains("saltMenu: showing") { menu = true; }
        if m.contains("Sound engine started") || m.contains("Created: ") { title = true; }
        if m.contains("FAILED during APPLY") || m.contains("was not applied") || m.contains("Mixin apply failed") { mixin_fail += 1; }
        if l.level == "ERROR" || l.level == "FATAL" { errors += 1; }
        let interesting = l.level != "INFO" || m.contains("salt") || m.contains("Salt") || m.contains("Loading ") || m.contains("Chlorine") || m.contains("Sound engine") || m.contains("Backend library");
        if interesting {
            let first: String = m.lines().next().unwrap_or("").chars().take(220).collect();
            println!("  game [{}] {}", l.level, first);
        }
    }
    let _ = child.kill().await;
    println!("\n== RESULT {}: reached title={title} saltHUD loaded={salt_ready} saltMenu shown={menu} mixin failures={mixin_fail} error lines={errors}", inst.mc_version);
    Ok(())
}
