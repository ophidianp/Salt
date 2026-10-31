package gg.salt.freelook;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import net.fabricmc.fabric.api.client.keymapping.v1.KeyMappingHelper;
import net.minecraft.client.CameraType;
import net.minecraft.client.KeyMapping;
import net.minecraft.util.Mth;

import java.util.List;

/**
 * saltFreelook — hold (or toggle) Left Alt to orbit the camera around yourself
 * without turning your player. Settings live in saltConfig.
 */
public class SaltFreelook implements ClientModInitializer {
	static final ModConfig CFG = SaltConfig.mod("saltfreelook", "saltFreelook", "Look around without turning")
		.choice("mode", "Key mode", "Hold the key, or press once to toggle", List.of("Hold", "Toggle"), "Hold")
		.toggle("thirdPerson", "Switch to third person", "Show your player while looking around", true)
		.toggle("invert", "Invert vertical", "Flip up/down while freelooking", false);

	private static KeyMapping key;
	private static boolean active, toggled;
	private static CameraType savedCamera;
	private static float yaw, pitch;

	@Override
	public void onInitializeClient() {
		key = KeyMappingHelper.registerKeyMapping(new KeyMapping("key.saltfreelook.freelook", InputConstants.Type.KEYSYM, InputConstants.KEY_LALT, SaltConfig.category()));
		ClientTickEvents.END_CLIENT_TICK.register(mc -> {
			boolean toggleMode = "Toggle".equals(CFG.choice("mode"));
			while (key.consumeClick()) if (toggleMode) toggled = !toggled;
			boolean down = CFG.enabled() && mc.screen == null && mc.player != null && (toggleMode ? toggled : key.isDown());
			if (down && !active) {
				active = true;
				yaw = mc.player.getYRot();
				pitch = mc.player.getXRot();
				savedCamera = mc.options.getCameraType();
				if (CFG.bool("thirdPerson") && savedCamera.isFirstPerson()) mc.options.setCameraType(CameraType.THIRD_PERSON_BACK);
			} else if (!down && active) {
				active = false;
				toggled = false;
				if (savedCamera != null) mc.options.setCameraType(savedCamera);
			}
		});
	}

	public static boolean active() {
		return active;
	}

	/** Same maths as Entity#turn, applied to the free camera instead. */
	public static void turn(double dx, double dy) {
		if (CFG.bool("invert")) dy = -dy;
		yaw += (float) dx * 0.15f;
		pitch = Mth.clamp(pitch + (float) dy * 0.15f, -90f, 90f);
	}

	public static float yaw() { return yaw; }
	public static float pitch() { return pitch; }
}
