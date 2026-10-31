use std::sync::OnceLock;
use std::time::Duration;

/// One shared HTTP client. No cookies, no analytics headers — only a plain
/// User-Agent, which Modrinth's API terms require.
pub fn client() -> &'static reqwest::Client {
    static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
    CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .user_agent(format!(
                "SaltLauncher/{} (github.com/salt-launcher)",
                crate::LAUNCHER_VERSION
            ))
            .connect_timeout(Duration::from_secs(15))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build HTTP client")
    })
}

pub async fn get_json<T: serde::de::DeserializeOwned>(url: &str) -> anyhow::Result<T> {
    let res = client().get(url).send().await?;
    let status = res.status();
    if !status.is_success() {
        anyhow::bail!("GET {url} failed: HTTP {status}");
    }
    Ok(res.json::<T>().await?)
}
