"""Version bands below 1.21.11, used by port_26_to_1_21.py.

Each folder is generated from the one before it, so each list only holds
what changed in that band (checked with javap against that version's jar):

    1.21.11 -> 1.21.10 (1.21.9-1.21.10) -> 1.21.8 (1.21.6-1.21.8)
            -> 1.21.5 (1.21.2-1.21.5) -> 1.21.1 (1.21-1.21.1)
"""

# 1.21.11 -> 1.21.10: Identifier was still ResourceLocation, and buttons draw
# everything in renderWidget (no renderContents / renderDefaultSprite).
RULES_1210 = [
    ("renderContents(", "renderWidget("),
    ('method = "renderContents"', 'method = "renderWidget"'),
    ("g.renderOutline(", "g.submitOutline("),
    ("Lnet/minecraft/resources/Identifier;", "Lnet/minecraft/resources/ResourceLocation;"),
    ("k.identifier()", "k.location()"),
]
REGEX_1210 = [
    (r"\bIdentifier\b", "ResourceLocation"),
]

# 1.21.10 -> 1.21.8: before 1.21.9 input arrives as (x, y, button), buttons
# press with onPress(), and key categories are plain translation keys.
RULES_1218 = [
    ("CreateWorldScreen.openFresh(mc, () -> mc.setScreen(null));", "CreateWorldScreen.openFresh(mc, null);"),
    ("g.submitOutline(", "g.renderOutline("),
    ("import net.minecraft.client.input.MouseButtonEvent;", "import gg.salt.config.MouseEvent;"),
    ("import net.minecraft.client.input.InputWithModifiers;\n", ""),
    ("public void onPress(InputWithModifiers input)", "public void onPress()"),
    ("private static String fmt(MouseButtonEvent e)", "private static String fmt(MouseEvent e)"),
    ("public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {",
     "public boolean mouseClicked(double mouseX, double mouseY, int mouseButton) {\n\t\tMouseEvent event = new MouseEvent(mouseX, mouseY, mouseButton);"),
    ("super.mouseClicked(event, doubleClick)", "super.mouseClicked(mouseX, mouseY, mouseButton)"),
    ("public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {",
     "public boolean mouseDragged(double mouseX, double mouseY, int mouseButton, double dx, double dy) {\n\t\tMouseEvent event = new MouseEvent(mouseX, mouseY, mouseButton);"),
    ("super.mouseDragged(event, dx, dy)", "super.mouseDragged(mouseX, mouseY, mouseButton, dx, dy)"),
    ("public boolean mouseReleased(MouseButtonEvent event) {",
     "public boolean mouseReleased(double mouseX, double mouseY, int mouseButton) {\n\t\tMouseEvent event = new MouseEvent(mouseX, mouseY, mouseButton);"),
    ("super.mouseReleased(event)", "super.mouseReleased(mouseX, mouseY, mouseButton)"),
    ("KeyMapping.Category CATEGORY", "String CATEGORY"),
    ('"key.category.salt.salt"', '"key.categories.salt"'),
]
REGEX_1218 = [
    (r"(?s)(\t/\*\*(?:(?!\*/).)*\*/\n)?\t((?:public )?static) KeyMapping\.Category (\w+)\(\) \{.*?\n\t\}\n",
     "\t\\2 String \\3() {\n\t\treturn \"key.categories.salt\";\n\t}\n"),
]

# 1.21.8 -> 1.21.5: no Matrix3x2fStack, RenderPipelines or HUD element
# registry before 1.21.6.
RULES_1215 = [
    # sprites draw through a RenderType function, not a RenderPipeline
    ("import com.mojang.blaze3d.pipeline.RenderPipeline;", "import java.util.function.Function;\nimport net.minecraft.client.renderer.RenderType;"),
    ("RenderPipeline pipeline", "Function<ResourceLocation, RenderType> pipeline"),
    ("Lcom/mojang/blaze3d/pipeline/RenderPipeline;", "Ljava/util/function/Function;"),
    ("g.blit(RenderPipelines.GUI_TEXTURED, ", "g.blit(RenderType::guiTextured, "),
    ("import net.minecraft.client.renderer.RenderPipelines;", "import net.minecraft.client.renderer.RenderType;"),
    ("Screenshot.grab(mc.gameDirectory, name, mc.getMainRenderTarget(), 1, ", "Screenshot.grab(mc.gameDirectory, name, mc.getMainRenderTarget(), "),
    ("g.pose().pushMatrix()", "g.pose().pushPose()"),
    ("g.pose().popMatrix()", "g.pose().popPose()"),
    ("import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElement;\nimport net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;\nimport net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;\n",
     "import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;\n"),
    ("import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;\n",
     "import net.fabricmc.fabric.api.client.rendering.v1.HudRenderCallback;\n"),
    ("HudElementRegistry.addLast(ResourceLocation.fromNamespaceAndPath(MOD_ID, \"hud\"), HudRenderer::render);\n\t\tHudElementRegistry.replaceElement(VanillaHudElements.CROSSHAIR, vanilla -> (HudElement) (g, delta) -> {\n\t\t\tif (!HudRenderer.drawCrosshair(g)) vanilla.render(g, delta);\n\t\t});",
     "HudRenderCallback.EVENT.register(HudRenderer::render);\n\t\t// The custom crosshair hooks Gui#renderCrosshair (mixin/GuiMixin)."),
    ("HudElementRegistry.addLast(ResourceLocation.fromNamespaceAndPath(\"salttogglesprint\", \"status\"), (g, delta) -> {",
     "HudRenderCallback.EVENT.register((g, delta) -> {"),
]
REGEX_1215 = [
    # Matrix3x2fStack (2 args) -> PoseStack (3 args)
    (r"g\.pose\(\)\.translate\(([^;]+)\);", r"g.pose().translate(\1, 0);"),
    (r"g\.pose\(\)\.scale\(([^,;]+), ([^;]+)\);", r"g.pose().scale(\1, \2, 1f);"),
]

# 1.21.5 -> 1.21.1: sprites take no RenderType or colour, and list
# scrollbars are drawn inside AbstractSelectionList#renderWidget.
RULES_1211 = [
    ("Ljava/util/function/Function;Lnet/minecraft/resources/ResourceLocation;IIIII)V", "Lnet/minecraft/resources/ResourceLocation;IIII)V"),
    ("Ljava/util/function/Function;Lnet/minecraft/resources/ResourceLocation;IIII)V", "Lnet/minecraft/resources/ResourceLocation;IIII)V"),
    ("Function<ResourceLocation, RenderType> pipeline, ResourceLocation sprite, int x, int y, int w, int h, int color)",
     "ResourceLocation sprite, int x, int y, int w, int h)"),
    ("Function<ResourceLocation, RenderType> pipeline, ResourceLocation sprite, int x, int y, int w, int h)",
     "ResourceLocation sprite, int x, int y, int w, int h)"),
    ("g.blitSprite(pipeline, sprite, x, y, w, h, color);", "g.blitSprite(sprite, x, y, w, h);"),
    ("g.blitSprite(pipeline, sprite, x, y, w, h);", "g.blitSprite(sprite, x, y, w, h);"),
    ("g.blit(RenderType::guiTextured, ", "g.blit("),
    ("import net.minecraft.client.gui.components.AbstractScrollArea;", "import net.minecraft.client.gui.components.AbstractSelectionList;"),
    ("@Mixin(AbstractScrollArea.class)", "@Mixin(AbstractSelectionList.class)"),
    ('method = "renderScrollbar"', 'method = "renderWidget"'),
]
REGEX_1211 = []

# (folder, source folder, rules, regexes, minecraft range, gradle properties,
#  versions the jars are installed for)
STAGES = [
    ("1.21.10", "1.21.11", RULES_1210, REGEX_1210, ">=1.21.9 <1.21.11",
     {"minecraft_version": "1.21.10", "fabric_api_version": "0.138.4+1.21.10", "cloth_config_version": "20.0.149"},
     ["1.21.9", "1.21.10"]),
    ("1.21.8", "1.21.10", RULES_1218, REGEX_1218, ">=1.21.6 <1.21.9",
     {"minecraft_version": "1.21.8", "fabric_api_version": "0.136.1+1.21.8", "cloth_config_version": "19.0.147"},
     ["1.21.6", "1.21.7", "1.21.8"]),
    ("1.21.5", "1.21.8", RULES_1215, REGEX_1215, ">=1.21.2 <1.21.6",
     {"minecraft_version": "1.21.5", "fabric_api_version": "0.128.2+1.21.5", "cloth_config_version": "18.0.145"},
     ["1.21.2", "1.21.3", "1.21.4", "1.21.5"]),
    ("1.21.1", "1.21.5", RULES_1211, REGEX_1211, ">=1.21 <1.21.2",
     {"minecraft_version": "1.21.1", "fabric_api_version": "0.116.17+1.21.1", "cloth_config_version": "15.0.140"},
     ["1.21", "1.21.1"]),
]
