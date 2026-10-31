package gg.salt.menu;

import gg.salt.config.ModConfig;
import gg.salt.config.SaltConfig;
import net.fabricmc.api.ClientModInitializer;
import net.fabricmc.fabric.api.client.event.lifecycle.v1.ClientTickEvents;
import org.slf4j.Logger;
import org.slf4j.LoggerFactory;

/** saltMenu — replaces Minecraft's title screen with Salt's own. */
public class SaltMenu implements ClientModInitializer {
	public static final Logger LOGGER = LoggerFactory.getLogger("saltMenu");
	public static final ModConfig CFG = SaltConfig.mod("saltmenu", "saltMenu", "Salt's title screen")
		.toggle("classic", "\"Classic\" button", "Show a button that opens the normal Minecraft menu", true)
		.toggle("splash", "Splash line", "Show a short line under the logo", true)
		.toggle("vanilla", "Salt-style vanilla menus", "Rounded buttons, sliders, fields and lists in the launcher's accent", true);

	/** Set by the "Classic" button so the next TitleScreen isn't swapped. */
	public static boolean showVanillaOnce;
	public static boolean loggedOnce;

	/** Whether the mixin should swap the title screen right now. */
	public static boolean active() {
		return CFG.enabled() && !showVanillaOnce;
	}

	@Override
	public void onInitializeClient() {
		// Everything else happens in the mixins.
		if (Tour.ON) ClientTickEvents.END_CLIENT_TICK.register(Tour::tick);
	}
}
