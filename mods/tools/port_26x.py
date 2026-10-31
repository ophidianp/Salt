"""Generates mods/26.2 and mods/26.1.2 from mods/26.3 (the source of truth).

    python -I mods/tools/port_26x.py [26.2|26.1.2]

Each target lists text rules checked with javap against that version's jar.
Hand-written files for a target live in mods/<version>-overrides/.
"""
import pathlib
import re
import shutil
import sys

ROOT = pathlib.Path(__file__).resolve().parents[1]
SRC = ROOT / "26.3"

# 26.2 and older: RenderPipeline package, KEYSYM name, OptionsScreen(..., inWorld).
SHARED_26_2 = [
    ("InputConstants.Type.KEYBOARD", "InputConstants.Type.KEYSYM"),
    ("com.mojang.renderpearl.api.pipeline.RenderPipeline", "com.mojang.blaze3d.pipeline.RenderPipeline"),
    ("Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;", "Lcom/mojang/blaze3d/pipeline/RenderPipeline;"),
    # OptionsScreen(parent, options, inWorld)
    ("new OptionsScreen(this, minecraft.options)", "new OptionsScreen(this, minecraft.options, false)"),
    ("new OptionsScreen(title, mc.options)", "new OptionsScreen(title, mc.options, false)"),
]

TARGETS = {
    "26.2": {
        "props": {"fabric_api_version": "0.161.0+26.2", "cloth_config_version": "26.2.155"},
        "rules": SHARED_26_2,
        "drop": [],
    },
    "26.1.2": {
        "range": "~26.1",
        "props": {"fabric_api_version": "0.155.3+26.1.2", "cloth_config_version": "26.1.154"},
        # 26.1 keeps the current screen on Minecraft (26.2+ moved it to Gui).
        "rules": SHARED_26_2 + [
            ("Minecraft.getInstance().gui.screen()", "Minecraft.getInstance().screen"),
            ("Minecraft.getInstance().gui.setScreen(", "Minecraft.getInstance().setScreen("),
            ("mc.gui.screen()", "mc.screen"),
            ("mc.gui.setScreen(", "mc.setScreen("),
            ("minecraft.gui.setScreen(", "minecraft.setScreen("),
            ("mc.gameRenderer.mainRenderTarget()", "mc.getMainRenderTarget()"),
        ],
        # No MenuTabBar before 26.2 (tabs are plain TabButtons).
        "drop": ["saltmenu/src/main/java/gg/salt/menu/mixin/GuiMixin.java",
                 "saltmenu/src/main/java/gg/salt/menu/mixin/skin/MenuTabButtonMixin.java"],
    },
}


def port(v):
    t = TARGETS[v]
    dst = ROOT / v
    # Replace each mod's sources (files only: folders can't be removed here).
    for mod in SRC.iterdir():
        for f in (dst / mod.name / "src").rglob("*"):
            if f.is_file():
                f.unlink()
    shutil.copytree(SRC, dst, ignore=shutil.ignore_patterns("build", ".gradle"), dirs_exist_ok=True)
    for rel in t["drop"]:
        (dst / rel).unlink()
    for f in dst.rglob("*"):
        if not f.is_file() or f.suffix not in (".java", ".json", ".gradle", ".properties"):
            continue
        s = f.read_text(encoding="utf-8")
        o = s
        s = s.replace('"~26.3"', '"%s"' % t.get("range", "~" + v)).replace("salt-mods-26.3", f"salt-mods-{v}")
        if f.name == "gradle.properties":
            s = re.sub(r"minecraft_version=.*", f"minecraft_version={v}", s)
            for k, val in t["props"].items():
                s = re.sub(rf"{k}=.*", f"{k}={val}", s)
        if f.suffix == ".java":
            for a, b in t["rules"]:
                s = s.replace(a, b)
        if s != o:
            f.write_text(s, encoding="utf-8")
    ov = ROOT / f"{v}-overrides"
    if ov.exists():
        for f in ov.rglob("*"):
            if f.is_file():
                out = dst / f.relative_to(ov)
                out.parent.mkdir(parents=True, exist_ok=True)
                shutil.copy(f, out)
    print("generated", dst)


for v in sys.argv[1:] or TARGETS:
    port(v)
