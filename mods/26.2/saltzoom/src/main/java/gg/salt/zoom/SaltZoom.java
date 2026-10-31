package gg.salt.zoom;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.minecraft.client.KeyMapping;

import java.util.List;

/**
 * saltZoom — hold (or toggle) C to zoom, scroll while zooming to zoom further.
 * Settings live in saltConfig (Right Shift).
 */
public class SaltZoom implements ClientModInitializer {
	static final ModConfig CFG = SaltConfig.mod("saltzoom", "saltZoom", "Smooth zoom with scroll-to-zoom")
		.slider("zoom", "Zoom amount", "How far the key zooms in", 1.5, 10, 0.5, 4)
		.choice("mode", "Key mode", "Hold the key, or press once to toggle", List.of("Hold", "Toggle"), "Hold")
		.toggle("smooth", "Smooth zoom", "Ease in and out instead of snapping", true)
		.toggle("scroll", "Scroll to zoom", "Mouse wheel zooms further while zoomed", true)
		.toggle("cinematic", "Cinematic camera", "Smoother mouse movement while zoomed", true);

	private static KeyMapping key;
	private static boolean zooming, toggled;
	private static boolean savedSmooth;
	private static double target = 4.0;
	private static double current = 1.0;
	private static long lastNanos = System.nanoTime();

	@Override
	public void onInitializeClient() {
		key = KeyMappingHelper.registerKeyMapping(new KeyMapping("key.saltzoom.zoom", InputConstants.Type.KEYSYM, InputConstants.KEY_C, SaltConfig.category()));
		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			boolean toggleMode = "Toggle".equals(CFG.choice("mode"));
			while (key.consumeClick()) if (toggleMode) toggled = !toggled;
			boolean want = CFG.enabled() && mc.gui.screen() == null && (toggleMode ? toggled : key.isDown());
			if (want && !zooming) {
				zooming = true;
				savedSmooth = mc.options.smoothCamera;
				if (CFG.bool("cinematic")) mc.options.smoothCamera = true;
				target = CFG.number("zoom");
			} else if (!want && zooming) {
				zooming = false;
				toggled = false;
				mc.options.smoothCamera = savedSmooth;
			}
		});
	}

	public static boolean isZooming() {
		return zooming;
	}

	/** Scroll up = zoom in. Returns true when the scroll was used. */
	public static boolean scroll(double amount) {
		if (!zooming || !CFG.bool("scroll")) return false;
		target = Math.max(1.5, Math.min(50.0, target * (amount > 0 ? 1.2 : 1 / 1.2)));
		return true;
	}

	/** Called with vanilla's FOV every frame. */
	public static float apply(float fov) {
		long now = System.nanoTime();
		double dt = Math.min(0.1, (now - lastNanos) / 1e9);
		lastNanos = now;
		double want = zooming ? target : 1.0;
		if (CFG.bool("smooth")) {
			// Frame-rate independent easing (~90 ms to get most of the way there).
			current += (want - current) * (1 - Math.exp(-dt * 18));
			if (Math.abs(current - want) < 0.001) current = want;
		} else {
			current = want;
		}
		return (float) (fov / current);
	}
}
