//! Microsoft account sign-in for Minecraft: Java Edition.
//!
//! Privacy model (also written up in PRIVACY.md):
//! * We use Microsoft's **device code flow**. The user types a code on
//!   microsoft.com in their own browser; the launcher never sees their
//!   password and never embeds a login page.
//! * The only hosts contacted here are `login.microsoftonline.com`,
//!   `user.auth.xboxlive.com`, `xsts.auth.xboxlive.com` and
//!   `api.minecraftservices.com`. Nothing is sent anywhere else.
//! * Tokens are stored in the operating system's credential store (Windows
//!   Credential Manager / macOS Keychain / Secret Service), never in plain
//!   files and never in logs. `accounts.json` only holds the public profile:
//!   UUID, username and skin URL.
//! * Scopes are `XboxLive.signin offline_access` — the minimum that lets us
//!   check game ownership and keep the user signed in.

use std::time::Duration;

use anyhow::{anyhow, bail, Context, Result};
use serde::{Deserialize, Serialize};
use serde_json::json;

use crate::http::client;
use crate::secrets;
use crate::Paths;

const MS_DEVICE_CODE: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/devicecode";
const MS_TOKEN: &str = "https://login.microsoftonline.com/consumers/oauth2/v2.0/token";
const XBL_AUTH: &str = "https://user.auth.xboxlive.com/user/authenticate";
const XSTS_AUTH: &str = "https://xsts.auth.xboxlive.com/xsts/authorize";
const MC_LOGIN: &str = "https://api.minecraftservices.com/authentication/login_with_xbox";
const MC_ENTITLEMENTS: &str = "https://api.minecraftservices.com/entitlements/mcstore";
pub const MC_PROFILE: &str = "https://api.minecraftservices.com/minecraft/profile";
const SCOPE: &str = "XboxLive.signin offline_access";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    pub expires_in: u64,
    pub interval: u64,
    pub message: String,
}

/// Public, non-secret account info. Safe to write to disk and show in the UI.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Account {
    pub uuid: String,
    pub username: String,
    #[serde(default)]
    pub skin_url: Option<String>,
    #[serde(default)]
    pub xuid: Option<String>,
    /// How ownership was proven. Missing in older files = Microsoft.
    #[serde(default)]
    pub kind: AccountKind,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum AccountKind {
    /// Microsoft sign-in: has tokens, plays online-mode servers.
    #[default]
    Microsoft,
    /// "Sign in with Minecraft" (see `verify`): ownership proven by joining
    /// Salt's local sign-in server. No tokens; launches in offline mode.
    Verified,
}

impl Account {
    /// The account for someone who proved ownership via `verify`.
    pub fn verified(p: crate::verify::VerifiedProfile) -> Self {
        Self { uuid: p.uuid, username: p.name, skin_url: p.skin_url, xuid: None, kind: AccountKind::Verified }
    }
}

/// A ready-to-use session for launching the game. Lives in memory only.
#[derive(Clone)]
pub struct Session {
    pub account: Account,
    pub access_token: String,
}

impl std::fmt::Debug for Session {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // Never print the token, even in debug output.
        f.debug_struct("Session").field("account", &self.account).field("access_token", &"<redacted>").finish()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AccountStore {
    pub accounts: Vec<Account>,
    pub active: Option<String>,
}

impl AccountStore {
    pub fn load(paths: &Paths) -> Self {
        std::fs::read(paths.accounts_file())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, paths: &Paths) -> Result<()> {
        std::fs::write(paths.accounts_file(), serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn upsert(&mut self, account: Account) {
        self.active = Some(account.uuid.clone());
        if let Some(existing) = self.accounts.iter_mut().find(|a| a.uuid == account.uuid) {
            *existing = account;
        } else {
            self.accounts.push(account);
        }
    }

    pub fn remove(&mut self, uuid: &str) {
        self.accounts.retain(|a| a.uuid != uuid);
        if self.active.as_deref() == Some(uuid) {
            self.active = self.accounts.first().map(|a| a.uuid.clone());
        }
        secrets::delete(&format!("{uuid}:refresh"));
        secrets::delete(&format!("{uuid}:minecraft"));
    }

    pub fn active_account(&self) -> Option<&Account> {
        let id = self.active.as_deref()?;
        self.accounts.iter().find(|a| a.uuid == id)
    }
}

// ---------------------------------------------------------------- device code

pub async fn start_device_code(client_id: &str) -> Result<DeviceCode> {
    if client_id.trim().is_empty() {
        bail!("No Microsoft client ID is configured. See docs/MICROSOFT_LOGIN.md.");
    }
    let res = client()
        .post(MS_DEVICE_CODE)
        .form(&[("client_id", client_id), ("scope", SCOPE)])
        .send()
        .await?;
    if !res.status().is_success() {
        let body = res.text().await.unwrap_or_default();
        bail!("Microsoft rejected the sign-in request: {}", short_error(&body));
    }
    Ok(res.json().await?)
}

#[derive(Deserialize)]
struct MsToken {
    access_token: String,
    refresh_token: Option<String>,
}

#[derive(Deserialize)]
struct MsError {
    error: String,
    #[serde(default)]
    error_description: String,
}

/// Polls until the user finishes signing in on microsoft.com, then runs the
/// rest of the chain and stores the tokens.
pub async fn poll_device_code(paths: &Paths, client_id: &str, code: &DeviceCode) -> Result<Session> {
    let mut interval = code.interval.max(1);
    let deadline = std::time::Instant::now() + Duration::from_secs(code.expires_in);
    loop {
        if std::time::Instant::now() > deadline {
            bail!("The sign-in code expired. Please try again.");
        }
        tokio::time::sleep(Duration::from_secs(interval)).await;
        let res = client()
            .post(MS_TOKEN)
            .form(&[
                ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
                ("client_id", client_id),
                ("device_code", code.device_code.as_str()),
            ])
            .send()
            .await?;
        if res.status().is_success() {
            let token: MsToken = res.json().await?;
            return finish_login(paths, &token.access_token, token.refresh_token.as_deref()).await;
        }
        let err: MsError = res.json().await.unwrap_or(MsError { error: "unknown".into(), error_description: String::new() });
        match err.error.as_str() {
            "authorization_pending" => continue,
            "slow_down" => interval += 5,
            "authorization_declined" => bail!("Sign-in was cancelled."),
            "expired_token" => bail!("The sign-in code expired. Please try again."),
            _ => bail!("Sign-in failed: {} {}", err.error, short_error(&err.error_description)),
        }
    }
}

/// Signs the account in again using its stored refresh token. Re-uses the
/// cached Minecraft token while it is still valid.
pub async fn refresh(paths: &Paths, client_id: &str, account: &Account) -> Result<Session> {
    if let Some(cached) = secrets::get(&format!("{}:minecraft", account.uuid)) {
        if let Ok(c) = serde_json::from_str::<CachedMcToken>(&cached) {
            // Keep a 10 minute safety margin before expiry.
            if c.expires_at > chrono::Utc::now().timestamp() + 600 {
                return Ok(Session { account: account.clone(), access_token: c.token });
            }
        }
    }
    let refresh_token = secrets::get(&format!("{}:refresh", account.uuid))
        .ok_or_else(|| anyhow!("You've been signed out of {}. Please sign in again.", account.username))?;
    let res = client()
        .post(MS_TOKEN)
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id),
            ("refresh_token", refresh_token.as_str()),
            ("scope", SCOPE),
        ])
        .send()
        .await?;
    if !res.status().is_success() {
        bail!("Your Microsoft session expired. Please sign in again.");
    }
    let token: MsToken = res.json().await?;
    finish_login(paths, &token.access_token, token.refresh_token.as_deref()).await
}

#[derive(Serialize, Deserialize)]
struct CachedMcToken {
    token: String,
    expires_at: i64,
}

async fn finish_login(paths: &Paths, ms_access: &str, ms_refresh: Option<&str>) -> Result<Session> {
    // 1. Xbox Live user token
    let xbl: XboxToken = post_json(
        XBL_AUTH,
        json!({
            "Properties": { "AuthMethod": "RPS", "SiteName": "user.auth.xboxlive.com", "RpsTicket": format!("d={ms_access}") },
            "RelyingParty": "http://auth.xboxlive.com",
            "TokenType": "JWT"
        }),
    )
    .await
    .context("Xbox Live sign-in failed")?;

    // 2. XSTS token for the Minecraft relying party
    let res = client()
        .post(XSTS_AUTH)
        .json(&json!({
            "Properties": { "SandboxId": "RETAIL", "UserTokens": [xbl.token] },
            "RelyingParty": "rp://api.minecraftservices.com/",
            "TokenType": "JWT"
        }))
        .send()
        .await?;
    if res.status().as_u16() == 401 {
        let body: serde_json::Value = res.json().await.unwrap_or_default();
        let code = body["XErr"].as_u64().unwrap_or(0);
        bail!(xsts_error(code));
    }
    if !res.status().is_success() {
        bail!("Xbox security token request failed (HTTP {})", res.status());
    }
    let xsts: XboxToken = res.json().await?;
    let uhs = xsts.uhs().ok_or_else(|| anyhow!("Xbox did not return a user hash"))?;

    // 3. Minecraft access token
    let mc: McLogin = post_json(MC_LOGIN, json!({ "identityToken": format!("XBL3.0 x={uhs};{}", xsts.token) }))
        .await
        .context("Minecraft services sign-in failed")?;

    // 4. Ownership: the profile only exists if the account owns Java Edition
    //    (including Game Pass). Entitlements are checked for a clearer error.
    let profile = fetch_profile(&mc.access_token).await?;
    let profile = match profile {
        Some(p) => p,
        None => {
            let owns = owns_game(&mc.access_token).await.unwrap_or(false);
            if owns {
                bail!("This account owns Minecraft but hasn't picked a username yet. Visit minecraft.net to finish setup.");
            }
            bail!("This Microsoft account doesn't own Minecraft: Java Edition.");
        }
    };

    let account = Account {
        uuid: profile.id.clone(),
        username: profile.name.clone(),
        skin_url: profile.skins.iter().find(|s| s.state == "ACTIVE").map(|s| s.url.clone()),
        xuid: xsts.xuid(),
        kind: AccountKind::Microsoft,
    };

    if let Some(r) = ms_refresh {
        secrets::set(&format!("{}:refresh", account.uuid), r)?;
    }
    let cached = CachedMcToken {
        token: mc.access_token.clone(),
        expires_at: chrono::Utc::now().timestamp() + mc.expires_in as i64,
    };
    secrets::set(&format!("{}:minecraft", account.uuid), &serde_json::to_string(&cached)?)?;

    let mut store = AccountStore::load(paths);
    store.upsert(account.clone());
    store.save(paths)?;

    Ok(Session { account, access_token: mc.access_token })
}

fn xsts_error(code: u64) -> &'static str {
    match code {
        2148916233 => "This Microsoft account has no Xbox profile. Sign in once at xbox.com to create one, then try again.",
        2148916235 => "Xbox Live isn't available in your country/region.",
        2148916236 | 2148916237 => "This account needs adult verification on xbox.com (South Korea).",
        2148916238 => "This is a child account. An adult must add it to a Microsoft Family before it can sign in.",
        _ => "Xbox Live refused the sign-in for this account.",
    }
}

#[derive(Deserialize)]
struct XboxToken {
    #[serde(rename = "Token")]
    token: String,
    #[serde(rename = "DisplayClaims")]
    display_claims: serde_json::Value,
}

impl XboxToken {
    fn uhs(&self) -> Option<String> {
        self.display_claims["xui"][0]["uhs"].as_str().map(str::to_string)
    }
    fn xuid(&self) -> Option<String> {
        self.display_claims["xui"][0]["xid"].as_str().map(str::to_string)
    }
}

#[derive(Deserialize)]
struct McLogin {
    access_token: String,
    expires_in: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub skins: Vec<McSkin>,
    #[serde(default)]
    pub capes: Vec<McCape>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McSkin {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub variant: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McCape {
    pub id: String,
    pub state: String,
    pub url: String,
    #[serde(default)]
    pub alias: String,
}

pub async fn fetch_profile(token: &str) -> Result<Option<McProfile>> {
    let res = client().get(MC_PROFILE).bearer_auth(token).send().await?;
    if res.status().as_u16() == 404 {
        return Ok(None);
    }
    if !res.status().is_success() {
        bail!("Couldn't load your Minecraft profile (HTTP {})", res.status());
    }
    Ok(Some(res.json().await?))
}

async fn owns_game(token: &str) -> Result<bool> {
    let v: serde_json::Value = client().get(MC_ENTITLEMENTS).bearer_auth(token).send().await?.json().await?;
    Ok(v["items"]
        .as_array()
        .map(|items| items.iter().any(|i| matches!(i["name"].as_str(), Some("product_minecraft" | "game_minecraft"))))
        .unwrap_or(false))
}

async fn post_json<T: serde::de::DeserializeOwned>(url: &str, body: serde_json::Value) -> Result<T> {
    let res = client().post(url).header("Accept", "application/json").json(&body).send().await?;
    if !res.status().is_success() {
        bail!("HTTP {}", res.status());
    }
    Ok(res.json().await?)
}

/// Keeps error bodies short and token-free before they reach the UI.
fn short_error(s: &str) -> String {
    s.lines().next().unwrap_or("").chars().take(200).collect()
}
