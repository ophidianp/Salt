# Salt mods

Client-side Fabric mods built for Salt Launcher. Each is its own jar, named
`salt<Mod>`:

| Mod | What it does | Keys |
|---|---|---|
| **saltHUD** | FPS, CPS, ping, coordinates/biome/facing, keystrokes, armor + durability, potion timers, clock, speed, memory, server, combo, reach, resource pack, custom crosshair. Layout comes from the launcher's HUD manager (`config/salthud.json`). | Right Shift: in-game editor (drag to move, scroll to resize) |
| **saltMenu** | Replaces Minecraft's title screen with Salt's clean black one (Singleplayer, Multiplayer, Mods, Options). "Classic menu" opens the normal one. | — |
| **saltZoom** | Smooth zoom, scroll to zoom further, cinematic camera while zoomed | Hold C |
| **saltFreelook** | Look around without turning (third-person orbit) | Hold Left Alt |
| **saltToggleSprint** | Toggle sprint / sneak with an on-screen indicator | V / B |
| **saltFullbright** | Full brightness without touching your gamma setting | G |

All keys are rebindable under Controls → **Salt**.

## Building

```powershell
powershell -ExecutionPolicy Bypass -File mods\build-all.ps1 -Chlorine "<path to your Chlorine checkout>"
```

This builds every version folder here, plus your Chlorine checkout, and copies
the jars to `%APPDATA%\SaltLauncher\salt-mods\<version>\`. The launcher installs
them into matching instances on the next launch.

Requirements: a Java 25+ JDK (the script finds `~\.jdks\*` automatically); it also builds the Java 21 targets.

## Versions

Every folder is one API band; `targets.txt` lists the Minecraft versions its
jars are installed for. Only `26.3/` is edited by hand — the rest are generated.

| Folder | Installed for | Toolchain | Status |
|---|---|---|---|
| `26.3/` | 26.3 | Fabric, mojmap no-remap, Java 25 | **Source of truth.** Boot-tested. |
| `26.2/` | 26.2 | same | Generated (`tools/port_26x.py`). Boot-tested. |
| `26.1.2/` | 26.1 – 26.1.2 | same | Generated (`tools/port_26x.py`). Boot-tested on 26.1.2. |
| `1.21.11/` | 1.21.11 | Fabric, Mojang mappings + remap, Java 21 | Generated from 26.3. Boot-tested. |
| `1.21.10/` | 1.21.9 – 1.21.10 | same | Generated from 1.21.11. Boot-tested on 1.21.9. |
| `1.21.8/` | 1.21.6 – 1.21.8 | same | Generated from 1.21.10. Boot-tested on 1.21.6 and 1.21.8. |
| `1.21.5/` | 1.21.2 – 1.21.5 | same | Generated from 1.21.8. Builds; not boot-tested yet. |
| `1.21.1/` | 1.21 – 1.21.1 | same | Generated from 1.21.5. Boot-tested on 1.21.1. |
| `1.8.9/` | 1.8.9 | Legacy Fabric (yarn), Java 8 | **Work in progress** (separate code base; skipped by `build-all.ps1`). |

### Regenerating the other versions

Don't edit generated folders by hand — change `26.3/`, then:

```powershell
python -I mods/tools/port_26x.py              # 26.2, 26.1.2
python -I mods/tools/port_26_to_1_21.py       # 1.21.11 -> 1.21.10 -> 1.21.8 -> 1.21.5 -> 1.21.1
python -I mods/tools/audit_mixins.py <folder> <that version's mapped Minecraft jar> <javap>
```

Each step only lists what changed in its band (javap-verified text rules in
`tools/port_26x.py` and `tools/stages_1_21.py`). Files whose mixin target
moved live in `<folder>-overrides/` and are copied over the generated output.
Every Salt mixin is optional (`defaultRequire: 0`), so a hook that misses on
some version leaves that feature vanilla instead of crashing the game —
`audit_mixins.py` is how misses are caught before shipping.

**Porting method:** never guess another version's API. Copy the nearest
folder, change `gradle.properties`, run `gradlew clean compileJava`, and look
up each error with `javap` against that version's Minecraft jar. Then validate:
clean build → smoke launch (`cargo run -p salt-core --example smoke -- <version>`)
→ each feature in game.

## Not built yet

* **saltSocial** (in-game friends/chat overlay). Friends and chat work in the
  launcher today; an in-game overlay needs a Java WebSocket client and the
  same Mojang `join` handshake. It's listed in the bundle so it installs
  automatically once a jar exists in `salt-mods/<version>/`.
