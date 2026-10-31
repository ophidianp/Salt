package com.terraformersmc.modmenu.api;

import net.minecraft.client.gui.screens.Screen;

/** Mod Menu API: builds a mod's config screen on top of {@code parent}. */
@FunctionalInterface
public interface ConfigScreenFactory<S extends Screen> {
	S create(Screen parent);
}
