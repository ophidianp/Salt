package gg.salt.fullbright;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.minecraft.client.KeyMapping;

/**
 * saltFullbright — full brightness without touching your gamma setting.
 * G toggles it in game; the "Active" switch in saltConfig is the same state.
 */
public class SaltFullbright implements ClientModInitializer {
	static final ModConfig CFG = SaltConfig.mod("saltfullbright", "saltFullbright", "Full brightness, toggled with G")
		.toggle("active", "Active", "Same as pressing G", false);

	private static KeyMapping key;
	private static boolean lastOn;
	private static boolean dirty;

	@Override
	public void onInitializeClient() {
		key = KeyMappingHelper.registerKeyMapping(new KeyMapping("key.saltfullbright.toggle", InputConstants.Type.KEYSYM, InputConstants.KEY_G, SaltConfig.category()));
		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			while (key.consumeClick()) CFG.set("active", !CFG.bool("active"));
			// Rebuild the lightmap whenever the effective state changes
			// (key, config screen, or the mod being disabled).
			boolean now = on();
			if (now != lastOn) {
				lastOn = now;
				dirty = true;
			}
		});
	}

	public static boolean on() {
		return CFG.enabled() && CFG.bool("active");
	}

	/** True once after a change, so the lightmap is rebuilt immediately. */
	public static boolean consumeDirty() {
		boolean d = dirty;
		dirty = false;
		return d;
	}
}
