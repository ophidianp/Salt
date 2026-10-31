"""Regenerates the 1.21.x Salt mod folders from mods/26.3 (the source of truth).

    python -I mods/tools/port_26_to_1_21.py [folder...]

26.3 -> 1.21.11 is below; the older bands chain on from 1.21.11 (see
stages_1_21.py).

26.x and 1.21.11 share most of the client API (Mojang names, Identifier,
Matrix3x2fStack pose, MouseButtonEvent). The differences, all checked with
javap against the 1.21.11 jar, are applied below as text rules. Mixins whose
target method moved are kept as hand-written overrides in
mods/1.21.11-overrides/ and copied over the generated files.
"""
import pathlib
import shutil

ROOT = pathlib.Path(__file__).resolve().parents[1]
SRC = ROOT / "26.3"
DST = ROOT / "1.21.11"
OVERRIDES = ROOT / "1.21.11-overrides"
MODS = ["saltconfig", "salthud", "saltmenu", "saltzoom", "saltfreelook", "salttogglesprint", "saltfullbright"]

RULES = [
    # retained-mode GUI (26.2+) -> immediate-mode GuiGraphics (1.21.x)
    ("GuiGraphicsExtractor", "GuiGraphics"),
    ("com.mojang.renderpearl.api.pipeline.RenderPipeline", "com.mojang.blaze3d.pipeline.RenderPipeline"),
    ("Lcom/mojang/renderpearl/api/pipeline/RenderPipeline;", "Lcom/mojang/blaze3d/pipeline/RenderPipeline;"),
    # widget/screen hooks used by the menu reskin (javap, 1.21.11)
    ("extractDefaultSprite", "renderDefaultSprite"),
    ("extractWidgetRenderState", "renderWidget"),
    ("extractMenuBackgroundTexture", "renderMenuBackgroundTexture"),
    ("extractScrollbar", "renderScrollbar"),
    ("extractListBackground", "renderListBackground"),
    ("extractListSeparators", "renderListSeparators"),
    ("extractSelection", "renderSelection"),
    ("extractPanorama(", "renderPanorama("),
    ("extractContents(", "renderContents("),
    ('"method = \"extractContents\""', '"method = \"renderContents\""'),
    ('method = "extractContents"', 'method = "renderContents"'),
    ("mc.gameRenderer.mainRenderTarget()", "mc.getMainRenderTarget()"),
    # mixin config: Minecraft#setScreen instead of Gui#setScreen; no MenuTabBar
    ('"GuiMixin",', '"MinecraftMixin",'),
    ('\t\t"skin.MenuTabButtonMixin",\n', ""),
    ("g.centeredText(", "g.drawCenteredString("),
    ("g.text(", "g.drawString("),
    ("g.outline(", "g.renderOutline("),
    ("g.item(", "g.renderItem("),
    ("public void extractRenderState(GuiGraphics", "public void render(GuiGraphics"),
    ("super.extractRenderState(", "super.render("),
    ("public void extractBackground(GuiGraphics", "public void renderBackground(GuiGraphics"),
    ("vanilla.extractRenderState(g, delta)", "vanilla.render(g, delta)"),
    # Fabric API key bindings were renamed to key mappings in 26.x
    ("net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper", "net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper"),
    ("KeyMappingHelper.registerKeyMapping(", "KeyBindingHelper.registerKeyBinding("),
    ("InputConstants.Type.KEYBOARD", "InputConstants.Type.KEYSYM"),
    # 26.3 moved the current screen onto Gui
    ("Minecraft.getInstance().gui.screen()", "Minecraft.getInstance().screen"),
    ("mc.gui.screen()", "mc.screen"),
    ("mc.gui.setScreen(", "mc.setScreen("),
    ("Minecraft.getInstance().gui.setScreen(", "Minecraft.getInstance().setScreen("),
    ("minecraft.gui.setScreen(", "minecraft.setScreen("),
    # F1 hide-GUI flag still lives on Options in 1.21
    ("Minecraft mc = Minecraft.getInstance();\n\t\tif (mc.player == null) return;\n\t\tdraw(", "Minecraft mc = Minecraft.getInstance();\n\t\tif (mc.options.hideGui || mc.player == null) return;\n\t\tdraw("),
    ("if (mc.player == null || (!sprint && !sneak)) return;", "if (mc.options.hideGui || mc.player == null || (!sprint && !sneak)) return;"),
    # metadata
    ('"minecraft": "~26.3"', '"minecraft": "~1.21.11"'),
    ('"java": ">=25"', '"java": ">=21"'),
    ('"JAVA_25"', '"JAVA_21"'),
]

# Files that only exist in 26.3 because a mixin target moved.
DROP = {
    "saltzoom/src/main/java/gg/salt/zoom/mixin/CameraMixin.java",
    "saltmenu/src/main/java/gg/salt/menu/mixin/GuiMixin.java",
    "saltfullbright/src/main/java/gg/salt/fullbright/mixin/LightmapExtractorMixin.java",
    "saltmenu/src/main/java/gg/salt/menu/mixin/skin/MenuTabButtonMixin.java",
}

TEXT = {".java", ".json"}


def copy_tree(src_root, dst_root, rules, regexes=(), drop=()):
    import re
    for mod in MODS:
        # Files only: folders can't always be removed here (OneDrive).
        for old in (dst_root / mod / "src").rglob("*"):
            if old.is_file():
                old.unlink()
        for f in (src_root / mod / "src").rglob("*"):
            if f.is_dir():
                continue
            rel = f.relative_to(src_root)
            if rel.as_posix() in drop:
                continue
            out = dst_root / rel
            out.parent.mkdir(parents=True, exist_ok=True)
            if f.suffix in TEXT:
                s = f.read_text(encoding="utf-8")
                for a, b in rules:
                    s = s.replace(a, b)
                for pat, rep in regexes:
                    s = re.sub(pat, rep, s)
                out.write_text(s, encoding="utf-8")
            else:
                shutil.copy2(f, out)


def apply_overrides(overrides, dst_root):
    if not overrides.exists():
        return
    for f in overrides.rglob("*"):
        if f.is_file():
            out = dst_root / f.relative_to(overrides)
            out.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(f, out)


def setup_folder(dst, props, targets):
    """Gradle files for an older band come from 1.21.11 (same remap toolchain)."""
    import re
    base = ROOT / "1.21.11"
    for f in ["build.gradle", "settings.gradle", "gradle.properties", "gradlew", "gradlew.bat",
              "gradle/wrapper/gradle-wrapper.jar", "gradle/wrapper/gradle-wrapper.properties",
              "saltconfig/build.gradle"]:
        (dst / f).parent.mkdir(parents=True, exist_ok=True)
        shutil.copy2(base / f, dst / f)
    s = (dst / "settings.gradle").read_text(encoding="utf-8")
    (dst / "settings.gradle").write_text(re.sub(r"rootProject.name = '.*'", f"rootProject.name = 'salt-mods-{dst.name}'", s), encoding="utf-8")
    s = (dst / "gradle.properties").read_text(encoding="utf-8")
    s = s.replace("# Minecraft 1.21.11:", f"# Minecraft {dst.name}:")
    for k, v in props.items():
        s = re.sub(rf"(?m)^{k}=.*", f"{k}={v}", s)
    (dst / "gradle.properties").write_text(s, encoding="utf-8")
    (dst / "targets.txt").write_text(
        "# Minecraft versions these jars are installed for (one per line).\n" + "\n".join(targets) + "\n", encoding="utf-8")


def set_range(dst, mc_range):
    import re
    for f in dst.glob("*/src/main/resources/fabric.mod.json"):
        s = f.read_text(encoding="utf-8")
        f.write_text(re.sub(r'"minecraft": "[^"]*"', f'"minecraft": "{mc_range}"', s), encoding="utf-8")


def main():
    import sys
    sys.path.insert(0, str(pathlib.Path(__file__).parent))
    from stages_1_21 import STAGES
    only = sys.argv[1:]
    if not only or DST.name in only:
        copy_tree(SRC, DST, RULES, drop=DROP)
        apply_overrides(OVERRIDES, DST)
        print("Generated", DST)
    for name, src, rules, regexes, mc_range, props, targets in STAGES:
        if only and name not in only:
            continue
        dst = ROOT / name
        setup_folder(dst, props, targets)
        copy_tree(ROOT / src, dst, rules, regexes)
        apply_overrides(ROOT / f"{name}-overrides", dst)
        set_range(dst, mc_range)
        print("Generated", dst)


if __name__ == "__main__":
    main()
