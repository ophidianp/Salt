//! Launcher side of friends + chat: one WebSocket to the salt-social server.
//! Server messages are forwarded to the UI as `salt://social` events.

use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use salt_core::social::{mojang_join, ClientMsg, ServerMsg};
use serde_json::json;
use tauri::{AppHandle, Emitter, State};
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::commands::{active_session, err, CmdResult};
use crate::AppState;

#[derive(Default)]
pub struct SocialHandle {
    tx: Mutex<Option<mpsc::UnboundedSender<ClientMsg>>>,
}

fn emit_conn(app: &AppHandle, state: &str, message: Option<String>) {
    let _ = app.emit("salt://social", json!({ "t": "_connection", "state": state, "message": message }));
}

#[tauri::command]
pub async fn social_connect(app: AppHandle, state: State<'_, AppState>) -> CmdResult<()> {
    social_disconnect(state.clone());
    let url = state.settings.read().social_server.clone();
    if !(url.starts_with("wss://") || url.starts_with("ws://")) {
        return Err("The social server address must start with wss:// or ws://".into());
    }
    let session = active_session(&state).await.map_err(err)?;
    emit_conn(&app, "connecting", None);
    let (ws, _) = tokio_tungstenite::connect_async(url.as_str()).await.map_err(|e| {
        emit_conn(&app, "offline", Some(e.to_string()));
        format!("Couldn't reach the social server: {e}")
    })?;
    let (mut sink, mut stream) = ws.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ClientMsg>();
    *state.social.tx.lock() = Some(tx.clone());

    let hello = serde_json::to_string(&ClientMsg::Hello { name: session.account.username.clone() }).unwrap();
    sink.send(Message::Text(hello)).await.map_err(|e| e.to_string())?;

    let reporter = state.reporter.clone();
    tauri::async_runtime::spawn(async move {
        let mut ping = tokio::time::interval(std::time::Duration::from_secs(25));
        loop {
            tokio::select! {
                out = rx.recv() => {
                    let Some(msg) = out else { break };
                    if sink.send(Message::Text(serde_json::to_string(&msg).unwrap())).await.is_err() { break; }
                }
                _ = ping.tick() => {
                    if sink.send(Message::Text(serde_json::to_string(&ClientMsg::Ping).unwrap())).await.is_err() { break; }
                }
                incoming = stream.next() => {
                    let Some(Ok(Message::Text(text))) = incoming else {
                        if matches!(incoming, Some(Ok(_))) { continue; }
                        break;
                    };
                    let Ok(msg) = serde_json::from_str::<ServerMsg>(&text) else { continue };
                    match &msg {
                        ServerMsg::Challenge { server_id } => {
                            // Prove our identity via Mojang; the token goes
                            // to Mojang only, never to the social server.
                            match mojang_join(&session.access_token, &session.account.uuid, server_id).await {
                                Ok(()) => { let _ = tx.send(ClientMsg::Verify); }
                                Err(e) => {
                                    reporter.warn(&format!("Social sign-in failed: {e:#}"));
                                    emit_conn(&app, "offline", Some(format!("{e:#}")));
                                    break;
                                }
                            }
                        }
                        ServerMsg::Welcome { .. } => emit_conn(&app, "online", None),
                        ServerMsg::Pong => continue,
                        _ => {}
                    }
                    let _ = app.emit("salt://social", &msg);
                }
            }
        }
        emit_conn(&app, "offline", None);
    });
    Ok(())
}

#[tauri::command]
pub fn social_disconnect(state: State<'_, AppState>) {
    // Dropping the sender ends the connection task.
    state.social.tx.lock().take();
}

/// Presence update used by game.rs on launch/exit. Silently does nothing
/// when friends are off or the user doesn't share activity.
pub fn set_presence(state: &AppState, activity: Option<String>) {
    let share = state.settings.read().share_activity;
    if let Some(tx) = state.social.tx.lock().as_ref() {
        let msg = match activity {
            Some(a) if share => ClientMsg::Status { status: "playing".into(), activity: Some(a) },
            _ => ClientMsg::Status { status: "online".into(), activity: None },
        };
        let _ = tx.send(msg);
    }
}

#[tauri::command]
pub fn social_send(state: State<'_, AppState>, msg: ClientMsg) -> CmdResult<()> {
    if let ClientMsg::Dm { text, .. } = &msg {
        if text.trim().is_empty() || text.len() > 1000 {
            return Err("Messages must be 1–1000 characters.".into());
        }
    }
    match state.social.tx.lock().as_ref() {
        Some(tx) => tx.send(msg).map_err(|_| "Not connected".to_string()),
        None => Err("Not connected to the social server".into()),
    }
}
