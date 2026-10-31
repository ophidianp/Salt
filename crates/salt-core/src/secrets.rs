//! OS credential store wrapper. Windows Credential Manager caps a secret at
//! 2560 bytes, and Microsoft refresh tokens can get close to that, so long
//! values are split across numbered entries.

const KEYRING_SERVICE: &str = "SaltLauncher";

const CHUNK: usize = 1000;

fn entry(key: &str) -> Option<keyring::Entry> {
    keyring::Entry::new(KEYRING_SERVICE, key).ok()
}

pub fn set(key: &str, value: &str) -> anyhow::Result<()> {
    delete(key);
    let chars: Vec<char> = value.chars().collect();
    let chunks: Vec<String> = chars.chunks(CHUNK).map(|c| c.iter().collect()).collect();
    for (i, chunk) in chunks.iter().enumerate() {
        entry(&format!("{key}#{i}"))
            .ok_or_else(|| anyhow::anyhow!("credential store unavailable"))?
            .set_password(chunk)?;
    }
    entry(&format!("{key}#n"))
        .ok_or_else(|| anyhow::anyhow!("credential store unavailable"))?
        .set_password(&chunks.len().to_string())?;
    Ok(())
}

pub fn get(key: &str) -> Option<String> {
    let n: usize = entry(&format!("{key}#n"))?.get_password().ok()?.parse().ok()?;
    let mut out = String::new();
    for i in 0..n {
        out.push_str(&entry(&format!("{key}#{i}"))?.get_password().ok()?);
    }
    Some(out)
}

pub fn delete(key: &str) {
    if let Some(n) = entry(&format!("{key}#n")).and_then(|e| e.get_password().ok()).and_then(|s| s.parse::<usize>().ok()) {
        for i in 0..n {
            if let Some(e) = entry(&format!("{key}#{i}")) {
                let _ = e.delete_credential();
            }
        }
    }
    if let Some(e) = entry(&format!("{key}#n")) {
        let _ = e.delete_credential();
    }
}
