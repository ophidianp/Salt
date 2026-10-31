package gg.salt.togglesprint;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keybinding.v1.KeyBindingHelper;
import net.fabricmc.fabric.api.client.rendering.v1.hud.HudElementRegistry;
import net.minecraft.client.KeyMapping;
import net.minecraft.client.Minecraft;
import net.minecraft.network.chat.Component;
import net.minecraft.resources.ResourceLocation;

/**
 * saltToggleSprint — press V to toggle sprint and B to toggle sneak, so you
 * don't have to hold them. Settings live in saltConfig.
 */
public class SaltToggleSprint implements ClientModInitializer {
	static final ModConfig CFG = SaltConfig.mod("salttogglesprint", "saltToggleSprint", "Toggle sprint and sneak")
		.toggle("sneak", "Toggle sneak", "Let B toggle sneaking too", true)
		.toggle("indicator", "Show indicator", "\"[Sprinting (Toggled)]\" in the corner", true);

	private static KeyMapping sprintKey, sneakKey;
	private static boolean sprint, sneak;

	@Override
	public void onInitializeClient() {
		sprintKey = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.salttogglesprint.sprint", InputConstants.Type.KEYSYM, InputConstants.KEY_V, SaltConfig.category()));
		sneakKey = KeyBindingHelper.registerKeyBinding(new KeyMapping("key.salttogglesprint.sneak", InputConstants.Type.KEYSYM, InputConstants.KEY_B, SaltConfig.category()));

		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			while (sprintKey.consumeClick()) sprint = !sprint;
			while (sneakKey.consumeClick()) {
				sneak = !sneak;
				if (!sneak) mc.options.keyShift.setDown(false);
			}
			if (!CFG.enabled()) {
				if (sneak) mc.options.keyShift.setDown(false);
				sprint = sneak = false;
				return;
			}
			if (!CFG.bool("sneak") && sneak) {
				sneak = false;
				mc.options.keyShift.setDown(false);
			}
			if (mc.player == null || mc.screen != null) return;
			// Holding the key "for" the player each tick keeps vanilla's own
			// sprint rules (hunger, blindness, using items) in charge.
			if (sprint && mc.options.keyUp.isDown()) mc.options.keySprint.setDown(true);
			if (sneak) mc.options.keyShift.setDown(true);
		});

		HudElementRegistry.addLast(ResourceLocation.fromNamespaceAndPath("salttogglesprint", "status"), (g, delta) -> {
			Minecraft mc = Minecraft.getInstance();
			if (mc.player == null || !CFG.enabled() || !CFG.bool("indicator") || (!sprint && !sneak)) return;
			String text = sneak ? "[Sneaking (Toggled)]" : mc.player.isSprinting() ? "[Sprinting (Toggled)]" : "[Sprint (Toggled)]";
			g.drawString(mc.font, Component.literal(text), 4, g.guiHeight() - 12, 0xFFFFFFFF, true);
		});
	}
}
