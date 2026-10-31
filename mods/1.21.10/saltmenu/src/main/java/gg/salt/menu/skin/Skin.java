package gg.salt.menu.skin;

import gg.salt.config.SaltTheme;
import gg.salt.config.SaltUi;
import gg.salt.menu.SaltMenu;
import net.minecraft.client.gui.GuiGraphics;

/**
 * Salt's look for vanilla widgets: the mixins in this package swap
 * Minecraft's stone sprites for these rounded, accent-tinted shapes, so every
 * vanilla menu (and most modded ones, which reuse vanilla widgets) matches
 * the launcher.
 */
public final class Skin {
	private Skin() {}

	public static final int FIELD = 0xE00C0D10;
	public static final int LIST_BG = 0x55000000;

	/** "Salt-style vanilla menus" is on. */
	public static boolean on() {
		return SaltMenu.CFG.enabled() && SaltMenu.CFG.bool("vanilla");
	}

	/** The widget's own fade (vanilla's `alpha`) applied on top of a colour. */
	private static int fade(int color, float alpha) {
		return alpha >= 1 ? color : SaltUi.alpha(color, alpha);
	}

	public static void button(GuiGraphics g, Object key, int x, int y, int w, int h, boolean active, boolean hot, float alpha) {
		float t = SaltUi.anim(key, active && hot ? 1 : 0, 14);
		int fill = active ? SaltUi.mix(0xC0141518, 0xE0202228, t) : 0x800F1013;
		int edge = active ? SaltUi.mix(0x26FFFFFF, SaltUi.alpha(SaltUi.accent(), 0.8f), t) : 0x12FFFFFF;
		SaltUi.box(g, x, y, w, h, fade(fill, alpha));
		SaltUi.border(g, x, y, w, h, fade(edge, alpha));
	}

	public static void sliderTrack(GuiGraphics g, Object key, int x, int y, int w, int h, double value, boolean active, boolean hot, float alpha) {
		button(g, key, x, y, w, h, active, hot, alpha);
		// Accent fill up to the handle, behind the label.
		int fillW = (int) Math.round(value * (w - 8)) + 4;
		if (active && fillW > 2) SaltUi.box(g, x + 1, y + 1, fillW, h - 2, fade(SaltUi.alpha(SaltUi.accent(), 0.22f), alpha));
	}

	public static void sliderHandle(GuiGraphics g, int x, int y, int w, int h, boolean active, boolean hot, float alpha) {
		// A slim pill in the handle's slot, so it covers less of the label.
		int c = !active ? 0x60FFFFFF : hot ? SaltUi.accent() : 0xE6E8ECF2;
		SaltUi.box(g, x + w / 2 - 2, y + 3, 4, h - 6, 2, fade(c, alpha));
	}

	public static void checkbox(GuiGraphics g, Object key, int x, int y, int w, int h, boolean selected, boolean hot, float alpha) {
		float t = SaltUi.anim(key, selected ? 1 : 0, 16);
		int r = Math.min(4, SaltTheme.radius());
		SaltUi.box(g, x, y, w, h, r, fade(SaltUi.mix(FIELD, SaltUi.accent(), t), alpha));
		SaltUi.border(g, x, y, w, h, r, fade(hot ? SaltUi.accent() : SaltUi.mix(0x33FFFFFF, SaltUi.accent(), t), alpha));
		if (t > 0.05f) {
			// A small tick, drawn in the colour that reads on the accent.
			int ink = fade(SaltUi.alpha(SaltUi.ink(), t), alpha);
			int cx = x + w / 2, cy = y + h / 2;
			for (int i = 0; i < 3; i++) g.fill(cx - 4 + i, cy + i - 1, cx - 2 + i, cy + i + 1, ink);
			for (int i = 0; i < 6; i++) g.fill(cx - 1 + i, cy + 1 - i, cx + 1 + i, cy + 3 - i, ink);
		}
	}

	public static void field(GuiGraphics g, int x, int y, int w, int h, boolean focused, boolean hot, boolean active) {
		SaltUi.box(g, x, y, w, h, active ? FIELD : 0x800C0D10);
		int edge = focused ? SaltUi.accent() : hot ? 0x44FFFFFF : 0x26FFFFFF;
		SaltUi.border(g, x, y, w, h, edge);
	}

	public static void scrollbar(GuiGraphics g, boolean track, int x, int y, int w, int h) {
		// Thinner than vanilla's, centred in the same column.
		int tw = Math.max(2, w - 2);
		int tx = x + (w - tw) / 2;
		SaltUi.box(g, tx, y, tw, h, tw / 2, track ? 0x18FFFFFF : 0x99FFFFFF);
	}

	public static void selection(GuiGraphics g, int x, int y, int w, int h, boolean focused) {
		SaltUi.box(g, x, y, w, h, SaltUi.alpha(SaltUi.accent(), 0.14f));
		SaltUi.border(g, x, y, w, h, focused ? SaltUi.accent() : SaltUi.alpha(SaltUi.accent(), 0.55f));
	}

	public static void tab(GuiGraphics g, Object key, int x, int y, int w, int h, boolean selected, boolean hot) {
		float t = SaltUi.anim(key, selected ? 1 : hot ? 0.45f : 0, 14);
		if (t < 0.02f) return;
		int r = Math.max(2, SaltTheme.radius());
		SaltUi.box(g, x + 1, y + 2, w - 2, h - 3, r, SaltUi.alpha(SaltUi.mix(0x30FFFFFF, SaltUi.alpha(SaltUi.accent(), 0.28f), selected ? 1 : 0), t));
		if (selected) SaltUi.border(g, x + 1, y + 2, w - 2, h - 3, r, SaltUi.alpha(SaltUi.accent(), 0.7f * t));
	}
}
