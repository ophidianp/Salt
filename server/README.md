# salt-social

Friends, presence ("Playing 26.3"), direct messages and world invites for
Salt Launcher. One small Rust binary; host it anywhere.

```powershell
cargo run -p salt-social --release -- 0.0.0.0:7878
```

* Data file: `salt-social.json` (override with `SALT_SOCIAL_DB`). It stores
  usernames, UUIDs, friendships and pending requests. **Messages are relayed
  live and never stored.**
* Identity: the launcher calls Mojang's `session/minecraft/join`, the server
  checks `hasJoined` — exactly what Minecraft servers do. The server never
  receives an access token.
* Limits: 30 messages / 10 s per connection, 1000-character messages,
  300 friends.

## Production

Put it behind TLS and point Salt at `wss://…/ws` (Settings → Friends). With
Caddy:

```
social.example.com {
    reverse_proxy /ws 127.0.0.1:7878
}
```

To ship Salt with a default server, change `social_server` in
`crates/salt-core/src/settings.rs`.
