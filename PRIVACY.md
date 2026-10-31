# Salt Launcher privacy

Short version: **Salt only checks that you own Minecraft, and keeps your
sign-in on your PC.** No analytics, no telemetry, no ads.

## Signing in

* Microsoft **device code** sign-in, in your own browser. Salt never sees or
  stores your Microsoft password.
* Scopes requested: `XboxLive.signin offline_access` — the minimum needed to
  prove you own Minecraft and to stay signed in.
* The chain Salt runs: Microsoft → Xbox Live → XSTS → Minecraft services →
  `GET /minecraft/profile` (does this account own Java Edition?). Nothing else
  is read from your account.

## Where your data lives

| What | Where | Leaves your PC? |
|---|---|---|
| Microsoft refresh token, Minecraft token | Windows Credential Manager (encrypted by Windows) | Only to Microsoft / Mojang, to sign in and launch |
| Username, UUID, skin URL | `%APPDATA%\SaltLauncher\accounts.json` | No |
| CurseForge API key | Windows Credential Manager | Only to api.curseforge.com |
| Settings, instances, logs, skin library | `%APPDATA%\SaltLauncher\` | No |

Tokens are never written to plain files, never printed to logs, and are
scrubbed from the launch command and game output shown in the Logs page.

## Who Salt talks to

* **Microsoft / Xbox / Mojang** — sign-in, skins & capes, game downloads.
* **Adoptium** — Java runtimes.
* **Fabric / Legacy Fabric meta** — mod loader files.
* **Modrinth, CurseForge, GitHub** — only the mods/packs you browse or the
  Salt bundle installs (GitHub for Chlorine releases).
* **Your friends server** — only if you turn Friends on. It identifies you
  with Mojang's standard `hasJoined` check (the same one every Minecraft
  server uses), so it learns your **username and UUID only — never your
  tokens**. Messages are relayed live and not stored.

## Removing your data

Settings → Accounts → **Sign out** deletes that account's tokens from
Windows Credential Manager. Deleting `%APPDATA%\SaltLauncher` removes
everything else.
