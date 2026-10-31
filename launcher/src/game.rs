//! Running games: launch, live log streaming, kill, playtime tracking.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use parking_lot::Mutex;
use salt_core::launch::{self, LogLine, LogParser, PrepareOptions};
use salt_core::{catalog, instance, settings};
use serde_json::json;
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::sync::{mpsc, oneshot};

use crate::commands::{apply_launch_behaviour, err, launch_session, CmdResult};
use crate::AppState;

struct Running {
    kill: Option<oneshot::Sender<()>>,
    started: Instant,
}

#[derive(Default)]
pub struct Games {
    inner: Mutex<HashMap<String, Running>>,
}

impl Games {
    pub fn is_running(&self, id: &str) -> bool {
        self.inner.lock().contains_key(id)
    }
}

fn emit_state(app: &AppHandle, id: &str, state: &str, extra: serde_json::Value) {
    let mut payload = json!({ "id": id, "state": state });
    if let (Some(p), Some(e)) = (payload.as_object_mut(), extra.as_object()) {
        p.extend(e.clone());
    }
    let _ = app.emit("salt://game", payload);
}

#[tauri::command]
pub async fn launch_instance(app: AppHandle, state: State<'_, AppState>, id: String) -> CmdResult<()> {
    if state.games.is_running(&id) {
        return Err("This instance is already running.".into());
    }
    // Reserve the slot so a double-click can't launch twice.
    state.games.inner.lock().insert(id.clone(), Running { kill: None, started: Instant::now() });
    match start(&app, &state, &id).await {
        Ok(()) => Ok(()),
        Err(e) => {
            state.games.inner.lock().remove(&id);
            let msg = format!("{e:#}");
            state.reporter.error(&format!("Launch failed: {msg}"));
            emit_state(&app, &id, "error", json!({ "error": msg }));
            Err(msg)
        }
    }
}

async fn start(app: &AppHandle, state: &AppState, id: &str) -> anyhow::Result<()> {
    let r = state.reporter.as_ref();
    let mut inst = instance::get(&state.paths, id)?;
    let settings = state.settings.read().clone();
    emit_state(app, id, "preparing", json!({}));
    r.info(&format!("Launching {} ({} {:?})", inst.name, inst.mc_version, inst.loader));

    let session = launch_session(state).await?;
    catalog::sync(&state.paths, &inst, &settings.bundle_overrides, r).await?;
    settings::sync_hud_into(&state.paths, &inst.game_dir(&state.paths))?;
    settings::sync_theme_into(&settings.theme, &inst.game_dir(&state.paths))?;

    let prepared = launch::prepare(
        &state.paths,
        &inst,
        &session,
        PrepareOptions {
            concurrency: settings.max_concurrent_downloads,
            global_java_path: settings.java_path.as_deref(),
            extra_jvm_args: &settings.extra_jvm_args,
        },
        r,
    )
    .await?;
    r.info(&format!("Command: {}", prepared.display_command()));
    let mut child = launch::spawn(&prepared)?;

    inst.last_played = Some(chrono::Utc::now());
    instance::save(&state.paths, &inst)?;

    let (kill_tx, kill_rx) = oneshot::channel();
    if let Some(slot) = state.games.inner.lock().get_mut(id) {
        slot.kill = Some(kill_tx);
        slot.started = Instant::now();
    }
    emit_state(app, id, "running", json!({ "pid": child.id() }));
    crate::social::set_presence(state, Some(format!("Playing {}", inst.mc_version)));
    apply_launch_behaviour(app, &settings.launch_behaviour);

    // stdout + stderr → parser → batched UI events
    let (log_tx, mut log_rx) = mpsc::unbounded_channel::<LogLine>();
    for stream in [child.stdout.take().map(|s| Box::new(s) as Box<dyn tokio::io::AsyncRead + Unpin + Send>), child.stderr.take().map(|s| Box::new(s) as _)] {
        let Some(stream) = stream else { continue };
        let tx = log_tx.clone();
        let secret = prepared.secret.clone();
        tauri::async_runtime::spawn(async move {
            let mut parser = LogParser::new(&secret);
            let mut lines = BufReader::new(stream).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                if let Some(l) = parser.feed(&line) {
                    if tx.send(l).is_err() {
                        break;
                    }
                }
            }
        });
    }
    drop(log_tx);
    let app2 = app.clone();
    let id2 = id.to_string();
    tauri::async_runtime::spawn(async move {
        let mut batch = Vec::new();
        let mut tick = tokio::time::interval(Duration::from_millis(100));
        loop {
            tokio::select! {
                line = log_rx.recv() => match line {
                    Some(l) => batch.push(l),
                    None => break,
                },
                _ = tick.tick() => {
                    if !batch.is_empty() {
                        let _ = app2.emit("salt://game-log", json!({ "id": id2, "lines": std::mem::take(&mut batch) }));
                    }
                }
            }
        }
        if !batch.is_empty() {
            let _ = app2.emit("salt://game-log", json!({ "id": id2, "lines": batch }));
        }
    });

    // Wait for exit (or a kill request).
    let app3 = app.clone();
    let id3 = id.to_string();
    tauri::async_runtime::spawn(async move {
        let status = tokio::select! {
            s = child.wait() => s.ok(),
            _ = kill_rx => {
                let _ = child.kill().await;
                child.wait().await.ok()
            }
        };
        let code = status.and_then(|s| s.code());
        let state = app3.state::<AppState>();
        let started = state.games.inner.lock().remove(&id3).map(|r| r.started);
        if let (Some(started), Ok(mut inst)) = (started, instance::get(&state.paths, &id3)) {
            inst.playtime_secs += started.elapsed().as_secs();
            let _ = instance::save(&state.paths, &inst);
        }
        let crashed = matches!(code, Some(c) if c != 0);
        if crashed {
            state.reporter.warn(&format!("{id3} exited with code {}", code.unwrap_or(-1)));
        } else {
            state.reporter.info(&format!("{id3} closed"));
        }
        emit_state(&app3, &id3, "exited", json!({ "code": code, "crashed": crashed }));
        if state.games.inner.lock().is_empty() {
            crate::social::set_presence(&state, None);
        }
        // Bring the launcher back if it was hidden/minimised on launch.
        if let Some(w) = app3.get_webview_window("main") {
            let _ = w.show();
            if crashed {
                let _ = w.unminimize();
                let _ = w.set_focus();
            }
        }
    });
    Ok(())
}

#[tauri::command]
pub fn kill_instance(state: State<'_, AppState>, id: String) -> CmdResult<()> {
    let tx = state.games.inner.lock().get_mut(&id).and_then(|r| r.kill.take());
    match tx {
        Some(tx) => {
            let _ = tx.send(());
            state.reporter.warn(&format!("Force-closing {id}"));
            Ok(())
        }
        None => Err(err(anyhow::anyhow!("That instance isn't running"))),
    }
}

#[tauri::command]
pub fn running_instances(state: State<'_, AppState>) -> Vec<String> {
    state.games.inner.lock().keys().cloned().collect()
}
