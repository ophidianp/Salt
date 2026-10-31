//! Runs "Sign in with Minecraft" on its own, for testing the protocol with a
//! real game: `cargo run -p salt-core --example verify_listen`, then join the
//! printed address from Minecraft.

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt().with_env_filter("info").init();
    let listener = salt_core::verify::listen().await?;
    println!("join {} from Minecraft", listener.address());
    match listener.wait(std::time::Duration::from_secs(300)).await {
        Ok(p) => println!("VERIFIED {} {} skin={:?}", p.name, p.uuid, p.skin_url),
        Err(e) => println!("not verified: {e:#}"),
    }
    Ok(())
}
