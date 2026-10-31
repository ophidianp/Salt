package com.terraformersmc.modmenu.api;

import gg.salt.config.SaltConfigScreen;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.network.chat.Component;

import java.util.Map;
import java.util.function.Consumer;

/**
 * Mod Menu's public API, signature-for-signature, so mods that integrate with
 * Mod Menu (most Cloth Config / YACL users) hand their config screens to
 * saltConfig instead. Salt doesn't ship Mod Menu; saltConfig is the mod list.
 */
public interface ModMenuApi {
	static Screen createModsScreen(Screen previous) {
		return new SaltConfigScreen(previous);
	}

	static Component createModsButtonText() {
		return Component.literal("Mods");
	}

	default ConfigScreenFactory<?> getModConfigScreenFactory() {
		return screen -> null;
	}

	default UpdateChecker getUpdateChecker() {
		return null;
	}

	default Map<String, ConfigScreenFactory<?>> getProvidedConfigScreenFactories() {
		return Map.of();
	}

	default Map<String, UpdateChecker> getProvidedUpdateCheckers() {
		return Map.of();
	}

	default void attachModpackBadges(Consumer<String> consumer) {
	}
}
