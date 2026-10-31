package gg.salt.hud;

import gg.salt.config.SaltUi;
import net.minecraft.client.Minecraft;
import net.minecraft.client.gui.Font;
import net.minecraft.client.gui.GuiGraphics;
import net.minecraft.client.multiplayer.PlayerInfo;
import net.minecraft.client.multiplayer.ServerData;
import net.minecraft.core.BlockPos;
import net.minecraft.server.packs.repository.Pack;
import net.minecraft.world.effect.MobEffectInstance;
import net.minecraft.world.entity.EquipmentSlot;
import net.minecraft.world.item.ItemStack;

import java.time.LocalTime;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.List;
import java.util.Locale;

/** Builds the content of each saltHUD element for the current frame. */
final class Elements {
	private Elements() {}

	/** A piece of an element: knows its size and how to draw itself at (x, y). */
	interface Widget {
		int width();
		int height();
		void draw(GuiGraphics g, int x, int y);
	}

	record Seg(String text, boolean accent) {}

	/** Lines of text, each made of plain + accent-coloured segments. */
	static final class Text implements Widget {
		final List<List<Seg>> lines = new ArrayList<>();
		final Font font;
		final HudConfig.Global style;

		Text(Font font, HudConfig.Global style) {
			this.font = font;
			this.style = style;
		}

		Text line(Object... parts) {
			List<Seg> l = new ArrayList<>();
			for (Object p : parts) l.add(p instanceof Seg s ? s : new Seg(String.valueOf(p), false));
			lines.add(l);
			return this;
		}

		static Seg acc(Object o) {
			return new Seg(String.valueOf(o), true);
		}

		public int width() {
			int w = 0;
			for (List<Seg> l : lines) {
				int lw = 0;
				for (Seg s : l) lw += font.width(s.text());
				w = Math.max(w, lw);
			}
			return w;
		}

		public int height() {
			return lines.isEmpty() ? 0 : lines.size() * (font.lineHeight + 1) - 1;
		}

		public void draw(GuiGraphics g, int x, int y) {
			int ly = y;
			for (List<Seg> l : lines) {
				int lx = x;
				for (Seg s : l) {
					g.drawString(font, s.text(), lx, ly, s.accent() ? SaltUi.accent() : style.textColor, style.textShadow);
					lx += font.width(s.text());
				}
				ly += font.lineHeight + 1;
			}
		}
	}

	static Widget build(HudConfig.Element el, Minecraft mc, HudConfig.Global style, boolean editing) {
		Font f = mc.font;
		Text t = new Text(f, style);
		switch (el.id) {
			case "fps" -> t.line(Text.acc(mc.getFps()), " " + el.opt("label", "FPS"));
			case "cps" -> {
				if (el.opt("showRight", true)) t.line(Text.acc(Trackers.leftCps()), " | ", Text.acc(Trackers.rightCps()), " CPS");
				else t.line(Text.acc(Trackers.leftCps()), " CPS");
			}
			case "ping" -> {
				int ping = ping(mc);
				if (ping < 0 && !editing) return null;
				t.line(Text.acc(Math.max(ping, 0)), " ms");
			}
			case "coordinates" -> {
				if (mc.player == null) return editing ? t.line("XYZ: ", Text.acc("0, 64, 0")) : null;
				BlockPos p = mc.player.blockPosition();
				String dir = el.opt("showDirection", true) ? " " + facing(mc.player.getYRot()) : "";
				t.line("XYZ: ", Text.acc(p.getX() + ", " + p.getY() + ", " + p.getZ()), dir);
				if (el.opt("showBiome", true) && mc.level != null) t.line("Biome: ", Text.acc(biome(mc, p)));
			}
			case "keystrokes" -> {
				return new Keystrokes(mc, style, el);
			}
			case "armor" -> {
				Armor a = new Armor(mc, style, el.opt("vertical", true), el.opt("showDurability", true));
				return a.items.isEmpty() && !editing ? null : a;
			}
			case "potions" -> {
				if (mc.player == null || mc.player.getActiveEffects().isEmpty()) return editing ? t.line("Speed II ", Text.acc("1:30")) : null;
				for (MobEffectInstance e : mc.player.getActiveEffects()) {
					String name = e.getEffect().value().getDisplayName().getString();
					String amp = e.getAmplifier() > 0 ? " " + roman(e.getAmplifier() + 1) : "";
					t.line(name + amp + " ", Text.acc(e.isInfiniteDuration() ? "∞" : time(e.getDuration())));
				}
			}
			case "clock" -> t.line(LocalTime.now().format(DateTimeFormatter.ofPattern(el.opt("twentyFour", true) ? "HH:mm" : "h:mm a", Locale.ROOT)));
			case "speed" -> {
				double bps = Trackers.speed();
				boolean kmh = "kmh".equals(el.opt("unit", "bps"));
				t.line(Text.acc(String.format(Locale.ROOT, "%.2f", kmh ? bps * 3.6 : bps)), kmh ? " km/h" : " b/s");
			}
			case "memory" -> {
				Runtime rt = Runtime.getRuntime();
				long used = rt.totalMemory() - rt.freeMemory(), max = rt.maxMemory();
				t.line("Mem ", Text.acc((used * 100 / max) + "%"), String.format(Locale.ROOT, " %.1f/%.1f GB", used / 1073741824.0, max / 1073741824.0));
			}
			case "server" -> {
				ServerData s = mc.getCurrentServer();
				if (s == null && mc.hasSingleplayerServer()) t.line("Singleplayer");
				else if (s != null) t.line(s.ip);
				else if (editing) t.line("play.example.net");
				else return null;
			}
			case "combo" -> {
				int c = Trackers.combo();
				if (c == 0 && !editing) return null;
				t.line(Text.acc(c), " Combo");
			}
			case "reach" -> {
				double r = Trackers.reach();
				if (r < 0 && !editing) return null;
				t.line(Text.acc(String.format(Locale.ROOT, "%.2f", Math.max(r, 0))), " blocks");
			}
			case "pack" -> {
				String name = packName(mc);
				if (name == null && !editing) return null;
				t.line(name == null ? "Default" : name);
			}
			default -> {
				return null;
			}
		}
		return t;
	}

	static int ping(Minecraft mc) {
		if (mc.player == null || mc.getConnection() == null || mc.hasSingleplayerServer()) return -1;
		PlayerInfo info = mc.getConnection().getPlayerInfo(mc.player.getUUID());
		return info == null ? -1 : info.getLatency();
	}

	static String facing(float yRot) {
		String[] dirs = {"S", "SW", "W", "NW", "N", "NE", "E", "SE"};
		int i = Math.floorMod(Math.round(yRot / 45f), 8);
		return dirs[i];
	}

	static String biome(Minecraft mc, BlockPos p) {
		return mc.level.getBiome(p).unwrapKey().map(k -> pretty(k.identifier().getPath())).orElse("?");
	}

	static String pretty(String path) {
		StringBuilder sb = new StringBuilder();
		for (String w : path.split("_")) {
			if (w.isEmpty()) continue;
			if (!sb.isEmpty()) sb.append(' ');
			sb.append(Character.toUpperCase(w.charAt(0))).append(w.substring(1));
		}
		return sb.toString();
	}

	static String time(int ticks) {
		int s = ticks / 20;
		return (s / 60) + ":" + String.format(Locale.ROOT, "%02d", s % 60);
	}

	static String roman(int n) {
		String[] r = {"", "I", "II", "III", "IV", "V", "VI", "VII", "VIII", "IX", "X"};
		return n < r.length ? r[n] : String.valueOf(n);
	}

	static String packName(Minecraft mc) {
		String last = null;
		for (Pack p : mc.getResourcePackRepository().getSelectedPacks()) {
			String id = p.getId();
			if (id.equals("vanilla") || id.startsWith("fabric") || id.equals("high_contrast") || id.startsWith("programmer_art")) continue;
			last = p.getTitle().getString();
		}
		return last;
	}

	// ---------------------------------------------------------------- widgets

	/** WASD + mouse + space, lighting up while held. */
	static final class Keystrokes implements Widget {
		static final int K = 22, GAP = 2;
		final Minecraft mc;
		final HudConfig.Global style;
		final boolean mouse, space, cps;

		Keystrokes(Minecraft mc, HudConfig.Global style, HudConfig.Element el) {
			this.mc = mc;
			this.style = style;
			this.mouse = el.opt("showMouse", true);
			this.space = el.opt("showSpace", true);
			this.cps = el.opt("showCps", true);
		}

		public int width() { return K * 3 + GAP * 2; }

		public int height() {
			return K * 2 + GAP + (mouse ? GAP + K : 0) + (space ? GAP + 10 : 0);
		}

		void key(GuiGraphics g, int x, int y, int w, int h, String label, boolean down) {
			g.fill(x, y, x + w, y + h, down ? 0xCCFFFFFF : style.backgroundColor);
			Font f = mc.font;
			int color = down ? 0xFF000000 : style.textColor;
			String[] parts = label.split("\n");
			int ty = y + (h - parts.length * (f.lineHeight + 1)) / 2 + 1;
			for (String part : parts) {
				g.drawString(f, part, x + (w - f.width(part)) / 2, ty, color, !down && style.textShadow);
				ty += f.lineHeight + 1;
			}
		}

		public void draw(GuiGraphics g, int x, int y) {
			var o = mc.options;
			key(g, x + K + GAP, y, K, K, "W", o.keyUp.isDown());
			int r2 = y + K + GAP;
			key(g, x, r2, K, K, "A", o.keyLeft.isDown());
			key(g, x + K + GAP, r2, K, K, "S", o.keyDown.isDown());
			key(g, x + 2 * (K + GAP), r2, K, K, "D", o.keyRight.isDown());
			int ry = r2 + K + GAP;
			if (mouse) {
				int mw = (width() - GAP) / 2;
				key(g, x, ry, mw, K, cps ? "LMB\n" + Trackers.leftCps() : "LMB", o.keyAttack.isDown());
				key(g, x + mw + GAP, ry, mw, K, cps ? "RMB\n" + Trackers.rightCps() : "RMB", o.keyUse.isDown());
				ry += K + GAP;
			}
			if (space) {
				boolean down = o.keyJump.isDown();
				g.fill(x, ry, x + width(), ry + 10, down ? 0xCCFFFFFF : style.backgroundColor);
				int bar = x + width() / 2;
				g.fill(bar - 10, ry + 4, bar + 10, ry + 5, down ? 0xFF000000 : style.textColor);
			}
		}
	}

	/** Armor pieces + held item with durability. */
	static final class Armor implements Widget {
		final List<ItemStack> items = new ArrayList<>();
		final Minecraft mc;
		final HudConfig.Global style;
		final boolean vertical, durability;

		Armor(Minecraft mc, HudConfig.Global style, boolean vertical, boolean durability) {
			this.mc = mc;
			this.style = style;
			this.vertical = vertical;
			this.durability = durability;
			if (mc.player != null) {
				for (EquipmentSlot slot : new EquipmentSlot[]{EquipmentSlot.HEAD, EquipmentSlot.CHEST, EquipmentSlot.LEGS, EquipmentSlot.FEET, EquipmentSlot.MAINHAND}) {
					ItemStack s = mc.player.getItemBySlot(slot);
					if (!s.isEmpty()) items.add(s);
				}
			}
		}

		String label(ItemStack s) {
			if (!durability) return "";
			if (s.isDamageableItem()) return String.valueOf(s.getMaxDamage() - s.getDamageValue());
			return s.getCount() > 1 ? String.valueOf(s.getCount()) : "";
		}

		int cell() {
			int w = 0;
			for (ItemStack s : items) w = Math.max(w, mc.font.width(label(s)));
			return 16 + (w > 0 ? 3 + w : 0);
		}

		public int width() {
			return vertical ? cell() : Math.max(1, items.size()) * (cell() + 4) - 4;
		}

		public int height() {
			return vertical ? Math.max(1, items.size()) * 17 - 1 : 16;
		}

		public void draw(GuiGraphics g, int x, int y) {
			int cx = x, cy = y;
			for (ItemStack s : items) {
				g.renderItem(s, cx, cy);
				String l = label(s);
				if (!l.isEmpty()) {
					int color = style.textColor;
					if (s.isDamageableItem() && s.getMaxDamage() > 0) {
						float frac = 1f - (float) s.getDamageValue() / s.getMaxDamage();
						color = frac > 0.5f ? style.textColor : frac > 0.2f ? 0xFFFFC860 : 0xFFFF6B6B;
					}
					g.drawString(mc.font, l, cx + 19, cy + 4, color, style.textShadow);
				}
				if (vertical) cy += 17;
				else cx += cell() + 4;
			}
		}
	}
}
