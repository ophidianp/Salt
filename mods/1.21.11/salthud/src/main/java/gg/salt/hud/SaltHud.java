package gg.salt.hud;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElement;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.fabricmc.fabric.api.client.rendering.v1.hud.VanillaHudElements;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.resources.Identifier;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** saltHUD — the Salt Launcher HUD: FPS, CPS, keystrokes, armor, potions… */
public class SaltHud implements ClientModInitializer {
	public static final String MOD_ID = "salthud";
	public static final Logger LOGGER = LoggerFactory.getLogger("saltHUD");

	static final ModConfig CFG = SaltConfig.mod("salthud", "saltHUD", "FPS, CPS, keystrokes, armor, potions and more")
		.action("Edit HUD layout", "Drag elements around the screen", () -> Minecraft.getInstance().setScreen(new HudEditorScreen(Minecraft.getInstance().screen)))
		.slider("size", "HUD size", "Scales every element", 0.5, 2.0, 0.05, 1.0)
		.toggle("crosshair", "Custom crosshair", "Use saltHUD's crosshair (style in the HUD manager)", true);

	private static HudConfig config;
	private static KeyMapping editKey;

	public static HudConfig config() {
		if (config == null) config = HudConfig.load();
		return config;
	}

	@Override
	public void onInitializeClient() {
		config = HudConfig.load();
		// Unbound by default: Right Shift opens saltConfig, which has an
		// "Edit HUD layout" button. Users can still bind this directly.
		editKey = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.salthud.edit", InputConstants.Type.KEYSYM, InputConstants.UNKNOWN.getValue(), SaltConfig.category()));

		HudElementRegistry.addLast(Identifier.fromNamespaceAndPath(MOD_ID, "hud"), HudRenderer::render);
		HudElementRegistry.replaceElement(VanillaHudElements.CROSSHAIR, vanilla -> (HudElement) (g, delta) -> {
			if (!HudRenderer.drawCrosshair(g)) vanilla.render(g, delta);
		});

		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			Trackers.tick(mc);
			while (editKey.consumeClick()) {
				if (mc.screen == null) mc.setScreen(new HudEditorScreen(null));
			}
		});
		LOGGER.info("saltHUD ready ({} elements)", config.elements.size());
	}

	static boolean enabled() {
		return CFG.enabled();
	}

	static float sizeMultiplier() {
		return (float) CFG.number("size");
	}

	static boolean crosshairAllowed() {
		return CFG.enabled() && CFG.bool("crosshair");
	}
}
