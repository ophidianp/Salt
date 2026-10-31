//! Friends + chat protocol shared by the launcher and `salt-social` server.
//!
//! Identity is proven the same way Minecraft servers do it: the server sends
//! a random `server_id`, the launcher tells Mojang "I'm joining server_id"
//! (`/session/minecraft/join`, using the access token), and the server asks
//! Mojang `hasJoined?username=…&serverId=…`. The social server therefore
//! learns your UUID and username — and never sees your access token.

use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::client;

const JOIN: &str = "https://sessionserver.mojang.com/session/minecraft/join";
const HAS_JOINED: &str = "https://sessionserver.mojang.com/session/minecraft/hasJoined";

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ClientMsg {
    Hello { name: String },
    Verify,
    Status { status: String, activity: Option<String> },
    FriendRequest { name: String },
    FriendAccept { uuid: String },
    FriendDecline { uuid: String },
    FriendRemove { uuid: String },
    Dm { to: String, text: String },
    /// Invite a friend to a world hosted with e4mc / a server address.
    Invite { to: String, address: String, version: String },
    Ping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Friend {
    pub uuid: String,
    pub name: String,
    pub status: String,
    pub activity: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "t", rename_all = "snake_case")]
pub enum ServerMsg {
    Challenge { server_id: String },
    Welcome { uuid: String, name: String },
    Friends { friends: Vec<Friend>, incoming: Vec<Friend>, outgoing: Vec<Friend> },
    Presence { friend: Friend },
    Dm { from: String, from_name: String, to: String, text: String, ts: i64 },
    Invite { from: String, from_name: String, address: String, version: String },
    Error { message: String },
    Pong,
}

/// Launcher side: tell Mojang we're "joining" the social server.
pub async fn mojang_join(access_token: &str, uuid: &str, server_id: &str) -> Result<()> {
    let res = client()
        .post(JOIN)
        .json(&json!({ "accessToken": access_token, "selectedProfile": uuid.replace('-', ""), "serverId": server_id }))
        .send()
        .await?;
    if !res.status().is_success() {
        bail!("Mojang session check failed (HTTP {})", res.status());
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
pub struct VerifiedProfile {
    pub id: String,
    pub name: String,
}

/// Server side: confirm the player really did the join above.
pub async fn mojang_has_joined(username: &str, server_id: &str) -> Result<Option<VerifiedProfile>> {
    let url = format!("{HAS_JOINED}?username={}&serverId={}", urlencoding::encode(username), urlencoding::encode(server_id));
    let res = client().get(url).send().await?;
    if res.status().as_u16() == 200 {
        Ok(Some(res.json().await?))
    } else {
        Ok(None)
    }
}

/// Random 20-char hex id, like vanilla servers use.
pub fn new_server_id() -> String {
    use rand::Rng;
    let bytes: [u8; 10] = rand::thread_rng().gen();
    hex::encode(bytes)
}
