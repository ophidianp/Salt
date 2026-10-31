package gg.salt.hud;

import net.minecraft.client.DeltaTracker;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphics;

import java.util.HashMap;
import java.util.Map;

/** Draws every enabled saltHUD element (also used by the in-game editor). */
public final class HudRenderer {
	private HudRenderer() {}

	/** Screen-space box of each element from the last frame, for the editor. */
	public record Box(int x, int y, int w, int h) {
		boolean contains(double px, double py) {
			return px >= x && py >= y && px < x + w && py < y + h;
		}
	}

	static final Map<String, Box> BOXES = new HashMap<>();

	public static void render(GuiGraphics g, DeltaTracker delta) {
		Minecraft mc = Minecraft.getInstance();
		if (mc.player == null || !SaltHud.enabled()) return;
		draw(g, SaltHud.config(), false);
	}

	static void draw(GuiGraphics g, HudConfig cfg, boolean editing) {
		Minecraft mc = Minecraft.getInstance();
		BOXES.clear();
		int sw = g.guiWidth(), sh = g.guiHeight();
		for (HudConfig.Element el : cfg.elements) {
			if (!el.enabled || el.id.equals("crosshair")) continue;
			Elements.Widget w;
			try {
				w = Elements.build(el, mc, cfg.global, editing);
			} catch (RuntimeException e) {
				// One broken element must never take the whole HUD (or game) down.
				continue;
			}
			if (w == null) continue;
			boolean bg = cfg.global.background && !el.id.equals("keystrokes") && !el.id.equals("armor");
			int pad = bg ? cfg.global.padding : 0;
			float s = Math.max(0.25f, cfg.global.scale * el.scale * SaltHud.sizeMultiplier());
			int bw = w.width() + pad * 2, bh = w.height() + pad * 2;
			float sw2 = bw * s, sh2 = bh * s;
			float ax = el.x * sw, ay = el.y * sh;
			float left, top;
			switch (el.anchor) {
				case "top_right" -> { left = sw - ax - sw2; top = ay; }
				case "top_center" -> { left = ax - sw2 / 2; top = ay; }
				case "center" -> { left = ax - sw2 / 2; top = ay - sh2 / 2; }
				case "bottom_left" -> { left = ax; top = sh - ay - sh2; }
				case "bottom_right" -> { left = sw - ax - sw2; top = sh - ay - sh2; }
				default -> { left = ax; top = ay; }
			}
			left = Math.max(0, Math.min(sw - sw2, left));
			top = Math.max(0, Math.min(sh - sh2, top));

			g.pose().pushMatrix();
			g.pose().translate(left, top);
			g.pose().scale(s, s);
			if (bg) g.fill(0, 0, bw, bh, cfg.global.backgroundColor);
			w.draw(g, pad, pad);
			g.pose().popMatrix();
			BOXES.put(el.id, new Box(Math.round(left), Math.round(top), Math.round(sw2), Math.round(sh2)));
		}
	}

	/** saltHUD's custom crosshair, or vanilla's when it's off. */
	public static boolean drawCrosshair(GuiGraphics g) {
		HudConfig.Element el = SaltHud.config().elements.stream().filter(e -> e.id.equals("crosshair")).findFirst().orElse(null);
		Minecraft mc = Minecraft.getInstance();
		if (el == null || !el.enabled || !SaltHud.crosshairAllowed() || !mc.options.getCameraType().isFirstPerson()) return false;
		int cx = g.guiWidth() / 2, cy = g.guiHeight() / 2;
		int color = HudConfig.parseColor(el.opt("color", "#FFFFFFFF"), 0xFFFFFFFF);
		int gap = el.optInt("gap", 2);
		int len = Math.round(4 * el.scale);
		switch (el.opt("style", "plus")) {
			case "dot" -> g.fill(cx - 1, cy - 1, cx + 1, cy + 1, color);
			case "circle" -> {
				for (int a = 0; a < 360; a += 15) {
					int px = cx + (int) Math.round(Math.cos(Math.toRadians(a)) * (gap + 2));
					int py = cy + (int) Math.round(Math.sin(Math.toRadians(a)) * (gap + 2));
					g.fill(px, py, px + 1, py + 1, color);
				}
			}
			case "cross" -> {
				for (int i = gap; i < gap + len; i++) {
					g.fill(cx + i, cy + i, cx + i + 1, cy + i + 1, color);
					g.fill(cx - i - 1, cy + i, cx - i, cy + i + 1, color);
					g.fill(cx + i, cy - i - 1, cx + i + 1, cy - i, color);
					g.fill(cx - i - 1, cy - i - 1, cx - i, cy - i, color);
				}
			}
			default -> {
				g.fill(cx - gap - len, cy, cx - gap, cy + 1, color);
				g.fill(cx + gap + 1, cy, cx + gap + 1 + len, cy + 1, color);
				g.fill(cx, cy - gap - len, cx + 1, cy - gap, color);
				g.fill(cx, cy + gap + 1, cx + 1, cy + gap + 1 + len, color);
			}
		}
		return true;
	}
}
