//! "Sign in with Minecraft": proves the player owns Minecraft: Java Edition
//! without Microsoft sign-in (which needs an Azure app approved by Mojang).
//!
//! How it works: Salt runs a tiny Minecraft server on this computer
//! (127.0.0.1 only). The player joins it from their normal Minecraft. Their
//! game then does exactly what it does for any online-mode server: it tells
//! Mojang's session server "I'm joining server X". We ask Mojang
//! (`hasJoined`) whether that's true. Mojang only says yes for a real,
//! paid-for account that is signed in, so a yes proves ownership. Then the
//! player is kicked with a "signed in" message.
//!
//! Privacy: no password, token or Microsoft data ever reaches Salt. We learn
//! the public profile (UUID, name, skin) from Mojang's public session server,
//! the only host contacted. Nothing is stored except that public profile.
//!
//! Limits: a verified account has no Minecraft access token, so Salt launches
//! it in offline mode. Singleplayer, LAN and offline-mode servers work;
//! online-mode servers still need Microsoft sign-in.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use rsa::pkcs8::EncodePublicKey;
use rsa::{Pkcs1v15Encrypt, RsaPrivateKey};
use serde::Deserialize;
use serde_json::json;
use sha1::{Digest, Sha1};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::mpsc;

/// The port Salt tries first, so the address is the same every time.
pub const PREFERRED_PORT: u16 = 25599;
const HAS_JOINED: &str = "https://sessionserver.mojang.com/session/minecraft/hasJoined";
/// The 1.19–1.19.2 login format (chat signing) differs and isn't supported.
const UNSUPPORTED: std::ops::RangeInclusive<i32> = 759..=760;
/// 1.20.5+ sends a "should authenticate" flag with the encryption request.
const SHOULD_AUTHENTICATE_SINCE: i32 = 766;

/// The owner of the account that joined.
#[derive(Debug, Clone)]
pub struct VerifiedProfile {
    pub uuid: String,
    pub name: String,
    pub skin_url: Option<String>,
}

/// A running sign-in listener. Dropping it stops listening.
pub struct Listener {
    pub port: u16,
    rx: mpsc::Receiver<VerifiedProfile>,
    task: tokio::task::JoinHandle<()>,
}

impl Listener {
    /// "localhost:25599" — what the player types into Direct Connection.
    pub fn address(&self) -> String {
        if self.port == 25565 {
            "localhost".into()
        } else {
            format!("localhost:{}", self.port)
        }
    }

    /// Waits until someone who owns Minecraft joins (or `timeout` passes).
    pub async fn wait(mut self, timeout: Duration) -> Result<VerifiedProfile> {
        tokio::time::timeout(timeout, self.rx.recv())
            .await
            .map_err(|_| anyhow!("Sign-in timed out. Start it again when you're ready."))?
            .ok_or_else(|| anyhow!("Sign-in stopped."))
    }
}

impl Drop for Listener {
    fn drop(&mut self) {
        self.task.abort();
    }
}

/// Starts the local sign-in server.
pub async fn listen() -> Result<Listener> {
    let listener = match TcpListener::bind(("127.0.0.1", PREFERRED_PORT)).await {
        Ok(l) => l,
        Err(_) => TcpListener::bind(("127.0.0.1", 0)).await.context("couldn't open a local port for sign-in")?,
    };
    let port = listener.local_addr()?.port();
    // 1024-bit like vanilla servers; generating it takes a moment, so off the async threads.
    let key = tokio::task::spawn_blocking(|| RsaPrivateKey::new(&mut rand::thread_rng(), 1024)).await??;
    let public_der = key.to_public_key().to_public_key_der()?.as_bytes().to_vec();
    let (tx, rx) = mpsc::channel(1);
    let task = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else { continue };
            let (key, public_der, tx) = (key.clone(), public_der.clone(), tx.clone());
            tokio::spawn(async move {
                // One bad or curious connection must never end the sign-in.
                let result = tokio::time::timeout(Duration::from_secs(30), handle(stream, &key, &public_der)).await;
                if let Ok(Ok(Some(profile))) = result {
                    let _ = tx.send(profile).await;
                }
            });
        }
    });
    Ok(Listener { port, rx, task })
}

// ------------------------------------------------------------------ protocol

async fn handle(mut s: TcpStream, key: &RsaPrivateKey, public_der: &[u8]) -> Result<Option<VerifiedProfile>> {
    s.set_nodelay(true)?;
    let (id, body) = read_packet(&mut s).await?;
    if id != 0x00 {
        return Ok(None);
    }
    let mut hs = Reader::new(&body);
    let protocol = hs.varint()?;
    let _host = hs.string()?;
    let _port = hs.u16()?;
    let next = hs.varint()?;
    match next {
        1 => status(&mut s, protocol).await.map(|_| None),
        2 => login(&mut s, protocol, key, public_der).await,
        _ => Ok(None),
    }
}

/// Server-list entry, so the player sees what this "server" is before joining.
async fn status(s: &mut TcpStream, protocol: i32) -> Result<()> {
    let (id, _) = read_packet(s).await?;
    if id != 0x00 {
        return Ok(());
    }
    let motd = json!({
        "version": { "name": "Salt sign-in", "protocol": protocol },
        "players": { "max": 1, "online": 0 },
        "description": [
            { "text": "Salt Launcher sign-in\n", "color": "aqua", "bold": true },
            { "text": "Join to prove you own Minecraft", "color": "gray" }
        ]
    });
    let mut out = Vec::new();
    write_string(&mut out, &motd.to_string());
    write_packet(s, 0x00, &out, None).await?;
    // Ping → pong with the same payload.
    if let Ok((0x01, payload)) = read_packet(s).await {
        write_packet(s, 0x01, &payload, None).await?;
    }
    Ok(())
}

async fn login(s: &mut TcpStream, protocol: i32, key: &RsaPrivateKey, public_der: &[u8]) -> Result<Option<VerifiedProfile>> {
    let (id, body) = read_packet(s).await?;
    if id != 0x00 {
        bail!("expected login start");
    }
    let name = Reader::new(&body).string()?;
    tracing::info!("sign-in: {name} joined (protocol {protocol})");
    if UNSUPPORTED.contains(&protocol) {
        kick(s, None, &rejected("Minecraft 1.19–1.19.2 can't sign in to Salt. Use any other version.")).await?;
        return Ok(None);
    }

    // Encryption request: the same handshake every online-mode server does.
    let verify_token: [u8; 4] = rand::random();
    let mut req = Vec::new();
    write_string(&mut req, "");
    write_bytes(&mut req, public_der);
    write_bytes(&mut req, &verify_token);
    if protocol >= SHOULD_AUTHENTICATE_SINCE {
        req.push(1);
    }
    write_packet(s, 0x01, &req, None).await?;

    let (id, body) = read_packet(s).await?;
    if id != 0x01 {
        bail!("expected encryption response");
    }
    let mut r = Reader::new(&body);
    let secret = key.decrypt(Pkcs1v15Encrypt, r.bytes()?).context("bad shared secret")?;
    let token = key.decrypt(Pkcs1v15Encrypt, r.bytes()?).context("bad verify token")?;
    if secret.len() != 16 || token != verify_token {
        bail!("encryption handshake didn't match");
    }

    // Everything from here on is encrypted (AES/CFB8, key = IV = secret).
    let server_id = server_hash(&secret, public_der);
    tracing::info!("sign-in: encryption ok, asking Mojang about {name}");
    let profile = match has_joined(&name, &server_id).await {
        Ok(Some(p)) => p,
        Ok(None) => {
            tracing::info!("sign-in: Mojang says {name} didn't join (not owned or not signed in)");
            kick(s, Some(&secret), &rejected("Mojang couldn't confirm this account owns Minecraft. Make sure you're signed in to your normal Minecraft launcher, then join again.")).await?;
            return Ok(None);
        }
        Err(e) => {
            kick(s, Some(&secret), &rejected("Couldn't reach Mojang's session server. Check your internet and try again.")).await?;
            return Err(e);
        }
    };
    let ok = json!([
        { "text": "Salt Launcher\n\n", "color": "aqua", "bold": true },
        { "text": format!("Signed in as {}.\n", profile.name), "color": "white" },
        { "text": "You can close Minecraft and go back to Salt.", "color": "gray" }
    ]);
    kick(s, Some(&secret), &ok).await?;
    Ok(Some(profile))
}

fn rejected(why: &str) -> serde_json::Value {
    json!([
        { "text": "Salt Launcher\n\n", "color": "aqua", "bold": true },
        { "text": why, "color": "white" }
    ])
}

/// Login-phase Disconnect (0x00) with a chat-component reason.
async fn kick(s: &mut TcpStream, secret: Option<&[u8]>, reason: &serde_json::Value) -> Result<()> {
    let mut out = Vec::new();
    write_string(&mut out, &reason.to_string());
    write_packet(s, 0x00, &out, secret).await?;
    s.flush().await?;
    // Let the client read it before the socket closes.
    tokio::time::sleep(Duration::from_millis(200)).await;
    Ok(())
}

#[derive(Deserialize)]
struct Joined {
    id: String,
    name: String,
    #[serde(default)]
    properties: Vec<Property>,
}

#[derive(Deserialize)]
struct Property {
    name: String,
    value: String,
}

/// Asks Mojang whether `name` really joined our server (i.e. owns the game and is signed in).
async fn has_joined(name: &str, server_id: &str) -> Result<Option<VerifiedProfile>> {
    let res = crate::http::client()
        .get(HAS_JOINED)
        .query(&[("username", name), ("serverId", server_id)])
        .timeout(Duration::from_secs(15))
        .send()
        .await?;
    if res.status() == reqwest::StatusCode::NO_CONTENT || res.status() == reqwest::StatusCode::FORBIDDEN {
        return Ok(None);
    }
    if !res.status().is_success() {
        bail!("session server returned HTTP {}", res.status());
    }
    let j: Joined = res.json().await?;
    Ok(Some(VerifiedProfile { uuid: j.id, name: j.name, skin_url: skin_url(&j.properties) }))
}

/// The skin URL from the profile's base64 "textures" property.
fn skin_url(props: &[Property]) -> Option<String> {
    use base64::Engine;
    let p = props.iter().find(|p| p.name == "textures")?;
    let raw = base64::engine::general_purpose::STANDARD.decode(&p.value).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&raw).ok()?;
    v["textures"]["SKIN"]["url"].as_str().map(str::to_string)
}

/// Minecraft's server hash: SHA-1 of (server id "" + secret + public key),
/// printed as a signed (two's complement) hex number.
pub fn server_hash(secret: &[u8], public_der: &[u8]) -> String {
    let mut h = Sha1::new();
    h.update(secret);
    h.update(public_der);
    mc_hex(h.finalize().into())
}

pub fn mc_hex(mut digest: [u8; 20]) -> String {
    let negative = digest[0] & 0x80 != 0;
    if negative {
        // Two's complement: invert, then add one.
        let mut carry = true;
        for b in digest.iter_mut().rev() {
            *b = !*b;
            if carry {
                let (v, c) = b.overflowing_add(1);
                *b = v;
                carry = c;
            }
        }
    }
    let hex = hex::encode(digest);
    let trimmed = hex.trim_start_matches('0');
    format!("{}{}", if negative { "-" } else { "" }, if trimmed.is_empty() { "0" } else { trimmed })
}

// ------------------------------------------------------------------ framing

async fn read_varint(s: &mut TcpStream) -> Result<i32> {
    let mut value = 0i32;
    for i in 0..5 {
        let b = s.read_u8().await?;
        value |= ((b & 0x7F) as i32) << (7 * i);
        if b & 0x80 == 0 {
            return Ok(value);
        }
    }
    bail!("varint too long")
}

async fn read_packet(s: &mut TcpStream) -> Result<(i32, Vec<u8>)> {
    let len = read_varint(s).await?;
    if !(1..=32 * 1024).contains(&len) {
        bail!("bad packet length {len}");
    }
    let mut buf = vec![0; len as usize];
    s.read_exact(&mut buf).await?;
    let mut r = Reader::new(&buf);
    let id = r.varint()?;
    Ok((id, buf[r.pos..].to_vec()))
}

async fn write_packet(s: &mut TcpStream, id: i32, body: &[u8], secret: Option<&[u8]>) -> Result<()> {
    let mut inner = Vec::new();
    write_varint(&mut inner, id);
    inner.extend_from_slice(body);
    let mut out = Vec::new();
    write_varint(&mut out, inner.len() as i32);
    out.extend_from_slice(&inner);
    if let Some(key) = secret {
        use cfb8::cipher::{AsyncStreamCipher, KeyIvInit};
        cfb8::Encryptor::<aes::Aes128>::new_from_slices(key, key)
            .map_err(|_| anyhow!("bad key"))?
            .encrypt(&mut out);
    }
    s.write_all(&out).await?;
    Ok(())
}

fn write_varint(out: &mut Vec<u8>, v: i32) {
    let mut v = v as u32;
    loop {
        if v & !0x7F == 0 {
            out.push(v as u8);
            return;
        }
        out.push((v & 0x7F | 0x80) as u8);
        v >>= 7;
    }
}

fn write_bytes(out: &mut Vec<u8>, b: &[u8]) {
    write_varint(out, b.len() as i32);
    out.extend_from_slice(b);
}

fn write_string(out: &mut Vec<u8>, s: &str) {
    write_bytes(out, s.as_bytes());
}

struct Reader<'a> {
    b: &'a [u8],
    pos: usize,
}

impl<'a> Reader<'a> {
    fn new(b: &'a [u8]) -> Self {
        Self { b, pos: 0 }
    }

    fn take(&mut self, n: usize) -> Result<&'a [u8]> {
        let end = self.pos.checked_add(n).filter(|&e| e <= self.b.len()).ok_or_else(|| anyhow!("packet too short"))?;
        let out = &self.b[self.pos..end];
        self.pos = end;
        Ok(out)
    }

    fn varint(&mut self) -> Result<i32> {
        let mut value = 0i32;
        for i in 0..5 {
            let b = self.take(1)?[0];
            value |= ((b & 0x7F) as i32) << (7 * i);
            if b & 0x80 == 0 {
                return Ok(value);
            }
        }
        bail!("varint too long")
    }

    fn u16(&mut self) -> Result<u16> {
        let b = self.take(2)?;
        Ok(u16::from_be_bytes([b[0], b[1]]))
    }

    fn bytes(&mut self) -> Result<&'a [u8]> {
        let n = self.varint()?;
        if !(0..=4096).contains(&n) {
            bail!("bad length");
        }
        self.take(n as usize)
    }

    fn string(&mut self) -> Result<String> {
        Ok(String::from_utf8(self.bytes()?.to_vec())?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sha(s: &str) -> [u8; 20] {
        Sha1::digest(s.as_bytes()).into()
    }

    #[test]
    fn minecraft_hex_digest() {
        // The examples from the protocol documentation.
        assert_eq!(mc_hex(sha("Notch")), "4ed1f46bbe04bc756bcb17c0c7ce3e4632f06a48");
        assert_eq!(mc_hex(sha("jeb_")), "-7c9d5b0044c130109a5d7b5fb5c317c02b4e28c1");
        assert_eq!(mc_hex(sha("simon")), "88e16a1019277b15d58faf0541e11910eb756f6");
    }
}
