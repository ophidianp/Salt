package gg.salt.menu;

import gg.salt.config.SaltConfigScreen;
import net.minecraft.client.Minecraft;
import net.minecraft.client.Screenshot;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.gui.screens.multiplayer.JoinMultiplayerScreen;
import net.minecraft.client.gui.screens.options.OptionsScreen;
import net.minecraft.client.gui.screens.options.VideoSettingsScreen;
import net.minecraft.client.gui.screens.worldselection.CreateWorldScreen;

import java.util.List;
import java.util.function.Supplier;

/**
 * Dev check (-Dsaltmenu.tour=true): once the Salt title screen is up, opens
 * the main vanilla menus one by one and screenshots each, then checks that
 * closing Create World lands back on the Salt title screen.
 */
final class Tour {
	static final boolean ON = Boolean.getBoolean("saltmenu.tour");
	private static final int STEP = 40;

	private static int tick = -1;
	private static int step;

	private Tour() {}

	private static List<Supplier<Screen>> steps(Minecraft mc) {
		Screen title = new SaltTitleScreen();
		return List.of(
			() -> title,
			() -> new OptionsScreen(title, mc.options),
			() -> new VideoSettingsScreen(title, mc, mc.options),
			() -> new JoinMultiplayerScreen(title),
			() -> {
				// Create World, opened like Singleplayer does with no worlds.
				CreateWorldScreen.openFresh(mc, null);
				return null;
			},
			() -> {
				// Cancel Create World: must come back to Salt's title, not vanilla's.
				mc.screen.onClose();
				SaltMenu.LOGGER.info("saltMenu tour: after Create World cancel -> {}", mc.screen.getClass().getSimpleName());
				return null;
			},
			() -> new SaltConfigScreen(title),
			() -> hudEditor(title));
	}

	private static Screen hudEditor(Screen parent) {
		try {
			return (Screen) Class.forName("gg.salt.hud.HudEditorScreen").getConstructor(Screen.class).newInstance(parent);
		} catch (ReflectiveOperationException e) {
			return parent;
		}
	}

	static void tick(Minecraft mc) {
		if (tick < 0) {
			if (!(mc.screen instanceof SaltTitleScreen)) return;
			tick = 0;
		}
		tick++;
		List<Supplier<Screen>> steps = steps(mc);
		if (step >= steps.size()) return;
		if (tick == 20) {
			Screen s = steps.get(step).get();
			if (s != null) mc.setScreen(s);
		} else if (tick == STEP) {
			String name = "tour-" + step + "-" + mc.screen.getClass().getSimpleName() + ".png";
			Screenshot.grab(mc.gameDirectory, name, mc.getMainRenderTarget(), msg -> SaltMenu.LOGGER.info("saltMenu tour: {}", msg.getString()));
			step++;
			tick = 0;
		}
	}
}
