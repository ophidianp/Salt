package gg.salt.hud;

import com.mojang.blaze3d.platform.InputConstants;
import gg.salt.config.SaltUi;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.AbstractButton;
import net.minecraft.client.gui.narration.NarrationElementOutput;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.InputWithModifiers;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;

/**
 * Right Shift in game: drag elements to move them, scroll over one to
 * resize it, right-click to hide it. A bar at the bottom opens on hover with
 * a toggle for every element. Saved to config/salthud.json when closed; the
 * launcher picks the change up on the next launch.
 */
public class HudEditorScreen extends Screen {
	private static final Map<String, String> NAMES = Map.ofEntries(
		Map.entry("fps", "FPS"), Map.entry("cps", "CPS"), Map.entry("ping", "Ping"),
		Map.entry("coordinates", "Coordinates"), Map.entry("keystrokes", "Keystrokes"),
		Map.entry("armor", "Armor"), Map.entry("potions", "Potions"), Map.entry("clock", "Clock"),
		Map.entry("speed", "Speed"), Map.entry("memory", "Memory"), Map.entry("server", "Server"),
		Map.entry("combo", "Combo"), Map.entry("reach", "Reach"), Map.entry("pack", "Resource pack"),
		Map.entry("crosshair", "Crosshair"));
	private static final int HANDLE_W = 90, HANDLE_H = 11, CHIP_H = 16, GAP = 4, PAD = 8, TITLE = 12;

	private final Screen parent;
	private final List<Toggle> toggles = new ArrayList<>();
	private final Object trayKey = new Object();
	private String dragging;
	private double grabX, grabY;
	/** Last cursor position from real mouse events (the render call's isn't reliable while a button is held). */
	private double cursorX, cursorY;
	private float open;
	private int trayH;
	/** Self-test only: keep the bar open without a real hover. */

	public HudEditorScreen(Screen parent) {
		super(Component.literal("saltHUD"));
		this.parent = parent;
	}

	static String name(String id) {
		return NAMES.getOrDefault(id, id);
	}

	/**
	 * One element's on/off switch in the bar. A real vanilla widget, so
	 * clicks go through Minecraft's own button handling.
	 */
	private final class Toggle extends AbstractButton {
		final HudConfig.Element el;

		Toggle(HudConfig.Element el, int w) {
			super(0, 0, w, CHIP_H, Component.literal(name(el.id)));
			this.el = el;
		}

		@Override
		public void onPress(InputWithModifiers input) {
			el.enabled = !el.enabled;
			trace("toggle " + el.id + " -> " + el.enabled);
		}

		@Override
		protected void extractContents(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
			int a = Math.round(open * 255);
			if (a < 8) return;
			boolean hot = isHoveredOrFocused();
			float t = SaltUi.anim(this, el.enabled ? 1 : 0, 16);
			int fill = SaltUi.mix(hot ? SaltUi.SURFACE_HI : SaltUi.SURFACE, SaltUi.alpha(SaltUi.accent(), 0.85f), t);
			int edge = hot ? SaltUi.accent() : SaltUi.mix(0x33FFFFFF, SaltUi.accent(), t);
			int text = SaltUi.mix(SaltUi.TEXT_2, SaltUi.ink(), t);
			SaltUi.box(g, getX(), getY(), getWidth(), getHeight(), SaltUi.alpha(fill, open));
			SaltUi.border(g, getX(), getY(), getWidth(), getHeight(), SaltUi.alpha(edge, open));
			String label = getMessage().getString();
			g.text(font, label, getX() + (getWidth() - font.width(label)) / 2, getY() + 4, SaltUi.alpha(text, open), false);
		}

		@Override
		protected void updateWidgetNarration(NarrationElementOutput output) {
			defaultButtonNarrationText(output);
		}
	}

	@Override
	protected void init() {
		toggles.clear();
		for (var el : SaltHud.config().elements) {
			Toggle t = new Toggle(el, font.width(name(el.id)) + 16);
			toggles.add(addRenderableWidget(t));
		}
		trayH = layout();
	}

	/** Lays the toggles out in centred rows; returns the open bar's height. */
	private int layout() {
		int maxW = Math.min(width - 24, 420);
		List<List<Toggle>> rows = new ArrayList<>();
		List<Toggle> row = new ArrayList<>();
		int rowW = 0;
		for (Toggle t : toggles) {
			if (!row.isEmpty() && rowW + GAP + t.getWidth() > maxW) {
				rows.add(row);
				row = new ArrayList<>();
				rowW = 0;
			}
			rowW += (row.isEmpty() ? 0 : GAP) + t.getWidth();
			row.add(t);
		}
		if (!row.isEmpty()) rows.add(row);
		int h = PAD * 2 + TITLE + rows.size() * CHIP_H + Math.max(0, rows.size() - 1) * GAP;
		int y = height - h + PAD + TITLE;
		for (List<Toggle> r : rows) {
			int w = r.stream().mapToInt(Toggle::getWidth).sum() + GAP * (r.size() - 1);
			int x = width / 2 - w / 2;
			for (Toggle t : r) {
				t.setPosition(x, y);
				x += t.getWidth() + GAP;
			}
			y += CHIP_H + GAP;
		}
		return h;
	}

	@Override
	public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
		SaltUi.frame();
		g.fill(0, 0, width, height, 0x55000000);
		// Center guides help line things up.
		g.fill(width / 2, 0, width / 2 + 1, height, 0x22FFFFFF);
		g.fill(0, height / 2, width, height / 2 + 1, 0x22FFFFFF);
		// The held element follows the cursor from mouse events, re-placed
		// every frame so it also picks up its real size once drawn.
		if (dragging != null) moveTo(cursorX, cursorY);
		HudRenderer.draw(g, SaltHud.config(), true);
		for (var e : HudRenderer.BOXES.entrySet()) {
			var b = e.getValue();
			boolean hot = e.getKey().equals(dragging) || b.contains(mouseX, mouseY);
			g.outline(b.x() - 1, b.y() - 1, b.w() + 2, b.h() + 2, hot ? SaltUi.accent() : 0x88FFFFFF);
		}
		g.centeredText(font, "Drag to move · scroll to resize · right-click to hide · Esc to save", width / 2, 6, SaltUi.TEXT_2);

		// Open the bar while hovering it (not while dragging something around).
		boolean over = dragging == null && overTray(mouseX, mouseY);
		open = SaltUi.anim(trayKey, over ? 1 : 0, 10);
		// Hidden toggles can't be clicked.
		for (Toggle t : toggles) t.visible = open > 0.5f;
		drawTray(g);
		super.extractRenderState(g, mouseX, mouseY, delta);
	}

	@Override
	public void extractBackground(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
		// No blur/dirt background: the game behind must stay visible.
	}

	// --- bar ---

	private int trayW() {
		return Math.min(width - 16, 440);
	}

	/** Mouse is over the bar (or its handle while it's closed). */
	private boolean overTray(double mx, double my) {
		int tw = trayW();
		// Once it starts opening, the whole bar counts, so moving up to the toggles keeps it open.
		if (open > 0.05f) return SaltUi.inside(mx, my, width / 2 - tw / 2, height - trayH - 4, tw, trayH + 4);
		return SaltUi.inside(mx, my, width / 2 - HANDLE_W / 2 - 20, height - HANDLE_H - 8, HANDLE_W + 40, HANDLE_H + 8);
	}

	private void drawTray(GuiGraphicsExtractor g) {
		int tw = trayW(), tx = width / 2 - tw / 2;
		long on = SaltHud.config().elements.stream().filter(e -> e.enabled).count();

		// Closed: a small handle at the bottom edge.
		float handle = 1 - open;
		if (handle > 0.02f) {
			int hy = height - HANDLE_H + Math.round(open * HANDLE_H);
			int hx = width / 2 - HANDLE_W / 2;
			SaltUi.box(g, hx, hy, HANDLE_W, HANDLE_H + 4, SaltUi.alpha(0xF0141518, handle));
			SaltUi.border(g, hx, hy, HANDLE_W, HANDLE_H + 4, SaltUi.alpha(0x33FFFFFF, handle));
			String hint = "▲ Elements (" + on + "/" + toggles.size() + ")";
			g.text(font, hint, width / 2 - font.width(hint) / 2, hy + 2, SaltUi.alpha(SaltUi.TEXT_2, handle), false);
		}
		if (open < 0.02f) return;

		// Open: fades in behind the toggles (which sit at their final spot so clicks always land).
		int ty = height - trayH;
		SaltUi.box(g, tx, ty, tw, trayH + 6, SaltUi.alpha(0xF00B0C0E, open));
		SaltUi.border(g, tx, ty, tw, trayH + 6, SaltUi.alpha(0x33FFFFFF, open));
		String title = "Click to turn elements on or off";
		g.text(font, title, width / 2 - font.width(title) / 2, ty + PAD, SaltUi.alpha(SaltUi.TEXT_3, open), false);
	}

	// --- input ---

	private String hit(double x, double y) {
		for (var e : HudRenderer.BOXES.entrySet()) if (e.getValue().contains(x, y)) return e.getKey();
		return null;
	}

	@Override
	public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
		cursorX = event.x();
		cursorY = event.y();
		trace("click " + fmt(event) + " over=" + hit(event.x(), event.y()) + " barOpen=" + open);
		// The toggles (real widgets) get first go.
		if (super.mouseClicked(event, doubleClick)) return true;
		String id = hit(event.x(), event.y());
		if (id == null) return false;
		// 26.3 numbers mouse buttons from 1 (left = 1, right = 3), so use the constants.
		if (event.button() == InputConstants.MOUSE_BUTTON_RIGHT) {
			setEnabled(id, false);
			return true;
		}
		if (event.button() != InputConstants.MOUSE_BUTTON_LEFT) return false;
		dragging = id;
		var b = HudRenderer.BOXES.get(id);
		grabX = event.x() - b.x();
		grabY = event.y() - b.y();
		return true;
	}

	@Override
	public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {
		cursorX = event.x();
		cursorY = event.y();
		if (dragging == null) return super.mouseDragged(event, dx, dy);
		moveTo(cursorX, cursorY);
		return true;
	}

	@Override
	public void mouseMoved(double x, double y) {
		cursorX = x;
		cursorY = y;
		super.mouseMoved(x, y);
	}

	/** Puts the held element under the cursor (keeping where it was grabbed). */
	private void moveTo(double mx, double my) {
		var b = HudRenderer.BOXES.get(dragging);
		int w = b != null ? b.w() : 40, h = b != null ? b.h() : 12;
		double left = Math.max(0, Math.min(width - w, mx - grabX));
		double top = Math.max(0, Math.min(height - h, my - grabY));
		// Snap to the screen centre lines.
		if (Math.abs(left + w / 2.0 - width / 2.0) < 4) left = width / 2.0 - w / 2.0;
		if (Math.abs(top + h / 2.0 - height / 2.0) < 4) top = height / 2.0 - h / 2.0;
		for (var el : SaltHud.config().elements) {
			if (el.id.equals(dragging)) place(el, left, top, w, h);
		}
	}

	/** Stores a top-left screen position as fractions from the element's anchor. */
	private void place(HudConfig.Element el, double left, double top, int w, int h) {
		double ax, ay;
		switch (el.anchor) {
			case "top_right" -> { ax = width - left - w; ay = top; }
			case "top_center" -> { ax = left + w / 2.0; ay = top; }
			case "center" -> { ax = left + w / 2.0; ay = top + h / 2.0; }
			case "bottom_left" -> { ax = left; ay = height - top - h; }
			case "bottom_right" -> { ax = width - left - w; ay = height - top - h; }
			default -> { ax = left; ay = top; }
		}
		el.x = (float) (ax / width);
		el.y = (float) (ay / height);
	}

	private void setEnabled(String id, boolean on) {
		trace("set " + id + " enabled=" + on);
		for (var el : SaltHud.config().elements) if (el.id.equals(id)) el.enabled = on;
	}

	@Override
	public boolean mouseReleased(MouseButtonEvent event) {
		trace("release " + fmt(event) + " dragging=" + dragging);
		if (dragging != null) moveTo(event.x(), event.y());
		dragging = null;
		return super.mouseReleased(event);
	}

	@Override
	public boolean mouseScrolled(double x, double y, double h, double v) {
		String id = hit(x, y);
		if (id == null) return false;
		for (var el : SaltHud.config().elements) {
			if (el.id.equals(id)) el.scale = (float) Math.max(0.5, Math.min(3.0, el.scale + v * 0.1));
		}
		return true;
	}

	@Override
	public void onClose() {
		SaltHud.config().save();
		// Back to where it was opened from (saltConfig, the title screen…), or the game.
		minecraft.gui.setScreen(parent);
	}

	/** Logs what the editor does, so a broken drag can be read from latest.log. */
	private static void trace(String what) {
		SaltHud.LOGGER.debug("[hud-editor] {}", what);
	}

	private static String fmt(MouseButtonEvent e) {
		return String.format(java.util.Locale.ROOT, "(%.0f,%.0f) button=%d", e.x(), e.y(), e.button());
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}
}
