package gg.salt.config;

import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;

import java.util.HashMap;
import java.util.Map;

/**
 * Salt's in-game look: flat dark surfaces, thin borders, one accent colour,
 * small eased animations and rounded corners. Shared by saltConfig, saltMenu
 * and saltHUD so they match the launcher instead of vanilla's stone buttons.
 * The accent and corner radius follow the launcher ({@link SaltTheme}).
 */
public final class SaltUi {
	private SaltUi() {}

	public static final int BG = 0xF00B0C0E;
	public static final int SURFACE = 0xFF141518;
	public static final int SURFACE_HI = 0xFF1C1E22;
	public static final int BORDER = 0x22FFFFFF;
	public static final int TEXT = 0xFFF2F3F5;
	public static final int TEXT_2 = 0xA0F2F3F5;
	public static final int TEXT_3 = 0x66F2F3F5;

	private static final Map<Object, Float> ANIM = new HashMap<>();
	private static long lastFrame = System.nanoTime();
	private static float dt = 0.016f;

	/** Call once per frame before drawing animated widgets. */
	public static void frame() {
		long now = System.nanoTime();
		dt = Math.min(0.1f, (now - lastFrame) / 1e9f);
		lastFrame = now;
	}

	/** Eases a value attached to `key` toward `target` (frame-rate independent). */
	public static float anim(Object key, float target, float speed) {
		float v = ANIM.getOrDefault(key, target);
		v += (target - v) * (1 - (float) Math.exp(-dt * speed));
		if (Math.abs(v - target) < 0.002f) v = target;
		ANIM.put(key, v);
		return v;
	}

	/** The launcher's accent colour; read every frame so changes apply live. */
	public static int accent() {
		return SaltTheme.accent();
	}

	/** Text colour that reads on top of the accent (dark on light accents). */
	public static int ink() {
		int c = accent();
		double lum = 0.299 * (c >> 16 & 255) + 0.587 * (c >> 8 & 255) + 0.114 * (c & 255);
		return lum > 150 ? 0xFF06121F : 0xFFFFFFFF;
	}

	public static int mix(int a, int b, float t) {
		int aa = a >>> 24, ar = a >> 16 & 255, ag = a >> 8 & 255, ab = a & 255;
		int ba = b >>> 24, br = b >> 16 & 255, bg = b >> 8 & 255, bb = b & 255;
		return (int) (aa + (ba - aa) * t) << 24 | (int) (ar + (br - ar) * t) << 16 | (int) (ag + (bg - ag) * t) << 8 | (int) (ab + (bb - ab) * t);
	}

	public static int alpha(int color, float a) {
		return (int) ((color >>> 24) * a) << 24 | (color & 0xFFFFFF);
	}

	/** Rounded rectangle using the launcher's corner radius. */
	public static void box(GuiGraphics g, int x, int y, int w, int h, int fill) {
		box(g, x, y, w, h, SaltTheme.radius(), fill);
	}

	/** Rounded rectangle; a radius of h/2 or more makes a pill. */
	public static void box(GuiGraphics g, int x, int y, int w, int h, int r, int fill) {
		r = Math.min(r, Math.min(w, h) / 2);
		if (r <= 0) {
			g.fill(x, y, x + w, y + h, fill);
			return;
		}
		g.fill(x, y + r, x + w, y + h - r, fill);
		for (int i = 0; i < r; i++) {
			int in = inset(r, i);
			g.fill(x + in, y + i, x + w - in, y + i + 1, fill);
			g.fill(x + in, y + h - 1 - i, x + w - in, y + h - i, fill);
		}
	}

	public static void border(GuiGraphics g, int x, int y, int w, int h, int color) {
		border(g, x, y, w, h, SaltTheme.radius(), color);
	}

	/** 1px rounded outline matching {@link #box(GuiGraphics, int, int, int, int, int, int)}. */
	public static void border(GuiGraphics g, int x, int y, int w, int h, int r, int color) {
		r = Math.min(r, Math.min(w, h) / 2);
		int top = r <= 0 ? 0 : inset(r, 0);
		g.fill(x + top, y, x + w - top, y + 1, color);
		g.fill(x + top, y + h - 1, x + w - top, y + h, color);
		g.fill(x, y + Math.max(r, 1), x + 1, y + h - Math.max(r, 1), color);
		g.fill(x + w - 1, y + Math.max(r, 1), x + w, y + h - Math.max(r, 1), color);
		for (int i = 1; i < r; i++) {
			// Each corner row covers from its own inset to the row above's.
			int in = inset(r, i), to = Math.max(in + 1, inset(r, i - 1));
			g.fill(x + in, y + i, x + to, y + i + 1, color);
			g.fill(x + w - to, y + i, x + w - in, y + i + 1, color);
			g.fill(x + in, y + h - 1 - i, x + to, y + h - i, color);
			g.fill(x + w - to, y + h - 1 - i, x + w - in, y + h - i, color);
		}
	}

	/** How far row `i` of a radius-`r` corner is pulled in. */
	private static int inset(int r, int i) {
		double dy = r - i - 0.5;
		return r - (int) Math.round(Math.sqrt(r * r - dy * dy));
	}

	public static boolean inside(double mx, double my, int x, int y, int w, int h) {
		return mx >= x && my >= y && mx < x + w && my < y + h;
	}

	/** Flat button with an animated hover tint. Returns nothing; hit-test with {@link #inside}. */
	public static void button(GuiGraphics g, Font font, Object key, int x, int y, int w, int h, String label, boolean hovered, boolean primary) {
		float t = anim(key, hovered ? 1 : 0, 14);
		int accent = accent();
		int base = primary ? alpha(accent, 0.9f) : 0xB0141518;
		int hover = primary ? accent : 0xD0202228;
		box(g, x, y, w, h, mix(base, hover, t));
		border(g, x, y, w, h, mix(BORDER, alpha(accent, 0.7f), primary ? 0 : t));
		int color = primary ? ink() : mix(TEXT_2, TEXT, t);
		g.drawString(font, label, x + (w - font.width(label)) / 2, y + (h - 8) / 2, color, false);
	}

	/** iOS-style switch, 22×12. */
	public static void toggle(GuiGraphics g, Object key, int x, int y, boolean on) {
		float t = anim(key, on ? 1 : 0, 16);
		box(g, x, y, 22, 12, 6, mix(0xFF2A2C31, accent(), t));
		int kx = x + 2 + Math.round(t * 10);
		box(g, kx, y + 2, 8, 8, 4, 0xFFFFFFFF);
	}

	/** Slider track + knob; `frac` is 0..1. */
	public static void slider(GuiGraphics g, int x, int y, int w, float frac, boolean active) {
		box(g, x, y + 4, w, 4, 2, 0xFF2A2C31);
		box(g, x, y + 4, Math.max(4, Math.round(w * frac)), 4, 2, accent());
		int kx = x + Math.round(w * frac) - 4;
		box(g, kx, y + 2, 8, 8, 4, active ? accent() : 0xFFFFFFFF);
	}

	public static String trim(Font font, String s, int maxWidth) {
		if (font.width(s) <= maxWidth) return s;
		while (s.length() > 1 && font.width(s + "…") > maxWidth) s = s.substring(0, s.length() - 1);
		return s + "…";
	}
}
