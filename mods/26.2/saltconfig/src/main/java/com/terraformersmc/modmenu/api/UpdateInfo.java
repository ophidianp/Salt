package com.terraformersmc.modmenu.api;

import net.minecraft.network.chat.Component;

/** Mod Menu API (kept so integrations that implement it still load). */
public interface UpdateInfo {
	boolean isUpdateAvailable();

	default Component getUpdateMessage() {
		return null;
	}

	String getDownloadLink();

	UpdateChannel getUpdateChannel();
}
