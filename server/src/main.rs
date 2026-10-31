//! salt-social — friends, presence, direct messages and world invites for
//! Salt Launcher.
//!
//! * Identity is proven with Mojang's `hasJoined` check (see
//!   `salt_core::social`), so this server never receives access tokens.
//! * Stored on disk: usernames, UUIDs, friendships and pending requests
//!   (`salt-social.json`). Messages are relayed live and never stored.
//!
//! Run: `cargo run -p salt-social --release -- 0.0.0.0:7878`
//! Put it behind a TLS reverse proxy (Caddy/nginx) and point Salt at
//! `wss://your.domain/ws`.

use std::collections::{HashMap, HashSet};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::extract::State;
use axum::response::IntoResponse;
use axum::routing::get;
use axum::Router;
use futures::{SinkExt, StreamExt};
use parking_lot::Mutex;
use salt_core::social::{mojang_has_joined, new_server_id, ClientMsg, Friend, ServerMsg};
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;

const MAX_FRIENDS: usize = 300;
const MAX_MSG_LEN: usize = 1000;

#[derive(Default, Serialize, Deserialize)]
struct Db {
    /// uuid -> last known username
    names: HashMap<String, String>,
    /// sorted pairs (a < b)
    friends: HashSet<(String, String)>,
    /// (from, to)
    requests: HashSet<(String, String)>,
}

fn pair(a: &str, b: &str) -> (String, String) {
    if a < b { (a.into(), b.into()) } else { (b.into(), a.into()) }
}

struct Online {
    tx: mpsc::UnboundedSender<ServerMsg>,
    status: String,
    activity: Option<String>,
}

struct Hub {
    db: Mutex<Db>,
    db_path: std::path::PathBuf,
    /// uuid -> connections (a player may have the launcher open twice)
    online: Mutex<HashMap<String, Vec<(u64, Online)>>>,
    next_id: std::sync::atomic::AtomicU64,
}

impl Hub {
    fn save(&self) {
        let data = serde_json::to_vec_pretty(&*self.db.lock()).unwrap_or_default();
        let tmp = self.db_path.with_extension("tmp");
        if std::fs::write(&tmp, data).is_ok() {
            let _ = std::fs::rename(tmp, &self.db_path);
        }
    }

    fn send(&self, uuid: &str, msg: ServerMsg) -> bool {
        let online = self.online.lock();
        let Some(conns) = online.get(uuid) else { return false };
        for (_, c) in conns {
            let _ = c.tx.send(msg.clone());
        }
        !conns.is_empty()
    }

    fn friend_view(&self, uuid: &str) -> Friend {
        let name = self.db.lock().names.get(uuid).cloned().unwrap_or_else(|| uuid.to_string());
        let online = self.online.lock();
        match online.get(uuid).and_then(|c| c.last()) {
            Some((_, o)) => Friend { uuid: uuid.into(), name, status: o.status.clone(), activity: o.activity.clone() },
            None => Friend { uuid: uuid.into(), name, status: "offline".into(), activity: None },
        }
    }

    fn friends_of(&self, uuid: &str) -> Vec<String> {
        self.db
            .lock()
            .friends
            .iter()
            .filter_map(|(a, b)| if a == uuid { Some(b.clone()) } else if b == uuid { Some(a.clone()) } else { None })
            .collect()
    }

    fn friends_msg(&self, uuid: &str) -> ServerMsg {
        let friends = self.friends_of(uuid).iter().map(|f| self.friend_view(f)).collect();
        let (incoming, outgoing): (Vec<_>, Vec<_>) = {
            let db = self.db.lock();
            let inc: Vec<String> = db.requests.iter().filter(|(_, to)| to == uuid).map(|(f, _)| f.clone()).collect();
            let out: Vec<String> = db.requests.iter().filter(|(f, _)| f == uuid).map(|(_, t)| t.clone()).collect();
            (inc, out)
        };
        ServerMsg::Friends {
            friends,
            incoming: incoming.iter().map(|u| self.friend_view(u)).collect(),
            outgoing: outgoing.iter().map(|u| self.friend_view(u)).collect(),
        }
    }

    fn refresh(&self, uuid: &str) {
        let m = self.friends_msg(uuid);
        self.send(uuid, m);
    }

    fn broadcast_presence(&self, uuid: &str) {
        let me = self.friend_view(uuid);
        for f in self.friends_of(uuid) {
            self.send(&f, ServerMsg::Presence { friend: me.clone() });
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let addr: SocketAddr = std::env::args().nth(1).unwrap_or_else(|| "127.0.0.1:7878".into()).parse()?;
    let db_path = std::path::PathBuf::from(std::env::var("SALT_SOCIAL_DB").unwrap_or_else(|_| "salt-social.json".into()));
    let db: Db = std::fs::read(&db_path).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default();
    let hub = Arc::new(Hub { db: Mutex::new(db), db_path, online: Mutex::new(HashMap::new()), next_id: 0.into() });

    let app = Router::new()
        .route("/", get(|| async { "salt-social ok" }))
        .route("/ws", get(ws_handler))
        .with_state(hub);
    tracing::info!("salt-social listening on {addr}");
    axum::serve(tokio::net::TcpListener::bind(addr).await?, app).await?;
    Ok(())
}

async fn ws_handler(ws: WebSocketUpgrade, State(hub): State<Arc<Hub>>) -> impl IntoResponse {
    ws.max_message_size(16 * 1024).on_upgrade(move |socket| connection(socket, hub))
}

async fn connection(socket: WebSocket, hub: Arc<Hub>) {
    let (mut sink, mut stream) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMsg>();
    let writer = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if sink.send(Message::Text(serde_json::to_string(&msg).unwrap())).await.is_err() {
                break;
            }
        }
    });

    // ---- handshake: hello -> challenge -> (client calls Mojang) -> verify
    let server_id = new_server_id();
    let mut claimed_name = None;
    let mut me: Option<(String, String)> = None;
    let deadline = tokio::time::sleep(Duration::from_secs(30));
    tokio::pin!(deadline);
    while me.is_none() {
        let msg = tokio::select! {
            m = stream.next() => m,
            _ = &mut deadline => break,
        };
        let Some(Ok(Message::Text(text))) = msg else { break };
        match serde_json::from_str::<ClientMsg>(&text) {
            Ok(ClientMsg::Hello { name }) if name.len() <= 16 => {
                claimed_name = Some(name);
                let _ = tx.send(ServerMsg::Challenge { server_id: server_id.clone() });
            }
            Ok(ClientMsg::Verify) => {
                let Some(name) = &claimed_name else { break };
                match mojang_has_joined(name, &server_id).await {
                    Ok(Some(p)) => me = Some((p.id, p.name)),
                    _ => {
                        let _ = tx.send(ServerMsg::Error { message: "Couldn't verify your Minecraft account with Mojang.".into() });
                        break;
                    }
                }
            }
            _ => {}
        }
    }
    let Some((uuid, name)) = me else {
        drop(tx);
        let _ = writer.await;
        return;
    };

    let conn_id = hub.next_id.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    hub.db.lock().names.insert(uuid.clone(), name.clone());
    hub.online.lock().entry(uuid.clone()).or_default().push((conn_id, Online { tx: tx.clone(), status: "online".into(), activity: None }));
    hub.save();
    tracing::info!("{name} connected");
    let _ = tx.send(ServerMsg::Welcome { uuid: uuid.clone(), name: name.clone() });
    hub.refresh(&uuid);
    hub.broadcast_presence(&uuid);

    // ---- main loop with a simple rate limit (30 messages / 10 s)
    let mut window = Instant::now();
    let mut count = 0u32;
    while let Some(Ok(msg)) = stream.next().await {
        let Message::Text(text) = msg else { continue };
        if window.elapsed() > Duration::from_secs(10) {
            window = Instant::now();
            count = 0;
        }
        count += 1;
        if count > 30 {
            let _ = tx.send(ServerMsg::Error { message: "Slow down a little.".into() });
            continue;
        }
        let Ok(msg) = serde_json::from_str::<ClientMsg>(&text) else { continue };
        handle(&hub, &uuid, &name, &tx, msg).await;
    }

    // ---- disconnect
    {
        let mut online = hub.online.lock();
        if let Some(list) = online.get_mut(&uuid) {
            list.retain(|(id, _)| *id != conn_id);
            if list.is_empty() {
                online.remove(&uuid);
            }
        }
    }
    hub.broadcast_presence(&uuid);
    tracing::info!("{name} disconnected");
    drop(tx);
    writer.abort();
}

async fn handle(hub: &Hub, me: &str, my_name: &str, tx: &mpsc::UnboundedSender<ServerMsg>, msg: ClientMsg) {
    let err = |m: &str| {
        let _ = tx.send(ServerMsg::Error { message: m.to_string() });
    };
    match msg {
        ClientMsg::Ping => {
            let _ = tx.send(ServerMsg::Pong);
        }
        ClientMsg::Status { status, activity } => {
            let status = match status.as_str() { "online" | "away" | "playing" => status, _ => "online".into() };
            if let Some(list) = hub.online.lock().get_mut(me) {
                for (_, c) in list.iter_mut() {
                    c.status = status.clone();
                    c.activity = activity.clone().map(|a| a.chars().take(64).collect());
                }
            }
            hub.broadcast_presence(me);
        }
        ClientMsg::FriendRequest { name } => {
            let target = match lookup_uuid(hub, &name).await {
                Some(t) => t,
                None => return err("No Minecraft player has that name."),
            };
            if target == me {
                return err("You can't add yourself.");
            }
            {
                let mut db = hub.db.lock();
                if db.friends.contains(&pair(me, &target)) {
                    drop(db);
                    return err("You're already friends.");
                }
                if db.friends.iter().filter(|(a, b)| a == me || b == me).count() >= MAX_FRIENDS {
                    drop(db);
                    return err("Friend list is full.");
                }
                // If they already asked us, accept instead.
                if db.requests.remove(&(target.clone(), me.to_string())) {
                    db.friends.insert(pair(me, &target));
                } else {
                    db.requests.insert((me.to_string(), target.clone()));
                }
            }
            hub.save();
            hub.refresh(me);
            hub.refresh(&target);
        }
        ClientMsg::FriendAccept { uuid } => {
            let accepted = {
                let mut db = hub.db.lock();
                let ok = db.requests.remove(&(uuid.clone(), me.to_string()));
                if ok {
                    db.friends.insert(pair(me, &uuid));
                }
                ok
            };
            if accepted {
                hub.save();
                hub.refresh(me);
                hub.refresh(&uuid);
            }
        }
        ClientMsg::FriendDecline { uuid } => {
            hub.db.lock().requests.remove(&(uuid.clone(), me.to_string()));
            hub.save();
            hub.refresh(me);
            hub.refresh(&uuid);
        }
        ClientMsg::FriendRemove { uuid } => {
            {
                let mut db = hub.db.lock();
                db.friends.remove(&pair(me, &uuid));
                db.requests.remove(&(me.to_string(), uuid.clone()));
            }
            hub.save();
            hub.refresh(me);
            hub.refresh(&uuid);
        }
        ClientMsg::Dm { to, text } => {
            let text = text.trim().to_string();
            if text.is_empty() || text.len() > MAX_MSG_LEN {
                return err("Messages must be 1–1000 characters.");
            }
            if !hub.db.lock().friends.contains(&pair(me, &to)) {
                return err("You can only message friends.");
            }
            let m = ServerMsg::Dm { from: me.into(), from_name: my_name.into(), to: to.clone(), text, ts: chrono::Utc::now().timestamp_millis() };
            if !hub.send(&to, m.clone()) {
                return err("They're offline — messages aren't stored, so try again when they're on.");
            }
            hub.send(me, m); // echo to all of the sender's windows
        }
        ClientMsg::Invite { to, address, version } => {
            if !hub.db.lock().friends.contains(&pair(me, &to)) {
                return err("You can only invite friends.");
            }
            let address: String = address.trim().chars().take(255).collect();
            if !hub.send(&to, ServerMsg::Invite { from: me.into(), from_name: my_name.into(), address, version: version.chars().take(16).collect() }) {
                err("They're offline.");
            }
        }
        ClientMsg::Hello { .. } | ClientMsg::Verify => {}
    }
}

/// Name → UUID. Known Salt users first, then Mojang's public profile API.
async fn lookup_uuid(hub: &Hub, name: &str) -> Option<String> {
    if !name.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') || name.is_empty() || name.len() > 16 {
        return None;
    }
    if let Some((u, _)) = hub.db.lock().names.iter().find(|(_, n)| n.eq_ignore_ascii_case(name)) {
        return Some(u.clone());
    }
    #[derive(Deserialize)]
    struct P {
        id: String,
        name: String,
    }
    let p: P = salt_core::http::get_json(&format!("https://api.mojang.com/users/profiles/minecraft/{name}")).await.ok()?;
    hub.db.lock().names.insert(p.id.clone(), p.name);
    Some(p.id)
}
