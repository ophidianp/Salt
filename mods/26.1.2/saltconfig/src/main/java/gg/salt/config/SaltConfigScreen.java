package gg.salt.config;

import com.mojang.blaze3d.platform.InputConstants;
import net.fabricmc.loader.api.FabricLoader;
import net.minecraft.client.gui.GuiGraphicsExtractor;
import net.minecraft.client.gui.components.EditBox;
import net.minecraft.client.gui.screens.Screen;
import net.minecraft.client.input.MouseButtonEvent;
import net.minecraft.network.chat.Component;
import net.minecraft.util.FormattedCharSequence;

import java.util.ArrayList;
import java.util.HashMap;
import java.util.List;
import java.util.Map;
import java.util.Locale;

/**
 * The saltConfig window, which also replaces Mod Menu: Salt's own mods and
 * every other installed mod on the left, the selected one on the right.
 * Salt mods show their options inline, and so do mods that build their
 * settings with Cloth Config (read through {@link ClothBridge}). Anything
 * else (YACL, Sodium…) gets a button that opens its own screen. Everything is custom-drawn with
 * {@link SaltUi} and clipped to the window, whatever the GUI scale.
 */
public class SaltConfigScreen extends Screen {
	private static final int ROW = 28, HEADER = 30, ITEM = 20, GROUP = 14;
	private static String selectedKey;
	private static final boolean CLOTH = FabricLoader.getInstance().isModLoaded("cloth-config");
	/** Marks mods whose config screen isn't Cloth (so we don't rebuild it every frame). */
	private static final ClothBridge.Page NOT_CLOTH = new ClothBridge.Page(List.of(), () -> {}, new boolean[1]);

	/** One line of the sidebar: a group label or a selectable entry. */
	private record Line(String key, String label, boolean group, boolean dot, int dotColor, int y, int h) {}

	private final Screen parent;
	private EditBox search;
	private String filter = "";
	private double scroll, sideScroll;
	private Option.Slider draggingSlider;
	private ModConfig draggingMod;
	private ExtRow draggingExt;
	private int extTrackX, extTrackW;
	private final Map<String, ClothBridge.Page> pages = new HashMap<>();
	private int px, py, pw, ph, side;

	public SaltConfigScreen(Screen parent) {
		super(Component.literal("saltConfig"));
		this.parent = parent;
	}

	private boolean matches(String... text) {
		return filter.isEmpty() || String.join(" ", text).toLowerCase(Locale.ROOT).contains(filter);
	}

	/** Sidebar lines with y relative to the top of the list (before scrolling). */
	private List<Line> lines() {
		List<Line> out = new ArrayList<>();
		int y = 0;
		List<ModConfig> salt = SaltConfig.mods().stream().filter(m -> matches(m.name, m.description)).toList();
		if (!salt.isEmpty()) {
			out.add(new Line(null, "SALT", true, false, 0, y, GROUP));
			y += GROUP;
			for (ModConfig m : salt) {
				out.add(new Line("salt:" + m.id, m.name, false, true, m.enabled() ? 0xFF63E6B0 : 0xFF55575C, y, ITEM));
				y += ITEM + 2;
			}
		}
		List<ExternalMods.Mod> mods = ExternalMods.all().stream().filter(m -> matches(m.name(), m.id(), m.description())).toList();
		if (!mods.isEmpty()) {
			out.add(new Line(null, "MODS · " + mods.size(), true, false, 0, y + 4, GROUP));
			y += GROUP + 4;
			for (ExternalMods.Mod m : mods) {
				out.add(new Line("mod:" + m.id(), m.name(), false, m.openConfig() != null, SaltUi.accent(), y, ITEM));
				y += ITEM + 2;
			}
		}
		return out;
	}

	private String selected(List<Line> lines) {
		for (Line l : lines) if (!l.group() && l.key().equals(selectedKey)) return selectedKey;
		for (Line l : lines) if (!l.group()) return l.key();
		return null;
	}

	private ModConfig saltMod(String key) {
		if (key == null || !key.startsWith("salt:")) return null;
		for (ModConfig m : SaltConfig.mods()) if (key.equals("salt:" + m.id)) return m;
		return null;
	}

	private ExternalMods.Mod extMod(String key) {
		if (key == null || !key.startsWith("mod:")) return null;
		for (ExternalMods.Mod m : ExternalMods.all()) if (key.equals("mod:" + m.id())) return m;
		return null;
	}

	/** The inline rows for another mod, or null when it has none we can show. */
	private ClothBridge.Page page(ExternalMods.Mod m) {
		if (!CLOTH || m.openConfig() == null) return null;
		ClothBridge.Page p = pages.computeIfAbsent(m.id(), id -> {
			try {
				Screen s = m.openConfig().apply(this);
				ClothBridge.Page read = s == null ? null : ClothBridge.read(s);
				return read == null || read.rows().isEmpty() ? NOT_CLOTH : read;
			} catch (Throwable t) {
				SaltConfig.LOGGER.warn("Couldn't read the settings of {}: {}", id, t.toString());
				return NOT_CLOTH;
			}
		});
		return p == NOT_CLOTH ? null : p;
	}

	/** Logs what saltConfig can show inline for each mod (see SaltConfig's self-test). */
	void selfTest() {
		for (ExternalMods.Mod m : ExternalMods.all()) {
			ClothBridge.Page p = page(m);
			String what = p != null ? p.rows().size() + " inline rows" : m.openConfig() != null ? "own screen" : "no settings";
			SaltConfig.LOGGER.info("selftest {}: {}", m.id(), what);
			if (p != null) for (ExtRow r : p.rows()) SaltConfig.LOGGER.info("selftest   {} {} = {}", r.kind(), r.label(), r.value() == null ? (r.bool() == null ? "" : r.bool().getAsBoolean()) : r.value().get());
		}
	}

	/** Runs each changed mod's own save code. */
	private void savePages() {
		pages.forEach((id, p) -> {
			if (!p.dirty()[0]) return;
			p.dirty()[0] = false;
			try {
				p.save().run();
			} catch (Throwable t) {
				SaltConfig.LOGGER.warn("Couldn't save the settings of {}", id, t);
			}
		});
	}

	@Override
	protected void init() {
		pw = Math.min(width - 16, 620);
		ph = Math.min(height - 16, 380);
		px = (width - pw) / 2;
		py = (height - ph) / 2;
		side = Math.max(96, Math.min(150, pw / 4));
		search = new EditBox(font, px + 14, py + HEADER + 10, side - 24, 12, Component.literal("Search"));
		search.setBordered(false);
		search.setHint(Component.literal("Search mods…"));
		search.setMaxLength(40);
		search.setValue(filter);
		search.setResponder(s -> {
			filter = s.toLowerCase(Locale.ROOT).trim();
			scroll = 0;
			sideScroll = 0;
		});
		addRenderableWidget(search);
	}

	// sidebar list area
	private int listTop() { return py + HEADER + 30; }
	private int listBottom() { return py + ph - 6; }
	// content area
	private int contentX() { return px + side + 14; }
	private int contentW() { return pw - side - 28; }
	private int contentTop() { return py + HEADER + 42; }
	private int contentBottom() { return py + ph - 8; }

	@Override
	public void extractRenderState(GuiGraphicsExtractor g, int mouseX, int mouseY, float delta) {
		SaltUi.frame();
		float open = SaltUi.anim(this, 1, 10);
		g.fill(0, 0, width, height, SaltUi.alpha(0x99000000, open));

		// window
		// The window gets twice the control radius, like the launcher's cards.
		int r = SaltTheme.radius() * 2;
		SaltUi.box(g, px, py, pw, ph, r, SaltUi.BG);
		SaltUi.border(g, px, py, pw, ph, r, SaltUi.BORDER);
		g.fill(px + 1, py + HEADER, px + pw - 1, py + HEADER + 1, SaltUi.BORDER);
		g.fill(px + side, py + HEADER + 1, px + side + 1, py + ph - r, SaltUi.BORDER);

		// header
		g.text(font, "salt", px + 12, py + 11, SaltUi.TEXT, false);
		g.text(font, "Config", px + 12 + font.width("salt "), py + 11, SaltUi.TEXT_3, false);
		boolean closeHot = SaltUi.inside(mouseX, mouseY, px + pw - 24, py + 6, 18, 18);
		g.text(font, "✕", px + pw - 18, py + 11, closeHot ? SaltUi.TEXT : SaltUi.TEXT_3, false);

		// sidebar
		SaltUi.box(g, px + 8, py + HEADER + 6, side - 16, 16, 8, SaltUi.SURFACE);
		List<Line> lines = lines();
		String sel = selected(lines);
		clampSideScroll(lines);
		g.enableScissor(px + 1, listTop(), px + side, listBottom());
		for (Line l : lines) {
			int y = listTop() + l.y() - (int) sideScroll;
			if (y + l.h() < listTop() || y > listBottom()) continue;
			if (l.group()) {
				g.text(font, l.label(), px + 12, y + 3, SaltUi.TEXT_3, false);
				continue;
			}
			boolean hot = mouseY >= listTop() && mouseY < listBottom() && SaltUi.inside(mouseX, mouseY, px + 6, y, side - 12, l.h());
			boolean isSel = l.key().equals(sel);
			float t = SaltUi.anim("side:" + l.key(), isSel ? 1 : hot ? 0.5f : 0, 14);
			if (t > 0) SaltUi.box(g, px + 6, y, side - 12, l.h(), SaltUi.alpha(SaltUi.alpha(SaltUi.accent(), 0.17f), t));
			if (isSel) SaltUi.box(g, px + 6, y + 5, 2, 10, 1, SaltUi.accent());
			if (l.dot()) SaltUi.box(g, px + 14, y + 8, 4, 4, 2, l.dotColor());
			g.text(font, SaltUi.trim(font, l.label(), side - 36), px + 24, y + 6, isSel ? SaltUi.TEXT : SaltUi.TEXT_2, false);
		}
		g.disableScissor();

		// content
		ModConfig salt = saltMod(sel);
		ExternalMods.Mod ext = extMod(sel);
		g.enableScissor(px + side + 1, py + HEADER + 1, px + pw - 1, py + ph - 1);
		if (salt != null) drawSalt(g, salt, mouseX, mouseY);
		else if (ext != null) drawExternal(g, ext, mouseX, mouseY);
		else g.text(font, "No mods match your search.", contentX(), py + HEADER + 12, SaltUi.TEXT_3, false);
		g.disableScissor();
		super.extractRenderState(g, mouseX, mouseY, delta);
	}

	private void clampSideScroll(List<Line> lines) {
		int total = lines.isEmpty() ? 0 : lines.getLast().y() + lines.getLast().h();
		int max = Math.max(0, total - (listBottom() - listTop()));
		sideScroll = Math.max(0, Math.min(max, sideScroll));
	}

	/** Width a control takes on the right of an option row. */
	private int controlWidth(ModConfig m, Option o) {
		return switch (o) {
			case Option.Toggle t -> 26;
			case Option.Slider s -> Math.min(124, contentW() / 2);
			case Option.Choice c -> choiceWidth(m, c);
			case Option.Action a -> Math.min(90, contentW() / 3);
		};
	}

	private int choiceWidth(ModConfig m, Option.Choice c) {
		return Math.min(Math.max(70, font.width("‹ " + m.choice(c.key()) + " ›") + 12), contentW() / 2);
	}

	private void drawSalt(GuiGraphicsExtractor g, ModConfig m, int mouseX, int mouseY) {
		int x = contentX(), w = contentW();
		g.text(font, SaltUi.trim(font, m.name, w - 34), x, py + HEADER + 10, SaltUi.TEXT, false);
		g.text(font, SaltUi.trim(font, m.description, w - 34), x, py + HEADER + 22, SaltUi.TEXT_3, false);
		SaltUi.toggle(g, "en:" + m.id, x + w - 22, py + HEADER + 12, m.enabled());

		clampScroll(m);
		g.enableScissor(x - 6, contentTop() - 2, x + w + 6, contentBottom());
		int y = contentTop() - (int) scroll;
		for (Option o : m.options) {
			if (y + ROW > contentTop() - 2 && y < contentBottom()) {
				boolean hot = mouseY >= contentTop() - 2 && mouseY < contentBottom() && SaltUi.inside(mouseX, mouseY, x - 4, y, w + 8, ROW - 4);
				SaltUi.box(g, x - 4, y, w + 8, ROW - 4, hot ? SaltUi.SURFACE_HI : SaltUi.SURFACE);
				int textW = w - controlWidth(m, o) - 14;
				g.text(font, SaltUi.trim(font, o.label(), textW), x + 4, y + 4, m.enabled() ? SaltUi.TEXT : SaltUi.TEXT_3, false);
				if (!o.description().isEmpty()) g.text(font, SaltUi.trim(font, o.description(), textW), x + 4, y + 14, SaltUi.TEXT_3, false);
				int cx = x + w - 4;
				switch (o) {
					case Option.Toggle t -> SaltUi.toggle(g, m.id + ":" + t.key(), cx - 22, y + 6, m.bool(t.key()));
					case Option.Slider s -> {
						double v = m.number(s.key());
						float frac = (float) ((v - s.min()) / (s.max() - s.min()));
						int tw = controlWidth(m, o) - 34;
						SaltUi.slider(g, cx - tw - 30, y + 6, tw, frac, draggingSlider == s);
						String val = s.step() >= 1 ? String.valueOf((int) v) : String.format(Locale.ROOT, "%.1f", v);
						g.text(font, val, cx - font.width(val), y + 8, SaltUi.TEXT_2, false);
					}
					case Option.Choice c -> {
						int bw = choiceWidth(m, c);
						String val = SaltUi.trim(font, "‹ " + m.choice(c.key()) + " ›", bw - 8);
						SaltUi.button(g, font, m.id + ":" + c.key(), cx - bw, y + 3, bw, 16, val, hot, false);
					}
					case Option.Action a -> {
						int bw = controlWidth(m, o);
						SaltUi.button(g, font, a, cx - bw, y + 3, bw, 16, "Open", hot, true);
					}
				}
			}
			y += ROW;
		}
		g.disableScissor();
	}

	private static final int FULL_W = 78;

	private void drawExternal(GuiGraphicsExtractor g, ExternalMods.Mod m, int mouseX, int mouseY) {
		int x = contentX(), w = contentW();
		ClothBridge.Page page = page(m);
		int titleW = page != null ? w - FULL_W - 8 : w;
		g.text(font, SaltUi.trim(font, m.name(), titleW), x, py + HEADER + 10, SaltUi.TEXT, false);
		String sub = m.version() + (m.authors().isEmpty() ? "" : " · " + m.authors()) + (m.library() ? " · Library" : "");
		g.text(font, SaltUi.trim(font, sub, titleW), x, py + HEADER + 22, SaltUi.TEXT_3, false);

		if (page != null) {
			boolean hot = SaltUi.inside(mouseX, mouseY, x + w - FULL_W, py + HEADER + 9, FULL_W, 16);
			SaltUi.button(g, font, "full:" + m.id(), x + w - FULL_W, py + HEADER + 9, FULL_W, 16, "Full settings", hot, false);
			drawRows(g, page.rows(), mouseX, mouseY);
			return;
		}

		int y = contentTop();
		if (m.openConfig() != null) {
			boolean hot = SaltUi.inside(mouseX, mouseY, x, y, 110, 18);
			SaltUi.button(g, font, "open:" + m.id(), x, y, Math.min(110, w), 18, "Open settings", hot, true);
			y += 28;
		} else {
			g.text(font, SaltUi.trim(font, "This mod has no settings screen.", w), x, y + 4, SaltUi.TEXT_3, false);
			y += 22;
		}
		if (!m.description().isEmpty()) {
			for (FormattedCharSequence line : font.split(Component.literal(m.description()), w)) {
				if (y + 10 > contentBottom()) break;
				g.text(font, line, x, y, SaltUi.TEXT_2, false);
				y += 11;
			}
		}
		g.text(font, SaltUi.trim(font, m.id(), w), x, contentBottom() - 10, SaltUi.alpha(SaltUi.TEXT_3, 0.6f), false);
	}

	/** Right-hand control width for an inline row of another mod. */
	private int extControlWidth(ExtRow r) {
		int w = contentW();
		return switch (r.kind()) {
			case TOGGLE -> 26;
			case SLIDER -> Math.min(150, w / 2);
			case CHOICE, STEPPER -> Math.min(Math.max(70, font.width("‹ " + r.value().get() + " ›") + 12), w / 2);
			case READONLY -> Math.min(font.width(r.value().get()) + 4, w / 2);
			default -> 0;
		};
	}

	private int rowsHeight(List<ExtRow> rows) {
		int h = 0;
		for (ExtRow r : rows) h += r.height() + 4;
		return h;
	}

	private void drawRows(GuiGraphicsExtractor g, List<ExtRow> rows, int mouseX, int mouseY) {
		int x = contentX(), w = contentW();
		scroll = Math.max(0, Math.min(Math.max(0, rowsHeight(rows) - (contentBottom() - contentTop())), scroll));
		g.enableScissor(x - 6, contentTop() - 2, x + w + 6, contentBottom());
		int y = contentTop() - (int) scroll;
		for (ExtRow r : rows) {
			int h = r.height();
			if (y + h > contentTop() - 2 && y < contentBottom()) {
				switch (r.kind()) {
					case HEADER -> g.text(font, SaltUi.trim(font, r.label(), w), x, y + 7, SaltUi.accent(), false);
					case TEXT -> g.text(font, SaltUi.trim(font, r.label(), w), x, y + 2, SaltUi.TEXT_3, false);
					default -> drawExtRow(g, r, x, y, w, h, mouseX, mouseY);
				}
			}
			y += h + 4;
		}
		g.disableScissor();
	}

	private void drawExtRow(GuiGraphicsExtractor g, ExtRow r, int x, int y, int w, int h, int mouseX, int mouseY) {
		boolean hot = mouseY >= contentTop() - 2 && mouseY < contentBottom() && SaltUi.inside(mouseX, mouseY, x - 4, y, w + 8, h);
		SaltUi.box(g, x - 4, y, w + 8, h, hot ? SaltUi.SURFACE_HI : SaltUi.SURFACE);
		int cw = extControlWidth(r);
		int textW = w - cw - 14;
		boolean two = !r.description().isEmpty();
		g.text(font, SaltUi.trim(font, r.label(), textW), x + 4, two ? y + 3 : y + 8, SaltUi.TEXT, false);
		if (two) g.text(font, SaltUi.trim(font, r.description(), textW), x + 4, y + 13, SaltUi.TEXT_3, false);
		int cx = x + w - 4;
		switch (r.kind()) {
			case TOGGLE -> SaltUi.toggle(g, r, cx - 22, y + 6, r.bool().getAsBoolean());
			case SLIDER -> {
				String val = SaltUi.trim(font, r.value().get(), 40);
				SaltUi.slider(g, cx - cw, y + 6, cw - 46, (float) r.frac().getAsDouble(), draggingExt == r);
				g.text(font, val, cx - font.width(val), y + 8, SaltUi.TEXT_2, false);
			}
			case CHOICE, STEPPER -> {
				String val = SaltUi.trim(font, "‹ " + r.value().get() + " ›", cw - 8);
				SaltUi.button(g, font, r, cx - cw, y + 4, cw, 16, val, hot, false);
			}
			case READONLY -> {
				String val = SaltUi.trim(font, r.value().get(), cw);
				g.text(font, val, cx - font.width(val), y + 8, SaltUi.TEXT_3, false);
			}
			default -> {}
		}
	}

	/** Handles a click on another mod's inline rows; true if it hit one. */
	private boolean clickRows(List<ExtRow> rows, double mx, double my, int button) {
		if (my < contentTop() - 2 || my >= contentBottom()) return false;
		int x = contentX(), w = contentW();
		int y = contentTop() - (int) scroll;
		for (ExtRow r : rows) {
			int h = r.height();
			if (SaltUi.inside(mx, my, x - 4, y, w + 8, h)) {
				switch (r.kind()) {
					case TOGGLE -> r.setBool().accept(!r.bool().getAsBoolean());
					// left click steps forward, right click back
					case CHOICE, STEPPER -> (button == InputConstants.MOUSE_BUTTON_RIGHT ? r.prev() : r.next()).run();
					case SLIDER -> {
						int cw = extControlWidth(r);
						extTrackX = x + w - 4 - cw;
						extTrackW = cw - 46;
						if (mx >= extTrackX - 4) {
							draggingExt = r;
							dragExt(mx);
						}
					}
					default -> {}
				}
				return true;
			}
			y += h + 4;
		}
		return false;
	}

	private void dragExt(double mx) {
		double f = Math.max(0, Math.min(1, (mx - extTrackX) / (double) extTrackW));
		draggingExt.setFrac().accept(f);
	}

	private void clampScroll(ModConfig m) {
		int max = Math.max(0, m.options.size() * ROW - (contentBottom() - contentTop()));
		scroll = Math.max(0, Math.min(max, scroll));
	}

	@Override
	public boolean mouseClicked(MouseButtonEvent event, boolean doubleClick) {
		double mx = event.x(), my = event.y();
		if (SaltUi.inside(mx, my, px + pw - 24, py + 6, 18, 18)) {
			onClose();
			return true;
		}
		List<Line> lines = lines();
		String sel = selected(lines);
		// sidebar
		if (my >= listTop() && my < listBottom()) {
			for (Line l : lines) {
				int y = listTop() + l.y() - (int) sideScroll;
				if (!l.group() && SaltUi.inside(mx, my, px + 6, y, side - 12, l.h())) {
					if (!l.key().equals(sel)) savePages();
					selectedKey = l.key();
					scroll = 0;
					return true;
				}
			}
		}
		ExternalMods.Mod ext = extMod(sel);
		if (ext != null) {
			ClothBridge.Page page = page(ext);
			if (page != null) {
				if (SaltUi.inside(mx, my, contentX() + contentW() - FULL_W, py + HEADER + 9, FULL_W, 16)) {
					openExternal(ext);
					return true;
				}
				if (clickRows(page.rows(), mx, my, event.button())) return true;
			} else if (ext.openConfig() != null && SaltUi.inside(mx, my, contentX(), contentTop(), 110, 18)) {
				openExternal(ext);
				return true;
			}
		}
		ModConfig m = saltMod(sel);
		if (m != null) {
			int x = contentX(), w = contentW();
			if (SaltUi.inside(mx, my, x + w - 26, py + HEADER + 8, 30, 20)) {
				m.setEnabled(!m.enabled());
				return true;
			}
			int y = contentTop() - (int) scroll;
			for (Option o : m.options) {
				if (my >= contentTop() - 2 && my < contentBottom() && SaltUi.inside(mx, my, x - 4, y, w + 8, ROW - 4)) {
					int cx = x + w - 4;
					switch (o) {
						case Option.Toggle t -> m.set(t.key(), !m.bool(t.key()));
						case Option.Slider s -> {
							int tw = controlWidth(m, o) - 34;
							if (mx >= cx - tw - 34) {
								draggingSlider = s;
								draggingMod = m;
								setSlider(mx);
							}
						}
						case Option.Choice c -> {
							List<String> cs = c.choices();
							int i = cs.indexOf(m.choice(c.key()));
							int step = event.button() == InputConstants.MOUSE_BUTTON_RIGHT ? -1 : 1;
							m.set(c.key(), cs.get(Math.floorMod(i + step, cs.size())));
						}
						case Option.Action a -> a.run().run();
					}
					return true;
				}
				y += ROW;
			}
		}
		return super.mouseClicked(event, doubleClick);
	}

	private void openExternal(ExternalMods.Mod m) {
		// Save inline changes first; the full screen re-reads the mod's config.
		savePages();
		pages.remove(m.id());
		try {
			Screen s = m.openConfig().apply(this);
			if (s != null) minecraft.setScreen(s);
		} catch (Throwable t) {
			SaltConfig.LOGGER.warn("Couldn't open the settings of {}", m.id(), t);
		}
	}

	private void setSlider(double mx) {
		Option.Slider s = draggingSlider;
		int tw = controlWidth(draggingMod, s) - 34;
		int trackX = contentX() + contentW() - 4 - tw - 30;
		double frac = Math.max(0, Math.min(1, (mx - trackX) / (double) tw));
		double v = s.min() + frac * (s.max() - s.min());
		v = Math.round(v / s.step()) * s.step();
		draggingMod.set(s.key(), Math.max(s.min(), Math.min(s.max(), v)));
	}

	@Override
	public boolean mouseDragged(MouseButtonEvent event, double dx, double dy) {
		if (draggingSlider != null) {
			setSlider(event.x());
			return true;
		}
		if (draggingExt != null) {
			dragExt(event.x());
			return true;
		}
		return super.mouseDragged(event, dx, dy);
	}

	@Override
	public boolean mouseReleased(MouseButtonEvent event) {
		draggingSlider = null;
		draggingMod = null;
		draggingExt = null;
		return super.mouseReleased(event);
	}

	@Override
	public boolean mouseScrolled(double x, double y, double h, double v) {
		if (x < px + side) {
			sideScroll -= v * 16;
			return true;
		}
		scroll -= v * 16;
		return true;
	}

	@Override
	public void onClose() {
		SaltConfig.save();
		savePages();
		minecraft.setScreen(parent);
	}

	@Override
	public boolean isPauseScreen() {
		return false;
	}
}
