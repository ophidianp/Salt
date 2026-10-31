package com.terraformersmc.modmenu.api;

/** Mod Menu API (kept so integrations that implement it still load). */
public interface UpdateChecker {
	UpdateInfo checkForUpdates();
}
