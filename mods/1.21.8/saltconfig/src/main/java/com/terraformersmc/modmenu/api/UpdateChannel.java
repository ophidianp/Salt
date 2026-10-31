package com.terraformersmc.modmenu.api;

/** Mod Menu API (kept so integrations that implement it still load). */
public enum UpdateChannel {
	ALPHA, BETA, RELEASE;

	public static UpdateChannel getUserPreference() {
		return RELEASE;
	}
}
