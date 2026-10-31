package gg.salt.hud;

import com.google.gson.Gson;
import com.google.gson.GsonBuilder;
import com.google.gson.JsonArray;
import com.google.gson.JsonElement;
import com.google.gson.JsonObject;
import com.google.gson.JsonParser;
import net.fabricmc.loader.api.FabricLoader;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

/**
 * config/salthud.json — the same file the Salt Launcher HUD manager writes.
 * Positions are screen fractions (0–1) measured from {@link Element#anchor},
 * so a layout looks the same at any resolution or GUI scale.
 */
public final class HudConfig {
	private static final Gson GSON = new GsonBuilder().setPrettyPrinting().create();

	public static final class Global {
		public boolean textShadow = true;
		public boolean background = true;
		public int backgroundColor = 0x66000000;
		public int textColor = 0xFFFFFFFF;
		public int accentColor = 0xFF7CC8FF;
		public float scale = 1f;
		public int padding = 4;
	}

	public static final class Element {
		public String id;
		public boolean enabled;
		public float x, y;
		public String anchor = "top_left";
		public float scale = 1f;
		public JsonObject options = new JsonObject();

		public boolean opt(String key, boolean def) {
			JsonElement e = options.get(key);
			return e != null && e.isJsonPrimitive() ? e.getAsBoolean() : def;
		}

		public String opt(String key, String def) {
			JsonElement e = options.get(key);
			return e != null && e.isJsonPrimitive() ? e.getAsString() : def;
		}

		public int optInt(String key, int def) {
			JsonElement e = options.get(key);
			try {
				return e != null && e.isJsonPrimitive() ? e.getAsInt() : def;
			} catch (NumberFormatException ex) {
				return def;
			}
		}
	}

	public final Global global = new Global();
	public final List<Element> elements = new ArrayList<>();
	private JsonObject raw = new JsonObject();

	private static Path path() {
		return FabricLoader.getInstance().getConfigDir().resolve("salthud.json");
	}

	public static HudConfig load() {
		HudConfig c = new HudConfig();
		try {
			Path p = path();
			if (Files.exists(p)) {
				c.read(JsonParser.parseString(Files.readString(p, StandardCharsets.UTF_8)).getAsJsonObject());
				return c;
			}
		} catch (Exception e) {
			SaltHud.LOGGER.warn("saltHUD: couldn't read salthud.json, using defaults", e);
		}
		c.read(Defaults.json());
		return c;
	}

	private void read(JsonObject o) {
		raw = o;
		JsonObject g = o.has("global") ? o.getAsJsonObject("global") : new JsonObject();
		global.textShadow = bool(g, "textShadow", true);
		global.background = bool(g, "background", true);
		global.backgroundColor = color(g, "backgroundColor", 0x66000000);
		global.textColor = color(g, "textColor", 0xFFFFFFFF);
		global.accentColor = color(g, "accentColor", 0xFF7CC8FF);
		global.scale = g.has("scale") ? g.get("scale").getAsFloat() : 1f;
		global.padding = g.has("padding") ? g.get("padding").getAsInt() : 4;
		elements.clear();
		JsonArray arr = o.has("elements") ? o.getAsJsonArray("elements") : new JsonArray();
		for (JsonElement je : arr) {
			JsonObject e = je.getAsJsonObject();
			Element el = new Element();
			el.id = e.get("id").getAsString();
			el.enabled = bool(e, "enabled", false);
			el.x = e.has("x") ? e.get("x").getAsFloat() : 0f;
			el.y = e.has("y") ? e.get("y").getAsFloat() : 0f;
			el.anchor = e.has("anchor") ? e.get("anchor").getAsString() : "top_left";
			el.scale = e.has("scale") ? e.get("scale").getAsFloat() : 1f;
			el.options = e.has("options") ? e.getAsJsonObject("options") : new JsonObject();
			elements.add(el);
		}
	}

	/** Writes positions back (after editing in game) without dropping unknown keys. */
	public void save() {
		try {
			JsonArray arr = raw.has("elements") ? raw.getAsJsonArray("elements") : new JsonArray();
			for (Element el : elements) {
				for (JsonElement je : arr) {
					JsonObject e = je.getAsJsonObject();
					if (e.get("id").getAsString().equals(el.id)) {
						e.addProperty("enabled", el.enabled);
						e.addProperty("x", round(el.x));
						e.addProperty("y", round(el.y));
						e.addProperty("anchor", el.anchor);
						e.addProperty("scale", round(el.scale));
					}
				}
			}
			raw.add("elements", arr);
			// The launcher compares this to its own copy and keeps the newer one.
			raw.addProperty("modified", System.currentTimeMillis());
			Files.createDirectories(path().getParent());
			Files.writeString(path(), GSON.toJson(raw), StandardCharsets.UTF_8);
		} catch (Exception e) {
			SaltHud.LOGGER.warn("saltHUD: couldn't save salthud.json", e);
		}
	}

	private static float round(float f) {
		return Math.round(f * 10000f) / 10000f;
	}

	private static boolean bool(JsonObject o, String k, boolean def) {
		return o.has(k) ? o.get(k).getAsBoolean() : def;
	}

	/** "#AARRGGBB" or "#RRGGBB" → ARGB int. */
	static int color(JsonObject o, String k, int def) {
		if (!o.has(k)) return def;
		return parseColor(o.get(k).getAsString(), def);
	}

	static int parseColor(String s, int def) {
		try {
			String h = s.startsWith("#") ? s.substring(1) : s;
			if (h.length() == 6) h = "FF" + h;
			return (int) Long.parseLong(h, 16);
		} catch (Exception e) {
			return def;
		}
	}
}
